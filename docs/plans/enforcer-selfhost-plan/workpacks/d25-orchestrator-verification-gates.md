# d25 Orchestrator Verification Gates

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Orchestrator Verification Gates`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-plan/src/verify_gates.rs, crates/enforcer-plan/tests/fixtures/verify_gates/**`
- deps: `arc-20-enforcer-plan, arc-05-enforcer-validator, arc-04-enforcer-rules, arc-18-enforcer-harness, d01-rule-mechanization-engine`
- tier: `P1 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
ADBP's `ergonomics/orchestrator-verification` defines a trust-but-verify contract (Gates A/B/C, rows ORCH-1.1..1.10 in [ADBP_GAPS](../ADBP_GAPS.md#group-3--command--ergonomics-gates)). The `enforcer-plan` crate has no analog: nothing re-derives a sub-agent's self-reported counts, nothing checks that staging came from `git status --porcelain` rather than `git diff`, and nothing detects a sub-agent commit made ahead of the base branch. A phase can go green on a fabricated "789 passed" line, and untracked source modules can slip past CI (the untracked-module-reds-CI trap).

## Where We Want To Be
A `verify_gates` module in `enforcer-plan` (arc-20) — a `crates/enforcer-plan/src/verify_gates.rs` implementing three deterministic gates as `Validator` impls (from `enforcer-validator`, arc-05) that emit structured `Finding`s, each keyed to a typed rule record in `enforcer-rules` (arc-04) and scaffolded through d01. Git-state reads (`git status --porcelain`, `git diff --stat`, `git rev-list <base>..HEAD`) run through the `enforcer-harness` (arc-18) run-adapters, not ad-hoc shell-outs. Each gate is fail-closed:
- **Gate A (re-derive counts):** distrust the sub-agent self-report; the gate re-runs the tooling (via the harness) and compares the captured summary line against `git diff --stat`. "already implemented / nothing to do" is treated as UNVERIFIED unless a re-run artifact exists. A report-vs-re-run discrepancy fails.
- **Gate B (staging source of truth):** staging must derive from `git status --porcelain` (which includes untracked source), not from `git diff --name-only`; a committed-tree import/collection smoke must be recorded before green.
- **Gate C (commit-boundary):** detect commits ahead of base; require reconciliation via `git reset --soft <base>` (never a rewrite of pushed history), with the reconciliation stated in the report.
- The intro constraint (ORCH-1.10): the orchestrator-behavior file must not be pasted into sub-agent prompts (evaluator instructions must not leak into the generator). This overlaps d26's prompt surface but is asserted here as an orchestrator-side check.
- The T3 residue (row 153: recurring-pattern-graduation heuristic, no-invented-findings, heal-must-not-edit-source) lands as a labeled advisory only on its `enforcer-rules` record.

## Requirement Checklist
- [ ] Gate A re-derives every count itself (harness re-run); a self-reported number with no captured re-run artifact fails (ORCH-1.1/1.2/1.3).
- [ ] Gate A treats "nothing to do" as UNVERIFIED against `git diff --stat` and fails on report-vs-re-run discrepancy.
- [ ] Gate B derives the staged set from `git status --porcelain`; a `git diff`-based staging that drops untracked source fails (ORCH-1.4/1.5/1.6).
- [ ] Gate B records a committed-tree import/collection smoke before allowing green.
- [ ] Gate C detects commits ahead of base and requires a documented soft-reset reconcile; unreconciled commits-ahead fails (ORCH-1.7/1.8/1.9).
- [ ] ORCH-1.10: orchestrator-behavior text appearing in a dispatched sub-agent prompt fails.
- [ ] Each gate is deterministic over git state; the `Finding` names the specific gate, count, or path.
- [ ] Row-153 graduation/heal-discipline residue carried on the rule record as `advisory, no mechanization possible + agent-runtime behavior, no git-observable artifact`.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier T1, P1, Rust-native (`Validator` impls + fail/pass fixtures + `cargo test` detection). All checks are deterministic over `git status --porcelain`, `git diff --stat`, `git rev-list <base>..HEAD`, and recorded re-run summaries, read through the arc-18 harness. Select detection tests in TEST_PROOF_EXPECTATIONS.md before DONE.

Per-rule 5-way parity (ruleId <-> rule-record <-> validator <-> {fail,pass} <-> `cargo test`):
- **ORCH-1.1..1.3 (Gate A re-derive):** fail-fixture `crates/enforcer-plan/tests/fixtures/verify_gates/gate_a/bad/` (phase report cites "789 passed" with no captured tooling-summary re-run artifact) must be flagged; pass-fixture `.../gate_a/good/` (report embeds an orchestrator re-run summary matching `git diff --stat`) must stay clean.
- **ORCH-1.4..1.6 (Gate B porcelain staging):** fail-fixture `.../verify_gates/gate_b/bad/` (staging derived from `git diff --name-only` while an untracked source module exists) must be flagged; pass-fixture `.../gate_b/good/` (staging from `git status --porcelain` + committed-tree import smoke recorded) must stay clean.
- **ORCH-1.7..1.9 (Gate C commit-boundary):** fail-fixture `.../verify_gates/gate_c/bad/` (`git rev-list <base>..HEAD` non-empty, no reconciliation record) must be flagged; pass-fixture `.../gate_c/good/` (zero commits ahead, or a documented soft-reset reconcile) must stay clean.
- **ORCH-1.10 (behavior-file isolation):** fail-fixture `.../verify_gates/orch_1_10/bad/` (dispatched prompt contains orchestrator-gate text); pass-fixture `.../orch_1_10/good/`.
- Detection test for all rows: `#[test]` under `#[cfg(test)]` in `verify_gates.rs`, proven by `cargo test -p enforcer-plan`.

## Parallel Ownership Notes
Depends on `arc-20` (which stands up the `enforcer-plan` crate skeleton + module root + `Validator` registration), `arc-05` (the `Validator` trait + parity harness), `arc-04` (the rule registry), `arc-18` (the harness run-adapters the git reads go through), and `d01` (validator scaffold/parity conventions). Owns a disjoint `verify_gates` module + its fixtures inside the arc-20 crate, NOT the crate itself; sequenced after arc-20's skeleton exists. ORCH-1.10 touches the same conceptual surface as d26 (dispatch prompts) but from the orchestrator/evaluator side and against disjoint files — d26 owns `crates/enforcer-coordination/src/dispatch/*`, this pack owns `crates/enforcer-plan/src/verify_gates.rs`; no file overlap, safe to run concurrently.
