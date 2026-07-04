# a-conv-08 Source Policy TS Package Manifest Deps

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Source Policy TS Package Manifest Deps`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/source-policy-typescript-package-manifest-dependencies-section.mjs, src/source-policy-typescript-package-manifest-dependencies-schema.mjs, src/source-policy-typescript-package-manifest-dependencies-loose.mjs, src/source-policy-typescript-package-manifest-dependencies.mjs, src/source-policy-typescript-package-manifest-lockfiles.mjs, src/source-policy-typescript-package-manifest.mjs`
- deps: `a-conv-03`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The package-manifest dependency rules (section, schema, loose, dependencies rollup, lockfiles, manifest rollup) validate package.json/lockfile policy for TS projects. Untyped .mjs consuming a-conv-03 primitives.

## Where We Want To Be
The manifest-deps family is strict TS with a typed manifest schema and dependency-rule descriptors.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Introduce an explicit TypeScript type for the parsed manifest/dependencies schema.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-09 and a-conv-10. Deps only on a-conv-03; owns the package-manifest-dependency files exclusively.
