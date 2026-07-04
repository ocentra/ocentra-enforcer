# h01 Money-Critical Classifier

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Money-Critical Classifier`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-security/src/rules/money_critical.rs, crates/enforcer-security/tests/fixtures/money_critical/**`
- deps: `arc-19-enforcer-security, arc-14-enforcer-mechanization, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine`
- tier: `P0/P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
The `enforcer-rules` registry has no notion of "money-critical code". The ingested spec (§8.2) defines it in prose — "if unsure, treat as money-critical" — but no `Validator` detects or gates it. Every downstream security-testing rule (h02 required categories, h03 threat/invariant mapping, h05 economic-invariant suite) needs a mechanical answer to "is this unit money-critical?" and there is none. Value handling today is invisible to the enforcer; adjacent detection existed only in the retired Node engine.

## Where We Want To Be
A foundational classifier family in `enforcer-security` (arc-19) — a `crates/enforcer-security/src/rules/money_critical.rs` module implementing the `Validator` trait (from `enforcer-validator`, arc-05), emitting structured `Finding`s (from `enforcer-domain`), with each `RuleId` carried as a typed rule record in `enforcer-rules` (arc-04), scaffolded through d01 (arc-14) with full 5-way parity (fixtures `crates/enforcer-security/tests/fixtures/money_critical/**`). Target code is parsed with `syn` (Rust) / `tree-sitter` (TS/JS/Python/Dart) so classification is AST-driven, not regex. The money-critical manifest is a `serde` record with a branded newtype key (`RuleId`/`RelPath` from `enforcer-domain`, never a bare `String`). A T2 scored classifier tags any unit that creates/transfers/modifies/destroys value; performs economic calculation; applies rewards/credits/balances/cooldowns; signs or authorizes payments; executes rollback/compensation; changes time-based state; or toggles kill-switches — GENERICALLY across any value system (fiat, Stripe, AWS-billed metering, internal ledger, or the optional crypto/Anchor instance), never crypto-only. A T1 gate then requires every classified unit to be explicitly annotated/registered in the money-critical manifest; unannotated-but-classified code fails. Doctrine: silence ≠ permission; if unsure, treat as money-critical.

## Requirement Checklist
Each rule is scaffolded via d01 `rule new`, landing a doc-anchor in its `enforcer-rules` record, a `Validator` impl in `src/rules/money_critical.rs`, and a fail+pass fixture pair under `crates/enforcer-security/tests/fixtures/money_critical/<rule>/{bad,good}/`.

- [ ] **T2 MONEY-CRIT-CLASSIFY — scored classifier (§8.2).** A `syn`/`tree-sitter` AST `Validator` scores units on the enumerated value-touching signals (balance/credit/reward/cooldown mutation, transfer/mint/burn, economic calc, payment sign/authorize, rollback/compensation, time-based state change, kill-switch toggle), emitting `score`+`confidence` on the scored model. Crossing threshold ⇒ classified money-critical.
- [ ] **T1 MONEY-CRIT-ANNOTATED — annotation/registration gate (§8.2).** A classified unit MUST carry an explicit annotation and appear in the money-critical manifest (the `serde` record); classified-but-unannotated ⇒ `Finding`.
- [ ] **T1 MONEY-CRIT-UNSURE-DEFAULT — "if unsure, treat as money-critical".** Ambiguous value-adjacent units default to money-critical unless explicitly annotated otherwise.
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> rule-record <-> `Validator` <-> {fail,pass} fixtures <-> `cargo test` detection.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier P0/P1. 5-way parity per rule, Rust-native (`Validator` impl + fail/pass fixtures + a `cargo test` detection test). T2 classifier fixtures assert the score crosses the fail threshold for value-touching code and stays under it for neutral code; the T1 gate fixtures assert a `Finding` on unannotated classified units and clean on annotated+registered ones.

Representative triples:
- balance-crediting fn: fail `crates/enforcer-security/tests/fixtures/money_critical/classify/bad/credit_balance.rs` (classified, unannotated → flagged), pass `.../good/credit_balance_annotated.rs`, `#[test]` detection in `money_critical.rs`.
- pure formatter: fail-negative `crates/enforcer-security/tests/fixtures/money_critical/classify/good/pure_formatter.rs` (below threshold, not classified, must stay clean), asserted in the classify detection test.
- payment-signing fn unannotated: fail `crates/enforcer-security/tests/fixtures/money_critical/annotated/bad/sign_payment_unannotated.rs` (T1 flag), pass `.../good/sign_payment_registered.rs`, annotated-gate detection test.

Prove via `cargo test -p enforcer-security` (all money_critical fixtures) and the d01 `rule-scaffold-parity` oracle; record detection-test artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `crates/enforcer-security/src/rules/money_critical.rs` and `crates/enforcer-security/tests/fixtures/money_critical/**` — a specific rule module inside the arc-19 crate, NOT the crate itself. Depends on `arc-19` (crate skeleton + `src/rules/` module-root + `Validator` registration), `arc-14`/`d01` (scaffolder + parity oracle), `arc-05` (the `Validator` trait + parity harness), and `arc-04` (the rule registry); sequenced after arc-19's skeleton exists. This pack is foundational: h02, h03, and h05 key off the money-critical manifest it produces but must not redefine classification — they consume the manifest and own their own rule/fixture surfaces. Distinct from d18 security-stop (vulnerability source-patterns) which does not classify value, and from the arc-19 no-bypass meta-check.
