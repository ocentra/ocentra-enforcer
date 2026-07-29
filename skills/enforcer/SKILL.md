---
name: enforcer
description: Use the current native Rust Enforcer CLI for focused repository validation and its presently wired MCP status, coordination-status/claim, or UI tools.
---

# Enforcer

<!-- ai-dense -->
```yaml
binary: single native `enforcer` executable
scopes: "paths | --base/--head | --all"
wired_cli: "check, scan, serve, ui, install, verify, advise, architecture, onboard"
reserved_cli: "plan, proof, coordination/ledger are visible but not wired"
wired_mcp: "mcp_status, coordination_status, coordination_claim, ui"
banned: "bypass comments, skipped tests, broad waivers, re-export shims, rule downgrades"
```
<!-- /ai-dense -->

Use this skill for the native Rust implementation. Do not infer runtime
support from registered command or MCP schemas; execute only the boundaries
listed as wired above.

## Workflow

1. Confirm the exact binary with `enforcer --help`.
2. Choose one explicit scope: paths, a `--base`/`--head` diff pair, or
   `--all`.
3. Run the smallest relevant `check`, `scan`, or `verify` command.
4. Repair violations without suppressions or policy weakening.
5. Run the repository's compiler, lint, and test commands directly when they
   are required; native `run` and `runs` commands are not available.
6. Widen validation before reporting completion.

## Current CLI

```powershell
enforcer check path/to/file
enforcer scan --base origin/main --head HEAD
enforcer verify --mode local --all
enforcer architecture check --language rust --all
enforcer advise literals
```

`enforcer install` accepts no flags. It registers the absolute native binary
with all supported user-level harness adapters and runs an internal read-only
health check. Public `doctor`, `init`, `update`, and `uninstall` commands are
not exposed. `plan`, `proof`, and `coordination` (`ledger`) return not-wired
errors.

## Current MCP

The Rust MCP router currently wires:

```text
ocentra_enforcer_mcp_status
ocentra_enforcer_coordination_status
ocentra_enforcer_coordination_claim
ocentra_enforcer_ui
```

Routing, validation, diagnostics, proof, and broader coordination tools may
appear in discovery but currently return a structured not-wired error. Use the
native CLI for source validation.

## Failure Reporting

Report the exact binary and command, scope, first rule IDs, first affected
files, and the smallest next fix. Do not claim proof, closeout, or diagnostic
operations that the current public boundary cannot execute.
