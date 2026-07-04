# b04 Parallel Orchestrator Binding

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Parallel Orchestrator Binding`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/plan/orchestrator/**, test/plan/orchestrator.*.test.ts`
- deps: `b02-plan-structure-validator`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
A validated plan tells you which workpacks may run in parallel (disjoint owns, no dep edge) but nothing turns that into an execution frontier. The coordination MCP (`ocentra_enforcer_coordination_*`: claim/guard/closeout/tasks) exists but is not bound to plan structure, so lane assignment is manual.

## Where We Want To Be
A deterministic binding that computes the parallel frontier from the validated plan graph and drives it through the coordination MCP: frontier -> hub lanes -> claim/guard/closeout, with an intent-queue for overlap resolution.

## Requirement Checklist
- [ ] Build the dep DAG from workpack deps: and compute the ready frontier (deps satisfied).
- [ ] Assign frontier workpacks to hub lanes so no two concurrent lanes share owns globs (reuse b02 PLAN-PARALLEL-SAFETY logic, do not reimplement).
- [ ] Bind lane lifecycle to `coordination_claim` -> `coordination_guard` -> `coordination_closeout`.
- [ ] Intent-queue serializes any residual owns overlap that slips past static checks (fail-closed: refuse concurrent claim on overlapping owns).
- [ ] Reuse the existing coordination MCP; add no parallel coordination store.

## Acceptance And Proof
Tier P1. Proof: `test/plan/orchestrator.frontier.test.ts` asserts the frontier for a fixture plan graph; `orchestrator.lanes.test.ts` asserts disjoint-owns lane assignment; `orchestrator.claim-guard.test.ts` uses a coordination MCP stub/fake to assert claim/guard/closeout are called in order and that overlapping-owns claims are rejected. Name these in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on b02 (imports its parallel-safety predicate and consumes a validated plan graph), so it must start after b02 lands. owns: (orchestrator src + its tests) is disjoint from b01/b02/b03/b05. It blocks nothing else in Track B.
