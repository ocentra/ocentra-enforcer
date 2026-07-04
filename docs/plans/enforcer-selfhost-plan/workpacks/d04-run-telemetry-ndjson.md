# d04 Run Telemetry NDJSON

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Run Telemetry NDJSON`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/run-telemetry.ts, schemas/effect/run-telemetry-schema.ts, tests/run-telemetry.test.mjs`
- deps: `d01-rule-mechanization-engine`
- tier: `P1 unit`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The enforcer produces findings but keeps no machine-readable record of each run. Proof storage exists (`src/proof-storage.mjs`) but there is no per-run telemetry line usable for trend analysis (d05 budget, d10 failure modes). ADBP's "measure everything" is prose.

## Where We Want To Be
Every enforcer run appends exactly one NDJSON line capturing command, rule set, finding counts by severity, and duration, against a schema-validated shape.

## Requirement Checklist
- [ ] One line per run appended to a stable telemetry path (e.g. `proof/telemetry/runs.ndjson`).
- [ ] Line fields: timestamp, command, ruleIds-in-scope count, findings by severity, duration ms, exit status.
- [ ] Line validated against an Effect schema in `schemas/effect/` before write; invalid shape fails the run.
- [ ] Append is atomic and newline-terminated; a crashed run does not write a half line.
- [ ] Telemetry emission never changes exit code or findings (observer, not gate).

## Acceptance And Proof
Tier T1 (P1 unit). Prove via `tests/run-telemetry.test.mjs`: a scripted run appends exactly one valid NDJSON line; a forced schema violation is rejected; two runs append two independently-parseable lines. Mechanism: schema-decode-then-append writer, asserted by re-parsing every emitted line.

## Parallel Ownership Notes
Depends on d01 for ruleId enumeration in scope. Downstream d05 and d10 consume this NDJSON. Owns disjoint telemetry/schema files, so runs concurrently with d02/d03.
