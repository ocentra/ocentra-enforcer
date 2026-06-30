# Ocentra Enforcer

Standalone enforcement system for humans, CI, Codex skills, and MCP clients.
Rust, TypeScript/JavaScript, Python, common security/generated-artifact guards,
and compact harness diagnostics are implemented as the first reusable platform
slice.

Default consumption is package plus Codex plugin/MCP. Git submodules are allowed
only for projects that need source pinning; they are not the default install
model.

## 1. Hard Gates Over Trust

Rules, AGENTS files, and skill docs are guidance, not enforcement. They help a
strong model choose the right path and they save tokens with indexed routing, but
models can miss rules when context is full, a smaller model is used, or the task
pressure is high. Humans can miss or bypass the same rules.

Ocentra Enforcer is built on zero trust for AI and humans. The point is not to
hope the writer remembers every rule; the harness, hooks, MCP tools, and CI gates
must reject bad code before it is accepted. A normal flow is:

1. Route to the smallest relevant rule docs.
2. Validate the exact file, crate, package, or repo scope.
3. Store compact structured diagnostics instead of forcing agents to read raw
   terminal walls.
4. Hard-fail violations in local runs, pre-commit, PR checks, and CI.

Policy therefore has two layers:

- Indexed rules explain what to do and keep agent context small.
- Validators and harness checks decide whether the work is accepted.

If docs and validators disagree, the hard gate wins. Fix the code, fix the docs,
or strengthen the validator; do not add bypass comments or weaken checks to make
an agent pass.

## 2. Indexed Decision Trees Save Context

Long plans, AGENTS files, workpacks, and rulebooks can consume the same context
the agent needs for the actual implementation. Ocentra Enforcer treats those
documents as routed knowledge, not default reading.

The intended pattern is:

1. Read a small index or decision matrix first.
2. Classify the task by language, files, scope, risk, and command.
3. Open only the rule docs and workpack sections that apply.
4. Fall back to broad reading only when the route is unknown or policy itself is
   being changed.

This gives large models less noise and gives small models a bounded path they can
follow. The route is also machine-readable, so Codex or another MCP client can
ask for the relevant rule IDs and docs before scanning or editing.

## 3. Structured Diagnostics Save Tokens

Raw command output is a poor agent interface. `cargo check`, test runners,
linters, and security tools often produce duplicate lines, progress noise, and
large terminal walls that burn context before the agent reaches the real failure.

Ocentra Enforcer runs commands through a harness that keeps the full raw
artifacts, then emits compact structured data:

- Raw stdout and stderr are preserved for audit and fallback.
- NDJSON events and diagnostics capture the useful facts.
- DuckDB-backed queries are the intended compact retrieval path when available.
- MCP tools expose last failure, diagnostics by run, file, rule, severity, crate,
  package, test, and artifact.

The agent should normally ask the harness for the last failure or scoped
diagnostics instead of reading the full terminal dump. Raw logs remain available
only when the compact result is not enough.

## Commands

```bash
npm test
npm run test:rules
npm run test:mcp
npm run enforcer:init:dry-run
npm run codex:install:dry-run
npm run codex:doctor
npm run enforcer:rules:scan
npm run enforcer:rules
```

Direct CLI forms:

```bash
node scripts/rust-rules.mjs init --root C:/path/to/repo --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
node scripts/rust-rules.mjs codex install --root C:/path/to/repo --profile strict --dry-run
node scripts/rust-rules.mjs codex install --root C:/path/to/repo --profile strict
node scripts/rust-rules.mjs codex doctor --root C:/path/to/repo
node scripts/rust-rules.mjs scan --root C:/path/to/repo --files src/lib.rs
node scripts/rust-rules.mjs scan --root C:/path/to/repo --crate my-crate
node scripts/rust-rules.mjs scan --root C:/path/to/repo --workspace
node scripts/rust-rules.mjs scan --root C:/path/to/repo --base origin/main --head HEAD
node scripts/rust-rules.mjs scan --root C:/path/to/repo --languages typescript,python,common --files src tests
node scripts/rust-rules.mjs check no-zod-source --root C:/path/to/repo --files src/index.ts
node scripts/rust-rules.mjs check validation-bypass --root C:/path/to/repo --files src/index.ts
node scripts/rust-rules.mjs check weak-assertions --root C:/path/to/repo --files tests/example.test.ts
node scripts/rust-rules.mjs check placeholder-implementation --root C:/path/to/repo --files src/index.ts
node scripts/rust-rules.mjs check source-shape --root C:/path/to/repo --workspace
node scripts/rust-rules.mjs check single-source-contracts --root C:/path/to/repo --check-config scripts/check-single-source-contracts.json
node scripts/rust-rules.mjs check sbom --root C:/path/to/repo --output target/security --dry-run
node scripts/rust-rules.mjs cargo --root C:/path/to/repo --crate my-crate
node scripts/rust-rules.mjs doctor --root C:/path/to/repo --workspace
node scripts/rust-rules.mjs explain RR-7.3
node scripts/rust-rules.mjs run --root C:/path/to/repo --tool tsc -- npx tsc --noEmit --pretty false
node scripts/rust-rules.mjs runs last-failure --root C:/path/to/repo --json
```

