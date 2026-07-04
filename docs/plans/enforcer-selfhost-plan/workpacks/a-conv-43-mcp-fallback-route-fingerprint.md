# a-conv-43 MCP Fallback Route Fingerprint

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MCP Fallback Route Fingerprint`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `mcp/rust-rules-mcp-fallback-command-builders.mjs, mcp/rust-rules-mcp-fallback-options.mjs, mcp/rust-rules-mcp-fallback-routing.mjs, mcp/rust-rules-mcp-fallback.mjs, mcp/rust-rules-mcp-route-shared.mjs, mcp/rust-rules-mcp-route.mjs, mcp/rust-rules-mcp-fingerprint.mjs`
- deps: `a-conv-02, a-conv-41, a-conv-42`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The MCP fallback (command-builders, options, routing, rollup), route (shared, entry), and fingerprint modules map tool calls to CLI fallbacks and route requests, using routing (a-conv-02) and the MCP helpers/schemas.

## Where We Want To Be
The fallback/route/fingerprint modules are strict TS with typed route descriptors and a typed fallback command builder.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-45. Deps on a-conv-02, a-conv-41, a-conv-42; owns the fallback/route/fingerprint files, disjoint from tool-registry and runner.
