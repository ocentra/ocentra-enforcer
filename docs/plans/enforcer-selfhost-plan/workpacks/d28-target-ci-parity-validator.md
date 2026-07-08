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

- owns: `crates/enforcer-harness/src/target_ci_parity.rs, crates/enforcer-harness/tests/fixtures/target_ci_parity/**`
- deps: `arc-18-enforcer-harness, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine`
- tier: `P2`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Existing **d11-ci-parity-validator** proves local==CI parity for the enforcer's OWN repo (its `enforcer-harness` `ci_parity.rs`). ADBP's `ergonomics/ci` + `commands/audit-ci` (rows CIP-1.1..1.3, CIGATE-1.1..1.7, DOCGATE-1.1/1.2 in [ADBP_GAPS](../ADBP_GAPS.md#group-3--command--ergonomics-gates)) require the SAME parity check to run against an arbitrary SCANNED target repo — a distinct surface d11 does not cover. Nothing in `enforcer-harness` today audits a target project's local-hook-runner vs its CI job set, pinned-version agreement, coverage `fail_under` floor, path-filter parity, recipe/linter-freshness drift, or the presence of `ARCHITECTURE.md`/`decisions.md`.

## Where We Want To Be
A `target_ci_parity` module in `enforcer-harness` (arc-18) — a `crates/enforcer-harness/src/target_ci_parity.rs` implementing the `Validator` trait (from `enforcer-validator`, arc-05), emitting structured `Finding`s, keyed to typed rule records in `enforcer-rules` (arc-04), scaffolded through d01 — complementing, not replacing, d11's `ci_parity.rs`. Any native-tool invocation (deployed-linter version/hash probe) goes through the arc-18 run-adapters. Given a scanned project root it asserts:
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
- [ ] Deterministic over target-repo manifests; the `Finding` names the mismatched check/version/path/doc.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier P2, Rust-native (`Validator` impl + fail/pass fixtures + `cargo test` detection). Structural checks are T1 (presence/set-equality/version-match) except DOCGATE header-completeness which is T2 (scored on required-H2 coverage). Select detection tests in TEST_PROOF_EXPECTATIONS.md before DONE. Fixtures are synthetic target-repo trees under `crates/enforcer-harness/tests/fixtures/target_ci_parity/`.

Per-rule 5-way parity (ruleId <-> rule-record <-> validator <-> {fail,pass} <-> `cargo test`):
- **CIP-1.1 / CIGATE-1.3 (check-set + version parity):** fail-fixture `crates/enforcer-harness/tests/fixtures/target_ci_parity/cip_1_1/bad/` (local runner runs a check/version absent from CI) flagged; pass-fixture `.../cip_1_1/good/` clean.
- **CIP-1.2 (idiomatic runner present):** fail-fixture `.../cip_1_2/bad/`; pass-fixture `.../cip_1_2/good/` (lefthook present).
- **CIGATE-1.1/1.2 (coverage floor set + invoked):** fail-fixture `.../cigate_1_1/bad/` (`[tool.coverage]` without `fail_under`; `vitest run` without `--coverage`); pass-fixture `.../cigate_1_1/good/` (`fail_under=70`).
- **CIGATE-1.4 (complete gate surface):** fail-fixture `.../cigate_1_4/bad/` (missing secret-scan); pass-fixture `.../cigate_1_4/good/` (full surface).
- **CIGATE-1.6 (path-filter parity):** fail-fixture `.../cigate_1_6/bad/` (sub-project with no matching workflow `paths:`); pass-fixture `.../cigate_1_6/good/`.
- **CIGATE-1.7 / CIP-1.3 (linter freshness / recipe drift):** fail-fixture `.../cigate_1_7/bad/` (stale linter hash); pass-fixture `.../cigate_1_7/good/` (version+hash match).
- **DOCGATE-1.1 (ARCHITECTURE.md + required H2s, T2):** fail-fixture `.../docgate_1_1/bad/` (score below threshold); pass-fixture `.../docgate_1_1/good/`. **DOCGATE-1.2 (decisions.md present, T1):** fail-fixture `.../docgate_1_2/bad/`; pass-fixture `.../docgate_1_2/good/`.
- Detection test for all rows: `#[test]` under `#[cfg(test)]` in `target_ci_parity.rs`, proven by `cargo test -p enforcer-harness`.

## Parallel Ownership Notes
Depends on `arc-18` (which stands up the `enforcer-harness` crate skeleton + module root + run-adapters + `Validator` registration), `arc-05` (the `Validator` trait + parity harness), `arc-04` (the rule registry), and `d01` (the validator harness/scaffold conventions). COMPLEMENTS d11 (which audits the enforcer's OWN repo) — this pack audits arbitrary SCANNED target repos and owns a disjoint `crates/enforcer-harness/src/target_ci_parity.rs` + `crates/enforcer-harness/tests/fixtures/target_ci_parity/**` — a specific module inside the arc-18 crate, NOT the crate itself. Sequenced after arc-18's skeleton exists. No file overlap with d11 (`crates/enforcer-harness/src/ci_parity.rs`). Doc-presence rows overlap conceptually with the plan-gates DOCGATE lane, but that lane owns plan/check-gate files; this pack asserts DOCGATE only within the CI-parity validator against target-repo fixtures — disjoint files, concurrent-safe.
