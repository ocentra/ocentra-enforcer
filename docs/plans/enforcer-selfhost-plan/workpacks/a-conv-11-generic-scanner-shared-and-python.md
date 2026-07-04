# a-conv-11 Generic Scanner Shared And Python

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Generic Scanner Shared And Python`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/generic-scanner-shared.mjs, src/generic-common-line-rules.mjs, src/generic-common-source-ownership.mjs, src/generic-python-scanner-rules.mjs, src/generic-python-scanner.mjs`
- deps: `a-conv-01, a-conv-02`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The generic scanner shared scaffolding plus the common line-rules, source-ownership, and the Python scanner (rules + entry) provide language-agnostic and Python-specific scanning. Untyped .mjs using the leaf helpers and rule registry.

## Where We Want To Be
The shared generic scaffolding and Python scanner are strict TS with typed line-rule and ownership descriptors.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-12. Deps on a-conv-01 and a-conv-02; owns the generic-shared and python-scanner files, disjoint from the generic rollup cluster.
