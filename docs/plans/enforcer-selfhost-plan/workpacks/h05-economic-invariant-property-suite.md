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

- owns: `rules/security/economic-invariants.md, src/validators/economic-invariant-suite.ts, tests/fixtures/economic-invariants/**`
- deps: `d01, h01`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
Spec §2.3 enumerates ten economic/logic invariants (map to G1–G6) that a money-critical unit MUST guarantee, and §4/§8.3 demand they be property-based (fast-check) suites that fail CI when absent. Today nothing checks that a money-critical unit (identified by h01's money-critical classifier) carries a property test for each invariant, nor that the property asserts the correct shape (a refutation over generated inputs, not a single hand-picked case). A settlement module can ship with zero property coverage of "failure ≠ reward" and pass.

## Where We Want To Be
A `rules/security/economic-invariants.md` doc plus `economic-invariant-suite.ts`, scaffolded through d01, that for each money-critical unit checks presence and assertion-shape of a fast-check property per invariant:
- `same-request-twice != more-value` · `failure != reward` · `retry != mutation` · `partial-failure != profit` · `order != advantage` · `attacker-cost >= system-cost` · `compensation idempotent+replay-safe` · `time-assumptions fail-closed` · `backend-never-signs-unverifiable` · `emergency-reduces-blast`.
- T1 (blocks): for a unit classified money-critical by h01, the corresponding invariant property must be present AND have property shape (`fc.assert(fc.property(...))` refuting the bad outcome over generated inputs), not a single literal case. Missing property or non-property shape flags.

## Requirement Checklist
- [ ] `rules/security/economic-invariants.md` created with one anchored section per invariant ruleId, each carrying its tier.
- [ ] T1 presence: a money-critical unit missing any required invariant property is flagged; a unit with all ten is clean.
- [ ] T1 assertion-shape: an invariant "test" that is a single literal case (not `fc.property` over generated inputs) is flagged.
- [ ] Consumes h01's money-critical classification to scope which units require the suite; does not redefine classification.
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> doc <-> validator <-> {fail,pass} fixtures <-> detection-test.

## Acceptance And Proof
5-way parity per rule. Fixtures live under `tests/fixtures/economic-invariants/`.

- `failure != reward` (T1): fail `missing-failure-not-reward.fail/` (money-critical unit + property suite covering nine invariants but NO `failure != reward` property — flagged); pass `full-suite.pass/` (all ten properties present with fast-check shape — clean).
- `same-request-twice != more-value` (T1): fail `idempotency-single-case.fail.test.ts` (one hand-picked replay assertion, not `fc.property` — flagged as non-property shape); pass `idempotency-property.pass.test.ts` (`fc.assert(fc.property(...))` refuting extra value on replay).
- `attacker-cost >= system-cost` (T1): fail `cost-no-property.fail/` (invariant undocumented/absent — flagged); pass `cost-property.pass/` (property over generated attacker inputs).
- `compensation idempotent+replay-safe` (T1): fail `compensation-missing.fail/`; pass `compensation-property.pass/`.
- non-money-critical scope: pass `plain-util.pass/` (unit not classified money-critical by h01 — no suite required, clean).

Prove via a detection test and the d01 `rule-scaffold-parity` oracle. Update TEST_PROOF_EXPECTATIONS.md rows before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `rules/security/economic-invariants.md`, `src/validators/economic-invariant-suite.ts`, and `tests/fixtures/economic-invariants/**`. Depends on `d01` (scaffolder + parity) and `h01` (money-critical classifier, consumed read-only to scope which units require the suite — h05 does not redefine what counts as money-critical). Sibling h04 (test-shape bans) and h06 (mechanics) are not touched here; h05 owns only the invariant-property presence/shape obligation. Can start once d01 and h01 land.
