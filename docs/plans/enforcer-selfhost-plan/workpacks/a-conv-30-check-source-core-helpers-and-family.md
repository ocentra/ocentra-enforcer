# a-conv-30 Check Source Core Helpers And Family

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Check Source Core Helpers And Family`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `scripts/check-source-core-helpers.mjs, scripts/check-source-core-checks.mjs, scripts/check-source-core-source-shape.mjs, scripts/check-source-core-tests.mjs, scripts/check-source-core.mjs, scripts/check-source-shape-scanners.mjs`
- deps: `a-conv-01, a-conv-02, a-conv-10, a-conv-12`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The check-source-core family (helpers, checks, source-shape, tests, entry, shape-scanners) is the source-shape checker driving governance and the checks bridge. Both `check-source-core-helpers.mjs` and `check-source-core.mjs` (972 lines) are oversized and must be split.

## Where We Want To Be
The check-source-core family is strict TS with typed source-shape results; the two oversized files become thin typed entries over cohesive modules.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] SPLIT `scripts/check-source-core-helpers.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] SPLIT `scripts/check-source-core.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Type the source-shape check result surface explicitly.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Each SPLIT target (`scripts/check-source-core-helpers.mjs`, `scripts/check-source-core.mjs`) is replaced by named modules whose combined exports match the original public surface, re-checked by dependent clusters. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-25, a-conv-26, a-conv-28. Deps span leaves, routing, and scanners; owns the check-source-core files exclusively.
