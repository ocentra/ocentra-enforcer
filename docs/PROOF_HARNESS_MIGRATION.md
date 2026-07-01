# Proof Harness Migration Contract

Proof collection is now an Enforcer-owned v1 surface. The first implementation
does not delete any product repo proof script, but it provides the reusable
contracts, routing, local storage, CLI, MCP tools, and claim validation needed
to migrate those scripts safely.

This file records the intended replacement pattern so product repos can stop
using legacy TypeScript domain packages as proof authority.

## Direction

Product repos should expose product artifacts and product configuration.
Enforcer should own reusable proof runners, structured diagnostics, and harness
storage.

For a Rust-owned contract, the authority chain should be:

1. Rust source or Rust-generated artifact is the source of truth.
2. Optional generated TypeScript/Python artifacts are checked as mirrors only.
3. Enforcer validates the source and mirrors through named checks.
4. Proof scripts in the product repo become thin calls into Enforcer or are
   deleted after parity.

Do not retain a TypeScript domain package only because old proof scripts import
it. If runtime consumers are gone and only proof scripts remain, the migration
path is to replace those scripts with Enforcer checks against Rust/generated
outputs.

## Implemented v1 Surface

- `proof/INDEX.md`: agent decision tree for proof routing.
- `proof/proofs.json`: Effect Schema validated proof registry.
- `ocentra-enforcer proof route`: route by files, plan, capability, or proof id.
- `ocentra-enforcer proof inventory`: read-only legacy proof script inventory.
- `ocentra-enforcer proof run`: run proof through bounded local artifacts.
- `ocentra-enforcer proof claim --pr-ready`: reject missing, stale, failed,
  manual-required, or artifact-broken claims.
- `ocentra_enforcer_proof_*`: MCP equivalents for route, run, status,
  inventory, claim, last failure, diagnostics, artifact, reset, prune, and
  export.

The detailed model is in `docs/PROOF_SYSTEM_DESIGN.md`. The short version:
proof is an evidence-backed claim for a commit, scope, profile, and capability.
It is not a raw terminal dump, and it is not a permanent source artifact.

Proof storage is local target-repo runtime state:

```text
.enforce/proofs/runs/<run-id>/
```

Do not commit proof outputs by default. CI should recollect proof and upload
short-lived workflow artifacts when needed.

Inventory is summary-only by default. Use `--include-scripts --limit <n>` only
when migrating a bounded batch of legacy scripts.

The inventory now emits a migration matrix with:

- `byPlanBucket`: plan or product bucket inferred from legacy script names and
  references.
- `byProofType`: command, test report, contract parity, device execution,
  proof composition, release readiness, and related claim shapes.
- `byMigrationTemplate`: the generic proof template that should replace the
  one-off script.
- `claimSignals`: whether scripts have explicit claims, explicit non-claims,
  expectation rows, prior-proof dependencies, artifact output, and capability
  gating.
- `migrationMatrix`: bounded rows that connect plan bucket, proof family,
  generic template, representative scripts, and deletion gate.

## Parent-Specific Interpretation

For Ocentra Parent, `packages/agent-protocol-domain` should not remain alive
only because `scripts/test/*proof*.mjs` imports it. The replacement is an
Enforcer proof definition that validates `crates/agent-protocol` and any
generated/mirrored artifacts directly.

Deletion rule:

1. Run `ocentra-enforcer proof inventory --root <Parent> --json` for compact
   counts, then `--include-scripts --limit <n>` for one migration batch.
2. Add or map Enforcer proof definitions for the script family being migrated.
3. Run old-vs-new parity through `PROOF-LEGACY-PARITY`.
4. Rewire Parent npm scripts to thin Enforcer calls.
5. Delete the old proof scripts only after the parity report is green.

Deleting proof output under `.enforce/proofs` is safe. It only means proof must
be recollected. Deleting legacy proof scripts is different: do that only after a
machine-readable parity report proves the Enforcer proof is equivalent or
stricter and CI can recollect it.
