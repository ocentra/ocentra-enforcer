# a-conv-09 Source Policy TS Manifest And Tsconfig

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Source Policy TS Manifest And Tsconfig`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/source-policy-typescript-manifest-tsconfig.mjs, src/source-policy-typescript-manifest-rules.mjs, src/source-policy-typescript-manifest.mjs, src/source-policy-typescript.mjs`
- deps: `a-conv-03, a-conv-07, a-conv-08`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The tsconfig/manifest rollups and the top-level `source-policy-typescript.mjs` entry compose the full TypeScript source-policy scanner from the tests (a-conv-07) and package-manifest (a-conv-08) families.

## Where We Want To Be
The TypeScript scanner entry and its manifest/tsconfig rollups are strict TS exposing one typed `scanTypeScript`-style surface.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Expose a single typed scanner entry aggregating source, test, and manifest rule sets.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-10. Deps on a-conv-03, a-conv-07, a-conv-08; owns the tsconfig/manifest/entry files, disjoint from the common/scanners rollup.
