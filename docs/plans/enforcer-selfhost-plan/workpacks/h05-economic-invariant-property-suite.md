# h05 Economic Invariant Property Suite

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Economic Invariant Property Suite`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-security/src/rules/economic_invariants.rs, crates/enforcer-security/tests/fixtures/economic_invariants/**`
- deps: `arc-19-enforcer-security, arc-14-enforcer-mechanization, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine, h01-money-critical-classifier`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
Spec §2.3 enumerates ten economic/logic invariants (map to G1–G6) that a money-critical unit MUST guarantee, and §4/§8.3 demand they be property-based suites that fail the build when absent. Today no `Validator` checks that a money-critical unit (identified by h01's money-critical classifier manifest) carries a property test for each invariant, nor that the property asserts the correct shape (a refutation over generated inputs, not a single hand-picked case). A settlement module can ship with zero property coverage of "failure ≠ reward" and pass.

## Where We Want To Be
An `economic_invariants` rule module in `enforcer-security` (arc-19) — a `crates/enforcer-security/src/rules/economic_invariants.rs` implementing the `Validator` trait (from `enforcer-validator`, arc-05), emitting structured `Finding`s, with each `RuleId` a typed rule record in `enforcer-rules` (arc-04), scaffolded through d01 (arc-14). It parses target test bodies via `tree-sitter` (TS/JS/Python/Dart targets) / `syn` (Rust targets) and, for each money-critical unit (scoped from the h01 manifest), checks presence and assertion-shape of a property test per invariant:
- `same-request-twice != more-value` · `failure != reward` · `retry != mutation` · `partial-failure != profit` · `order != advantage` · `attacker-cost >= system-cost` · `compensation idempotent+replay-safe` · `time-assumptions fail-closed` · `backend-never-signs-unverifiable` · `emergency-reduces-blast`.
- T1 (blocks): for a unit classified money-critical by h01, the corresponding invariant property must be present AND have property-test shape — a generator-driven refutation of the bad outcome (`fast-check` `fc.assert(fc.property(...))` for TS/JS, `proptest!`/`quickcheck` for Rust, Hypothesis `@given` for Python), not a single literal case. Missing property or non-property shape emits a `Finding`.

## Requirement Checklist
Each rule is scaffolded via d01 `rule new`, landing a doc-anchor in its `enforcer-rules` record, the `Validator` impl in `src/rules/economic_invariants.rs`, and a fail+pass fixture pair under `crates/enforcer-security/tests/fixtures/economic_invariants/`.

- [ ] Rule records registered in `enforcer-rules`, one per invariant ruleId, each carrying its tier. (Optional human-canonical `.md` may live in the g08 rules explorer surface.)
- [ ] T1 presence: a money-critical unit missing any required invariant property emits a `Finding`; a unit with all ten is clean.
- [ ] T1 assertion-shape: an invariant "test" that is a single literal case (not a generator-driven property refutation) emits a `Finding`.
- [ ] Consumes h01's money-critical classification manifest to scope which units require the suite; does not redefine classification.
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> rule-record <-> `Validator` <-> {fail,pass} fixtures <-> `cargo test` detection.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
5-way parity per rule, Rust-native (`Validator` impl + fail/pass fixtures + a `cargo test` detection test). Fixtures live under `crates/enforcer-security/tests/fixtures/economic_invariants/` (target test files in TS/JS/Python/Rust as the enforcer validates a user's test code).

- `failure != reward` (T1): fail `failure_not_reward/bad/` (money-critical unit + property suite covering nine invariants but NO `failure != reward` property — flagged); pass `full_suite/good/` (all ten properties present with property-test shape — clean).
- `same-request-twice != more-value` (T1): fail `idempotency/bad/single_case.test.ts` (one hand-picked replay assertion, not a property — flagged as non-property shape); pass `idempotency/good/property.test.ts` (`fc.assert(fc.property(...))` refuting extra value on replay).
- `attacker-cost >= system-cost` (T1): fail `attacker_cost/bad/` (invariant undocumented/absent — flagged); pass `attacker_cost/good/` (property over generated attacker inputs).
- `compensation idempotent+replay-safe` (T1): fail `compensation/bad/`; pass `compensation/good/`.
- non-money-critical scope: pass `plain_util/good/` (unit not classified money-critical by h01 — no suite required, clean).

Prove via `cargo test -p enforcer-security` (all economic_invariants fixtures) and the d01 `rule-scaffold-parity` oracle. Update TEST_PROOF_EXPECTATIONS.md rows before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `crates/enforcer-security/src/rules/economic_invariants.rs` and `crates/enforcer-security/tests/fixtures/economic_invariants/**` — a specific rule module inside the arc-19 crate, NOT the crate itself. Depends on `arc-19` (crate skeleton + module-root + `Validator` registration), `arc-14`/`d01` (scaffolder + parity), `arc-05` (the `Validator` trait), `arc-04` (the rule registry), and `h01` (money-critical classifier manifest, consumed read-only to scope which units require the suite — h05 does not redefine what counts as money-critical). Sibling h04 (security-test-shape bans) is not touched here; h05 owns only the invariant-property presence/shape obligation. Sequenced after arc-19's skeleton, d01 and h01 land.
