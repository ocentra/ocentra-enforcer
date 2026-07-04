# a-conv-20 Coordination Vendor Materialize And Views

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Coordination Vendor Materialize And Views`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/coordination/vendor/materialize.js, src/coordination/vendor/peers.js, src/coordination/vendor/presence.js, src/coordination/vendor/read-index.js, src/coordination/vendor/notify.js, src/coordination/vendor/doctor.js`
- deps: `a-conv-17, a-conv-18, a-conv-19`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The materialize, peers, presence, read-index, notify, and doctor modules build queryable views over the coordination stream (a-conv-19) and report health.

## Where We Want To Be
The view/materialize modules are strict TS with typed materialized-view records and a typed doctor report.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-21, a-conv-23, a-conv-24, a-conv-50. Deps on a-conv-17, a-conv-18, a-conv-19; owns the materialize/views/doctor files.
