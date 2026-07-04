# a-conv-22 Coordination Vendor CLI

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Coordination Vendor CLI`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/coordination/vendor/cli.js`
- deps: `a-conv-17, a-conv-18, a-conv-19, a-conv-20, a-conv-21`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`src/coordination/vendor/cli.js` (654 lines) is the vendored coordination CLI dispatching across every vendor subsystem. Its size and per-command branches require a split before strict typing.

## Where We Want To Be
The vendor CLI becomes a thin typed dispatcher over cohesive per-command TS modules.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] SPLIT `src/coordination/vendor/cli.js`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Split per-command handlers into named TS modules behind a typed command table.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Each SPLIT target (`src/coordination/vendor/cli.js`) is replaced by named modules whose combined exports match the original public surface, re-checked by dependent clusters. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Terminal leaf of the vendor tree. Deps span a-conv-17..a-conv-21; owns only the vendor CLI file, disjoint from the coordination-api cluster.
