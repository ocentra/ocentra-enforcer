# a-conv-49 Tests CLI Policy Multilang

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Tests CLI Policy Multilang`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `tests/enforcer-checks.test.mjs, tests/enforcer-edge-cases.test.mjs, tests/enforcer-fixtures.test.mjs, tests/enforcer-harness.test.mjs, tests/enforcer-policy.test.mjs, tests/enforcer-multilang.test.mjs, tests/rust-rules.test.mjs`
- deps: `a-conv-38, a-conv-48`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The CLI/policy/multilang test suites exercise the enforcer CLI (a-conv-38) via the shared harness (a-conv-48). `enforcer-multilang.test.mjs` (1231 lines), `rust-rules.test.mjs`, and `enforcer-policy.test.mjs` are oversized and must be split by concern.

## Where We Want To Be
All seven suites are strict TS using the typed harness; oversized suites are split into cohesive per-concern spec files.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] SPLIT `tests/enforcer-multilang.test.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] SPLIT `tests/rust-rules.test.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] SPLIT `tests/enforcer-policy.test.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Keep every existing test case; splits preserve coverage with no dropped assertions.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Each SPLIT target (`tests/enforcer-multilang.test.mjs`, `tests/rust-rules.test.mjs`, `tests/enforcer-policy.test.mjs`) is replaced by named modules whose combined exports match the original public surface, re-checked by dependent clusters. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Deps on a-conv-38 and a-conv-48; owns the CLI/policy/multilang test files, disjoint from the schema/proof/coord/mcp tests.
