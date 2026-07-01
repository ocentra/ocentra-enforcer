# Ocentra Enforcer Proof Index

This is the proof-routing index. Read this file first, then ask the CLI or MCP
for the smallest proof definition set. Do not open every proof doc or every
product proof artifact by default.

## Route Order

1. If the user names a proof id, route directly to that proof id.
2. If the task names a plan or workpack, route by `plan`.
3. If touched files are known, route by `files`.
4. If the task is device/platform readiness, route by `capability`.
5. If none match, return no proof docs and ask for an explicit proof id.

## File Routing

| Scope | Proof families |
| --- | --- |
| `*.rs`, `Cargo.toml` | Rust command, Cargo, contract parity |
| `*.ts`, `*.tsx`, `package.json` | TypeScript command, test reports, Playwright |
| `*.py`, `pyproject.toml` | Python command, pytest/JUnit |
| `scripts/test/*proof*.mjs` | Legacy proof inventory, artifact import, and migration parity |
| Android/iOS/Xcode named paths | Device/manual artifact proof |
| SARIF or CodeQL outputs | Security report proof |

## Agent Workflow

Use compact routing before proof execution:

```text
ocentra-enforcer proof route --root <repo> --files <touched files> --json
ocentra-enforcer proof inventory --root <repo> --json
ocentra-enforcer proof import-legacy --root <repo> --proof PROOF-LEGACY-ARTIFACT-IMPORT --legacy-paths <legacy-proof-root> --json
ocentra-enforcer proof parity --root <repo> --proof PROOF-LEGACY-ARTIFACT-IMPORT --legacy-paths <legacy-proof-root> --run-id <run-id> --json
ocentra-enforcer proof run --root <repo> --proof <proof-id> --json -- <command...>
ocentra-enforcer proof claim --root <repo> --proofs <ids> --pr-ready --json
```

With MCP:

```text
ocentra_enforcer_proof_route
ocentra_enforcer_proof_run
ocentra_enforcer_proof_import_legacy
ocentra_enforcer_proof_parity
ocentra_enforcer_proof_claim
ocentra_enforcer_proof_last_failure
```

Raw logs and large artifacts are explicit-only. Query compact diagnostics first,
then request `proof artifact` only when the compact result is insufficient.

## Claim Model

Read `docs/PROOF_SYSTEM_DESIGN.md` before migrating or deleting proof scripts.
Proof is an evidence-backed claim for a commit, scope, profile, and capability.
A proof must say what it proves and what it does not prove.

Common claim types:

| Claim type | Route |
| --- | --- |
| Command or tool passed | `PROOF-COMMAND-GENERIC` |
| Structured test report passed | `PROOF-JUNIT-TEST-REPORT`, `PROOF-PLAYWRIGHT-BROWSER`, `PROOF-CARGO-RUST`, `PROOF-PYTEST-PYTHON` |
| Contract mirror matches source of truth | contract parity proof definition or `PROOF-LEGACY-PARITY` during migration |
| Runtime/device behavior observed | capability-specific proof such as `PROOF-ANDROID-DEVICE` or `PROOF-XCODE-IOS` |
| Plan/workpack/PR is ready | `PROOF-CLAIM-PR-READY` plus the required proof ids for that plan |

Do not claim plan completion from a single command unless the proof definition
explicitly maps that command to the plan's required proof set.

## Legacy Script Migration

Use inventory as the matrix generator:

```text
ocentra-enforcer proof inventory --root <repo> --json
ocentra-enforcer proof inventory --root <repo> --include-scripts --limit 25 --json
```

The inventory result includes `byPlanBucket`, `byProofType`,
`byMigrationTemplate`, `claimSignals`, and `migrationMatrix`. Open individual
legacy scripts only after choosing one bounded migration row.

Use artifact import for read/write parity:

```text
ocentra-enforcer proof import-legacy --root <repo> --proof PROOF-LEGACY-ARTIFACT-IMPORT --legacy-paths test-results/foo-proof,output/foo-proof --json
ocentra-enforcer proof parity --root <repo> --proof PROOF-LEGACY-ARTIFACT-IMPORT --legacy-paths test-results/foo-proof,output/foo-proof --run-id <import-run-id> --json
```

`import-legacy` reads old proof artifacts, hashes and copies them into
`.enforce/proofs`, extracts legacy claims/non-claims when present, and writes a
canonical `proof-run.json`. `parity` compares current legacy artifact truth to
the imported Enforcer run and returns `deletionReady`. Do not delete a legacy
script when `deletionReady` is false.

Deletion rule:

1. Old proof script and new Enforcer proof both run against the same scope.
2. New proof is equivalent or stricter.
3. New proof has explicit claims, non-claims, artifacts, and capability state.
4. CI can recollect the proof, or the unavailable/manual state is explicit.
5. Target project wrapper is rewired to Enforcer before deleting the old script.

## Storage Contract

Proof runs are local runtime state under the target repository:

```text
.enforce/proofs/runs/<run-id>/
  proof-run.json
  summary.md
  events.ndjson
  diagnostics.ndjson
  raw/stdout.log
  raw/stderr.log
  attestation.json
```

Do not commit proof output by default. CI recollects proof and may upload
short-lived workflow artifacts.

## Ad-Hoc Proof

When no registered proof exists yet, run an ad-hoc proof with an explicit id:

```text
ocentra-enforcer proof run --proof my-temporary-proof --json -- node --version
```

Ad-hoc proof is useful during migration, but PR-ready claims should eventually
reference registered proof ids.
