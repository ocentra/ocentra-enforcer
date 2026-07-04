# a02 MCP Fingerprint Repoint

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MCP Fingerprint Repoint`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `mcp/rust-rules-mcp-fingerprint.mjs`, `mcp/rust-rules-mcp-helpers.mjs`
- deps: `a01`
- tier: `P3`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`mcp/rust-rules-mcp-helpers.mjs` holds `MCP_FINGERPRINT_FILES` (a hardcoded list of source `.mjs` paths, ~10 entries) consumed by `buildMcpFingerprint` in `mcp/rust-rules-mcp-fingerprint.mjs`. Once sources compile to `dist/`, the running MCP loads built artifacts but fingerprints stale source paths, so restart-detection no longer reflects what is actually executing.

## Where We Want To Be
Fingerprinting tracks the emitted `dist/` artifacts the process actually imports, not a frozen list of source `.mjs` paths, so the freshness/restart signal is truthful post-build.

## Requirement Checklist
- [ ] Replace the hardcoded `MCP_FINGERPRINT_FILES` list with resolution against `dist/` (or the resolved entry graph) of the running server.
- [ ] `buildMcpFingerprint` digest changes iff a shipped `dist/` artifact changes; unchanged source-adjacent files do not perturb it.
- [ ] Missing `dist/` (unbuilt) yields an explicit `exists:false` entry, not a silent pass.
- [ ] No path is hardcoded to a location that does not correspond to a loaded module.

## Acceptance And Proof
Tier P3 (live MCP-tool). A live test invokes the MCP `mcp_status`/freshness path and asserts the fingerprint digest over `dist/` artifacts; mutate a `dist/` file, reobserve a changed digest; unbuilt tree yields `exists:false`. Named in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01 (needs `dist/` to exist). Owns only the two fingerprint modules; a05 owns the branded `Sha256` types those modules will consume, so ordering is a05 or coordinate on the shared file — globs here are the fingerprint pair exclusively.
