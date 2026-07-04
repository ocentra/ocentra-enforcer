# Per-Harness MCP And Skill Setup

<!-- ai-dense -->
```yaml
scope: "neutral install/setup doc; Codex is ONE of 11 harness adapters, shown here as the worked example -- same shape applies to claude/cursor/windsurf/gemini/antigravity/opencode/aider/kilocode/kiro/generic"
binary: single native `enforcer` executable = MCP stdio server AND CLI; install points each harness's MCP config at this binary's absolute path
default_scope: user/global (mcpServers map in the harness's user-level config), never a per-repo file, so any repo already has it
canonical_command: "enforcer install [--root <repo>] [--profile <name>] [--dry-run]"
doctor_command: "enforcer doctor [--root <repo>]"
server_name: enforcer (never a harness-branded name; tools resolve as mcp__enforcer__*)
common_failure_mode: "path mistakes -- absolute path to the installed binary, not a relative path or repo checkout path"
harness_config_locations:
  codex: "~/.codex/config.toml [mcp_servers.enforcer]"
  claude: "~/.claude.json top-level mcpServers map (same registry codebase-memory-mcp uses)"
  generic: "the harness's own user-level equivalent of .mcp.json, resolved by the c02 autodetect adapter"
validation: "enforcer doctor; ask the harness to call mcp__enforcer__route and confirm serverInfo.name == enforcer"
```
<!-- /ai-dense -->

This document is per-harness MCP and skill setup. **Codex is used below as
the worked example** because its manual/CLI setup paths are the most
detailed to document, but the enforcer supports 11 harness adapters
(Claude Code, Codex, Cursor, Windsurf, Gemini, Antigravity, OpenCode, Aider,
KiloCode, Kiro, and any generic `.mcp.json`-based harness) and none of them
is the canonical/reference/default target. Setup must point to the
**installed enforcer binary**, not to the target repo being scanned.

## Preferred Setup: The Enforcer Installer

Run this once per machine per harness. It auto-detects installed harnesses,
writes each one's MCP server entry, installs the user skill, and creates or
updates a managed enforcer block in that harness's global agent
instructions. A target repo is optional; pass `--root` only when you also
want project-local wiring generated.

```bash
enforcer install --dry-run
enforcer install
enforcer doctor
```

The default ledger root lives alongside the installed binary; hub folders
live under it. Pass `--ledger-root <path>` during install to use a different
synced folder on a machine.

The installer writes a backup before changing any harness's config file.
Restart the harness (or start a new session) after installing. If the tool
does not appear, restart the harness app.

To generate target repo wiring at the same time:

```bash
enforcer install --root <target-repo> --profile strict
enforcer doctor --root <target-repo>
```

To remove only the enforcer-managed pieces for a harness:

```bash
enforcer uninstall --dry-run
enforcer uninstall
```

## Worked Example: Codex

Codex's config format (`config.toml`) needs the most manual detail of any
adapter, which is why it is spelled out here. The same install/doctor
commands above cover Codex automatically; the manual steps below are a
fallback only.

### Optional: Codex CLI

Use this only if you want to manage MCP entries through the Codex CLI
directly:

```bash
codex mcp add enforcer -- <path-to-installed-enforcer-binary>
codex mcp get enforcer
codex mcp list
```

### Manual Fallback: `config.toml`

Use this when `codex mcp add` is unavailable or gives a broken config. Edit
the user-level `config.toml` for your platform (Windows:
`%USERPROFILE%\.codex\config.toml`; macOS/Linux: `~/.codex/config.toml`) and
add:

```toml
[mcp_servers.enforcer]
command = "<absolute-path-to-installed-enforcer-binary>"
args = ["serve"]
startup_timeout_sec = 20
enabled = true
```

Use forward slashes in TOML paths on Windows to avoid backslash-escaping
mistakes; use `<absolute-path-to-installed-enforcer-binary>` — the actual
path the installer reports, never a hardcoded example path.

### Project `.mcp.json` Setup (any harness that reads it)

For repo-local MCP config, create or merge this in the target repo, using
the same absolute installed-binary path:

```json
{
  "mcpServers": {
    "enforcer": {
      "command": "<absolute-path-to-installed-enforcer-binary>",
      "args": ["serve"]
    }
  }
}
```

Target repos should point at the installed enforcer binary; they should not
copy the enforcer's own source or its own `.mcp.json`.

## Validate MCP Directly

```bash
enforcer doctor
```

