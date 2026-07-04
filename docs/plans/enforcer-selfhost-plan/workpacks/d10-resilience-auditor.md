# d10 Resilience Auditor

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Resilience Auditor`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/resilience-auditor.ts, src/failure-mode-smells.ts, tests/resilience-auditor.test.mjs, tests/fixtures/resilience/**`
- deps: `d01-rule-mechanization-engine, d04-run-telemetry-ndjson`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Nothing adversarially probes a change for missing failure-mode coverage. ADBP describes a "red team / resilience" reviewer as narrative. The enforcer has no mechanism turning adversarial review into required-test obligations.

## Where We Want To Be
An adversarial sub-agent pass whose output is mechanized twofold: it emits required-test rows (T1 obligations) and T2 failure-mode "smell" scores (e.g. unhandled error path, missing timeout, unbounded retry).

## Requirement Checklist
- [ ] Adversarial pass enumerates candidate failure modes for the changed surface.
- [ ] Each accepted failure mode becomes a required-test row (T1) that must be satisfied by a matching test before review passes.
- [ ] Emit T2 smell scores (score + confidence) for heuristically-detected failure-mode gaps; non-blocking.
- [ ] Required-test rows reference real ruleIds/fixtures via d01; smells consume d04 telemetry for trend.
- [ ] Missing a required test fails closed; smells never block.

## Acceptance And Proof
Tier T1 (required-test rows) + T2 (failure-mode smells), P1 unit. Prove via `tests/resilience-auditor.test.mjs` over `tests/fixtures/resilience/**`: a required-test row unmet fails; met passes; smell scores in [0,1] with confidence and no gating effect. Mechanism: obligation table (T1) plus regex/AST heuristic scorer (T2), asserted against fixtures.

## Parallel Ownership Notes
Depends on d01 and d04. Consumed by d06 `review`. Owns disjoint auditor files, concurrent with d07/d08.
