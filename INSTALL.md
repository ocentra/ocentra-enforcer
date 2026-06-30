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
npm run proof:smoke
npm run proof:run:smoke
```

Expected result:

- `npm test` passes.
- `npm run mcp:smoke` prints JSON with `"ok": true`.
- The listed MCP tools include `ocentra_enforcer_route`,
  `ocentra_enforcer_scan`, `ocentra_enforcer_doctor`, and
  `ocentra_enforcer_explain`, plus coordination tools such as
  `ocentra_enforcer_coordination_presence`,
  `ocentra_enforcer_coordination_sync`, and
  `ocentra_enforcer_coordination_peer`, plus proof tools such as
  `ocentra_enforcer_proof_route`, `ocentra_enforcer_proof_run`, and
  `ocentra_enforcer_proof_claim`.

## 3. Wire Codex Globally

Read [docs/CODEX_SETUP.md](docs/CODEX_SETUP.md). Use the Enforcer installer
first. It writes Codex Desktop's global MCP server config, installs the
canonical user skill, and creates or updates a managed Enforcer block in
`~/.codex/AGENTS.md` or `%USERPROFILE%\.codex\AGENTS.md`.

```powershell
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install --dry-run
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex doctor
```

By default this configures the per-PC ledger root as
`E:/ocentra-enforcer/.ledger`; hubs live below it, for example
`E:/ocentra-enforcer/.ledger/ocentra-parent`. Use `--ledger-root <path>` only
when this machine should use a different synced ledger folder.

You can also pass a target repo when you want project-local wiring generated at
the same time:

```powershell
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install --root C:/Users/sujan/.codex/worktrees/ocentra-parent-codex-a/OcentraParent --profile ocentra-parent --dry-run
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex install --root C:/Users/sujan/.codex/worktrees/ocentra-parent-codex-a/OcentraParent --profile ocentra-parent
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex doctor --root C:/Users/sujan/.codex/worktrees/ocentra-parent-codex-a/OcentraParent
```

For any worktree, the target root is the worktree being validated, not some
other checkout. Coordination/hub/lane state is Enforcer-managed Codex harness
state and lives under the installed Enforcer ledger root, not inside a product
repo.

Then restart Codex Desktop or start a new thread so the app reloads MCP
servers. If local CLI config parsing is blocked by unrelated config settings,
verify with:

```powershell
codex -c service_tier='"fast"' mcp list
```

If the installer cannot write the config, use the manual
`%USERPROFILE%\.codex\config.toml` method in [docs/CODEX_SETUP.md](docs/CODEX_SETUP.md).

To remove only Enforcer-managed Codex wiring:

```powershell
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex uninstall --dry-run
node E:/ocentra-enforcer/scripts/rust-rules.mjs codex uninstall
```

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
node E:/ocentra-enforcer/scripts/rust-rules.mjs proof route --root C:/path/to/target-repo --files Cargo.toml --json
node E:/ocentra-enforcer/scripts/rust-rules.mjs proof run --root C:/path/to/target-repo --proof PROOF-COMMAND-GENERIC --json -- node --version
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
