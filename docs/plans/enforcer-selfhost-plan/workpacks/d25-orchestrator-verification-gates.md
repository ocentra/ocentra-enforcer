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

- owns: `src/orchestrator/verify-gate-a.ts, src/orchestrator/verify-gate-b.ts, src/orchestrator/verify-gate-c.ts, src/orchestrator/verify-index.ts, tests/orchestrator-verify-gate-a.test.mjs, tests/orchestrator-verify-gate-b.test.mjs, tests/orchestrator-verify-gate-c.test.mjs`
- deps: `d01-rule-mechanization-engine`
- tier: `P1 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
ADBP's `ergonomics/orchestrator-verification` defines a trust-but-verify contract (Gates A/B/C, rows ORCH-1.1..1.10 in [ADBP_GAPS](../ADBP_GAPS.md#group-3--command--ergonomics-gates)). The enforcer has no analog: nothing re-derives a sub-agent's self-reported counts, nothing checks that staging came from `git status --porcelain` rather than `git diff`, and nothing detects a sub-agent commit made ahead of the base branch. A phase can go green on a fabricated "789 passed" line, and untracked source modules can slip past CI (the untracked-module-reds-CI trap).

## Where We Want To Be
Three deterministic gates over git state and orchestrator artifacts, each fail-closed:
- **Gate A (re-derive counts):** distrust the sub-agent self-report; the gate re-runs the tooling and compares the captured summary line against `git diff --stat`. "already implemented / nothing to do" is treated as UNVERIFIED unless a re-run artifact exists. A report-vs-re-run discrepancy fails.
- **Gate B (staging source of truth):** staging must derive from `git status --porcelain` (which includes untracked source), not from `git diff --name-only`; a committed-tree import/collection smoke must be recorded before green.
- **Gate C (commit-boundary):** detect commits ahead of base; require reconciliation via `git reset --soft <base>` (never a rewrite of pushed history), with the reconciliation stated in the report.
- The intro constraint (ORCH-1.10): the orchestrator-behavior file must not be pasted into sub-agent prompts (evaluator instructions must not leak into the generator). This overlaps d26's prompt surface but is asserted here as an orchestrator-side check.
- The T3 residue (row 153: recurring-pattern-graduation heuristic, no-invented-findings, heal-must-not-edit-source) lands as a labeled advisory only.

## Requirement Checklist
- [ ] Gate A re-derives every count itself; a self-reported number with no captured re-run artifact fails (ORCH-1.1/1.2/1.3).
- [ ] Gate A treats "nothing to do" as UNVERIFIED against `git diff --stat` and fails on report-vs-re-run discrepancy.
- [ ] Gate B derives the staged set from `git status --porcelain`; a `git diff`-based staging that drops untracked source fails (ORCH-1.4/1.5/1.6).
- [ ] Gate B records a committed-tree import/collection smoke before allowing green.
- [ ] Gate C detects commits ahead of base and requires a documented soft-reset reconcile; unreconciled commits-ahead fails (ORCH-1.7/1.8/1.9).
- [ ] ORCH-1.10: orchestrator-behavior text appearing in a dispatched sub-agent prompt fails.
- [ ] Each gate is deterministic over git state; failure names the specific gate, count, or path.
- [ ] Row-153 graduation/heal-discipline residue carried as `advisory, no mechanization possible + agent-runtime behavior, no git-observable artifact`.

## Acceptance And Proof
Tier T1, P1. All checks are deterministic over `git status --porcelain`, `git diff --stat`, `git rev-list <base>..HEAD`, and recorded re-run summaries. Select detection tests in TEST_PROOF_EXPECTATIONS.md before DONE.

Per-rule 5-way parity (ruleId <-> doc <-> validator <-> {fail,pass} <-> test):
- **ORCH-1.1..1.3 (Gate A re-derive):** fail-fixture `tests/fixtures/orchestrator/gate-a/fail-selfreport-no-rerun/` (phase report cites "789 passed" with no captured tooling-summary re-run artifact) must be flagged; pass-fixture `.../pass-rerun-captured/` (report embeds an orchestrator re-run summary matching `git diff --stat`) must stay clean. Test: `tests/orchestrator-verify-gate-a.test.mjs`.
- **ORCH-1.4..1.6 (Gate B porcelain staging):** fail-fixture `tests/fixtures/orchestrator/gate-b/fail-diff-staging-untracked/` (staging derived from `git diff --name-only` while an untracked source module exists) must be flagged; pass-fixture `.../pass-porcelain-smoke/` (staging from `git status --porcelain` + committed-tree import smoke recorded) must stay clean. Test: `tests/orchestrator-verify-gate-b.test.mjs`.
- **ORCH-1.7..1.9 (Gate C commit-boundary):** fail-fixture `tests/fixtures/orchestrator/gate-c/fail-commits-ahead/` (`git rev-list <base>..HEAD` non-empty, no reconciliation record) must be flagged; pass-fixture `.../pass-soft-reset-reconciled/` (zero commits ahead, or a documented soft-reset reconcile) must stay clean. Test: `tests/orchestrator-verify-gate-c.test.mjs`.
- **ORCH-1.10 (behavior-file isolation):** fail-fixture `tests/fixtures/orchestrator/orch-1.10/fail-behavior-in-prompt/` (dispatched prompt contains orchestrator-gate text); pass-fixture `.../pass-quality-blocks-only/`. Asserted in `tests/orchestrator-verify-gate-a.test.mjs` (index gate).

## Parallel Ownership Notes
Depends on d01 for the validator harness/scaffold conventions only. Owns a disjoint `src/orchestrator/verify-*` tree and its tests. ORCH-1.10 touches the same conceptual surface as d26 (dispatch prompts) but from the orchestrator/evaluator side and against disjoint files — d26 owns `src/dispatch/*`, this pack owns `src/orchestrator/*`; no file overlap, safe to run concurrently.
