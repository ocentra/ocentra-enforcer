# h03 Threat Test Invariant Mapping

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Threat Test Invariant Mapping`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-security/src/rules/threat_test_mapping.rs, crates/enforcer-security/tests/fixtures/threat_test_mapping/**`
- deps: `arc-19-enforcer-security, arc-14-enforcer-mechanization, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine, h01-money-critical-classifier`
- tier: `P1/P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
The spec (§0.5, §8.5) declares "unmapped logic is forbidden logic": every money-critical unit must map to at least one threat, one invariant, one property test, one concurrency test, and one replay test, and every declared threat must have at least one test or the threat model is incomplete. h01 now yields the money-critical manifest, but no `Validator` checks the mapping graph. Threats, invariants, and their tests exist only as prose intent; there is no typed THREAT_MAP record and no completeness gate.

## Where We Want To Be
A T1 mapping-completeness `Validator` in `enforcer-security` (arc-19) — a `crates/enforcer-security/src/rules/threat_test_mapping.rs` module implementing the `Validator` trait (from `enforcer-validator`, arc-05), emitting structured `Finding`s tagged with `ThreatId` (MITRE/OWASP) from `enforcer-domain`, with each `RuleId` a typed rule record in `enforcer-rules` (arc-04), scaffolded through d01 (arc-14) with full 5-way parity (fixtures `crates/enforcer-security/tests/fixtures/threat_test_mapping/**`). THREAT_MAP is a `serde` record (units × threats × invariants × tests) deserialized at the boundary with branded newtype keys (`ThreatId`/`RuleId`/`RelPath`, never bare `String`). The validator loads THREAT_MAP plus the h01 money-critical manifest and asserts: (a) every money-critical unit maps to ≥1 threat, ≥1 invariant, ≥1 property test, ≥1 concurrency test, ≥1 replay test; (b) no unit is unmapped ("unmapped logic is forbidden logic"); (c) every declared threat has ≥1 test — a threat with zero tests = incomplete model = `Finding`. GENERIC across any value system; the crypto/Anchor threat surface is one optional instance, never assumed.

## Requirement Checklist
Each rule is scaffolded via d01 `rule new`, landing a doc-anchor in its `enforcer-rules` record, a `Validator` impl in `src/rules/threat_test_mapping.rs`, and a fail+pass fixture pair under `crates/enforcer-security/tests/fixtures/threat_test_mapping/<rule>/{bad,good}/`.

- [ ] **T1 THREAT-MAP-UNIT-COVERAGE — per-unit mapping (§8.5).** Each money-critical unit MUST map to ≥1 threat + ≥1 invariant + ≥1 property test + ≥1 concurrency test + ≥1 replay test; any missing ⇒ `Finding`.
- [ ] **T1 THREAT-MAP-NO-UNMAPPED — unmapped logic forbidden (§0.5).** Any h01-classified unit absent from THREAT_MAP ⇒ `Finding`.
- [ ] **T1 THREAT-MAP-THREAT-HAS-TEST — declared threat completeness (§0.5).** Any threat declared in THREAT_MAP with zero associated tests ⇒ `Finding` (incomplete threat model).
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> rule-record <-> `Validator` <-> {fail,pass} fixtures <-> `cargo test` detection.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier P1/P4. 5-way parity per rule, Rust-native (`Validator` impl + fail/pass fixtures + a `cargo test` detection test). Fixtures are THREAT_MAP records (`.json`/RON) paired with an h01-shaped money-critical manifest stub; fail-fixtures break one mapping edge, pass-fixtures are fully mapped.

Representative triples:
- unmapped unit: fail `crates/enforcer-security/tests/fixtures/threat_test_mapping/unmapped/bad/unit_absent.json` (classified unit missing from map → flagged), pass `crates/enforcer-security/tests/fixtures/threat_test_mapping/mapped/good/full_mapping.json` (unit maps to all five edges → clean), threat-map-unit-coverage detection test.
- threat with no test: fail `crates/enforcer-security/tests/fixtures/threat_test_mapping/incomplete/bad/threat_zero_tests.json`, pass `.../good/threat_with_test.json`, threat-map-threat-has-test detection test.

Prove via `cargo test -p enforcer-security` (all threat_test_mapping fixtures) and the d01 `rule-scaffold-parity` oracle; record detection-test artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `crates/enforcer-security/src/rules/threat_test_mapping.rs` and `crates/enforcer-security/tests/fixtures/threat_test_mapping/**` — a specific rule module inside the arc-19 crate, NOT the crate itself. Depends on `arc-19` (crate skeleton + module-root + `Validator` registration), `arc-14`/`d01` (scaffold + parity), `arc-05` (the `Validator` trait), `arc-04` (the rule registry), and `h01` (money-critical manifest — consumed, not redefined). Sequenced after arc-19's skeleton exists. Distinct from h02: h02 asserts the seven required test CATEGORIES are present per unit; h03 asserts the threat↔invariant↔test MAPPING graph is complete. Property/concurrency/replay tests appear in both surfaces, but h03 owns only the mapping-completeness check over THREAT_MAP, never category-coverage.
