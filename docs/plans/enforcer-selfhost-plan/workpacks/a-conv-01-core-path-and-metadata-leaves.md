# a-conv-01 Core Path And Metadata Leaves

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Core Path And Metadata Leaves`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/path-utils.mjs, src/check-metadata.mjs, src/policy.mjs, src/rule-metadata.mjs, src/documentation-hints.mjs`
- deps: `a01`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
These are the dependency-free leaves the rest of `src/` imports: path helpers, metadata shape, policy constants, and documentation hints. `src/rule-metadata.mjs` is 776 lines and mixes rule-id constants with per-rule metadata tables, so it must be split before downstream clusters can type against a clean surface.

## Where We Want To Be
All five leaves are strict-TS `.ts` files with no wildcard imports, exporting typed surfaces that every downstream conversion cluster can depend on.

## Requirement Checklist
- [ ] Convert `src/path-utils.mjs`, `src/check-metadata.mjs`, `src/policy.mjs`, `src/documentation-hints.mjs` to strict TS with explicit exported types.
- [ ] SPLIT `src/rule-metadata.mjs`: separate rule-id/constant exports from the metadata tables into cohesive TS modules (no barrel wildcards).
- [ ] Drop all `import * as` wildcard imports; use named imports.
- [ ] Scoped `tsc --noEmit` over only these files passes with strict settings.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to these files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Split of `rule-metadata` produces named modules whose exports are re-verified by the downstream cluster's typecheck. Record the typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Root of Track A: every other a-conv cluster deps on this via the shared leaf surface. Its owns: set is disjoint from all siblings, so once green they run concurrently. Blocks nothing but must land first.
