---
name: ocentra-enforcer
description: Reusable Ocentra Enforcer workflow for Codex. Use when validating Rust, TypeScript, JavaScript, Python, security, dependency, generated-artifact, no-reexport, architecture, proof claims, harness diagnostics, or file/crate/package/diff/workspace checks.
---

# Ocentra Enforcer

Use this skill to run the standalone `ocentra-enforcer` pack instead of recreating project-local guard logic. The model is not trusted to remember rules; the harness must fail hard on violations.

The harness is the reviewer of first resort. AI and humans may write code; the
harness decides whether code is structurally acceptable. Human review starts only
after policy, compiler/type/lint, architecture, proof/test, dependency/security,
and local/CI parity gates pass.

## Workflow

1. Locate the pack root. It is a project-independent clone, package, plugin, or tool directory. Do not assume it is inside the target repo.
2. Read `rules/INDEX.md` first. Do not open detailed rule docs until routing says which docs apply.
3. Prefer MCP `ocentra_enforcer_route` for the target `root`, `profile` or `configPath`, and smallest scope.
4. Pick scope from the touched risk: `--files`, `--crate`, `--base --head`, or `--workspace`.
5. Run `doctor` when wiring, when config is uncertain, or when MCP/app visibility is in doubt.
6. Run `scan` for deterministic source/config policy rules.
7. Run `check <name>` for reusable migrated guards such as `architecture-policy`, `source-shape`, `required-tests`, `single-source-contracts`, `import-boundaries`, `generated-artifacts`, `secrets`, `dependency-policy`, and `sbom`.
8. Run compiler, lint, test, or cargo commands through `ocentra_enforcer_run` or `ocentra-enforcer run`; query `last_failure` or `runs last-failure` before raw logs.
9. For proof or PR-ready claims, read `proof/INDEX.md`, route with `ocentra_enforcer_proof_route`, run or inspect proof with proof MCP/CLI tools, then validate claims with `ocentra_enforcer_proof_claim`.
10. Use coordination tools for lane/mail/exact-file claims when a Codex workflow requires them. Coordination is Enforcer/Codex harness infrastructure, not product repo logic.
11. Before reporting `DONE` or `PR_READY`, run coordination closeout for the lane/thread scope. The task is not complete while matching active claims remain.
12. Treat `violations` as hard failures. Surface `warnings`, but do not block on advisory warnings unless the profile `failOn` includes `warning`.
13. Never add inline lint disables, validator bypass comments, skipped tests, or barrel/re-export shims to silence the gate.

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
ocentra-enforcer check mutation-risk --root <repo> --base origin/main --head HEAD
ocentra-enforcer verify --root <repo> --profile strict --json
ocentra-enforcer run --root <repo> --tool tsc -- npx tsc --noEmit --pretty false
ocentra-enforcer runs last-failure --root <repo> --json
ocentra-enforcer runs prune --root <repo> --json
ocentra-enforcer proof route --root <repo> --files <file-or-dir>... --json
ocentra-enforcer proof inventory --root <repo> --json
ocentra-enforcer proof inventory --root <repo> --include-scripts --limit 20 --json
ocentra-enforcer proof run --root <repo> --proof PROOF-COMMAND-GENERIC --json -- <command>...
ocentra-enforcer proof claim --root <repo> --proof <proof-id> --pr-ready --json
ocentra-enforcer proof last-failure --root <repo> --json
ocentra-enforcer coordination health --hub <hub> --json
ocentra-enforcer coordination presence --hub <hub> --json
ocentra-enforcer coordination claim --hub <hub> --lane <lane> --paths <file> --operation edit --on-conflict intent --reason <reason>
ocentra-enforcer coordination guard --hub <hub> --lane <lane> --paths <file> --operation commit --json
ocentra-enforcer coordination release --hub <hub> --lane <lane> --paths <file> --reason <reason>
ocentra-enforcer coordination closeout --hub <hub> --lane <lane> --thread-id <codex-thread-id> --reason done --json
ocentra-enforcer coordination repair legacy-hash --hub <hub>
ocentra-enforcer coordination repair legacy-hash --hub <hub> --write
ocentra-enforcer coordination repair sequence --hub <hub>
ocentra-enforcer coordination repair sequence --hub <hub> --write
ocentra-enforcer coordination repair stale-claims --hub <hub> --paths <file>
ocentra-enforcer coordination repair stale-claims --hub <hub> --paths <file> --owner <writer> --write
```

Normal coordination commands use `OCENTRA_LEDGER_HOME` plus `--hub <hub>`.
Only pass `--state-root <exact-hub-root>` for legacy-root repair/import or
emergency exact-root operations.

Proof inventory is summary-only unless `--include-scripts` is explicit. Do not
load all legacy proof script rows unless the migration task needs a bounded batch.

During migration from legacy Parent ledger wrappers, Enforcer keeps event hashes
compatible with the v1 ledger envelope and stores extended presence context as
metadata. If an older Enforcer build wrote context-hashed events, run
`coordination repair legacy-hash` as a dry-run first, then use `--write` only
after reviewing the reported stream backups.
If doctor reports `sequence break` or previous-pointer failures, run
`coordination repair sequence` as a dry-run first, then use `--write` only after
reviewing stream backups. If stale ownership conflicts remain, run
`coordination repair stale-claims --paths <exact-paths>` first. Use
`--owner <writer>` only when one current owner should be preserved; otherwise
the write form appends `claim.resolve` to clear active claims for those exact
paths.

When the command is not on `PATH`, run from the pack root:

```bash
node scripts/ocentra-enforcer.mjs <command> --root <repo> ...
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
ocentra_enforcer_mcp_status
ocentra_enforcer_run
ocentra_enforcer_last_failure
ocentra_enforcer_diagnostics
ocentra_enforcer_artifact
ocentra_enforcer_prune_runs
ocentra_enforcer_proof_route
ocentra_enforcer_proof_run
ocentra_enforcer_proof_status
ocentra_enforcer_proof_inventory
ocentra_enforcer_proof_claim
ocentra_enforcer_proof_last_failure
ocentra_enforcer_proof_diagnostics
ocentra_enforcer_proof_artifact
ocentra_enforcer_coordination_health
ocentra_enforcer_coordination_claim
ocentra_enforcer_coordination_release
ocentra_enforcer_coordination_repair
ocentra_enforcer_coordination_guard
ocentra_enforcer_coordination_message
ocentra_enforcer_coordination_inbox
```

Always pass the target project as `root`. Pass project-specific policy as `configPath`; pass pack policy as `profile`.

Before direct MCP coordination writes, call `ocentra_enforcer_mcp_status`. If it
reports `stale: true`, restart Codex/MCP or call the updated CLI through
`ocentra_enforcer_run`; stale MCP writes must not touch live coordination
streams. Also require `writeCompatible: true`; this confirms the writer excludes
presence `context` metadata from the legacy coordination event hash.

Coordination guard is path-focused by default when `paths` or `changedPaths` are
present. Treat `findings` as write blockers for the requested files and
`globalWarnings` as bounded ledger-health work for a separate repair/triage
task. Use `focused: false` only for broad ledger diagnosis.
Use `operation: "inspect"` for read-only context, `operation: "edit"` for
write-lock checks, `operation: "commit"` before commit, and
`operation: "pr_ready"` before PR-ready claims. `edit` may allow different-branch
same-file work as a merge-risk warning; `pr_ready` blocks unresolved merge risks.
When a claim is blocked, prefer `onConflict: "intent"` so Enforcer queues the
edit intent and mails the next lane on release. The notified lane must re-read
the file before claiming and editing.
Do not use dedicated write tools as generic action dispatchers:
`ocentra_enforcer_coordination_claim` rejects `action: "release"`; call
`ocentra_enforcer_coordination_release` instead.

For broad scan/check scopes, request compact MCP output first:

```json
{
  "diagnosticLimit": 20,
  "groupBy": "slice",
  "includeScope": false
}
```

Use `summaryOnly: true` when you only need counts, rule IDs, docs, and grouped slices. After a direct MCP scan/check, `ocentra_enforcer_run_status` can return the latest validation summary even if no harness command has run.

For proof work, do not read every product proof script. Route first:

```json
{
  "root": "<repo>",
  "files": ["scripts/test/example-proof.mjs"]
}
```

Use `ocentra_enforcer_proof_inventory` for legacy proof-script migration,
`ocentra_enforcer_proof_run` for fresh proof collection, and
`ocentra_enforcer_proof_claim` before any PR-ready or completion claim. Raw
proof artifacts are explicit-only through `ocentra_enforcer_proof_artifact`.

## Install Model

Use the installer before manual edits:

```bash
ocentra-enforcer codex install --root <repo> --profile strict --dry-run
ocentra-enforcer codex install --root <repo> --profile strict
ocentra-enforcer codex install --ledger-root <enforcer-install>/.ledger
ocentra-enforcer codex doctor --root <repo>
ocentra-enforcer codex uninstall --dry-run
ocentra-enforcer init --root <repo> --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
```

`codex install` is global first: it registers the MCP server, copies the user
skill, creates or updates the managed global `AGENTS.md` block, and configures
`OCENTRA_LEDGER_HOME`. The default ledger root is
`<enforcer-install>/.ledger`, with hubs below it. Passing `--root` additionally
writes target repo wiring. Do not require a product repo to host coordination,
hub, lane, mail, or worktree logic.

The MCP server runs from the Enforcer install path. Target repos store harness
output under `<repo>/.enforce/` and proof output under
`<repo>/.enforce/proofs/`; do not write run logs, proof artifacts, or DuckDB
state into the Enforcer install repo unless the Enforcer repo itself is the
target.

## Failure Handling

Report the exact command or MCP tool, root, profile/config, scope, first rule IDs, first affected files, and the smallest next fix. Do not weaken rules unless the human explicitly changes project policy.
