# a-conv-41 MCP Context And Schema Leaves

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MCP Context And Schema Leaves`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `mcp/rust-rules-mcp-context.mjs, mcp/rust-rules-mcp-input-schemas-coordination.mjs, mcp/rust-rules-mcp-input-schemas-proof-properties.mjs, mcp/rust-rules-mcp-input-schemas-query.mjs, mcp/rust-rules-mcp-input-schemas-proof.mjs, mcp/rust-rules-mcp-freshness.mjs`
- deps: `a01`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The MCP context, input-schema leaves (coordination, proof-properties, query, proof), and freshness helper are the dependency-free schema/context foundation of the MCP server.

## Where We Want To Be
All six MCP leaves are strict TS with typed JSON-schema objects and a typed context record.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Type the MCP input schemas as explicit JSON-schema-shaped TS objects.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Root of the MCP sub-track: a-conv-42..a-conv-46 dep on it. Deps on a01; owns the MCP context/schema/freshness leaf files.
