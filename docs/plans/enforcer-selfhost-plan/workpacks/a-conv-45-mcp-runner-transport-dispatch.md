# a-conv-45 MCP Runner Transport Dispatch

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MCP Runner Transport Dispatch`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `mcp/rust-rules-mcp-runner-scope.mjs, mcp/rust-rules-mcp-runner-cli.mjs, mcp/rust-rules-mcp-runner.mjs, mcp/rust-rules-mcp-transport-frames.mjs, mcp/rust-rules-mcp-transport-messages.mjs, mcp/rust-rules-mcp-transport.mjs, mcp/rust-rules-mcp-dispatch.mjs`
- deps: `a-conv-02, a-conv-18, a-conv-42, a-conv-43, a-conv-44`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The MCP runner (scope, cli, entry), transport (frames, messages, rollup), and dispatch modules run the stdio MCP loop, framing messages and dispatching to the tool registry (a-conv-44) and routes (a-conv-43).

## Where We Want To Be
The runner/transport/dispatch modules are strict TS with typed JSON-RPC frame/message types and a typed dispatch result.

## Requirement Checklist
- [ ] Convert every owned file to strict TS with explicit exported types; no implicit `any`.
- [ ] Drop all wildcard imports (`import * as`); replace with named imports.
- [ ] Scoped `tsc --noEmit` over only the owned files passes under strict mode.
- [ ] Type the MCP transport frame and message shapes as explicit interfaces.

## Acceptance And Proof
Tier P1. Scoped typecheck (tsconfig include limited to the owned files) exits 0 under strict mode. `grep` for `import *` across owned files returns empty. Record the scoped-typecheck artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Blocks a-conv-46. Deps span routing, coordination events, and MCP helpers/registry; owns the runner/transport/dispatch files.
