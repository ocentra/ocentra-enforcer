---
name: enforcer
description: Reusable enforcer workflow for any AI harness. Use when validating Rust, TypeScript, JavaScript, Python, Dart, CFML, security, dependency, generated-artifact, no-reexport, architecture, proof claims, harness diagnostics, or file/crate/package/diff/workspace checks.
---

# Enforcer

<!-- ai-dense -->
```yaml
binary: single native `enforcer` (MCP stdio server + CLI, same executable)
route_first: "enforcer route / mcp__enforcer__route before opening detailed rule docs"
scopes: "--files | --crate | --base/--head (diff) | --workspace -- smallest scope that covers the change"
rule_parity: "every finding = typed rule record (enforcer-rules) + Validator impl + fail/pass fixtures + explain path, for EVERY language family including TS/PY"
mcp_prefix: mcp__enforcer__*
pre_done: "coordination closeout for lane/thread scope before reporting DONE/PR_READY"
banned: "inline lint disables, validator bypass comments, skipped tests, barrel/re-export shims to silence the gate"
```
<!-- /ai-dense -->

Use this skill to run the standalone `enforcer` binary instead of recreating
project-local guard logic. The model is not trusted to remember rules; the
harness must fail hard on violations.

The harness is the reviewer of first resort. AI and humans may write code; the
harness decides whether the code is structurally acceptable. Human review starts only
after policy, compiler/type/lint, architecture, proof/test, dependency/security,
and local/CI parity gates pass.

## Workflow

1. Locate the installed enforcer binary. It is a project-independent install
   (see [INSTALL.md](../../INSTALL.md)), never assumed to live inside the
   target repo.
2. Prefer MCP `mcp__enforcer__route` for the target `root`, `profile` or
   `configPath`, and smallest scope. Do not open detailed rule docs until
   routing says which rule records apply.
3. Pick scope from the touched risk: `--files`, `--crate`, `--base --head`,
   or `--workspace`.
4. Run `doctor` when wiring, when config is uncertain, or when MCP/app
   visibility is in doubt.
5. Run `scan` for deterministic source/config policy rules (this uses the
   detect-and-route router by default — one call covers every language
   present).
6. Run `check <name>` for reusable named guards such as `architecture-policy`,
   `source-shape`, `required-tests`, `single-source-contracts`,
   `import-boundaries`, `generated-artifacts`, `secrets`,
   `dependency-policy`, and `sbom`.
7. Run compiler, lint, test, or cargo commands through `mcp__enforcer__run`
   or `enforcer run`; query `last_failure` or `runs last-failure` before raw
   logs.
8. For proof or PR-ready claims, route with `mcp__enforcer__proof_route`,
   run or inspect proof with proof MCP/CLI tools, then validate claims with
   `mcp__enforcer__proof_claim`.
9. Use coordination tools for lane/mail/exact-file claims when a multi-agent
   workflow requires them. Coordination is enforcer harness infrastructure,
   not product repo logic.
10. Before reporting `DONE` or `PR_READY`, run coordination closeout for the
    lane/thread scope. The task is not complete while matching active
    claims remain.
11. Treat `violations` as hard failures. Surface `warnings`, but do not block
    on advisory warnings unless the profile `failOn` includes `warning`.
12. Never add inline lint disables, validator bypass comments, skipped
    tests, or barrel/re-export shims to silence the gate.

## Rule And Validator Parity

Every enforced finding must have both sides:

- A typed rule record in `enforcer-rules` with `ruleId`, `language`,
  `family`, `severity`, `validator` reference, and `doc` anchor.
- A `Validator` implementation that returns the same `ruleId` in CLI/MCP
  reports.
- Fail/pass fixtures and a `cargo test` detection test.
- An explanation path through `mcp__enforcer__explain` or
  `enforcer explain <ruleId>`.

This applies to TypeScript and Python exactly as it applies to Rust, Dart,
and CFML. For example, `TS-1.3` and `PY-1.3` are not prose-only rules;
`check no-naked-domain-strings` must fail on the matching code and the
report must point back to the routed rule record.

## CLI

```bash
enforcer route --root <repo> --profile strict --files <file-or-dir>...
enforcer doctor --root <repo> --profile strict --workspace
enforcer scan --root <repo> --profile strict --files <file-or-dir>...
enforcer cargo --root <repo> --profile strict --crate <cargo-package-name>
enforcer check architecture-policy --root <repo> --profile strict --files <file-or-dir>...
enforcer check generated-artifacts --root <repo> --tracked --workspace
enforcer check secrets --root <repo> --staged
enforcer check mutation-risk --root <repo> --base origin/main --head HEAD
enforcer verify --root <repo> --profile strict --json
enforcer run --root <repo> --tool tsc -- npx tsc --noEmit --pretty false
enforcer runs last-failure --root <repo> --json
enforcer runs prune --root <repo> --json
enforcer proof route --root <repo> --files <file-or-dir>... --json
enforcer proof inventory --root <repo> --json
enforcer proof inventory --root <repo> --include-scripts --limit 20 --json
enforcer proof run --root <repo> --proof PROOF-COMMAND-GENERIC --json -- <command>...
enforcer proof claim --root <repo> --proof <proof-id> --pr-ready --json
enforcer proof last-failure --root <repo> --json
enforcer coordination health --hub <hub> --json
enforcer coordination presence --hub <hub> --json
enforcer coordination claim --hub <hub> --lane <lane> --paths <file> --operation edit --on-conflict intent --reason <reason>
enforcer coordination guard --hub <hub> --lane <lane> --paths <file> --operation commit --json
enforcer coordination release --hub <hub> --lane <lane> --paths <file> --reason <reason>
enforcer coordination closeout --hub <hub> --lane <lane> --thread-id <thread-id> --reason done --json
enforcer coordination repair legacy-hash --hub <hub>
enforcer coordination repair sequence --hub <hub>
enforcer coordination repair stale-claims --hub <hub> --paths <file>
enforcer coordination repair stale-claims --hub <hub> --paths <file> --owner <writer> --write
```

