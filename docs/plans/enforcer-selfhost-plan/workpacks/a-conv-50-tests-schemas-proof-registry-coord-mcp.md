# a-conv-50 Tests Schemas Proof Registry Coord MCP

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Tests Schemas Proof Registry Coord MCP`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `tests/enforcer-schemas.test.mjs, tests/enforcer-proof.test.mjs, tests/rust-rules-registry.test.mjs, tests/rust-rules-mcp.test.mjs, tests/coordination.test.mjs`
- deps: `a-conv-01, a-conv-12, a-conv-17, a-conv-19, a-conv-20, a-conv-21, a-conv-46`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The schema, proof, registry, MCP, and coordination test suites verify the contract, proof, scanner-registry, MCP server, and coordination subsystems. `coordination.test.mjs` (1641 lines) and `rust-rules-mcp.test.mjs` are oversized and must be split.

## Where We Want To Be
All five suites are strict TS against the typed subsystems; oversized suites are split into cohesive per-concern spec files.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] SPLIT `tests/coordination.test.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] SPLIT `tests/rust-rules-mcp.test.mjs`: divide into cohesive TS modules by responsibility; no barrel wildcard re-exports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Keep every existing test case; splits preserve coverage with no dropped assertions.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Each SPLIT target (`tests/coordination.test.mjs`, `tests/rust-rules-mcp.test.mjs`) is replaced by named modules whose combined exports match the original public surface, re-checked by dependent clusters. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Deps span leaves, scanners, coordination, and MCP entry; owns the schema/proof/registry/coord/mcp test files, disjoint from the CLI/policy tests.
