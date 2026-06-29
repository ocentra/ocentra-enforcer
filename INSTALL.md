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

## 3. Wire Codex MCP

Read [docs/CODEX_SETUP.md](docs/CODEX_SETUP.md). Use the CLI path first:

```powershell
codex mcp add ocentra-enforcer -- node E:/ocentra-enforcer/mcp/rust-rules-mcp.mjs
codex mcp get ocentra-enforcer
codex mcp list
```

If `codex mcp add` is not available or does not work, use the manual
`%USERPROFILE%\.codex\config.toml` method in [docs/CODEX_SETUP.md](docs/CODEX_SETUP.md).

## 4. Wire A Target Repo

Run a dry-run first:

```powershell
node E:/ocentra-enforcer/scripts/rust-rules.mjs init --root C:/path/to/target-repo --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
```

For Ocentra Parent:

```powershell
node E:/ocentra-enforcer/scripts/rust-rules.mjs init --root E:/OcentraParent --profile ocentra-parent --adapters codex,mcp,precommit,github-actions --dry-run
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
```

If these pass, the enforcer repo is installed and can validate that target repo.

## Install Model Decision

Use this order:

1. Git clone plus Codex MCP, recommended today.
2. npm global/package install, after package publishing exists.
3. Git submodule, only when the target project requires source pinning.

Do not copy the enforcer source into every target repo. Target repos should keep
thin config/wiring only.