Normal coordination commands use the resolved ledger home plus `--hub <hub>`.
Only pass `--state-root <exact-hub-root>` for legacy-root repair/import or
emergency exact-root operations.

Proof inventory is summary-only unless `--include-scripts` is explicit. Do
not load all legacy proof script rows unless the migration task needs a
bounded batch.

## MCP

Prefer MCP tools when available:

```text
mcp__enforcer__route
mcp__enforcer__scan
mcp__enforcer__check
mcp__enforcer__doctor
mcp__enforcer__explain
mcp__enforcer__mcp_status
mcp__enforcer__run
mcp__enforcer__last_failure
mcp__enforcer__diagnostics
mcp__enforcer__artifact
mcp__enforcer__prune_runs
mcp__enforcer__proof_route
mcp__enforcer__proof_run
mcp__enforcer__proof_status
mcp__enforcer__proof_inventory
mcp__enforcer__proof_claim
mcp__enforcer__proof_last_failure
mcp__enforcer__proof_diagnostics
mcp__enforcer__proof_artifact
mcp__enforcer__coordination_health
mcp__enforcer__coordination_claim
mcp__enforcer__coordination_release
mcp__enforcer__coordination_repair
mcp__enforcer__coordination_guard
mcp__enforcer__coordination_message
mcp__enforcer__coordination_inbox
```

Always pass the target project as `root`. Pass project-specific policy as
`configPath`; pass pack policy as `profile`.

Before direct MCP coordination writes, call `mcp__enforcer__mcp_status`. If
it reports `stale: true`, update the enforcer binary; stale MCP writes must
not touch live coordination streams. Also require `writeCompatible: true`.

Coordination guard is path-focused by default when `paths` or
`changedPaths` are present. Treat `findings` as write blockers for the
requested files and `globalWarnings` as bounded ledger-health work for a
separate repair/triage task. Use `focused: false` only for broad ledger
diagnosis. Use `operation: "inspect"` for read-only context,
`operation: "edit"` for write-lock checks, `operation: "commit"` before
commit, and `operation: "pr_ready"` before PR-ready claims. When a claim is
blocked, prefer `onConflict: "intent"` so the enforcer queues the edit
intent and mails the next lane on release. Do not use dedicated write tools
as generic action dispatchers: `mcp__enforcer__coordination_claim` rejects
`action: "release"`; call `mcp__enforcer__coordination_release` instead.

For broad scan/check scopes, request compact MCP output first:

```json
{
  "diagnosticLimit": 20,
  "groupBy": "slice",
  "includeScope": false
}
```

Use `summaryOnly: true` when you only need counts, rule IDs, docs, and
grouped slices.

For proof work, do not read every product proof script. Route first, then
use `mcp__enforcer__proof_inventory` for legacy proof-script migration,
`mcp__enforcer__proof_run` for fresh proof collection, and
`mcp__enforcer__proof_claim` before any PR-ready or completion claim. Raw
proof artifacts are explicit-only through `mcp__enforcer__proof_artifact`.

## Install Model

Use the installer before manual edits:

```bash
enforcer install --root <repo> --profile strict --dry-run
enforcer install --root <repo> --profile strict
enforcer install --ledger-root <enforcer-install>/.ledger
enforcer doctor --root <repo>
enforcer uninstall --dry-run
enforcer init --root <repo> --profile strict --adapters codex,mcp,precommit,github-actions --dry-run
```

`enforcer install` is global first: it registers the MCP server for every
detected harness (any of 11 adapters), copies the user skill, creates or
updates managed global agent instructions, and configures the ledger home.
Passing `--root` additionally writes target repo wiring. Do not require a
product repo to host coordination, hub, lane, mail, or worktree logic.

The MCP server runs from the installed enforcer binary. Target repos store
harness output under `<repo>/.enforce/` and proof output under
`<repo>/.enforce/proofs/`; do not write run logs, proof artifacts, or DB
state into the enforcer's own install directory unless the enforcer's own
repo is the target.

## Failure Handling

Report the exact command or MCP tool, root, profile/config, scope, first
rule IDs, first affected files, and the smallest next fix. Do not weaken
rules unless the human explicitly changes project policy.
