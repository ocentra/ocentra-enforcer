---
name: rust-rules-hard-gate
description: Ocentra Enforcer hard-fail workflow for Codex. Use when working on Rust, TypeScript, JavaScript, Python, dependency/security policy, generated artifacts, harness diagnostics, no-reexport policy, or when validating changes by file, crate/package, diff, or workspace.
---

# Ocentra Enforcer Hard Gate

## Overview

Use this skill to run the reusable `ocentra-enforcer` pack instead of recreating repo-local guard logic. Prefer the smallest honest scope that covers the touched risk, then escalate only when the result or task requires it.

## Workflow

1. Locate the pack root. The pack should be stable and project-independent: a cloned repo, installed plugin, or tool directory. Do not assume it lives inside the target project.
2. Read `rules/INDEX.md` before detailed rule docs. It is the routing layer and should be much smaller than the full rulebook.
3. Choose scope from the task, not convenience: use `--files` for exact touched files, `--crate` for one Cargo package, `--base --head` for PR/diff checks, and `--workspace` only for full repo gates.
4. Prefer MCP `ocentra_enforcer_route` before opening detailed docs. Read only the returned `docs`.
5. Run `doctor` when wiring a new repo or when a target root/config may be wrong.
6. Run `scan` for deterministic source/config policy failures. Pass `--languages` for TypeScript, Python, or common checks.
7. Run named reusable guard migrations through `check`, for example `no-zod-source`, `reexports`, `validation-bypass`, `weak-assertions`, `skipped-focused-tests`, `placeholder-implementation`, `source-shape`, `required-tests`, `single-source-contracts`, `dependency-policy`, `sbom`, and `ai-rule-index`.
8. Run command-line tools through `ocentra_enforcer_run` or `ocentra-enforcer run`, then query `last_failure` or `runs last-failure` before opening raw logs.
9. Run `cargo` for Rust crate/workspace readiness because it adds fmt, clippy, tests, docs when enabled, and dependency-policy tools.
10. Treat `violations` as hard failures. Surface `warnings`, but do not block completion on advisory findings unless the profile `failOn` includes `warning`. Do not add inline lint disables, validator bypass comments, skipped tests, or barrel/re-export shims to silence the gate.

## Commands

From the pack root:

```bash
node scripts/rust-rules.mjs doctor --root <repo> --config <config> --workspace
node scripts/rust-rules.mjs scan --root <repo> --config <config> --files <file-or-dir>...
node scripts/rust-rules.mjs cargo --root <repo> --config <config> --crate <cargo-package-name>
node scripts/rust-rules.mjs scan --root <repo> --config <config> --base <base> --head <head>
node scripts/rust-rules.mjs cargo --root <repo> --config <config> --workspace
node scripts/rust-rules.mjs explain RR-7.3
node scripts/rust-rules.mjs init --root <repo> --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
node scripts/rust-rules.mjs route --root <repo> --files <file-or-dir>...
node scripts/rust-rules.mjs scan --root <repo> --languages typescript,python,common --files <file-or-dir>...
node scripts/rust-rules.mjs check no-zod-source --root <repo> --files <file-or-dir>...
node scripts/rust-rules.mjs check validation-bypass --root <repo> --files <file-or-dir>...
node scripts/rust-rules.mjs check weak-assertions --root <repo> --files <file-or-dir>...
node scripts/rust-rules.mjs check placeholder-implementation --root <repo> --files <file-or-dir>...
node scripts/rust-rules.mjs check source-shape --root <repo> --workspace
node scripts/rust-rules.mjs check single-source-contracts --root <repo> --check-config <path>
node scripts/rust-rules.mjs check sbom --root <repo> --output target/security --dry-run
node scripts/rust-rules.mjs run --root <repo> --tool tsc -- npx tsc --noEmit --pretty false
node scripts/rust-rules.mjs runs last-failure --root <repo> --json
node scripts/rust-rules.mjs runs prune --root <repo> --json
```

When the pack is installed or symlinked, `ocentra-enforcer` and `ocentra-enforcer-mcp` are the canonical executable entrypoints. `rust-rules` and `rust-rules-mcp` remain temporary compatibility aliases. For standalone use, pass `--root` explicitly so the pack can validate any project.

