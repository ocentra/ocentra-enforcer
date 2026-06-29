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

## Preferred Setup: Codex CLI

Run this once on the machine:

```powershell
codex mcp add ocentra-enforcer -- node E:/ocentra-enforcer/mcp/rust-rules-mcp.mjs
codex mcp get ocentra-enforcer
codex mcp list
```

macOS/Linux:

```bash
codex mcp add ocentra-enforcer -- node ~/tools/ocentra-enforcer/mcp/rust-rules-mcp.mjs
codex mcp get ocentra-enforcer
codex mcp list
```

Start a new Codex thread after adding the MCP server. If the tool does not
appear, restart the Codex app.

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
- `Cargo.toml` routes to Rust toolchain/Cargo and dependency docs.
- Unknown files return no detailed docs instead of the whole rulebook.

## Skill Setup

MCP is the important part. The skill is optional but useful for routing behavior.

The bundled skill currently lives at:

```text
E:\ocentra-enforcer\skills\rust-rules-hard-gate\SKILL.md
```

To install it as a user skill on Windows:

```powershell
$skillRoot = "$env:USERPROFILE\.codex\skills\rust-rules-hard-gate"
New-Item -ItemType Directory -Force -Path $skillRoot
Copy-Item -Recurse -Force E:\ocentra-enforcer\skills\rust-rules-hard-gate\* $skillRoot
```

Then start a new Codex thread. The skill name is `rust-rules-hard-gate` for
compatibility, but its body tells Codex to use Ocentra Enforcer.

## Common MCP Failures

`codex mcp list` does not show `ocentra-enforcer`:

- Run `codex mcp add ...` again with an absolute path.
- Check `%USERPROFILE%\.codex\config.toml`.
- Restart Codex.

MCP server starts but tools do not appear:

- Run `node E:/ocentra-enforcer/scripts/mcp-smoke.mjs --root E:/ocentra-enforcer --file Cargo.toml`.
- Confirm Node.js 20+ is available with `node --version`.
- Confirm `npm install` was run in `E:\ocentra-enforcer`.

MCP tools appear but scans target the wrong repo:

- Always pass `root` in tool arguments.
- Do not rely on MCP server current working directory.
- Use `profile` for pack policy or `configPath` for target repo policy.

Windows path issues:

- Prefer forward slashes in Codex/TOML/JSON: `E:/ocentra-enforcer/...`.
- Quote paths containing spaces.
- Do not use relative MCP paths in global config.

Profile errors:

- `profile: "strict"` is built in.
- `profile: "ocentra-parent"` uses `profiles/ocentra-parent.json`.
- For project-specific policy, pass `configPath` instead of `profile`.
