# a-conv-17 Coordination Vendor Domain Leaf

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Coordination Vendor Domain Leaf`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/coordination/vendor/domain.js, src/coordination/vendor/paths.js, src/coordination/vendor/root.js, src/coordination/vendor/dashboard.js, src/coordination/vendor/context.js`
- deps: `a01`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The vendored coordination leaves (domain, paths, root, dashboard, context) are dependency-free `.js` modules that define the coordination domain model and filesystem layout consumed by the rest of the vendor tree.

## Where We Want To Be
All five vendor leaves are strict TS with typed domain records and path helpers.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Export a typed coordination domain model reused across the vendor tree.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Root of the coordination sub-track: a-conv-18..a-conv-24 and a-conv-50 dep on it. Owns only the vendor leaf files, disjoint from events/stream/guard clusters.
