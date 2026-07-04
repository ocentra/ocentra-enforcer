# a-conv-18 Coordination Vendor Events Identity Claim

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Coordination Vendor Events Identity Claim`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/coordination/vendor/events.js, src/coordination/vendor/identity.js, src/coordination/vendor/claim-policy.js, src/coordination/vendor/lock-policy.js`
- deps: `a-conv-17`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The vendor events, identity, claim-policy, and lock-policy modules encode the coordination event stream and claim/lock rules atop the domain leaf from a-conv-17.

## Where We Want To Be
All four modules are strict TS with a typed event union and typed claim/lock policy results.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Model coordination events as a typed discriminated union.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-19, a-conv-20, a-conv-21, a-conv-45, a-conv-50. Deps on a-conv-17; owns the events/identity/claim/lock files exclusively.
