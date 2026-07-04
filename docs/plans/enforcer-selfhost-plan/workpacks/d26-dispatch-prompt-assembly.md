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

- owns: `src/dispatch/quality-blocks.ts, src/dispatch/quality-blocks.md, src/dispatch/assemble-prompt.ts, tests/dispatch-quality-blocks.test.mjs, tests/fixtures/dispatch/**`
- deps: `d01-rule-mechanization-engine`
- tier: `P1 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
ADBP's `ergonomics/quality-blocks` (rows DISP-1.1..1.5 in [ADBP_GAPS](../ADBP_GAPS.md#group-3--command--ergonomics-gates)) mandates that every implementation sub-agent prompt is assembled from single-sourced blocks pasted verbatim, in a fixed order. The enforcer has secret/size families for its own source but nothing that governs the assembly of dispatched sub-agent prompts. A dispatch prompt can silently drop the SECURITY STOP reflex, reorder the git-boundary block, or omit the self-verify checklist.

## Where We Want To Be
A single canonical source (`src/dispatch/quality-blocks.md` -> `quality-blocks.ts`) for the four blocks, plus an assembler and a validator that asserts any produced dispatch prompt CONTAINS each required block verbatim and in the mandated order:
- **Block 1 — SECURITY STOP** (STOP-on-vulnerability CWE-watchlist reflex) present (also referenced by d18 row SEC-STOP-GATE / DISP-1.1).
- **Block 2 — iteration discipline** (anti-thrash) present.
- **Block 3 — per-stack self-verify checklist** present, positioned at the END.
- **Block 4 — git boundary** DEAD LAST.
- **Fix-track addendum** — for fix phases, the zero-match addendum is appended.
The validator is a snapshot/contains check: verbatim substring match against the single-sourced blocks, plus an order assertion.

## Requirement Checklist
- [ ] Blocks defined once in `src/dispatch/quality-blocks.*` and pasted verbatim (no per-call paraphrase).
- [ ] Validator asserts Block 1 (SECURITY STOP) present verbatim (DISP-1.1).
- [ ] Validator asserts Block 2 (iteration discipline) present verbatim (DISP-1.2).
- [ ] Validator asserts Block 3 (self-verify checklist) present AND is the last block before the git boundary (DISP-1.3).
- [ ] Validator asserts Block 4 (git boundary) is DEAD LAST (DISP-1.4).
- [ ] Validator asserts the fix-track zero-match addendum is present for fix-phase prompts (DISP-1.5).
- [ ] Missing block OR out-of-order blocks fail closed; failure names the offending block/position.
- [ ] Assembled prompts do not contain orchestrator-behavior/evaluator text (complements d25 ORCH-1.10).

## Acceptance And Proof
Tier T1, P1. Deterministic: verbatim `contains` + ordinal-position assertions over the assembled prompt string against the single-sourced block constants (the snapshot model). Select detection tests in TEST_PROOF_EXPECTATIONS.md before DONE.

Per-rule 5-way parity (ruleId <-> doc <-> validator <-> {fail,pass} <-> test):
- **DISP-1.1 (SECURITY STOP present):** fail-fixture `tests/fixtures/dispatch/disp-1.1/fail-missing-security-block.txt` (assembled prompt with Block 1 removed) flagged; pass-fixture `.../pass-all-blocks.txt` clean.
- **DISP-1.2 (iteration discipline present):** fail-fixture `.../disp-1.2/fail-missing-iteration.txt`; pass-fixture `.../pass-all-blocks.txt`.
- **DISP-1.3 (self-verify checklist at end):** fail-fixture `.../disp-1.3/fail-checklist-misplaced.txt` (checklist appears before Block 2); pass-fixture `.../pass-all-blocks.txt`.
- **DISP-1.4 (git boundary dead last):** fail-fixture `.../disp-1.4/fail-git-boundary-not-last.txt` (git block followed by other content); pass-fixture `.../pass-all-blocks.txt`.
- **DISP-1.5 (fix-track addendum):** fail-fixture `.../disp-1.5/fail-fix-phase-no-addendum.txt` (fix-phase prompt lacking the zero-match addendum); pass-fixture `.../pass-fix-phase-with-addendum.txt`.
- Detection test for all five: `tests/dispatch-quality-blocks.test.mjs`.

## Parallel Ownership Notes
Depends on d01 for validator harness conventions only. Owns disjoint `src/dispatch/*` + `tests/fixtures/dispatch/**`. Shares the sub-agent-prompt concept with d25's ORCH-1.10 but from the generator side and against non-overlapping files (d25 owns `src/orchestrator/*`); no file overlap, concurrent-safe.
