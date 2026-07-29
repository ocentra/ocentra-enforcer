# Skill And MCP System

<!-- ai-dense -->
```yaml
native_install: "enforcer install"
native_cli_validation: "check/scan/verify with paths, --base/--head, or --all"
wired_rust_mcp: "mcp_status | coordination_status | coordination_claim | ui"
registered_not_wired: "route, scan, check, diagnostics, proof, and broader coordination tools"
```
<!-- /ai-dense -->

The native Rust binary provides both CLI and MCP stdio entrypoints. The
current installer registers its absolute path with supported user-level
harness adapters and runs an internal health check.

## Commands

```powershell
enforcer install
enforcer check path/to/file
enforcer scan --base origin/main --head HEAD
enforcer verify --mode local --all
enforcer serve
```

`install` accepts no flags. The native CLI does not currently expose public
`doctor`, `init`, `update`, or `uninstall` commands. Its visible `plan`,
`proof`, and `coordination` groups are reserved but return not-wired errors.

## Current Rust MCP Surface

The current router executes:

- `ocentra_enforcer_mcp_status`;
- `ocentra_enforcer_coordination_status`;
- `ocentra_enforcer_coordination_claim`;
- `ocentra_enforcer_ui`.

MCP discovery also lists contracts for routing, validation, diagnostics,
proof, and broader coordination. Those registrations currently return a
structured not-wired error and are not supported actions.

## Skill Workflow

1. Confirm the binary with `enforcer --help`.
2. Choose the smallest native CLI scope that covers the change.
3. Run `check`, `scan`, or `verify`.
4. Treat violations as hard failures and repair the reported cause.
5. Widen to the repository's required gate before completion.

The frozen Node compatibility service has a different command and MCP surface.
Documentation for that service must identify it explicitly and must not be
used as evidence that the Rust boundary is wired.
