# a-conv-38 CLI Main Entry

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `CLI Main Entry`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/cli-main.mjs`
- deps: `a-conv-01, a-conv-02, a-conv-12, a-conv-16, a-conv-24, a-conv-28, a-conv-34, a-conv-35, a-conv-36, a-conv-37`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`src/cli-main.mjs` is the CLI entry that assembles routing, scanners, proof, coordination, checks, and dispatch into the runnable enforcer CLI.

## Where We Want To Be
The CLI main entry is strict TS with a typed argv->exit-code main function and no wildcard imports.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Expose a typed `main(argv)` returning a typed exit result.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-40, a-conv-49. Deps span the CLI and core clusters; owns only `src/cli-main.mjs`.
