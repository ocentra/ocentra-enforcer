# a-conv-07 Source Policy TS Tests And Blocks

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Source Policy TS Tests And Blocks`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/source-policy-typescript-test-blocks.mjs, src/source-policy-typescript-test-rules.mjs, src/source-policy-typescript-tests.mjs, src/source-policy-typescript-tests-domain.mjs, src/source-policy-typescript-lines.mjs`
- deps: `a-conv-03, a-conv-06`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The test-policy side of the TypeScript scanner (test-blocks, test-rules, tests rollup, tests-domain) plus the line scanner enforce test-shape rules. They build on the source rollup from a-conv-06 and the primitives from a-conv-03.

## Where We Want To Be
All five test/line modules are strict TS with a typed test-rule set and a typed line-scan result.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-09. Deps on a-conv-03 and a-conv-06; owns the test/blocks/lines files, disjoint from the manifest and tsconfig clusters.
