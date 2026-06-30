# Codex MCP And Skill Setup

This is the important part for Codex. MCP setup must point to the Ocentra
Enforcer install path, not to the target repo being scanned.

## Required Paths

Example Windows install:

```text
Enforcer repo: E:\ocentra-enforcer
MCP server:    E:\ocentra-enforcer\mcp\rust-rules-mcp.mjs
Target repo:   C:\path\to\your-project
```

Example macOS/Linux install:

```text
Enforcer repo: ~/tools/ocentra-enforcer
MCP server:    ~/tools/ocentra-enforcer/mcp/rust-rules-mcp.mjs
Target repo:   ~/src/your-project
```

## Preferred Setup: Enforcer Installer

Run this from the enforcer install path. It updates target repo wiring and
Codex Desktop's global MCP config in one idempotent flow.

```powershell
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install --root C:/path/to/target-repo --profile strict --dry-run
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install --root C:/path/to/target-repo --profile strict
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex doctor --root C:/path/to/target-repo
```

macOS/Linux:

```bash
node ~/tools/ocentra-enforcer/scripts/rust-rules.mjs codex install --root ~/src/target-repo --profile strict --dry-run
node ~/tools/ocentra-enforcer/scripts/rust-rules.mjs codex install --root ~/src/target-repo --profile strict
node ~/tools/ocentra-enforcer/scripts/rust-rules.mjs codex doctor --root ~/src/target-repo
```

The installer writes a backup before changing `~/.codex/config.toml` or
`%USERPROFILE%\.codex\config.toml`. Start a new Codex thread after installing.
If the tool does not appear, restart the Codex app.

## Optional Setup: Codex CLI

Use this only if you want to manage MCP entries through the Codex CLI directly:

```powershell
codex mcp add ocentra-enforcer -- node E:/ocentra-enforcer/mcp/rust-rules-mcp.mjs
codex mcp get ocentra-enforcer
codex mcp list
```

## Manual Setup: `config.toml`

Use this when `codex mcp add` is unavailable or gives a broken config.

Edit:

```text
%USERPROFILE%\.codex\config.toml
```

Add:

```toml
[mcp_servers.ocentra-enforcer]
command = "node"
args = ["E:/ocentra-enforcer/mcp/rust-rules-mcp.mjs"]
startup_timeout_sec = 20
enabled = true
```

macOS/Linux path:

```text
~/.codex/config.toml
```

```toml
[mcp_servers.ocentra-enforcer]
command = "node"
args = ["/home/YOU/tools/ocentra-enforcer/mcp/rust-rules-mcp.mjs"]
startup_timeout_sec = 20
enabled = true
```

Use forward slashes in TOML paths on Windows. They avoid backslash escaping
mistakes.

## Project `.mcp.json` Setup

For repo-local MCP config, create or merge this in the target repo:

```json
{
  "mcpServers": {
    "ocentra-enforcer": {
      "command": "node",
      "args": ["E:/ocentra-enforcer/mcp/rust-rules-mcp.mjs"]
    }
  }
}
```

The enforcer repo also has its own `.mcp.json`, but target repos should use an
absolute path to the installed enforcer.

## Validate MCP Directly

From the enforcer repo:

```powershell
npm run mcp:smoke
node E:/ocentra-enforcer/scripts/mcp-smoke.mjs --root C:/path/to/target-repo --profile strict --file Cargo.toml
node E:/ocentra-enforcer/scripts/mcp-smoke.mjs --root C:/path/to/target-repo --profile strict --file Cargo.toml --framing ndjson
```

Expected output includes:

```json
{
  "ok": true,
  "serverInfo": {
    "name": "ocentra-enforcer"
  }
}
```

## Validate MCP From Codex

In a new Codex thread, ask:

```text
Use the ocentra-enforcer MCP server. Call ocentra_enforcer_route for root C:/path/to/target-repo, profile strict, scope files, files ["Cargo.toml"]. Then summarize the docs and rules returned.
```

Expected behavior:

- Codex sees `ocentra_enforcer_route`.
- The tool returns compact JSON.
- `Cargo.toml` routes to Rust toolchain/Cargo, dependency, and common security docs.
- Unknown files return no detailed docs instead of the whole rulebook.

