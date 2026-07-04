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

- owns: `rules/security/required-test-categories.md, src/validators/required-security-tests.ts, tests/fixtures/required-security-tests/**`
- deps: `d01, h01, d23`
- tier: `P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
The spec (§4, §8.3) says CI MUST FAIL if any money-critical endpoint or change lacks the mandated seven test categories, but nothing maps money-critical units to their test coverage. h01 now classifies and registers money-critical units; d23 supplies test-companion/quality primitives. What is missing is the coverage gate that joins the two: no validator asserts that each money-critical unit actually carries a negative, replay, concurrency, rollback/compensation, economic-exhaustion, time-based, and signing/verification test.

## Where We Want To Be
A T1 coverage gate scaffolded via d01 with full 5-way parity: doc `rules/security/required-test-categories.md`, validator `src/validators/required-security-tests.ts`, fixtures `tests/fixtures/required-security-tests/**`. It reads the h01 money-critical manifest, resolves each unit's associated test files, and asserts all seven required categories are present. Category membership is detected mechanically (annotation/tag or category-named test file per the d23 companion convention). Missing any of the seven ⇒ CI fail. GENERIC across any value system — endpoints, jobs, or ledger mutations — not crypto-specific.

## Requirement Checklist
Scaffolded with `enforcer rule new <ID>` (d01), landing doc anchor, validator, and fail+pass fixtures.

- [ ] **T1 REQ-TESTCAT-SEVEN — all seven categories present (§4/§8.3).** Each money-critical unit MUST carry negative + replay + concurrency + rollback/compensation + economic-exhaustion + time-based + signing/verification tests. Any missing ⇒ fail.
- [ ] **T1 REQ-TESTCAT-MAP — unit→test resolution.** Every h01-classified unit MUST resolve to at least one test file bearing a recognized category tag; an unresolvable unit ⇒ fail.
- [ ] **T1 REQ-TESTCAT-CATEGORY-TAGGING — category detection.** Category is determined by annotation/tag or category-named file, not by heuristic guessing; untagged security tests do not satisfy a category.

## Acceptance And Proof
Tier P4. Fixtures pair a money-critical unit (from an h01-shaped manifest stub) with a set of test files; fail-fixtures omit one or more categories, pass-fixtures carry all seven.

Representative triples:
- missing categories: fail `tests/fixtures/required-security-tests/endpoint-a/fail_missing_replay_concurrency/` (endpoint present, no replay + no concurrency test → flagged), test `req-testcat-seven.test`.
- complete set: pass `tests/fixtures/required-security-tests/endpoint-a/pass_all_seven/` (all seven category-tagged tests present → clean), test `req-testcat-seven.test`.
- unresolvable unit: fail `tests/fixtures/required-security-tests/orphan/fail_unit_no_tests/`, test `req-testcat-map.test`.

Re-run the d01 `rule-scaffold-parity` oracle; record detection-test artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `rules/security/required-test-categories.md`, `src/validators/required-security-tests.ts`, and `tests/fixtures/required-security-tests/**` exclusively; disjoint from siblings. Depends on d01 (scaffold), h01 (money-critical manifest — consumed, not redefined), and d23 (test-companion/quality category convention — reused, not re-authored). Distinct from h03 which asserts threat/invariant/property/concurrency/replay MAPPING per unit; this pack asserts CATEGORY COVERAGE presence. Concurrency/replay appear in both surfaces but h02 owns only the seven-category coverage check.
