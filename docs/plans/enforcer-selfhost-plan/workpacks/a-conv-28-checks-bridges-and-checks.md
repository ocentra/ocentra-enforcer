# a-conv-28 Checks Bridges And Checks

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Checks Bridges And Checks`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/checks-ai-index-bridge.mjs, src/checks-governance-bridge.mjs, src/checks-literal-risk-bridge.mjs, src/checks-contracts.mjs, src/checks.mjs`
- deps: `a-conv-01, a-conv-10, a-conv-12, a-conv-26, a-conv-27, a-conv-30, a-conv-31`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The check bridges (ai-index, governance, literal-risk), `checks-contracts.mjs`, and `src/checks.mjs` (695 lines) aggregate every check into the top-level checks surface. The oversized entry mixes bridge wiring, the contract table, and dispatch and must be split.

## Where We Want To Be
`checks.mjs` becomes a thin typed aggregator over cohesive bridge/contract modules exposing a typed checks registry.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] SPLIT `src/checks.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Split `checks.mjs` into bridge-wiring, contract, and dispatch TS modules behind a typed checks registry.
- [ ] Type the check-contract table so each ruleId maps to a typed check.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Each SPLIT target (`src/checks.mjs`) is replaced by named modules whose combined exports match the original public surface, re-checked by dependent clusters. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-34, a-conv-35, a-conv-38. Deps span leaves, scanners, governance, docs/policy, and rust source patterns; owns the bridges/contracts/checks files.
