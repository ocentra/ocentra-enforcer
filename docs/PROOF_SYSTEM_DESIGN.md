# Ocentra Enforcer Proof System Design

This document defines the reusable proof system. It is intentionally generic:
large legacy repositories are consumers, not owners of the model.

## Why This Exists

Legacy product repos tend to grow one proof script per claim. The first large
migration corpus inspected by Enforcer has 520 `scripts/test` files. The
observed shape is:

| Signal | Count |
| --- | ---: |
| Scripts under `scripts/test` | 520 |
| Scripts with `proof` in the name | 463 |
| Scripts that spawn commands | 491 |
| Scripts that write proof artifacts | 483 |
| Scripts that mention or reference prior proof roots | 419 by old broad heuristic |
| Scripts that actually read prior proof artifacts | 93 by strict read classifier |
| Manual or device related scripts | 361 |
| Scripts mentioning expectation rows | 135 |
| Scripts with explicit `claimsProved` | 91 |
| Scripts with explicit non-claims | 90 |

The problem is not that proof exists. The problem is that selection, command
execution, output parsing, product assertions, claim wording, artifact writing,
and retention are mixed into hundreds of one-off files.

Enforcer splits those concerns into deterministic contracts.

## Core Vocabulary

| Term | Meaning |
| --- | --- |
| Proof definition | The reusable description of what evidence must be collected and what claim it can support. |
| Proof run | One execution or manual evidence capture for one definition, commit, scope, profile, and capability. |
| Proof artifact | A stored evidence file such as structured JSON, NDJSON diagnostics, screenshots, JUnit, SARIF, logs, or attestation. |
| Proof diagnostic | A compact machine-readable problem extracted from a run or artifact. |
| Proof claim | A statement that a proof run, proof set, plan, workpack, PR, or release is valid for a specific commit and scope. |
| Non-claim | An explicit boundary saying what the proof does not establish. |
| Capability | The environment required to collect proof, such as `ci`, `windows`, `android-device`, `ios-simulator`, or `manual-required`. |
| Waiver | A named, explicit decision to accept an unavailable proof for a bounded scope. Silent skip is never proof. |

## Claim Types

Proof is not just "a test passed." Each claim type has a different burden.

| Claim type | Valid evidence | Invalid evidence |
| --- | --- | --- |
| `command-passed` | Command, exit code, bounded raw logs, compact diagnostics, current commit. | A command name copied into markdown without a run. |
| `test-passed` | Structured report with nonzero tests and parsed failures/skips/focus state. | Zero-test run, missing report, raw terminal dump only. |
| `contract-parity` | Authority source, generated or mirrored artifact, drift tests, hash or schema checks. | Keeping an old TS package alive only because a proof imports it. |
| `artifact-present` | File path, hash, size, producer, current commit, retention policy. | Stale artifact copied from an older commit. |
| `runtime-observed` | Service/device/browser/network observation from a named environment. | Fixture row presented as real runtime evidence. |
| `manual-required` | Named operator, device/platform identity, artifact manifest, unavailable/waived state when not collected. | Silent skip, "not tested locally" with no state. |
| `proof-composition` | Current accepted proof ids plus their claims and non-claims. | Reading arbitrary old JSON and calling the plan done. |
| `plan-done` | Required proof set for the plan/workpack is current and accepted. | One passing unit test used as full plan completion. |
| `pr-ready` | Required proof set, clean or explicitly allowed dirty state, scope freshness, CI-ready artifacts. | Local-only manual artifact presented as CI proof. |

## Generic Templates

The migration target is not "copy every script into Enforcer." It is to reduce
them into reusable templates, similar to generic types:

| Template | Use case | Collector pieces |
| --- | --- | --- |
| `CommandProof<CommandPlan, AssertionPlan>` | Simple proof scripts that run commands and assert output/files. | command runner, stdout/stderr parser, artifact writer. |
| `StructuredTestReportProof<Runner, ReportParser>` | Vitest, Jest, pytest, Cargo, Playwright, JUnit. | command runner, report parser, zero-test/skipped/focused-test gate. |
| `ContractParityProof<Authority, Mirror, DriftTests>` | Rust-owned contracts mirrored into TS/Python or generated artifacts. | authority reader, generated artifact hash/schema check, drift test runner. |
| `RuntimeEventProof<Producer, Transport, Consumer>` | Eventing, LAN, network, websocket, bridge chains. | command runner, event parser, source/consumer assertion plan. |
| `DeviceProof<Capability, DeviceSelector, ArtifactPlan>` | Android/iOS/Windows/macOS physical or simulator proof. | capability detector, device selector, command plan, artifact manifest. |
| `ReleaseReadinessProof<Workflow, Artifact, ManualGate>` | Packaging, install, production support, billing, release readiness. | CI workflow parser, artifact collector, manual gate states. |
| `SecurityReportProof<SarifOrAudit, Policy>` | CodeQL, SARIF, dependency audit, secret scan, SBOM. | report parser, policy mapper, SARIF/export support. |
| `ProofBundleClaim<RequiredProofIds, ClosureMatrix>` | Final plan, PR-ready, or release proof composed from other proofs. | manifest reader, freshness checker, claim/non-claim resolver. |

## Storage Model

Proof output is runtime state, not source truth.

Current v1 storage stays under the target repo:

```text
.enforce/proofs/
  runs/<run-id>/
    proof-run.json
    summary.md
    events.ndjson
    diagnostics.ndjson
    raw/stdout.log
    raw/stderr.log
    attestation.json
  db/proof-manifest.json
```

Do not commit this folder by default. If it is deleted, the correct response is
to recollect proof. CI should recollect proof and upload short-retention
artifacts instead of trusting committed proof output.

