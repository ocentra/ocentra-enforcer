# a-conv-03 Source Policy Primitives

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Source Policy Primitives`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/source-policy-rule-registry.mjs, src/source-policy-paths.mjs, src/source-policy-text.mjs, src/source-policy-helpers.mjs, src/source-policy-test-double-patterns.mjs, src/source-policy-windows-command-patterns.mjs, src/source-policy-violation.mjs`
- deps: `a-conv-01`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The `source-policy-*` primitives (paths, text helpers, pattern tables, violation shape, and the local rule registry) are the foundation of every source-policy scanner. They are untyped .mjs leaves consumed by the common-security and typescript-domain families.

## Where We Want To Be
All seven primitives are strict TS with a typed Violation shape and typed pattern tables that downstream scanners import by name.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Export a single typed `Violation` interface reused by every scanner in a-conv-04/05/06/07/08.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks the whole source-policy tree (a-conv-04..a-conv-10). Owns only the seven primitive files; disjoint from scanner rollups and common-security.
