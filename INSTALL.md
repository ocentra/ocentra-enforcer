# Install Ocentra Enforcer

This guide is for a fresh machine, a fresh Codex thread, or a target project
that has never used Ocentra Enforcer before.

## 1. Clone The Enforcer Repo

Choose a stable tool path outside the target project. On Windows, prefer `E:\`
or another non-system drive if available.

```powershell
git clone https://github.com/ocentra/ocentra-enforcer.git E:\ocentra-enforcer
Set-Location E:\ocentra-enforcer
npm install
```

macOS/Linux example:

```bash
git clone https://github.com/ocentra/ocentra-enforcer.git ~/tools/ocentra-enforcer
cd ~/tools/ocentra-enforcer
npm install
```

## 2. Validate The Install

Run these from the enforcer repo root:

```bash
npm test
npm run rust:rules:scan
npm run rust:rules
npm run mcp:smoke
```

Expected result:

- `npm test` passes.
- `npm run mcp:smoke` prints JSON with `"ok": true`.
- The listed MCP tools include `ocentra_enforcer_route`,
  `ocentra_enforcer_scan`, `ocentra_enforcer_doctor`, and
  `ocentra_enforcer_explain`.

## 3. Wire Codex MCP And Target Repo

Read [docs/CODEX_SETUP.md](docs/CODEX_SETUP.md). Use the Enforcer installer
first. It writes both the target repo wiring and Codex Desktop's global MCP
server config.

```powershell
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install --root C:/path/to/target-repo --profile strict --dry-run
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install --root C:/path/to/target-repo --profile strict
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex doctor --root C:/path/to/target-repo
```

For this Ocentra Parent lane, the target root is the worktree, not the main
checkout:

```powershell
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install --root C:/Users/sujan/.codex/worktrees/ocentra-parent-codex-a/OcentraParent --profile ocentra-parent --dry-run
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install --root C:/Users/sujan/.codex/worktrees/ocentra-parent-codex-a/OcentraParent --profile ocentra-parent
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex doctor --root C:/Users/sujan/.codex/worktrees/ocentra-parent-codex-a/OcentraParent
```

Then restart Codex Desktop or start a new thread so the app reloads MCP
servers. If local CLI config parsing is blocked by unrelated config settings,
verify with:

```powershell
codex -c service_tier='"fast"' mcp list
```

If the installer cannot write the config, use the manual
`%USERPROFILE%\.codex\config.toml` method in [docs/CODEX_SETUP.md](docs/CODEX_SETUP.md).

## 4. Add Hooks And CI For A Target Repo

Run a dry-run first:

```powershell
node E:/ocentra-enforcer/scripts/rust-rules.mjs init --root C:/path/to/target-repo --profile strict --adapters precommit,github-actions --dry-run
```

Then follow [docs/TARGET_REPO_WIRING.md](docs/TARGET_REPO_WIRING.md).

## 5. Prove The Target Repo Works

CLI smoke:

```powershell
node E:/ocentra-enforcer/scripts/rust-rules.mjs doctor --root C:/path/to/target-repo --profile strict --workspace
node E:/ocentra-enforcer/scripts/rust-rules.mjs scan --root C:/path/to/target-repo --profile strict --files Cargo.toml
```

MCP smoke:

```powershell
node E:/ocentra-enforcer/scripts/mcp-smoke.mjs --root C:/path/to/target-repo --profile strict --file Cargo.toml
node E:/ocentra-enforcer/scripts/mcp-smoke.mjs --root C:/path/to/target-repo --profile strict --file Cargo.toml --framing ndjson
```

If these pass, the enforcer repo is installed and can validate that target repo.
`mcp-smoke` verifies the server itself; `codex doctor` verifies Codex's global
config points at that server. Restart Codex Desktop or start a new thread after
config changes.

## Install Model Decision

Use this order:

1. Git clone plus Codex MCP, recommended today.
2. npm global/package install, after package publishing exists.
3. Git submodule, only when the target project requires source pinning.

Do not copy the enforcer source into every target repo. Target repos should keep
thin config/wiring only.
