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
| `scripts/test/*proof*.mjs` | Parent proof inventory and migration parity |
| Android/iOS/Xcode named paths | Device/manual artifact proof |
| SARIF or CodeQL outputs | Security report proof |

## Agent Workflow

Use compact routing before proof execution:

```text
ocentra-enforcer proof route --root <repo> --files <touched files> --json
ocentra-enforcer proof run --root <repo> --proof <proof-id> --json -- <command...>
ocentra-enforcer proof claim --root <repo> --proofs <ids> --pr-ready --json
```

With MCP:

```text
ocentra_enforcer_proof_route
ocentra_enforcer_proof_run
ocentra_enforcer_proof_claim
ocentra_enforcer_proof_last_failure
```

Raw logs and large artifacts are explicit-only. Query compact diagnostics first,
then request `proof artifact` only when the compact result is insufficient.

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
