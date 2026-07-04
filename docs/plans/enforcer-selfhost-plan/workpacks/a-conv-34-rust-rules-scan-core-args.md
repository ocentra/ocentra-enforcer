# a-conv-34 Rust Rules Scan Core Args

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Rust Rules Scan Core Args`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `scripts/rust-rules-scan-core-args-options.mjs, scripts/rust-rules-scan-core-args-command.mjs, scripts/rust-rules-scan-core-args-files.mjs, scripts/rust-rules-scan-core-args.mjs`
- deps: `a-conv-28`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The scan-core args family (options, command, files, args rollup) parses CLI arguments for the Rust scan-core entry, consuming the checks surface from a-conv-28.

## Where We Want To Be
The scan-core args family is strict TS with a typed parsed-args interface.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Define an explicit typed parsed-args interface for the scan-core CLI.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-35, a-conv-38. Deps on a-conv-28; owns the scan-core-args files, disjoint from the scan-core entry.
