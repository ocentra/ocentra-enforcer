# a-conv-40 Rust Rules Output Rollup And Entrypoints

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rust Rules Output Rollup And Entrypoints`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `scripts/rust-rules-output.mjs, scripts/rust-rules.mjs, scripts/ocentra-enforcer.mjs, scripts/ci-local.mjs, scripts/mcp-smoke.mjs, scripts/profile-proof-runner.mjs, scripts/validate-codex-assets.mjs`
- deps: `a-conv-38, a-conv-39`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The output rollup, the `rust-rules.mjs`/`ocentra-enforcer.mjs` script entrypoints, and the supporting scripts (ci-local, mcp-smoke, profile-proof-runner, validate-codex-assets) are the runnable script surface built on the CLI main (a-conv-38) and output family (a-conv-39).

## Where We Want To Be
All script entrypoints and helpers are strict TS with typed entry signatures.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Terminal of the scripts sub-track. Deps on a-conv-38, a-conv-39; owns the output rollup and script entrypoint files exclusively.