When an MCP server is wired, prefer the MCP tools for Codex-triggered checks:

```text
ocentra_enforcer_scan
ocentra_enforcer_check
ocentra_enforcer_doctor
ocentra_enforcer_explain
ocentra_enforcer_route
ocentra_enforcer_run
ocentra_enforcer_last_failure
ocentra_enforcer_diagnostics
ocentra_enforcer_artifact
ocentra_enforcer_prune_runs
```

The old `rust_rules_scan`, `rust_rules_doctor`, `rust_rules_explain`, and `rust_rules_route` MCP tool names remain aliases for one Rust-pack compatibility release.

Pass the target repo as `root`. Pass `configPath` for a target repo config, or `profile` for a named pack profile such as `strict` or `ocentra-parent`.

Profiles control severity and enablement. Documentation/comment advisories such
as `DOC-1.1` default to warning, while bypass, architecture, secret, dependency,
and skipped-test rules remain hard errors unless a human-owned profile changes
them.

Harness state lives in the target repo, not the Enforcer install path. By
default it writes raw logs, NDJSON diagnostics, manifests, and optional DuckDB
state under `<repo>/.enforce/`; generated init wiring adds `.enforce/` to the
target `.gitignore`. Use `runs prune` or `ocentra_enforcer_prune_runs` before
raw logs become noisy. Use `runs reset` only when the target run store should be
cleared entirely.

Route before reading detailed docs:

```json
{
  "name": "ocentra_enforcer_route",
  "arguments": {
    "root": "<repo>",
    "profile": "strict",
    "scope": "files",
    "files": ["src/lib.rs"]
  }
}
```

Use the legacy full `docs/RustRules.md` only when the registry is missing a rule, the user asks for broad policy review, or no route can explain a failure.

For tool output, ask MCP for compact diagnostics first:

```json
{
  "name": "ocentra_enforcer_last_failure",
  "arguments": {
    "root": "<repo>",
    "limit": 10
  }
}
```

Open `ocentra_enforcer_artifact` only when compact diagnostics are insufficient.

## Standalone Install Model

Default consumption is package plus Codex plugin/MCP. Git submodules are optional only when a project needs source pinning. The MCP server runs from the enforcer install path; target projects always pass `root` plus either `configPath` or a pack `profile`.

Use `ocentra-enforcer codex install --root <repo> --profile <profile> --dry-run` before writing Codex/MCP wiring. The non-dry-run form updates the target repo and Codex Desktop's global MCP config with backup-before-write, and it configures `OCENTRA_LEDGER_HOME` to the Enforcer install `.ledger` folder by default. Run `ocentra-enforcer codex doctor --root <repo>` after install to verify the app-level MCP config separately from the MCP server smoke test. Use `ocentra-enforcer init --root <repo> --profile <profile> --adapters precommit,github-actions --dry-run` for hooks and CI. The default pre-commit adapter is a plain Git hook for cross-platform use; Husky is generated only when requested or when the target repo already uses Husky.

## Ocentra Parent Profile

For Ocentra Parent, use `profiles/ocentra-parent.json`. It preserves the repo's strict Rust posture:

- `pub use` is forbidden, not just limited to facade files.
- Runtime Rust string literals are forbidden in configured runtime/protocol owner crates.
- Public serialized id/ref/event/command fields must use domain types, not raw `String`, `bool`, or numeric primitives.
- Protocol crates cannot depend on configured runtime crates.
- Runtime crates cannot pull test-only crates as non-dev dependencies.

Keep this profile in the pack. Consuming repos should only carry a tiny config or script that points to it until the pack is installed as a plugin/MCP server.

## TypeScript And Python Modules

Use `rules/INDEX.md` and MCP route output instead of reading all docs. TypeScript and Python start with hard guards for re-exports, suppressions, skipped tests, generated artifacts, secrets, and harnessed native tool output. Native tools such as `tsc`, ESLint, Ruff, Pyright, mypy, and pytest should be run through the Enforcer harness when possible.

## Failure Handling

On failure, report:

- Exact command or MCP tool used.
- Root, config, and scope.
- Rule IDs and first affected files.
- Whether failures are source policy, Cargo/tooling, dependency policy, or target-root/config wiring.
- The smallest next fix, without weakening the rules.
