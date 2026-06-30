---
name: ocentra-enforcer
description: Reusable Ocentra Enforcer workflow for Codex. Use when validating Rust, TypeScript, JavaScript, Python, security, dependency, generated-artifact, no-reexport, architecture, harness diagnostics, or file/crate/package/diff/workspace checks.
---

# Ocentra Enforcer

Use this skill to run the standalone `ocentra-enforcer` pack instead of recreating project-local guard logic. The model is not trusted to remember rules; the harness must fail hard on violations.

## Workflow

1. Locate the pack root. It is a project-independent clone, package, plugin, or tool directory. Do not assume it is inside the target repo.
2. Read `rules/INDEX.md` first. Do not open detailed rule docs until routing says which docs apply.
3. Prefer MCP `ocentra_enforcer_route` for the target `root`, `profile` or `configPath`, and smallest scope.
4. Pick scope from the touched risk: `--files`, `--crate`, `--base --head`, or `--workspace`.
5. Run `doctor` when wiring, when config is uncertain, or when MCP/app visibility is in doubt.
6. Run `scan` for deterministic source/config policy rules.
7. Run `check <name>` for reusable migrated guards such as `architecture-policy`, `source-shape`, `required-tests`, `single-source-contracts`, `import-boundaries`, `generated-artifacts`, `secrets`, `dependency-policy`, and `sbom`.
8. Run compiler, lint, test, or cargo commands through `ocentra_enforcer_run` or `ocentra-enforcer run`; query `last_failure` or `runs last-failure` before raw logs.
9. Treat `violations` as hard failures. Surface `warnings`, but do not block on advisory warnings unless the profile `failOn` includes `warning`.
10. Never add inline lint disables, validator bypass comments, skipped tests, or barrel/re-export shims to silence the gate.

## Rule And Validator Parity

Every enforced finding must have both sides:

- A rule entry in `rules/rules.json` with `id`, `language`, `family`, `severity`, `validator`, and `doc`.
- A small routed doc under `rules/rust`, `rules/typescript`, `rules/python`, or `rules/common`.
- A validator path that returns the same `ruleId` in CLI/MCP reports.
- An explanation path through `ocentra_enforcer_explain` or `ocentra-enforcer explain <ruleId>`.

This applies to TypeScript and Python the same way it applies to Rust. For example, `TS-1.3` and `PY-1.3` are not prose-only rules; `check no-naked-domain-strings` must fail on the matching code and the report must point back to the routed doc.

## CLI

```bash
ocentra-enforcer route --root <repo> --profile strict --files <file-or-dir>...
ocentra-enforcer doctor --root <repo> --profile strict --workspace
ocentra-enforcer scan --root <repo> --profile strict --files <file-or-dir>...
ocentra-enforcer cargo --root <repo> --profile strict --crate <cargo-package-name>
ocentra-enforcer check architecture-policy --root <repo> --profile strict --files <file-or-dir>...
ocentra-enforcer check generated-artifacts --root <repo> --tracked --workspace
ocentra-enforcer check secrets --root <repo> --staged
ocentra-enforcer run --root <repo> --tool tsc -- npx tsc --noEmit --pretty false
ocentra-enforcer runs last-failure --root <repo> --json
ocentra-enforcer runs prune --root <repo> --json
```

When the command is not on `PATH`, run from the pack root:

```bash
node scripts/rust-rules.mjs <command> --root <repo> ...
```

`rust-rules` and `rust_rules_*` names are temporary compatibility aliases.

## MCP

Prefer MCP tools when available:

```text
ocentra_enforcer_route
ocentra_enforcer_scan
ocentra_enforcer_check
ocentra_enforcer_doctor
ocentra_enforcer_explain
ocentra_enforcer_run
ocentra_enforcer_last_failure
ocentra_enforcer_diagnostics
ocentra_enforcer_artifact
ocentra_enforcer_prune_runs
```

Always pass the target project as `root`. Pass project-specific policy as `configPath`; pass pack policy as `profile`.

For broad scan/check scopes, request compact MCP output first:

```json
{
  "diagnosticLimit": 20,
  "groupBy": "slice",
  "includeScope": false
}
```

Use `summaryOnly: true` when you only need counts, rule IDs, docs, and grouped slices. After a direct MCP scan/check, `ocentra_enforcer_run_status` can return the latest validation summary even if no harness command has run.

## Install Model

Use the installer before manual edits:

```bash
ocentra-enforcer codex install --root <repo> --profile strict --dry-run
ocentra-enforcer codex install --root <repo> --profile strict
ocentra-enforcer codex doctor --root <repo>
ocentra-enforcer init --root <repo> --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
```

The MCP server runs from the Enforcer install path. Target repos store harness output under `<repo>/.enforce/`; do not write run logs or DuckDB state into the Enforcer install repo unless the Enforcer repo itself is the target.

## Failure Handling

Report the exact command or MCP tool, root, profile/config, scope, first rule IDs, first affected files, and the smallest next fix. Do not weaken rules unless the human explicitly changes project policy.
