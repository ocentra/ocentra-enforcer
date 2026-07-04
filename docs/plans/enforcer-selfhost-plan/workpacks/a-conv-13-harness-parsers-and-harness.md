# a-conv-13 Harness Parsers And Harness

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Harness Parsers And Harness`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/harness-parsers-json-diagnostics.mjs, src/harness-parsers-json-lines.mjs, src/harness-parsers-json-payload.mjs, src/harness-parsers.mjs, src/harness.mjs`
- deps: `a-conv-01`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The harness JSON parsers (diagnostics, lines, payload), the parsers rollup, and `harness.mjs` normalize third-party harness output into the enforcer's finding shape. Untyped .mjs consuming path/metadata leaves.

## Where We Want To Be
The harness parser family is strict TS with typed parsed-payload interfaces and a typed harness normalizer.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Define explicit TypeScript interfaces for each parsed JSON payload variant.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-16. Deps only on a-conv-01; owns the harness-parser and harness files exclusively.
