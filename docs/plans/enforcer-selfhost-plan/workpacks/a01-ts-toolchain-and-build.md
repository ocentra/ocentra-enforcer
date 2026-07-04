# a01 TS Toolchain And Build

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `TS Toolchain And Build`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `tsconfig.json`, `package.json#scripts.build`, `package.json#scripts.typecheck`, `package.json#engines`
- deps: `none`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The repo is 100% `.mjs` with `"type": "module"` and `engines.node` `>=20 <23` in `package.json`. There is no `tsconfig.json` (confirmed absent) and no `build`/`typecheck` script. Every sibling migration workpack needs a compiler contract before it can emit `.ts`.

## Where We Want To Be
A committed strict `tsconfig.json` plus `build` and `typecheck` npm scripts and pinned `engines`, so `.ts` sources compile to `dist/` and `npm run typecheck` is a hard gate the whole plan depends on.

## Requirement Checklist
- [ ] `tsconfig.json` with `strict: true`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `module`/`moduleResolution` matching ESM, `outDir: dist`.
- [ ] `package.json` gains `typecheck` (`tsc --noEmit`) and `build` (`tsc`) scripts.
- [ ] `engines.node` retained/pinned; `typescript` added to `devDependencies`.
- [ ] `npm run typecheck` exits 0 on the pre-migration tree (allowing existing `.mjs` via `allowJs`/checkJs off) and non-zero on an injected type error.

## Acceptance And Proof
Tier P1. A CI/test row in TEST_PROOF_EXPECTATIONS.md asserts `npm run typecheck` exit code (0 clean, non-0 with a seeded type error fixture). `tsconfig.json` presence and required compiler flags asserted by a schema/config test.

## Parallel Ownership Notes
Blocks every other Track A workpack (they need a compiler). Owns only `tsconfig.json` and three `package.json` keys; sibling packs own disjoint `package.json` script keys (`enforcer:self`, CI) and source globs, so no overlap.
