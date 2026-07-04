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

- owns: `rules/security/threat-test-mapping.md, src/validators/threat-test-invariant-map.ts, tests/fixtures/threat-map/**`
- deps: `d01, h01`
- tier: `P1/P4`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
The spec (§0.5, §8.5) declares "unmapped logic is forbidden logic": every money-critical unit must map to at least one threat, one invariant, one property test, one concurrency test, and one replay test, and every declared threat must have at least one test or the threat model is incomplete. h01 now yields the money-critical manifest, but no validator checks the mapping graph. Threats, invariants, and their tests exist only as prose intent; there is no manifest and no completeness gate.

## Where We Want To Be
A T1 validator over a THREAT_MAP manifest, scaffolded via d01 with full 5-way parity: doc `rules/security/threat-test-mapping.md`, validator `src/validators/threat-test-invariant-map.ts`, fixtures `tests/fixtures/threat-map/**`. The validator loads THREAT_MAP (units × threats × invariants × tests) plus the h01 money-critical manifest and asserts: (a) every money-critical unit maps to ≥1 threat, ≥1 invariant, ≥1 property test, ≥1 concurrency test, ≥1 replay test; (b) no unit is unmapped ("unmapped logic is forbidden logic"); (c) every declared threat has ≥1 test — a threat with zero tests = incomplete model = fail. GENERIC across any value system; the crypto/Anchor threat surface is one optional instance, never assumed.

## Requirement Checklist
Scaffolded with `enforcer rule new <ID>` (d01), landing doc anchor, validator, and fail+pass fixtures.

- [ ] **T1 THREAT-MAP-UNIT-COVERAGE — per-unit mapping (§8.5).** Each money-critical unit MUST map to ≥1 threat + ≥1 invariant + ≥1 property test + ≥1 concurrency test + ≥1 replay test; any missing ⇒ fail.
- [ ] **T1 THREAT-MAP-NO-UNMAPPED — unmapped logic forbidden (§0.5).** Any h01-classified unit absent from THREAT_MAP ⇒ fail.
- [ ] **T1 THREAT-MAP-THREAT-HAS-TEST — declared threat completeness (§0.5).** Any threat declared in THREAT_MAP with zero associated tests ⇒ fail (incomplete threat model).

## Acceptance And Proof
Tier P1/P4. Fixtures are THREAT_MAP manifests paired with an h01-shaped money-critical manifest stub; fail-fixtures break one mapping edge, pass-fixtures are fully mapped.

Representative triples:
- unmapped unit: fail `tests/fixtures/threat-map/unmapped/fail_unit_absent.json` (classified unit missing from map → flagged), pass `tests/fixtures/threat-map/mapped/pass_full_mapping.json` (unit maps to all five edges → clean), test `threat-map-unit-coverage.test`.
- threat with no test: fail `tests/fixtures/threat-map/incomplete/fail_threat_zero_tests.json`, pass `.../pass_threat_with_test.json`, test `threat-map-threat-has-test.test`.

Re-run the d01 `rule-scaffold-parity` oracle; record detection-test artifact paths in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns `rules/security/threat-test-mapping.md`, `src/validators/threat-test-invariant-map.ts`, and `tests/fixtures/threat-map/**` exclusively; disjoint from siblings. Depends on d01 (scaffold) and h01 (money-critical manifest — consumed, not redefined). Distinct from h02: h02 asserts the seven required test CATEGORIES are present per unit; h03 asserts the threat↔invariant↔test MAPPING graph is complete. Property/concurrency/replay tests appear in both surfaces, but h03 owns only the mapping-completeness check over THREAT_MAP, never category-coverage.
