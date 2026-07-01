# MCP Contract Rules

## Covered Rules

- `MCP-1.1`: Every CLI scan/check surface must have an MCP equivalent.
- `MCP-1.2`: MCP inputs must be schema decoded.
- `MCP-1.3`: MCP schemas must reject unknown arguments.
- `MCP-1.4`: `summaryOnly` output must be bounded.
- `MCP-1.5`: `diagnosticLimit` must be respected.
- `MCP-1.6`: Stale MCP processes must fail closed for coordination writes.
- `MCP-1.7`: MCP status must include pack version and file hashes.
- `MCP-1.8`: MCP explain must match CLI explain.
- `MCP-1.9`: MCP route must match CLI route.
- `MCP-1.10`: MCP scan/check/route/explain must not mutate target repos.
- `MCP-1.11`: MCP write tools must be dedicated operations, not generic action dispatchers.
- `MCP-1.12`: MCP errors must be structured JSON.

## Fails

- A visible MCP tool accepts unknown args and silently ignores them.
- A stale MCP server writes coordination events after the pack source changed.
- `summaryOnly` returns full scope and raw terminal output.
- `coordination_claim` accepts `action: "release"` and appends a claim.
- Tool errors return raw strings or stack dumps.

## Passes

- Tool arguments are decoded through the shared schema layer.
- Stale write-capable coordination tools return `ok:false`, reload metadata, and an `ocentra_enforcer_run` fallback command.
- Compact scan/check output honors `summaryOnly`, `includeScope`, `groupBy`, and `diagnosticLimit`.
- Dedicated write tools route to dedicated API functions.
- Errors are JSON objects with `ok:false` and `error`.

## Fix Recipe

1. Add or update the Effect Schema contract first.
2. Expose the MCP schema with `additionalProperties: false`.
3. Decode tool args before calling implementation code.
4. For write-capable coordination tools, check MCP freshness before writing.
5. Add parity tests for CLI and MCP route/explain/scan/check behavior.

## Validator

- scanner: `common/mcp-contracts`
- implemented in: `mcp/rust-rules-mcp.mjs`
- command: `ocentra-enforcer check mcp-contracts --root <repo>`
