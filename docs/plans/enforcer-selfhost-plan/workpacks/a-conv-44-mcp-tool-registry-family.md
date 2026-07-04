# a-conv-44 MCP Tool Registry Family

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MCP Tool Registry Family`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `mcp/rust-rules-mcp-tool-registry-shared.mjs, mcp/rust-rules-mcp-tool-registry-coordination.mjs, mcp/rust-rules-mcp-tool-registry-proof.mjs, mcp/rust-rules-mcp-tool-registry-rules.mjs, mcp/rust-rules-mcp-tool-registry-harness.mjs, mcp/rust-rules-mcp-tool-registry.mjs`
- deps: `a-conv-42`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The MCP tool-registry family (shared, coordination, proof, rules, harness, rollup) declares every MCP tool and its schema, composing the schema/compact helpers from a-conv-42.

## Where We Want To Be
The tool-registry family is strict TS with a typed tool-descriptor and a typed registry keyed by tool name.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Type the tool registry so each tool has a typed input schema and handler signature.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-45, a-conv-46, a-conv-50. Deps on a-conv-42; owns the tool-registry files exclusively.
