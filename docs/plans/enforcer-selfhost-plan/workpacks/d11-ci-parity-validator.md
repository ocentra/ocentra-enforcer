# d11 CI Parity Validator

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `CI Parity Validator`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/ci-parity.ts, scripts/ci-parity-verify.mjs, tests/ci-parity.test.mjs`
- deps: `d01-rule-mechanization-engine`
- tier: `P2 CI cross-platform`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
`scripts/ci-local.mjs` lists local steps and there is an `enforcer:verify:ci` script, but drift between local hooks and CI jobs (or between pinned tool versions) is possible. ADBP's "local == CI" is a guideline, not a check.

## Where We Want To Be
A T1 validator asserting the set of local hook/steps equals the set of CI jobs, and that pinned versions (node, cargo/rust-toolchain, key deps) match between the two sources of truth.

## Requirement Checklist
- [ ] Parse local step list (`scripts/ci-local.mjs`) and the CI job definition into comparable step sets.
- [ ] Assert step-set equality: any local-only or CI-only step fails closed.
- [ ] Assert pinned versions agree (e.g. `rust-toolchain.toml`, engines in `package.json`, action/tool versions).
- [ ] Validator is deterministic and runs both locally and as a CI job (self-referential parity).
- [ ] Failure names the specific mismatched step or version.

## Acceptance And Proof
Tier T1, P2 CI cross-platform. Prove via `tests/ci-parity.test.mjs`: injected extra local step fails; injected version skew fails; matched sets pass. CI job runs `scripts/ci-parity-verify.mjs`. Mechanism: normalized set/version diff between the local step manifest and the CI manifest, fail-closed on any delta.

## Parallel Ownership Notes
Depends on d01 conventions only lightly (shares validator harness). Owns disjoint ci-parity files; shares a CI stage with d05 but no files, so concurrent.
