# a-conv-06 Source Policy TS Domain Source Rollup

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Source Policy TS Domain Source Rollup`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/source-policy-typescript-source-domain-domain.mjs, src/source-policy-typescript-source-domain-source.mjs, src/source-policy-typescript-source-domain-rules.mjs, src/source-policy-typescript-source-domain.mjs, src/source-policy-typescript-source.mjs`
- deps: `a-conv-03, a-conv-05`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
These roll up the domain and source rules for the TypeScript source-policy scanner, composing the boundary leaves from a-conv-05 into the `-source` entry. Currently .mjs with wildcard re-exports of the domain leaves.

## Where We Want To Be
The domain/source rollups are strict TS, importing named rule descriptors from a-conv-05 and exposing a typed aggregated rule set.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Replace wildcard rollup re-exports with explicit named exports of the aggregated rule set.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-07. Deps on a-conv-03 and a-conv-05; owns only the rollup files, disjoint from the boundary leaves and the tests/blocks cluster.
