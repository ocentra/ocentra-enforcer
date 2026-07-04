# a-conv-25 Check Governance Manifest Family

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Check Governance Manifest Family`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/check-governance-manifest-errors.mjs, src/check-governance-manifest-json.mjs, src/check-governance-manifest-package.mjs, src/check-governance-manifest-shared.mjs, src/check-governance-manifest-deps.mjs, src/check-governance-manifest-policy.mjs, src/check-governance-manifest-lock.mjs, src/check-governance-manifest-values.mjs`
- deps: `a-conv-01, a-conv-30`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The governance manifest family (errors, json, package, shared, deps, policy, lock, values) validates package/manifest governance and feeds the governance rollup. Untyped .mjs consuming leaves and the check-source helpers.

## Where We Want To Be
The eight manifest-governance modules are strict TS with a typed manifest-governance error and result shape.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Define an explicit typed governance-error result reused across the family.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-26. Deps on a-conv-01 and a-conv-30; owns the check-governance-manifest-* files, disjoint from the governance rollup.