The next storage step is a disposable read model:

```text
.enforce/proofs/views/by-plan/<plan>.json
.enforce/proofs/views/by-proof/<proof-id>.json
.enforce/proofs/views/by-claim/<claim-id>.json
```

These views must be rebuildable from `proof-run.json`, `events.ndjson`, and the
manifest. They are for fast Codex/MCP lookup, not canonical truth.

## Legacy Migration Matrix

The current large legacy corpus maps to these Enforcer migration tracks.

| Legacy family | Observed count | Enforcer target | Deletion gate |
| --- | ---: | --- | --- |
| Device/manual proof | 275 | `DeviceProof<Capability, DeviceSelector, ArtifactPlan>` | Device or manual state is explicit: passed, unavailable, waived, or manual-required with artifact manifest. |
| Test report proof | 187 | `StructuredTestReportProof<Runner, ReportParser>` | New proof parses report, rejects zero tests, skipped/focused tests where forbidden, and stale artifacts. |
| Contract parity proof | 26 | `ContractParityProof<Authority, Mirror, DriftTests>` | New proof checks real authority source and generated mirrors without retaining old TS domains as authority. |
| Event/network proof | 12 | `RuntimeEventProof<Producer, Transport, Consumer>` | New proof captures runtime chain assertions and authentic-runtime boundaries. |
| Security report proof | 11 | `SecurityReportProof<SarifOrAudit, Policy>` | New proof parses SARIF/audit data and ties findings to policy. |
| Command proof | 6 | `CommandProof<CommandPlan, AssertionPlan>` | New proof captures command, bounded logs, compact diagnostics, artifacts, claims, and non-claims. |
| Release/package proof | 3 | `ReleaseReadinessProof<Workflow, Artifact, ManualGate>` | New proof separates CI-reproducible artifacts from manual/platform gates. |

High-count plan buckets from the script names include `app-install`,
`browser`, `app-game`, `screen-ai`, `v0-8`, `browser-game`,
`production-support`, `network-remote`, `v0-9`, `child-android`, and
`eventing-network`. These should become profile proof groups, not separate
engine code.

The current inventory also emits template counts. In the first strict pass,
the large legacy corpus maps roughly to 274 device/capability proofs, 75
runtime-event proofs, 60 command proofs, 43 release-readiness proofs, 41
contract-parity proofs, 24 structured test-report proofs, 2 security-report
proofs, and 1 proof-bundle claim. These numbers are routing aids, not deletion
permission.

## Deterministic Migration Sequence

1. Inventory: run `ocentra-enforcer proof inventory --root <repo> --json` and
   inspect `migrationMatrix` before opening individual scripts.
2. Route: pick one plan bucket and one template, for example
   `eventing-network + RuntimeEventProof`.
3. Define: add Enforcer proof definitions or project profile entries for that
   bucket.
4. Run old: execute the old script once and capture its proof claim, artifacts,
   and non-claims.
5. Run new: execute the Enforcer proof definition against the same scope.
6. Import/compare: run `proof import-legacy` to read old artifacts into
   `.enforce/proofs`, then `proof parity` to produce a parity report saying
   equivalent, stricter, weaker, or not comparable.
7. Rewire: change the product repo wrapper to call Enforcer only.
8. Delete: remove the old script only after the parity report is equivalent or
   stricter and CI can recollect the proof.

## Executable Parity Bridge

The bridge is intentionally artifact-first. It does not compare two JavaScript
files line by line; it compares proof evidence.

```text
ocentra-enforcer proof import-legacy --root <repo> --proof PROOF-LEGACY-ARTIFACT-IMPORT --legacy-paths test-results/foo-proof,output/foo-proof --json
ocentra-enforcer proof parity --root <repo> --proof PROOF-LEGACY-ARTIFACT-IMPORT --legacy-paths test-results/foo-proof,output/foo-proof --run-id <run-id> --json
```

`import-legacy` reads existing legacy proof artifacts from `test-results`,
`output`, `docs/proof`, or explicit `--legacy-paths`; hashes them; extracts
status, `claimsProved`, and `claimsNotProved` when present; copies them into
the canonical `.enforce/proofs/runs/<run-id>/artifacts/legacy/` tree; and
writes `proof-run.json`, `events.ndjson`, `diagnostics.ndjson`, `summary.md`,
and `attestation.json`.

`parity` compares the current legacy artifact set to the imported Enforcer run.
It returns `deletionReady:true` only when the imported run covers the same
artifact hashes and preserved claims/non-claims without failed legacy statuses.
Anything else is a machine-readable stop sign, not an agent judgement call.

## PR-Ready Rule

A plan, workpack, or PR is not done because one proof file exists. It is done
only when the configured proof set is current for the commit and scope, all
required artifacts exist, unavailable capabilities are explicit, and all
non-claims are still true.

Any proof claim should answer:

| Question | Required answer |
| --- | --- |
| Who claims it? | lane/thread/human/CI identity when available. |
| What is claimed? | proof id, plan/workpack, scope, files, profile. |
| What commit? | git branch, commit, dirty state. |
| What evidence? | artifact manifest, hashes, diagnostics, raw log paths. |
| What is not claimed? | explicit non-claim list. |
| Can CI recollect it? | yes, no, or capability-gated with reason. |
| When can it be deleted? | after recollection or after retention expiry unless pinned. |

## What Not To Do

- Do not migrate a legacy repo by copying all proof scripts into Enforcer as-is.
- Do not keep product TypeScript packages alive solely for old proof imports.
- Do not commit `.enforce/proofs` as source truth.
- Do not treat manual/device proof as passed when the capability is unavailable.
- Do not let a raw terminal wall be the proof API for Codex. Use compact
  diagnostics and explicit artifact retrieval.