## Validate Named Checks From Codex

Ask Codex:

```text
Use the ocentra-enforcer MCP server. Call ocentra_enforcer_check for root C:/path/to/target-repo, profile strict, check "source-shape", scope workspace. Return only the compact JSON summary.
```

For migrated Ocentra Parent-style checks, use:

```text
Call ocentra_enforcer_check for root C:/path/to/target-repo, profile strict, check "no-zod-source", scope files, files ["src/index.ts"].
```

Expected behavior:

- Codex sees `ocentra_enforcer_check`.
- The tool runs from the Enforcer install path but targets the passed `root`.
- The result is a compact `check` report with `violations`, `warnings`, `bySeverity`, and exact rule IDs.

## Validate Harness From Codex

Ask Codex to run a small command through the harness:

```text
Use the ocentra-enforcer MCP server. Call ocentra_enforcer_run for root C:/path/to/target-repo with tool "node" and command ["node", "--version"]. Then call ocentra_enforcer_run_status for the same root.
```

For real checks, prefer:

```text
Call ocentra_enforcer_run for root C:/path/to/target-repo, tool "tsc", command ["npx", "tsc", "--noEmit", "--pretty", "false"]. If it fails, call ocentra_enforcer_last_failure before reading any raw artifact.
```

## Skill Setup

MCP is the important part. The skill is optional but useful for routing behavior.

The bundled canonical skill currently lives at:

```text
E:\ocentra-enforcer\skills\ocentra-enforcer\SKILL.md
```

To install it as a user skill on Windows:

```powershell
$skillRoot = "$env:USERPROFILE\.codex\skills\ocentra-enforcer"
New-Item -ItemType Directory -Force -Path $skillRoot
Copy-Item -Recurse -Force E:\ocentra-enforcer\skills\ocentra-enforcer\* $skillRoot
```

Then start a new Codex thread. The canonical skill name is `ocentra-enforcer`.
`rust-rules-hard-gate` remains a temporary compatibility alias for older prompts.

## Common MCP Failures

`codex mcp list` does not show `ocentra-enforcer`:

- Run `node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install --root C:/path/to/target-repo --profile strict`.
- Run `node E:/ocentra-enforcer/scripts/rust-rules.mjs codex doctor --root C:/path/to/target-repo`.
- Check `%USERPROFILE%\.codex\config.toml`.
- Restart Codex.

Server works but Codex still does not expose tools:

- Run both `npm run mcp:smoke` and `npm run mcp:smoke:ndjson` from the Enforcer repo.
- If smoke passes, the server protocol is healthy and the remaining issue is Codex app config/reload.
- If only `mcp:smoke:ndjson` fails, report an MCP framing regression.

MCP server starts but tools do not appear:

- Run `node E:/ocentra-enforcer/scripts/mcp-smoke.mjs --root E:/ocentra-enforcer --file Cargo.toml`.
- Confirm Node.js 20+ is available with `node --version`.
- Confirm `npm install` was run in `E:\ocentra-enforcer`.

MCP tools appear but scans target the wrong repo:

- Always pass `root` in tool arguments.
- Do not rely on MCP server current working directory.
- Use `profile` for pack policy or `configPath` for target repo policy.

Harness command fails but no useful diagnostics appear:

- Call `ocentra_enforcer_last_failure` first.
- If compact diagnostics are insufficient, call `ocentra_enforcer_artifact` with `artifact: "stderr"` and a small `limitBytes`.
- Native tool JSON is preferred: Cargo `--message-format=json`, ESLint `--format json`, Ruff `--output-format json`, Pyright `--outputjson`.

Windows path issues:

- Prefer forward slashes in Codex/TOML/JSON: `E:/ocentra-enforcer/...`.
- Quote paths containing spaces.
- Do not use relative MCP paths in global config.

Profile errors:

- `profile: "strict"` is built in.
- `profile: "ocentra-parent"` uses `profiles/ocentra-parent.json`.
- For project-specific policy, pass `configPath` instead of `profile`.