Expected output includes a healthy `serverInfo.name: "enforcer"` handshake
and the resolved harness/ledger paths.

## Validate MCP From The Harness

In a new session, ask:

```text
Use the enforcer MCP server. Call mcp__enforcer__route for root <target-repo>, profile strict, scope files, files ["Cargo.toml"]. Then summarize the docs and rules returned.
```

Expected behavior:

- The harness sees `mcp__enforcer__route`.
- The tool returns compact JSON.
- `Cargo.toml` routes to Rust toolchain/Cargo, dependency, and common
  security rule records.
- Unknown files return no detailed rules instead of the whole rule corpus.

For coordination/presence, ask:

```text
Use the enforcer MCP server. Call mcp__enforcer__coordination_presence for hub my-hub and summarize active machines, worktrees, lanes, harness threads, exact-file claims, unread inbox counts, and stale/offline rows.
```

For LAN/WAN sync health, ask:

```text
Use the enforcer MCP server. Call mcp__enforcer__coordination_peer with action "list", then call mcp__enforcer__coordination_streams for the same hub. Do not read raw stream files unless the compact manifest is insufficient.
```

For proof routing and PR-ready claims, ask:

```text
Use the enforcer MCP server. Call mcp__enforcer__proof_route for root <target-repo> with files ["package.json"]. Then call mcp__enforcer__proof_inventory for the same root and summarize proof families and device/manual-required counts.
```

For a fresh proof run:

```text
Call mcp__enforcer__proof_run for root <target-repo>, proofId "PROOF-COMMAND-GENERIC", and command ["node", "--version"]. Then call mcp__enforcer__proof_claim for the same root and proofId before making any PR-ready claim.
```

Expected behavior:

- The harness sees `mcp__enforcer__proof_route`, `mcp__enforcer__proof_run`,
  and `mcp__enforcer__proof_claim`.
- Proof output is stored under the target repo at `.enforce/proofs`.
- Raw proof artifacts are not read unless `mcp__enforcer__proof_artifact` is
  explicitly requested.

## Validate Named Checks

Ask the harness:

```text
Use the enforcer MCP server. Call mcp__enforcer__check for root <target-repo>, profile strict, check "source-shape", scope workspace. Return only the compact JSON summary.
```

Expected behavior:

- The harness sees `mcp__enforcer__check`.
- The tool runs from the installed enforcer binary but targets the passed
  `root`.
- The result is a compact `check` report with `violations`, `warnings`,
  `bySeverity`, and exact rule IDs.

## Validate Harness Diagnostics

Ask the harness to run a small command through the harness diagnostics tool:

```text
Use the enforcer MCP server. Call mcp__enforcer__run for root <target-repo> with tool "cargo" and command ["cargo", "--version"]. Then call mcp__enforcer__run_status for the same root.
```

## Skill Setup

The preferred installer above copies the canonical skill to the harness's
user-level skill directory and writes global agent instructions. Manual
copying is only needed if you intentionally use `--no-skill`. The canonical
skill name is `enforcer` (see [skills/enforcer/SKILL.md](../skills/enforcer/SKILL.md)).

## Common MCP Failures

The MCP server does not show up in `list`:

- Run `enforcer install --root <target-repo> --profile strict`.
- Run `enforcer doctor --root <target-repo>`.
- Check the harness's own user-level MCP config file.
- Restart the harness.

Server works but the harness still does not expose tools:

- Run `enforcer doctor` to separate MCP server protocol failures from
  harness app config failures before blaming the rule engine.

Tools appear but scans target the wrong repo:

- Always pass `root` in tool arguments.
- Do not rely on the MCP server's own current working directory.
- Use `profile` for pack policy or `configPath` for target repo policy.

Harness command fails but no useful diagnostics appear:

- Call `mcp__enforcer__last_failure` first.
- If compact diagnostics are insufficient, call `mcp__enforcer__artifact`
  with `artifact: "stderr"` and a small `limitBytes`.
- Native tool JSON is preferred: Cargo `--message-format=json`, ESLint
  `--format json`, Ruff `--output-format json`, Pyright `--outputjson`.

Windows path issues:

- Prefer forward slashes in TOML/JSON config on Windows.
- Quote paths containing spaces.
- Do not use relative MCP paths in global config; use the absolute installed
  binary path the installer reports.

Profile errors:

- `profile: "strict"` is built in.
- For project-specific policy, pass `configPath` instead of `profile`.
