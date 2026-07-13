# Enforced Checks

This is a concise catalog of the policy families in the Enforcer workspace.
It is not a complete rule list or a promise that every family is exposed by
every build or command. The current command and MCP-tool help are the source
of truth for availability and arguments.

## How to use it

For MCP work, use the installed Enforcer MCP route tool with the target root
and exact files under review. For the current Rust CLI, select one explicit
scope for `check`, `scan`, or `verify`:

- paths: `enforcer check path/to/file`
- diff: `enforcer scan --base <ref> --head <ref>`
- workspace: `enforcer verify --mode local --all`

Choose the smallest scope that covers the change, then widen the gate before a
release or merge decision.

## Policy families

| Family | Examples of concerns |
| --- | --- |
| Rust | manifest policy, unsafe and panic paths, error handling, domain boundaries, async/runtime shape, serialization, dependency policy, and test organization. |
| TypeScript and JavaScript | schema authority, unsafe typing, configuration and decoder boundaries, exports/imports, test integrity, and source shape. |
| Python | suppression and exception policy, typing, subprocess safety, test integrity, and external tool diagnostics. |
| Dart and CFML | language-appropriate structural checks, suppression policy, test integrity, and configured tool diagnostics. |
| Common repository policy | waivers, rule registry integrity, CI hygiene, secrets, generated artifacts, documentation, portability, package determinism, and required tests. |

Policy is evaluated by the Rust workspace and designated language or tool
adapters. Implementation detail does not change the acceptance rule: a
finding is repaired, proved in scope, and rechecked rather than bypassed.

## Test organization

Rust crates use external files under `tests/` by default. The typed
`inlineTestPolicy` makes source-level `#[cfg(test)]` modules explicit:
`forbid` blocks, `warn` reports an advisory, and `allow` is an intentional
exception. External `tests/` files are not inline-test findings.

## UI and coordination

The optional Tauri UI is a presentation layer over Rust-owned actions; CLI and
MCP use do not require it. Coordination is optional and keeps ledger state and
exact-file claims outside the target repository.
