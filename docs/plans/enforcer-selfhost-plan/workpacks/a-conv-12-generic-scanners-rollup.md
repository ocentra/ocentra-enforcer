# a-conv-12 Generic Scanners Rollup

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Generic Scanners Rollup`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/generic-common-scanner.mjs, src/generic-typescript-scanner.mjs, src/generic-scanners.mjs`
- deps: `a-conv-01, a-conv-10, a-conv-11`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The generic common scanner, the generic TypeScript scanner, and the `generic-scanners.mjs` rollup compose the source-policy scanners (a-conv-10) and Python scanner (a-conv-11) into the generic scanning surface used by checks and the CLI.

## Where We Want To Be
The three generic-scanner rollups are strict TS exposing a typed generic-scanner registry.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-28, a-conv-30, a-conv-35, a-conv-50. Deps on a-conv-01, a-conv-10, a-conv-11; owns only the three rollup files.