After package install, the canonical entrypoints are:

```bash
ocentra-enforcer scan --root C:/path/to/project --files src/lib.rs
ocentra-enforcer-mcp
```

Compatibility aliases remain for one Rust-pack release:

```bash
rust-rules scan --root C:/path/to/project --files src/lib.rs
rust-rules-mcp
```

## Install / Init Model

Start with [INSTALL.md](INSTALL.md) for a fresh machine or fresh Codex setup.
Use [docs/CODEX_SETUP.md](docs/CODEX_SETUP.md) for MCP and skill wiring, and
[docs/TARGET_REPO_WIRING.md](docs/TARGET_REPO_WIRING.md) for target repo setup.
For Ocentra Parent migration status, see
[docs/OCENTRA_PARENT_PARITY.md](docs/OCENTRA_PARENT_PARITY.md).

Run the Codex installer from the enforcer install path and pass the target repo
explicitly:

```bash
ocentra-enforcer codex install --root C:/path/to/repo --profile strict --dry-run
ocentra-enforcer codex install --root C:/path/to/repo --profile strict
ocentra-enforcer codex doctor --root C:/path/to/repo
```

This writes target repo Codex/MCP wiring and updates Codex Desktop's global
`config.toml` with an `ocentra-enforcer` MCP server. Existing Codex config is
backed up before it is changed. The installer writes TOML directly because
`codex mcp add` behavior can vary by app/CLI version and has been the most
common setup failure.

For hooks and CI adapters, run init separately:

```bash
ocentra-enforcer init --root C:/path/to/repo --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
```

`--dry-run` prints the exact file plan without writing. The default hook adapter
is a plain Git hook for cross-platform use. Husky is generated only when
requested or when the target repo already uses Husky. Lefthook is opt-in.

MCP runs from the enforcer install path. Target projects always pass `root` plus
either `configPath` for project-specific policy or `profile` for a named pack
policy such as `strict` or `ocentra-parent`.

Example global MCP wiring:

```json
{
  "mcpServers": {
    "ocentra-enforcer": {
      "command": "node",
      "args": ["C:/path/to/ocentra-enforcer/mcp/rust-rules-mcp.mjs"]
    }
  }
}
```

Canonical MCP tools:

```text
ocentra_enforcer_route
ocentra_enforcer_scan
ocentra_enforcer_check
ocentra_enforcer_doctor
ocentra_enforcer_explain
ocentra_enforcer_run
ocentra_enforcer_run_status
ocentra_enforcer_diagnostics
ocentra_enforcer_last_failure
ocentra_enforcer_artifact
ocentra_enforcer_reset_runs
```

Legacy `rust_rules_*` MCP tool aliases remain for one Rust-pack compatibility
release.

MCP setup is intentionally documented in detail because path mistakes are the
most common failure. Run `npm run mcp:smoke`, `npm run mcp:smoke:ndjson`, and
`npm run codex:doctor` from this repo to separate MCP server protocol failures
from Codex app config failures before blaming the rules engine.

## Indexed Rules

Agents should read `rules/INDEX.md` first, call MCP `ocentra_enforcer_route`,
and then open only the returned docs. `docs/RustRules.md` remains the legacy
monolithic reference, not the default context load.

