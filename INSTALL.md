# Install Enforcer

<!-- ai-dense -->
```yaml
model: one native Rust binary (`enforcer`) provides the CLI and MCP stdio server
current_install_command: "enforcer install"
install_scope: user-level harness registrations
post_install: internal read-only adapter health check
harness_adapters: "Antigravity, Claude, Codex, Cursor, Gemini, KiloCode, Kiro, Windsurf, Zed, Aider, OpenCode"
availability: build from source unless a separately verified release artifact is available
```
<!-- /ai-dense -->

This guide describes only behavior verified in the current Rust source and
CLI. It does not claim that a public binary distribution exists.

## Build The Binary

For development or a source installation:

```powershell
cargo build --release -p enforcer-cli
target/release/enforcer --help
```

Use the platform-specific executable suffix where required. If you obtain a
binary through another channel, verify its origin and run its own `--help`
before installing it.

## Register Supported Harnesses

The current installer has one public form and accepts no flags:

```powershell
enforcer install
```

It resolves the running binary to an absolute path, plans and applies
user-level registrations for the supported adapters, and then runs the
installer's internal read-only health check. The operation is idempotent:
running it again should preserve already-correct registrations.

The supported adapter set is Antigravity, Claude, Codex, Cursor, Gemini,
KiloCode, Kiro, Windsurf, Zed, Aider, and OpenCode. Some adapters are
CLI-only; an adapter's presence does not imply that every MCP tool is wired.

Restart each harness after installation so it reloads its configuration.

## Validate The Installed Binary

Confirm the public command surface:

```powershell
enforcer --help
enforcer serve --help
enforcer scan --help
```

From the target repository, run an explicit scope:

```powershell
enforcer scan Cargo.toml
enforcer verify --mode local --all
```

For MCP, start a new harness session and call
`ocentra_enforcer_mcp_status`. The current Rust MCP server also wires
coordination status, exact-path coordination claim, and UI launch/status.
Registered route, scan, check, diagnostics, proof, and broader coordination
tool contracts are not yet connected to engine delegates.

## Current Limitations

The native CLI does not currently expose public `doctor`, `update`,
`uninstall`, or `init` commands. `install` does not accept `--dry-run`,
`--root`, `--profile`, `--scope`, or `--ledger-root`. The visible `plan`,
`proof`, and `coordination` command groups are reserved but not wired.

Checked-in shell installers, package wrappers, and automation definitions are
development artifacts unless the distribution channel using them has been
independently verified. Do not construct install URLs or consumer CI promises
from their presence alone.

For target-repository validation scopes, see
[Target Repository Wiring](docs/TARGET_REPO_WIRING.md).
