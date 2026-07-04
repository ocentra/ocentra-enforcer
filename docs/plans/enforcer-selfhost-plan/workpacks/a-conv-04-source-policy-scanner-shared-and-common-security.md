# a-conv-04 Source Policy Scanner Shared And Common Security

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Source Policy Scanner Shared And Common Security`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/source-policy-scanner-shared.mjs, src/source-policy-common-security-manifest.mjs, src/source-policy-common-security-sensitive.mjs, src/source-policy-common-security-test-doubles.mjs, src/source-policy-common-policy.mjs, src/source-policy-common-security-rules.mjs, src/source-policy-common-security.mjs`
- deps: `a-conv-03`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The common-security family (manifest, sensitive, test-doubles, policy, rules rollup) plus the shared scanner scaffolding builds on a-conv-03 primitives. These emit T2 scored findings for cross-language security rules.

## Where We Want To Be
The whole common-security surface is strict TS, each rule returning a typed scored finding (score + confidence) via the shared scanner contract.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Preserve T2 scored-finding output shape (score + confidence) with an explicit result type.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-10. Deps on a-conv-03 primitives; owns the shared-scanner and common-security files exclusively, disjoint from the typescript-domain clusters.