`rules/rules.json` is validated by Effect Schema at runtime and by tests.
JSON-schema-compatible artifacts live under `schemas/json/` for docs, MCP
clients, and non-Effect consumers.

## Profiles And Severity

Profiles decide what runs, what fails, and what is advisory. The default model
is strict about safety, architecture, bypasses, secrets, dependency policy, and
test integrity, while documentation/comment checks are warnings unless a project
opts into making them hard gates.

```json
{
  "profileName": "strict",
  "failOn": ["error"],
  "rules": {
    "DOC-1.1": { "enabled": true, "severity": "warning" },
    "TS-2.1": { "severity": "error" }
  },
  "tools": {
    "cargoDoc": { "enabled": false, "severity": "warning" },
    "cargoDeny": { "enabled": true, "severity": "error" }
  }
}
```

`violations` are findings whose severity is listed in `failOn`; they fail CLI,
MCP, hook, and CI gates. `warnings` are returned in reports but do not fail when
`failOn` is `["error"]`. A project can disable noisy advisory rules with
`"enabled": false`, or upgrade them with `"severity": "error"`.

## Harness Diagnostics

Use `ocentra-enforcer run` or MCP `ocentra_enforcer_run` for cargo, npm, tsc,
ESLint, Ruff, Pyright, mypy, pytest, and similar checks. The harness stores raw
stdout/stderr plus schema-shaped NDJSON under the target repo:

```text
.enforce/runs/<runId>/
.enforce/db/
```

Agents should query `runs last-failure` or `ocentra_enforcer_last_failure`
before opening raw artifacts.

## Main Files

- `scripts/rust-rules.mjs`: CLI compatibility entrypoint, Rust scanner, generic scanner integration, init, route, and harness command handling.
- `mcp/rust-rules-mcp.mjs`: MCP stdio server with canonical `ocentra_enforcer_*` tools and legacy `rust_rules_*` aliases.
- `server.json`: MCP server manifest for registries and future package/plugin consumers.
- `src/`: reusable routing, generic scanner, migrated check, path, and harness modules.
- `src/codex-install.mjs`: Codex Desktop MCP config installer with idempotent TOML upsert and backup-before-write.
- `schemas/effect/enforcer-schemas.mjs`: Effect Schema contract source for configs, profiles, registry, route requests, reports, violations, init, runs, diagnostics, and MCP payloads.
- `schemas/json/*.schema.json`: JSON-schema-compatible contract artifacts.
- `rules/INDEX.md`: agent-facing routing index.
- `rules/rules.json`: machine-readable rule registry.
- `rules/rust/*.md`, `rules/typescript/*.md`, `rules/python/*.md`, `rules/common/*.md`: small rule-family docs for selective loading.
- `adapters/`: templates for Codex/MCP wiring, plain Git hooks, Husky, Lefthook, GitHub Actions, CodeQL, dependency policy, secret scan, and SBOM.
- `INSTALL.md`: clone/install/validate flow for a fresh machine or fresh Codex.
- `docs/CODEX_SETUP.md`: Codex MCP registration, manual config fallback, skill setup, and troubleshooting.
- `docs/TARGET_REPO_WIRING.md`: how a target repo calls the external enforcer.
- `docs/BOOTSTRAP_PROMPT.md`: copy-paste prompt for a future Codex to install and wire the enforcer.
- `docs/INSTALL_REFERENCE_LESSONS.md`: install lessons adopted from the codebase-memory-mcp setup pattern and remaining public-packaging gaps.
- `profiles/ocentra-parent.json`: migrated Ocentra Parent strict Rust profile.
- `rust-rules.config.json`: legacy strict default profile file, still supported.

## Migration Model

Generic guards should move into Ocentra Enforcer as reusable, profile-backed
checks. Ocentra Parent keeps ledger, hub, dev server, release packaging, and
product proof scripts in the main repo.

For current Ocentra Parent migration:

1. Add generic guard parity in Ocentra Enforcer.
2. Keep old repo guards as thin wrappers.
3. Prove file, crate, workspace, and diff scopes match or exceed old behavior.
4. Rewire wrappers to call `ocentra-enforcer scan`, `ocentra-enforcer check`, or `ocentra-enforcer run`.
5. Wire CI and hooks to the pack command.
6. Remove duplicated repo-local guards only after parity is proven.
