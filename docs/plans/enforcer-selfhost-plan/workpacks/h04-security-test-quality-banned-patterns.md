# h04 Security Test Quality Banned Patterns

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Security Test Quality Banned Patterns`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-security/src/rules/security_test_quality.rs, crates/enforcer-security/tests/fixtures/security_test_quality/**`
- deps: `arc-19-enforcer-security, arc-14-enforcer-mechanization, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine, d23-test-companion-and-quality`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
The source spec §7.2/§8.4.1 bans ten security-test anti-patterns and §7.1/§8.4.2 requires six positive properties, but this is PROSE — a target-project test that only asserts success, mocks the money logic it claims to protect, or passes even after the protection is deleted currently ships silently. d23 governs generic test quality (companion presence, assertion-free, naming) in `enforcer-lang-common`'s `test_quality.rs`, but has no notion of a *security* test's threat mapping, invariant assertion, or "fails-if-protection-removed" obligation. There is no `Validator` that reads a target test file classified as security/money-critical and rejects the banned shapes.

## Where We Want To Be
A `security_test_quality` rule module in `enforcer-security` (arc-19) — a `crates/enforcer-security/src/rules/security_test_quality.rs` implementing the `Validator` trait (from `enforcer-validator`, arc-05), emitting structured `Finding`s, with each `RuleId` a typed rule record in `enforcer-rules` (arc-04), scaffolded through d01 (arc-14). It parses target security/money-critical test bodies via `tree-sitter` (TS/JS/Python/Dart targets) / `syn` (Rust targets), composing d23's classifier convention and d03's deferred-work/waiver gate (both consumed read-only, never edited):
- T1 (blocks) BANNED: `asserts-success-only`, `pass-if-logic-deleted`, `rely-on-no-crash`, `snapshot-only`, `mocks-for-money-logic`, `non-deterministic-fuzz` (no logged seed), `order-dependent`, `global-mutation`.
- T1 (blocks) REQUIRED-presence: `asserts-rejection` (test asserts an operation is refused), `reproducible-seed` (seed logged for fuzz/property).
- T2 (scored): `no-threat-mapping`, `no-invariant-assertion`, `exact-failure-mode` (asserts the specific rejection mode, not any throw), `fails-if-protection-removed` (mutation-style presence heuristic) — each emits `score`+`confidence` on the scored model.

## Requirement Checklist
Each rule is scaffolded via d01 `rule new`, landing a doc-anchor in its `enforcer-rules` record, the `Validator` impl in `src/rules/security_test_quality.rs`, and a fail+pass fixture pair under `crates/enforcer-security/tests/fixtures/security_test_quality/`.

- [ ] Rule records registered in `enforcer-rules`, one per ruleId, each carrying its tier. (Optional human-canonical `.md` may live in the g08 rules explorer surface.)
- [ ] T1 banned-pattern detection over security-classified target tests: success-only, pass-if-deleted, no-crash, snapshot-only, mocks-for-money-logic, non-deterministic-fuzz, order-dependent, global-mutation each emit a `Finding`.
- [ ] T1 required-presence: missing rejection assertion or missing logged seed (fuzz/property) emits a `Finding`.
- [ ] T2 scored: threat-mapping, invariant assertion, exact-failure-mode, fails-if-protection-removed emit `score`+`confidence` against a threshold.
- [ ] Composes d23 (test classification, `enforcer-lang-common`) and d03 (deferred/waiver gate, `enforcer-lang-common`) without editing their files.
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> rule-record <-> `Validator` <-> {fail,pass} fixtures <-> `cargo test` detection.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
5-way parity per rule, Rust-native (`Validator` impl + fail/pass fixtures + a `cargo test` detection test). Fixtures live under `crates/enforcer-security/tests/fixtures/security_test_quality/` (target test files in TS/JS/Python/Rust as the enforcer validates a user's test code).

- `mocks-for-money-logic` (T1): fail `mocks_for_money_logic/bad/mock_money.test.ts` (mocks the balance/settlement unit under test — flagged); pass `mocks_for_money_logic/good/real_money.test.ts` (exercises real money logic, asserts rejection with reproducible seed — clean).
- `asserts-success-only` / `asserts-rejection` (T1): fail `asserts_success_only/bad/success_only.test.ts` (only `expect(res.ok)` — flagged); pass `asserts_success_only/good/rejects.test.ts` (asserts the operation is refused).
- `non-deterministic-fuzz` / `reproducible-seed` (T1): fail `reproducible_seed/bad/no_seed.test.ts` (fuzz with unlogged random — flagged); pass `reproducible_seed/good/seeded.test.ts` (logged seed).
- `order-dependent` / `global-mutation` (T1): fail `global_mutation/bad/shared_state.test.ts` (mutates module global across cases — flagged); pass `global_mutation/good/isolated.test.ts`.
- `no-threat-mapping` / `no-invariant-assertion` / `exact-failure-mode` (T2): fail `threat_mapping/bad/unmapped.test.ts` (no threat tag, generic throw — score crosses); pass `threat_mapping/good/mapped.test.ts` (threat annotation + invariant + exact failure mode — under).

Prove via `cargo test -p enforcer-security` (all security_test_quality fixtures) and the d01 `rule-scaffold-parity` oracle. Update TEST_PROOF_EXPECTATIONS.md rows before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `crates/enforcer-security/src/rules/security_test_quality.rs` and `crates/enforcer-security/tests/fixtures/security_test_quality/**` — a specific rule module inside the arc-19 crate, NOT the crate itself. Depends on `arc-19` (crate skeleton + module-root + `Validator` registration), `arc-14`/`d01` (scaffolder + parity), `arc-05` (the `Validator` trait), `arc-04` (the rule registry), and `d23` (test classification/companion convention in `enforcer-lang-common`, consumed read-only — d23 owns generic quality, h04 owns security-test shape). Composes d03's waiver gate; does not edit d03. Sibling h05 owns the economic-invariant property suite; not touched here. Sequenced after arc-19's skeleton, d01 and d23 land.
