# z01 Dogfood Proof Gate

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Dogfood Proof Gate`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `proof/dogfood-gate.mjs, proof/dogfood-gate.*, proof/dogfood-manifest.json, tests/dogfood-gate.test.mjs`
- deps: `ALL tracks (A, C, D, E) — this is the LAST gate`
- tier: `P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
The enforcer's central doctrine is "did we follow our own advice?" — but there is no terminal gate that actually **runs the finished enforcer against its own now-TypeScript, now-multi-language self** and refuses plan-DONE on any self-violation. The self-validation *code* is authored elsewhere (a09/a10 for the TS/source-policy self-checks, e01 for the universal literal-scan floor, b02 for plan-structure self-validation); nothing composes them into one run+prove gate.

## Where We Want To Be
A single terminal gate `proof/dogfood-gate.mjs` that, **after everything else is written and validated**, RUNS the enforcer on its own repository (all shipped source, config, rules, and plan surfaces) and produces a durable proof artifact. Plan-DONE is gated on **zero self-violations**. Any self-violation the enforcer would flag in someone else's project must also be zero in ours, or the gate fails and DONE cannot move.

## Requirement Checklist
- [ ] Runs the enforcer end-to-end against its own repo (source policy + literal-scan floor + all rule families), not a mocked subset.
- [ ] Composes the self-validation code from a09/a10, e01, and b02 rather than reimplementing it.
- [ ] Emits a proof artifact (`proof/dogfood-manifest.json`): timestamp, ruleset fingerprint, per-family finding counts, and the terminal PASS/FAIL verdict.
- [ ] Gate is fail-closed: any self-violation (or any advisory above its committed T2 ceiling) blocks plan-DONE.
- [ ] Runs LAST: it executes only after all writing/validating packs are complete; it is a run+prove gate, not an authoring pack.

## Acceptance And Proof
Tier T1 gate (blocking on plan-DONE). Proof: `tests/dogfood-gate.test.mjs` invokes the gate against the live repo and asserts the run completes and the manifest records a zero-self-violation verdict. Fail-fixture (proving the gate actually bites): a deliberately-planted self-violation (a fixture repo state seeded with a known T1 breach) makes the gate exit non-zero and refuse the DONE verdict. Pass-fixture: the clean repo state produces a PASS manifest. The gate itself IS the proof artifact for plan-DONE; TEST_PROOF_EXPECTATIONS.md row `dogfood-self-zero-violations` is the terminal green that authorizes moving product status.

## Parallel Ownership Notes
`owns:` is the gate runner + manifest + its test only — disjoint from every authoring pack. `deps` is intentionally the whole plan (Tracks A, C, D, E): this pack must not start its RUN until siblings are DONE, because it validates their output. It does not edit sibling source; it only reads their shipped artifacts and composes the self-validation entrypoints they expose (a09/a10, e01, b02).
