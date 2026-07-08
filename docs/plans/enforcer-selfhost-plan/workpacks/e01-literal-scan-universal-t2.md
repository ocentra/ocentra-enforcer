# e01 Literal-Scan Universal T2 Layer

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Literal-Scan Universal T2 Layer`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-literal-scan/src/bridge.rs`, `crates/enforcer-literal-scan/tests/fixtures/universal/**`, `crates/enforcer-literal-scan/tests/bridge.rs`
- deps: `arc-13`, `d01`
- tier: `P1 / T2`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
`arc-13` folds the existing Rust scored literal-risk scanner (covering ~65 languages, emitting `score`+`confidence`) into `crates/enforcer-literal-scan` and exposes it through the `enforcer-validator`/scan interfaces. But it is still only a per-family crate consumed on demand — it is **not** wired as an always-on standing layer over every scan target. Language-specific rule families (`enforcer-lang-py`/`-rust`/`-ts`, and the new Dart/CFML/frontend packs) leave every other language with **zero mechanical coverage**. ADBP_GAPS treats the Rust literal-scan model as the reference for the T2 rung (scored, non-blocking). The crate's language registry currently **lacks Dart and CFML** entries.

## Where We Want To Be
A `bridge` module inside `enforcer-literal-scan` wires the folded scanner into the engine as the **always-on universal T2 advisory layer**: it runs on every scan target regardless of language, emits per-finding `score`+`confidence` as `enforcer-domain` `Finding`s tagged T2/advisory, and is **non-blocking** (it never fails a gate on its own; it feeds the report and the scored proof). This is what gives every one of the ~65 languages a baseline mechanical floor even where no bespoke `Validator` family exists. The bridge is pure in-process Rust (no shell-out to the folded scorer — it is compiled into the same crate). This pack also **adds Dart and CFML** to the literal-scan language registry so the two new-language packs share the universal floor.

## Requirement Checklist
- [ ] `bridge.rs` exposes a `Validator` impl (from `enforcer-validator`, arc-05) that runs the folded scorer over any target and maps each `score`+`confidence` hit into an `enforcer-domain` `Finding` tagged `Tier::T2` / advisory (non-blocking severity).
- [ ] Layer is always-on: the bridge validator runs for every scan target, independent of which bespoke lang family (if any) matched the file's language — registered so the scan engine (arc-15) invokes it unconditionally.
- [ ] Non-blocking: a literal-scan `Finding` raises the report score but never sets a fatal/exit-nonzero gate on its own (advisory severity only; no `Violation` promotion).
- [ ] Dart and CFML are added to the literal-scan language registry (`.dart`, `.cfc`, `.cfm` recognized and scored) — additive registry rows only.
- [ ] Scaffolded via d01 so the bridge rule id carries doc + `{fail,pass}` fixtures + a `cargo test` detection test in 5-way parity.
- [ ] Obeys `[workspace.lints]` (no `unwrap/expect/panic/print_*`); clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier T2 (scored/advisory) per doctrine — fixtures test the **score threshold**, not a hard block (the Rust literal-scan model). Fail-fixture: a high-literal-risk source file (e.g. a Dart or CFML file dense with hardcoded literals/secrets-shaped strings) whose literal-scan `score` must **cross** the configured threshold and be reported. Pass-fixture: a clean equivalent whose `score` must **stay under** threshold. Detection test `crates/enforcer-literal-scan/tests/bridge.rs` (`cargo test -p enforcer-literal-scan`) asserts: (1) fail fixture crosses threshold and yields a T2 advisory `Finding`, (2) pass fixture stays under and yields none, (3) an advisory `Finding` never promotes to a blocking `Violation` / never sets a nonzero exit on its own, (4) `.dart`/`.cfc`/`.cfm` targets are recognized by the registry. Named proof rows in TEST_PROOF_EXPECTATIONS.md: `literal-scan-universal-threshold` and `literal-scan-advisory-nonblocking`.

## Parallel Ownership Notes
`owns:` is the new `bridge.rs` module + its fixtures/detection test only — disjoint from all sibling packs and from the arc-13 crate skeleton (arc-13 owns `Cargo.toml`/`lib.rs`/the folded scorer; this pack owns only the bridge module file, its fixtures, and its test, and `deps: arc-13` so it sequences after the crate exists). It does NOT own the folded scorer internals except the additive language-registry rows for Dart/CFML (coordinate the registry addition so e-pack-dart / e-pack-cfml can rely on the universal floor). z01 consumes this layer during the terminal dogfood run; e-pack-dart and e-pack-cfml assume this floor exists but do not depend on this file's completion to author their bespoke rules.
