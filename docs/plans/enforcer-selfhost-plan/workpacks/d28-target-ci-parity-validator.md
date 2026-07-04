# d28 Target CI Parity Validator

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Target CI Parity Validator`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `src/validators/target-ci-parity.ts, src/validators/target-ci-parity-manifest.ts, tests/target-ci-parity.test.mjs, tests/fixtures/target-ci-parity/**`
- deps: `d01-rule-mechanization-engine`
- tier: `P2`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
Existing **d11-ci-parity-validator** proves local==CI parity for the enforcer's OWN repo. ADBP's `ergonomics/ci` + `commands/audit-ci` (rows CIP-1.1..1.3, CIGATE-1.1..1.7, DOCGATE-1.1/1.2 in [ADBP_GAPS](../ADBP_GAPS.md#group-3--command--ergonomics-gates)) require the SAME parity check to run against an arbitrary SCANNED target repo — a distinct surface d11 does not cover. Nothing today audits a target project's local-hook-runner vs its CI job set, pinned-version agreement, coverage `fail_under` floor, path-filter parity, recipe/linter-freshness drift, or the presence of `ARCHITECTURE.md`/`decisions.md`.

## Where We Want To Be
A target-repo validator (complementing, not replacing, d11) that, given a scanned project root, asserts:
- Local hook runner exists and its check set == CI job check set at the SAME pinned tool versions — CIP-1.1, CIGATE-1.3.
- Ecosystem-idiomatic runner present per stack (pre-commit / lefthook / githooks+xtask) — CIP-1.2.
- Complete gate surface: format, lint, test, coverage floor, dep-audit, secret scan, local hook runner, CI parity — CIGATE-1.4.
- Coverage `fail_under` / vitest thresholds / `--cov-fail-under` present AND the test step actually invokes the coverage flag — CIGATE-1.1, CIGATE-1.2.
- Each sub-project path covered by a workflow `paths:` trigger — CIGATE-1.6.
- Deployed-linter freshness: version + hash match the recipe (recipe-drift) — CIGATE-1.7, CIP-1.3.
- Doc-presence: `ARCHITECTURE.md` with required H2s + `decisions.md` at root — DOCGATE-1.1, DOCGATE-1.2.

## Requirement Checklist
- [ ] Parse the target's local-hook manifest and CI job definition into comparable check sets; local-only or CI-only check fails (CIP-1.1, CIGATE-1.3).
- [ ] Assert pinned tool versions agree between local runner and CI (fail on skew).
- [ ] Assert an idiomatic hook runner file exists for each detected stack (CIP-1.2); missing runner fails.
- [ ] Assert the full gate surface is present (format/lint/test/coverage/dep-audit/secret-scan/runner/parity) — CIGATE-1.4.
- [ ] Assert a failing coverage floor is configured AND invoked by the test step (CIGATE-1.1/1.2).
- [ ] Assert each sub-project path has a matching workflow `paths:` trigger (CIGATE-1.6).
- [ ] Assert deployed-linter version+hash match the recipe (CIGATE-1.7/CIP-1.3).
- [ ] Assert `ARCHITECTURE.md` (with required H2s) and `decisions.md` presence (DOCGATE-1.1/1.2).
- [ ] Deterministic over target-repo manifests; failure names the mismatched check/version/path/doc.

## Acceptance And Proof
Tier P2. Structural checks are T1 (presence/set-equality/version-match) except DOCGATE header-completeness which is T2 (scored on required-H2 coverage). Select detection tests in TEST_PROOF_EXPECTATIONS.md before DONE. Fixtures are synthetic target-repo trees under `tests/fixtures/target-ci-parity/`.

Per-rule 5-way parity (ruleId <-> doc <-> validator <-> {fail,pass} <-> test):
- **CIP-1.1 / CIGATE-1.3 (check-set + version parity):** fail-fixture `tests/fixtures/target-ci-parity/cip-1.1/fail-local-only-check/` (local runner runs a check/version absent from CI) flagged; pass-fixture `.../pass-identical-sets/` clean.
- **CIP-1.2 (idiomatic runner present):** fail-fixture `.../cip-1.2/fail-no-hook-runner/`; pass-fixture `.../pass-lefthook-present/`.
- **CIGATE-1.1/1.2 (coverage floor set + invoked):** fail-fixture `.../cigate-1.1/fail-no-fail-under/` (`[tool.coverage]` without `fail_under`; `vitest run` without `--coverage`); pass-fixture `.../pass-fail-under-70/`.
- **CIGATE-1.4 (complete gate surface):** fail-fixture `.../cigate-1.4/fail-missing-secret-scan/`; pass-fixture `.../pass-full-surface/`.
- **CIGATE-1.6 (path-filter parity):** fail-fixture `.../cigate-1.6/fail-subproject-no-paths/` (sub-project with no matching workflow `paths:`); pass-fixture `.../pass-paths-covered/`.
- **CIGATE-1.7 / CIP-1.3 (linter freshness / recipe drift):** fail-fixture `.../cigate-1.7/fail-stale-linter-hash/`; pass-fixture `.../pass-version-hash-match/`.
- **DOCGATE-1.1 (ARCHITECTURE.md + required H2s, T2):** fail-fixture `.../docgate-1.1/fail-missing-h2s/` (score below threshold); pass-fixture `.../pass-all-h2s/`. **DOCGATE-1.2 (decisions.md present, T1):** fail-fixture `.../docgate-1.2/fail-no-decisions/`; pass-fixture `.../pass-decisions-present/`.
- Detection test for all rows: `tests/target-ci-parity.test.mjs`.

## Parallel Ownership Notes
Depends on d01 for the validator harness. COMPLEMENTS d11 (which audits the enforcer's OWN repo) — this pack audits arbitrary SCANNED target repos and owns a disjoint `src/validators/target-ci-parity.*` + `tests/fixtures/target-ci-parity/**`. No file overlap with d11 (`src/ci-parity.ts`). Doc-presence rows overlap conceptually with e-pack-plan-gates' DOCGATE lane, but that pack owns plan/check-gate files; this pack asserts DOCGATE only within the CI-parity validator against target-repo fixtures — disjoint files, concurrent-safe.
