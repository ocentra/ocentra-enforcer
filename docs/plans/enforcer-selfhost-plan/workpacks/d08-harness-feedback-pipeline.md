# d08 Harness Feedback Pipeline

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Harness Feedback Pipeline`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/harness-feedback.ts, src/harness-feedback-classify.ts, tests/harness-feedback.test.mjs, tests/fixtures/harness-feedback/**`
- deps: `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
When a harness surfaces an escaped defect, the lesson dies in a chat log. The enforcer parses harness output (`src/harness.mjs`, `src/harness-parsers*.mjs`) but never turns a failure into a candidate rule. ADBP's "close the loop" is prose.

## Where We Want To Be
A pipeline that ingests harness failures, classifies each as preventable (could have been a static rule) vs detect-only, and for preventable ones auto-scaffolds a PROPOSED validator via the d01 engine.

## Requirement Checklist
- [ ] Ingest structured harness failures (reuse existing `harness-parsers*` shapes).
- [ ] Classify each failure into `prevent` vs `detect` via explicit signal rules (mechanical, not vibe).
- [ ] For `prevent`, call d01 `rule new` to scaffold a validator/doc/fixtures marked status `PROPOSED`.
- [ ] PROPOSED rules do not gate builds until reviewed/promoted; status is machine-readable in the registry.
- [ ] Classification decisions logged to d04 telemetry with the input fingerprint.

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `tests/harness-feedback.test.mjs` over `tests/fixtures/harness-feedback/**`: a preventable failure produces a PROPOSED registry row + fixtures passing d01 parity; a detect-only failure produces none; PROPOSED rules are non-blocking. Mechanism: classifier over parsed failure fields feeding the d01 scaffolder, asserted by resulting registry state.

## Parallel Ownership Notes
Depends on d01 (scaffolder). Feeds d10 (auditor). Owns disjoint feedback files, concurrent with d06/d07.
