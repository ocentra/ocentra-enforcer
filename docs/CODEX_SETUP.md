# MCP And Skill Setup

<!-- ai-dense -->
```yaml
audience: "any MCP-capable harness; Codex is one supported development configuration"
binary: "CLI and MCP server share one executable; MCP starts with `enforcer serve`"
development: "temporary cargo-run configuration against a disposable target"
installed: "use an absolute binary path after the release binary is verified"
safe_rollout: "keep a known-good installation active until a replacement proves its contract"
```
<!-- /ai-dense -->

This guide describes the Rust implementation when it is built from this
repository. It does not ask you to replace a working Enforcer installation
with an unverified development build.

## Choose the right mode

| Situation | Use |
| --- | --- |
| You need a stable, already-installed MCP server | Keep using that installation and its existing configuration. |
| You are developing this Rust workspace | Run the CLI through Cargo and test it against a disposable target. |
| A release binary has been installed and verified | Point the harness at that absolute binary path. |

The MCP server and CLI share one executable. The server is started with
`enforcer serve`; do not point a harness at a repository checkout path or a
relative executable path.

## Build and verify the development CLI

From the repository root:

```powershell
cargo check --workspace
cargo test --workspace
cargo run -p enforcer-cli -- --help
cargo run -p enforcer-cli -- serve --help
```

Use a fresh harness session after changing its MCP configuration.

## Development-only Codex configuration

For a local development test, add a temporary entry to your user-level Codex
configuration (`%USERPROFILE%\.codex\config.toml` on Windows or
`~/.codex/config.toml` on macOS/Linux):

```toml
[mcp_servers.enforcer-dev]
command = "cargo"
args = ["run", "-p", "enforcer-cli", "--", "serve"]
cwd = "<absolute-path-to-this-repository>"
startup_timeout_sec = 30
enabled = true
```

Use the real absolute repository path for `cwd`. This entry is intentionally
named `enforcer-dev` so it cannot be confused with a stable installation.
Remove it when the development test is complete.

## Installed binary configuration

After a release binary is built, installed, and verified, configure the
harness with the absolute binary path:

```toml
[mcp_servers.enforcer]
command = "<absolute-path-to-enforcer-binary>"
args = ["serve"]
startup_timeout_sec = 30
enabled = true
```

On Windows, forward slashes in TOML paths avoid escaping mistakes. Do not use
a relative path or assume the MCP process starts in the target repository.

Other MCP-capable harnesses use the same executable contract: an absolute
`enforcer` command with `serve` as its argument. Their configuration file and
restart procedure are harness-specific; verify the registered command in that
harness before relying on it.

## Verify the connection

Start a new harness session and ask it to call the route tool for one explicit
file in a disposable target repository. Confirm that it returns a compact
structured response and that the target root is the path you supplied.

When diagnosing a failure, use the MCP diagnostics or the CLI command that
matches the operation before reading raw logs. The normal workflow is:

1. Route the file or scope.
2. Run the smallest relevant check.
3. Repair the reported condition.
4. Repeat the check, then run the wider verification gate.

## Skill setup

The repository skill documents how agents should route and validate work. A
stable installation may copy that skill into the harness's user-level skill
directory. During development, read the repository's `AGENTS.md` and skill
material directly rather than copying a partially validated implementation
into global configuration.

## Common problems

- **The tools do not appear:** restart the harness after changing its config
  and confirm the configured `command`, `args`, and `cwd` are absolute and
  valid.
- **The server targets the wrong repository:** always pass the intended target
  root in the MCP request; do not rely on the server process's working
  directory.
- **The development server is slow to start:** build the workspace once with
  Cargo before starting the harness, or increase `startup_timeout_sec` for the
  temporary development entry.
- **Diagnostics are too large:** use the structured failure summary first and
  inspect a bounded artifact only when that summary is insufficient.
