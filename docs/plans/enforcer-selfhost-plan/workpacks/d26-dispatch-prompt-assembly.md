# d26 Dispatch Prompt Assembly

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Dispatch Prompt Assembly`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-coordination/src/dispatch/quality_blocks.rs, crates/enforcer-coordination/src/dispatch/assemble_prompt.rs, crates/enforcer-coordination/tests/fixtures/dispatch/**`
- deps: `arc-16-enforcer-coordination, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine`
- tier: `P1 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
ADBP's `ergonomics/quality-blocks` (rows DISP-1.1..1.5 in [ADBP_GAPS](../ADBP_GAPS.md#group-3--command--ergonomics-gates)) mandates that every implementation sub-agent prompt is assembled from single-sourced blocks pasted verbatim, in a fixed order. The `enforcer-coordination` crate has the hub/lane/dispatch machinery but nothing that governs the assembly of dispatched sub-agent prompts. A dispatch prompt can silently drop the SECURITY STOP reflex, reorder the git-boundary block, or omit the self-verify checklist.

## Where We Want To Be
A `dispatch` submodule in `enforcer-coordination` (arc-16) — a single canonical source of the four blocks as Rust string constants in `crates/enforcer-coordination/src/dispatch/quality_blocks.rs`, an assembler in `crates/enforcer-coordination/src/dispatch/assemble_prompt.rs`, and a `Validator` (from `enforcer-validator`, arc-05) — keyed to typed rule records in `enforcer-rules` (arc-04), scaffolded through d01 — that asserts any produced dispatch prompt CONTAINS each required block verbatim and in the mandated order:
- **Block 1 — SECURITY STOP** (STOP-on-vulnerability CWE-watchlist reflex) present (also referenced by d18 row SEC-STOP-GATE / DISP-1.1).
- **Block 2 — iteration discipline** (anti-thrash) present.
- **Block 3 — per-stack self-verify checklist** present, positioned at the END.
- **Block 4 — git boundary** DEAD LAST.
- **Fix-track addendum** — for fix phases, the zero-match addendum is appended.
The validator is a snapshot/contains check: verbatim substring match against the single-sourced block constants, plus an order assertion, emitting `Finding`s.

## Requirement Checklist
- [ ] Blocks defined once as `const` strings in `src/dispatch/quality_blocks.rs` and pasted verbatim by the assembler (no per-call paraphrase).
- [ ] Validator asserts Block 1 (SECURITY STOP) present verbatim (DISP-1.1).
- [ ] Validator asserts Block 2 (iteration discipline) present verbatim (DISP-1.2).
- [ ] Validator asserts Block 3 (self-verify checklist) present AND is the last block before the git boundary (DISP-1.3).
- [ ] Validator asserts Block 4 (git boundary) is DEAD LAST (DISP-1.4).
- [ ] Validator asserts the fix-track zero-match addendum is present for fix-phase prompts (DISP-1.5).
- [ ] Missing block OR out-of-order blocks fail closed; the `Finding` names the offending block/position.
- [ ] Assembled prompts do not contain orchestrator-behavior/evaluator text (complements d25 ORCH-1.10).
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier T1, P1, Rust-native. Deterministic: verbatim `contains` + ordinal-position assertions over the assembled prompt `String` against the single-sourced block constants (the snapshot model). Select detection tests in TEST_PROOF_EXPECTATIONS.md before DONE.

Per-rule 5-way parity (ruleId <-> rule-record <-> validator <-> {fail,pass} <-> `cargo test`):
- **DISP-1.1 (SECURITY STOP present):** fail-fixture `crates/enforcer-coordination/tests/fixtures/dispatch/disp_1_1/bad.txt` (assembled prompt with Block 1 removed) flagged; pass-fixture `.../dispatch/all_blocks/good.txt` clean.
- **DISP-1.2 (iteration discipline present):** fail-fixture `.../dispatch/disp_1_2/bad.txt`; pass-fixture `.../dispatch/all_blocks/good.txt`.
- **DISP-1.3 (self-verify checklist at end):** fail-fixture `.../dispatch/disp_1_3/bad.txt` (checklist appears before Block 2); pass-fixture `.../dispatch/all_blocks/good.txt`.
- **DISP-1.4 (git boundary dead last):** fail-fixture `.../dispatch/disp_1_4/bad.txt` (git block followed by other content); pass-fixture `.../dispatch/all_blocks/good.txt`.
- **DISP-1.5 (fix-track addendum):** fail-fixture `.../dispatch/disp_1_5/bad.txt` (fix-phase prompt lacking the zero-match addendum); pass-fixture `.../dispatch/disp_1_5/good.txt`.
- Detection test for all five: `#[test]` under `#[cfg(test)]` in the `dispatch` module, proven by `cargo test -p enforcer-coordination`.

## Parallel Ownership Notes
Depends on `arc-16` (which stands up the `enforcer-coordination` crate skeleton + module root + `Validator` registration), `arc-05` (the `Validator` trait + parity harness), `arc-04` (the rule registry), and `d01` (validator scaffold/parity conventions). Owns a disjoint `src/dispatch/{quality_blocks,assemble_prompt}.rs` + `tests/fixtures/dispatch/**` inside the arc-16 crate, NOT the crate itself; sequenced after arc-16's skeleton exists. Also disjoint from d27's `loop_resilience.rs` module in the same crate. Shares the sub-agent-prompt concept with d25's ORCH-1.10 but from the generator side and against non-overlapping files (d25 owns `crates/enforcer-plan/src/verify_gates.rs`); no file overlap, concurrent-safe.
