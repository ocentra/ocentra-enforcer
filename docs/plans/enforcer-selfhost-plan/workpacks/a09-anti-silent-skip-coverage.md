# a09 Anti Silent Skip Coverage

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Anti Silent Skip Coverage`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/generic-*scanner*.*`, `src/cli-scan.*`
- deps: `a01`
- tier: `P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The generic scanners (`src/generic-common-scanner.mjs`, `src/generic-typescript-scanner.mjs`, `src/generic-python-scanner.mjs`, and friends) and `src/cli-scan.mjs` can early-return on unmatched extension, missing tool, or empty selection with no emitted record. A validator that runs on nothing looks identical to a validator that ran and passed — this is the hollow self-scan: green because it checked nothing.

## Where We Want To Be
Every validator/scanner emits an explicit outcome for every candidate: `ran`, or `skipped: <reason>`. A skip is never silent; the scan report distinguishes "passed" from "did not run" so a hollow scan cannot masquerade as a clean one.

## Requirement Checklist
- [ ] Each generic scanner returns a per-target result with status `ran | skipped` and, when skipped, a non-empty `reason`.
- [ ] `src/cli-scan.*` aggregates and surfaces skip reasons in the report/summary counts.
- [ ] No code path exits a scanner without recording a result for the target(s) it was handed.
- [ ] A scan over zero applicable targets reports `skipped` counts, not a bare success.

## Acceptance And Proof
Tier P4 (self-enforce green). A test feeds unmatched-extension, missing-tool, and empty-selection inputs and asserts each yields an explicit `skipped: <reason>` record; asserts the report exposes skip counts. Running `enforcer:self:scan` on the repo shows nonzero ran-count with reasons for any skips. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01. Owns the generic scanner modules and `src/cli-scan.*` exclusively; disjoint from a10 (which owns CI scripts/workflows and `enforcer:self` wiring). a09 makes the scan honest; a10 makes it hard-fail.
