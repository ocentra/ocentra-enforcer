# CP01 - Existing 41-Rule Reconciliation

<!-- agent-capsule -->
> Agent Capsule
> Plan: `cyberskills-parity-plan`
> Doc: `CP01 Existing 41-Rule Reconciliation`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `proof/cyberskills/cp01/**` and immutable mapping proposals; `cyberskills-ledger-integrator` owns ledger application
- deps: `cp00`
- tier: `P1 T1`

> Owner class: Luna-safe after CP00.
> Batch limit: at most 10 existing rule records per instance.
> Depends on: CP00.

## Where We Are

Forty-one native rule records exist, while only six vendor mappings have the full fingerprint, predicate, fixtures, and narrowed-coverage evidence required by the retention gate. Four registry IDs are still absent from the four existing CP01 evidence batches, so the workpack is not mechanically closed.

## Where We Want To Be

Reconcile existing implementation to vendor source in batches of at most 10 without changing rule behavior or inflating whole-skill parity.

## Owns

Only the assigned rule mapping/evidence files and CP01 batch fixtures. The worker submits proposed ledger rows for the named rule IDs; `cyberskills-ledger-integrator` applies them serially. Existing rule implementation files are read-only in CP01.

## Objective

Determine what the already implemented rules actually prove and connect them to vendor source without inflating parity. Similar names or comments are discovery leads, not evidence. The next bounded packet must close the CP01 registry partition before any graph status can become `DONE`.

## Requirement Checklist

- [x] Registry rule ID and validator path exist.
- [x] Exact vendor skill path and current source SHA-256 are recorded.
- [x] Stable anchors identify the source statements used.
- [x] The Rust predicate is stated narrowly and mechanically.
- [x] Existing fail and pass fixtures execute the named validator.
- [x] Malformed/boundary behavior is recorded.
- [x] `notProved` names the remainder of the skill and known limitations.
- [x] The component ledger changes only from evidence actually present.
- [x] Rules with no defensible source mapping remain implementation inventory, not vendor parity.
- [x] A bounded aggregate closure packet accounts for all 41 registry IDs, including the four IDs absent from batches 01-04, with explicit accepted/rejected/unproved disposition.
- [ ] The CP01 proof row records the aggregate artifact, exact gate evidence, derived counts, and non-proofs; only then may the graph lifecycle move to `DONE`.

## Existing-rule grammar classification

For each rule also record one of:

- `typed-structured-input-correct`: Serde/domain parser is appropriate; do not migrate.
- `textual-predicate-correct`: text/regex is the real predicate and boundary fixtures prove it.
- `syntax-candidate`: comments, strings, nesting, imports, calls, or arguments can affect correctness.
- `graph-candidate`: cross-file resolution is necessary.
- `external-engine-candidate`: the current rule only gates specialist evidence.

CP01 reports candidates; it does not refactor them.

## Required report

Include an accepted/rejected/unproved table for every rule in the batch, baseline and resulting derived counts, exact files, focused tests, and the recommended CP04 pilot. The boss reviews before any next batch.

## Acceptance And Proof

Run the disposition test plus the exact existing rule tests. A mapping is rejected if either side of the fail/pass pair is absent or vacuous.

## Parallel Ownership Notes

Each instance owns only new CP01 evidence files. Existing validators and fixtures are read-only; `cyberskills-ledger-integrator` serializes accepted ledger application.
