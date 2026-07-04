# h02 Required Test Categories Gate

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Required Test Categories Gate`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-security/src/rules/required_test_categories.rs, crates/enforcer-security/tests/fixtures/required_test_categories/**`
- deps: `arc-19-enforcer-security, arc-14-enforcer-mechanization, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine, h01-money-critical-classifier, d23-test-companion-and-quality`
- tier: `P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
The spec (§4, §8.3) says a build MUST FAIL if any money-critical endpoint or change lacks the mandated seven test categories, but no `Validator` maps money-critical units to their test coverage. h01 now classifies and registers money-critical units into the manifest; d23 supplies the test-companion/quality `test_quality.rs` category convention in `enforcer-lang-common`. What is missing is the coverage gate that joins the two: no `Validator` asserts that each money-critical unit actually carries a negative, replay, concurrency, rollback/compensation, economic-exhaustion, time-based, and signing/verification test.

## Where We Want To Be
A T1 coverage gate in `enforcer-security` (arc-19) — a `crates/enforcer-security/src/rules/required_test_categories.rs` module implementing the `Validator` trait (from `enforcer-validator`, arc-05), emitting structured `Finding`s, with each `RuleId` a typed rule record in `enforcer-rules` (arc-04), scaffolded through d01 (arc-14) with full 5-way parity (fixtures `crates/enforcer-security/tests/fixtures/required_test_categories/**`). It deserializes the h01 money-critical manifest (`serde` record), resolves each unit's associated test files (filesystem/module check; test bodies read via `syn`/`tree-sitter`), and asserts all seven required categories are present. Category membership is detected mechanically (annotation/tag or category-named test file per the d23 companion convention consumed read-only from `enforcer-lang-common`). Missing any of the seven ⇒ `Finding`. GENERIC across any value system — endpoints, jobs, or ledger mutations — not crypto-specific.

## Requirement Checklist
Each rule is scaffolded via d01 `rule new`, landing a doc-anchor in its `enforcer-rules` record, a `Validator` impl in `src/rules/required_test_categories.rs`, and a fail+pass fixture pair under `crates/enforcer-security/tests/fixtures/required_test_categories/<rule>/{bad,good}/`.

- [ ] **T1 REQ-TESTCAT-SEVEN — all seven categories present (§4/§8.3).** Each money-critical unit MUST carry negative + replay + concurrency + rollback/compensation + economic-exhaustion + time-based + signing/verification tests. Any missing ⇒ `Finding`.
- [ ] **T1 REQ-TESTCAT-MAP — unit→test resolution.** Every h01-classified unit MUST resolve to at least one test file bearing a recognized category tag; an unresolvable unit ⇒ `Finding`.
- [ ] **T1 REQ-TESTCAT-CATEGORY-TAGGING — category detection.** Category is determined by annotation/tag or category-named file (d23 convention), not by heuristic guessing; untagged security tests do not satisfy a category.
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> rule-record <-> `Validator` <-> {fail,pass} fixtures <-> `cargo test` detection.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier P4. 5-way parity per rule, Rust-native (`Validator` impl + fail/pass fixtures + a `cargo test` detection test). Fixtures pair a money-critical unit (from an h01-shaped manifest stub) with a set of test files; fail-fixtures omit one or more categories, pass-fixtures carry all seven.

Representative triples:
- missing categories: fail `crates/enforcer-security/tests/fixtures/required_test_categories/endpoint_a/bad/missing_replay_concurrency/` (endpoint present, no replay + no concurrency test → flagged), req-testcat-seven detection test.
- complete set: pass `crates/enforcer-security/tests/fixtures/required_test_categories/endpoint_a/good/all_seven/` (all seven category-tagged tests present → clean), req-testcat-seven detection test.
- unresolvable unit: fail `crates/enforcer-security/tests/fixtures/required_test_categories/orphan/bad/unit_no_tests/`, req-testcat-map detection test.

Prove via `cargo test -p enforcer-security` (all required_test_categories fixtures) and the d01 `rule-scaffold-parity` oracle; record detection-test artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `crates/enforcer-security/src/rules/required_test_categories.rs` and `crates/enforcer-security/tests/fixtures/required_test_categories/**` — a specific rule module inside the arc-19 crate, NOT the crate itself. Depends on `arc-19` (crate skeleton + module-root + `Validator` registration), `arc-14`/`d01` (scaffold + parity), `arc-05` (the `Validator` trait), `arc-04` (the rule registry), `h01` (money-critical manifest — consumed, not redefined), and `d23` (the `test_quality.rs` category convention in `enforcer-lang-common` — reused read-only, not re-authored). Sequenced after arc-19's skeleton exists. Distinct from h03 which asserts threat/invariant/property/concurrency/replay MAPPING per unit; this pack asserts CATEGORY COVERAGE presence. Concurrency/replay appear in both surfaces but h02 owns only the seven-category coverage check.
