# d27 Loop Resilience And Telemetry

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Loop Resilience And Telemetry`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-coordination/src/loop_resilience.rs, crates/enforcer-core/src/context_meter.rs, crates/enforcer-coordination/tests/fixtures/loop_resilience/**`
- deps: `arc-16-enforcer-coordination, arc-01-enforcer-core, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine, d04-run-telemetry-ndjson`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
ADBP's `ergonomics/loop-resilience` (rows LOOP-1.1..1.5 in [ADBP_GAPS](../ADBP_GAPS.md#group-3--command--ergonomics-gates)) requires the self-improving loop to survive compaction and context exhaustion: a PreCompact breadcrumb, a context-fill meter, a `.harness/` re-hydration guard, and consent-gated per-project install. The engine has none of these in Rust. The deterministic per-run telemetry RECORD itself is owned by existing **d04-run-telemetry-ndjson** (folded into `enforcer-core`/`enforcer-domain`) and the always-on context ceiling is owned by existing **d05-context-budget-brake**; this pack is the loop-resilience half and REFERENCES those, it does not duplicate them.

## Where We Want To Be
A `loop_resilience` module in `enforcer-coordination` (arc-16) plus a `context_meter` writer in `enforcer-core` (arc-01). The harness-side breadcrumb/meter artifacts are EMITTED by these Rust modules (the pack owns the Rust emitter, not a shell hook); the enforcer installer (arc-23) is what wires a target harness's PreCompact/statusline hook to invoke the binary. Both emit through the core append-only sink, and each obligation is a `Validator` (from `enforcer-validator`, arc-05) keyed to a typed rule record in `enforcer-rules` (arc-04), scaffolded through d01:
- `enforcer-core` `context_meter.rs` writes `.harness/context-meter.json` (per-tier + total token breakdown), serialized from a versioned serde record in `enforcer-domain` — LOOP-1.2. Distinct from d04's telemetry-record NDJSON writer.
- `loop_resilience.rs` drops `.harness/compaction-pending` on PreCompact, NON-BLOCKING (never fails compaction; write failure is swallowed, returns Ok) — LOOP-1.1.
- Both emitters are inert unless `.harness/` exists (re-hydration guard) — LOOP-1.3.
- Per-iteration re-hydrate from `state.json` — LOOP-1.5 (T3-labeled: agent-runtime behavior).
- Install is consent-gated via the installer's `/init-component` path, never an unattended `deploy` — LOOP-1.4.
- Telemetry linkage: the per-run RECORD (TEL-1.1..1.5) is proven by **d04** — this pack only asserts the resilience emitters feed the same `.harness`/`.harness-archive` surface; the context-ceiling (CTX-1.1..1.4) is proven by **d05**. No duplicate ruleIds authored here.

## Requirement Checklist
- [ ] `loop_resilience.rs` writes `.harness/compaction-pending` on PreCompact and returns Ok even on write failure (non-blocking) — LOOP-1.1.
- [ ] `context_meter.rs` emits `.harness/context-meter.json` with per-tier breakdown + total (versioned serde record) — LOOP-1.2.
- [ ] Both emitters no-op (return Ok, write nothing) when `.harness/` is absent — LOOP-1.3 guard.
- [ ] Consent-gated install path (installer `/init-component`), and a `Validator` check that an unattended `deploy` path does NOT install these emitters — LOOP-1.4.
- [ ] Re-hydration reads `state.json` per iteration — LOOP-1.5, carried on the rule record as `advisory, no mechanization possible + per-iteration agent-runtime behavior, only the guard file is observable`.
- [ ] Validators are deterministic over emitter output files and `.harness/` presence; no duplication of d04 (telemetry record) or d05 (context ceiling).
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier P1, Rust-native (`Validator` impls + fail/pass fixtures + `cargo test` detection). LOOP-1.1/1.2/1.3/1.4 are mechanizable (T1/T2) over emitter-produced files and install manifests; LOOP-1.5 is T3-labeled. Select detection tests in TEST_PROOF_EXPECTATIONS.md before DONE.

Per-rule 5-way parity (ruleId <-> rule-record <-> validator <-> {fail,pass} <-> `cargo test`):
- **LOOP-1.1 (non-blocking PreCompact breadcrumb):** fail-fixture `crates/enforcer-coordination/tests/fixtures/loop_resilience/loop_1_1/bad/` (emitter missing, or one that errors out on write failure) flagged; pass-fixture `.../loop_1_1/good/` (drops `compaction-pending`, returns Ok regardless) clean.
- **LOOP-1.2 (context meter):** fail-fixture `.../loop_resilience/loop_1_2/bad/` (`context-meter.json` missing per-tier breakdown or total); pass-fixture `.../loop_1_2/good/`.
- **LOOP-1.3 (`.harness/` guard):** fail-fixture `.../loop_resilience/loop_1_3/bad/` (emitter writes even when `.harness/` absent); pass-fixture `.../loop_1_3/good/`.
- **LOOP-1.4 (consent-gated install):** fail-fixture `.../loop_resilience/loop_1_4/bad/` (unattended `deploy` installs the emitters); pass-fixture `.../loop_1_4/good/`.
- **LOOP-1.5:** advisory, no mechanization possible + per-iteration agent-runtime behavior; only the presence of the `state.json` hydration guard file is asserted (label presence is T1).
- Detection tests: `#[cfg(test)]` `#[test]`s in `loop_resilience.rs` (LOOP-1.1/1.3/1.4) and `context_meter.rs` (LOOP-1.2), proven by `cargo test -p enforcer-coordination` and `cargo test -p enforcer-core`.

## Parallel Ownership Notes
Depends on `arc-16` (coordination crate skeleton + module root + `Validator` registration), `arc-01` (core crate skeleton + append-only sink), `arc-05` (the `Validator` trait), `arc-04` (the rule registry), `d01` (scaffold/parity), and `d04` (the telemetry record this feeds — referenced, not re-implemented). Owns a disjoint `loop_resilience.rs` (in arc-16) + `context_meter.rs` (in arc-01) + `tests/fixtures/loop_resilience/**` — specific modules inside those crates, NOT the crates. `context_meter.rs` is distinct from d04's telemetry-record writer and d05's context-ceiling validator — those stay in their home packs; this pack only writes/reads/asserts the shared `.harness` surface. Also disjoint from d26's `dispatch/*` modules in the coordination crate. Sequenced after arc-16/arc-01 skeletons exist; concurrent-safe with d04/d05/d26 (disjoint files).
