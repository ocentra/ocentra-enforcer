# d07 Self-Correct Fix Loop

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Self-Correct Fix Loop`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/fix-loop.ts, src/fix-loop-dispatch.ts, tests/fix-loop.test.mjs, tests/fixtures/fix-loop/**`
- deps: `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The enforcer only reports; it cannot attempt guided remediation. ADBP describes a "self-correcting" loop as aspiration. There is no bounded fix-verify-revert mechanism, so any autofix would risk unbounded churn.

## Where We Want To Be
A bounded fix loop: dispatch a fix generator for a finding, re-run the relevant validator, keep the change only if the finding count strictly improves and nothing regresses, else revert; hard iteration cap.

## Requirement Checklist
- [ ] Loop takes a finding set, dispatches a fix generator (pluggable), and re-checks with the same validator.
- [ ] Accept a change only if total findings strictly decrease and no new ruleId appears (measured via re-scan, not model claim).
- [ ] Revert to the prior tree state on non-improvement (deterministic snapshot/restore).
- [ ] Hard bound on iterations; loop always terminates.
- [ ] Every accept/revert decision logged to d04 telemetry.

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `tests/fix-loop.test.mjs` over `tests/fixtures/fix-loop/**`: an improving fix is kept; a neutral/regressing fix is reverted; the loop halts at the iteration cap; final state never has more findings than the start. Mechanism: re-scan-and-compare gate wrapping snapshot/restore, verified by before/after finding counts.

## Parallel Ownership Notes
Depends on d01. Invoked by d06 `fix`. Owns disjoint loop files, runnable concurrently with d06/d08 against their interfaces.
