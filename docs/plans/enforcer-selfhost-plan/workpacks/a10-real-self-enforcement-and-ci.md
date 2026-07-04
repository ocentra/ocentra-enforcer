# a10 Real Self Enforcement And CI

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Real Self Enforcement And CI`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `scripts/ci-local.mjs`, `.github/workflows/**`, `package.json#scripts.enforcer:self`, `package.json#scripts.enforcer:self:scan`, `package.json#scripts.enforcer:all`
- deps: `a01`, `a09`
- tier: `P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`enforcer:self` runs only `check source-shape`; `scripts/ci-local.mjs` and any `.github/workflows/` do not run `typecheck` or the TS-lane self-scan, and self-enforcement does not hard-fail on real findings (it leans on the a08 overrides to stay green). Self-enforcement today is partial and forgiving.

## Where We Want To Be
`enforcer:self` runs the full TS-lane enforcement (source-shape + scan + coverage + policy) and hard-fails on any real finding; `npm run typecheck` and `enforcer:self` are mandatory gates in `scripts/ci-local.mjs` and a GitHub Actions workflow.

## Requirement Checklist
- [ ] `enforcer:self` executes the TS lanes end-to-end and exits non-zero on any error-severity finding.
- [ ] `scripts/ci-local.mjs` runs `typecheck` then `enforcer:self` then tests, failing the run on any non-zero.
- [ ] A `.github/workflows/*` job runs the same gates on push/PR across the supported Node range.
- [ ] With a09's honest skips, a hollow (zero-ran) self-scan fails CI rather than passing.
- [ ] No `--no-verify`-style bypass; waivers (a08) are the only sanctioned exceptions.

## Acceptance And Proof
Tier P4. Proof: seed a self-violating fixture and show `enforcer:self` + `ci-local` exit non-zero; show `typecheck` failure fails CI; show the workflow file invokes both gates. A green run on the migrated tree with visible ran-counts. Rows in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on a01 (typecheck) and a09 (honest skips, so hard-fail is meaningful). Owns CI scripts/workflows and the three `enforcer:*` script keys exclusively; a01 owns the disjoint `build`/`typecheck` keys. This is the capstone that turns the whole track green under real gates.
