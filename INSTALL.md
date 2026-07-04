# Install Enforcer

<!-- ai-dense -->
```yaml
model: one native Rust binary (`enforcer`) is both the MCP stdio server and the CLI
install_default: user/global scope, one install per machine per harness, zero per-repo config
install_command: "enforcer install [--dry-run] [--root <repo>] [--profile <name>] [--scope user|project]"
consumer_ci: install.sh/install.ps1 (zero-Rust-toolchain) or the composite `.github/actions/enforcer-scan` GitHub Action or the npm wrapper; see docs/TARGET_REPO_WIRING.md
build_from_source_fallback: "cargo build --release" (only for developing the enforcer itself, or when no matching prebuilt binary exists)
harnesses: 11 adapters (codex, claude, cursor, windsurf, gemini, antigravity, opencode, aider, kilocode, kiro, generic .mcp.json) — Codex is one of eleven, never the reference target
update: binary swap only ("enforcer update" or the harness prompt "update enforcer") — no repo pull, no toolchain
```
<!-- /ai-dense -->

This guide is for a fresh machine, or a target project that has never used the
enforcer before. The enforcer ships as **one native binary** per platform
(Windows/macOS/Linux, including musl and Apple Silicon) — there is no runtime
toolchain to install for consumers.

## 1. Get The Binary

Preferred: download the released binary and register it with your harness in
one step.

```powershell
irm https://<release-host>/install.ps1 | iex
enforcer install --dry-run
enforcer install
```

macOS/Linux:

```bash
curl -fsSL https://<release-host>/install.sh | sh
enforcer install --dry-run
enforcer install
```

Building from source is a documented fallback only — for developing the
enforcer itself, or a platform without a published binary:

```bash
git clone <this-repo-url> enforcer-rust
cd enforcer-rust
cargo build --release --workspace
```

## 2. Validate The Install

```bash
enforcer doctor
enforcer scan --root . --workspace
enforcer proof run --proof PROOF-COMMAND-GENERIC --json -- enforcer --version
```

Expected result:

- `enforcer doctor` reports the MCP registration healthy for the detected
  harness(es) and the ledger root resolved.
- `enforcer scan --workspace` runs clean against this repo's own crates
  (native dogfood — the enforcer validates itself).
- The proof run produces a structured, artifact-backed result under
  `.enforce/proofs`.

## 3. Wire Your Harness Globally

`enforcer install` with no `--scope` flag resolves to **user/global** — one
install per machine per harness, so every repo you open already has the
enforcer's MCP server registered, with no per-repo config file. Read
[docs/CODEX_SETUP.md](docs/CODEX_SETUP.md) for the per-harness detail (Codex
setup shown as one concrete example; the same shape applies to every
supported harness's adapter).

```bash
enforcer install --dry-run
enforcer install
enforcer doctor
```

The installer writes each harness's MCP server entry pointing at the
**absolute path** of the installed binary — a relative path cannot resolve
from an arbitrary repo's working directory. Existing harness config is
backed up before it is changed.

You can also pass a target repo when you want project-local wiring generated
at the same time:

```bash
enforcer install --root <target-repo> --profile strict --dry-run
enforcer install --root <target-repo> --profile strict
enforcer doctor --root <target-repo>
```

For any worktree, the target root is the worktree being validated, not some
other checkout. Coordination/hub/lane state is enforcer-managed harness
state and lives under the installed enforcer's own ledger root, not inside a
product repo.

Restart your harness (or start a new session) so it reloads MCP servers
after install.

To remove only the enforcer-managed wiring:

```bash
enforcer uninstall --dry-run
enforcer uninstall
```

## 4. Add Hooks And CI For A Target Repo

Run a dry-run first:

```bash
enforcer init --root <target-repo> --profile strict --adapters precommit,github-actions --dry-run
```

Then follow [docs/TARGET_REPO_WIRING.md](docs/TARGET_REPO_WIRING.md).

## 5. Prove The Target Repo Works

CLI smoke:

```bash
enforcer doctor --root <target-repo> --profile strict --workspace
enforcer scan --root <target-repo> --profile strict --files Cargo.toml
enforcer proof route --root <target-repo> --files Cargo.toml --json
enforcer proof run --root <target-repo> --proof PROOF-COMMAND-GENERIC --json -- node --version
```

MCP smoke: ask your harness to call `mcp__enforcer__route` for the target
root, profile `strict`, scope `files`, files `["Cargo.toml"]`, and confirm
the response is `serverInfo.name == "enforcer"` with a compact routed
result, not the full rule corpus.

If these pass, the enforcer is installed and can validate that target repo.
Restart your harness after any config change.

## Install Model Decision

Use this order:

1. `enforcer install` from a released binary — recommended for every normal
   use.
2. `cargo build --release` from source — only for developing the enforcer
   itself or an unreleased platform.
3. Git submodule pinning — only when a target project genuinely requires
   source pinning of the enforcer itself; not the default model.

Do not copy the enforcer source into every target repo. Target repos should
keep thin config/wiring only.
