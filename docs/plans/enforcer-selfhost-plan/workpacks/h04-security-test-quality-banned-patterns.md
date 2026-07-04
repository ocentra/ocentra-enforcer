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

- owns: `rules/security/security-test-quality.md, src/validators/security-test-banned-patterns.ts, tests/fixtures/security-test-quality/**`
- deps: `d01, d23`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [security-testing source](../refs/security-testing-source.md).

## Where We Are
The source spec §7.2/§8.4.1 bans ten security-test anti-patterns and §7.1/§8.4.2 requires six positive properties, but this is PROSE — a test that only asserts success, mocks the money logic it claims to protect, or passes even after the protection is deleted currently ships silently. d23 governs generic test quality (companion presence, assertion-free, naming) but has no notion of a *security* test's threat mapping, invariant assertion, or "fails-if-protection-removed" obligation. There is no validator that reads a test file classified as security/money-critical and rejects the banned shapes.

## Where We Want To Be
A `rules/security/security-test-quality.md` doc plus `security-test-banned-patterns.ts`, scaffolded through d01, applying AST + pattern analysis over tests classified as security/money-critical (composing d23's classifier and d03's deferred-work gate):
- T1 (blocks) BANNED: `asserts-success-only`, `pass-if-logic-deleted`, `rely-on-no-crash`, `snapshot-only`, `mocks-for-money-logic`, `non-deterministic-fuzz` (no logged seed), `order-dependent`, `global-mutation`.
- T1 (blocks) REQUIRED-presence: `asserts-rejection` (test asserts an operation is refused), `reproducible-seed` (seed logged for fuzz/property).
- T2 (scored/advisory): `no-threat-mapping`, `no-invariant-assertion`, `exact-failure-mode` (asserts the specific rejection mode, not any throw), `fails-if-protection-removed` (mutation-style presence heuristic) — each emits `score`+`confidence`.

## Requirement Checklist
- [ ] `rules/security/security-test-quality.md` created with one anchored section per ruleId, each carrying its tier.
- [ ] T1 banned-pattern detection over security-classified tests: success-only, pass-if-deleted, no-crash, snapshot-only, mocks-for-money-logic, non-deterministic-fuzz, order-dependent, global-mutation each flag.
- [ ] T1 required-presence: missing rejection assertion or missing logged seed (fuzz/property) flags.
- [ ] T2 scored: threat-mapping, invariant assertion, exact-failure-mode, fails-if-protection-removed emit score+confidence against a threshold.
- [ ] Composes d23 (test classification) and d03 (deferred/waiver gate) without editing their files.
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> doc <-> validator <-> {fail,pass} fixtures <-> detection-test.

## Acceptance And Proof
5-way parity per rule. Fixtures live under `tests/fixtures/security-test-quality/`.

- `mocks-for-money-logic` (T1): fail `mock-money.fail.test.ts` (mocks the balance/settlement unit under test — flagged); pass `real-money.pass.test.ts` (exercises real money logic, asserts rejection with reproducible seed — clean).
- `asserts-success-only` / `asserts-rejection` (T1): fail `success-only.fail.test.ts` (only `expect(res.ok)` — flagged); pass `rejects.pass.test.ts` (asserts the operation is refused).
- `non-deterministic-fuzz` / `reproducible-seed` (T1): fail `no-seed.fail.test.ts` (fuzz with unlogged random — flagged); pass `seeded.pass.test.ts` (logged seed).
- `order-dependent` / `global-mutation` (T1): fail `shared-state.fail.test.ts` (mutates module global across cases — flagged); pass `isolated.pass.test.ts`.
- `no-threat-mapping` / `no-invariant-assertion` / `exact-failure-mode` (T2): fail `unmapped.fail.test.ts` (no threat tag, generic throw — score crosses); pass `mapped.pass.test.ts` (threat annotation + invariant + exact failure mode — under).

Prove via a detection test and the d01 `rule-scaffold-parity` oracle. Update TEST_PROOF_EXPECTATIONS.md rows before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `rules/security/security-test-quality.md`, `src/validators/security-test-banned-patterns.ts`, and `tests/fixtures/security-test-quality/**`. Depends on `d01` (scaffolder + parity) and `d23` (test classification/companion model, consumed read-only — d23 owns generic quality, h04 owns security-test shape). Composes d03's waiver gate; does not edit d03. Sibling h05/h06 own invariant suites and money-critical mechanics respectively and are not touched here. Can start once d01 and d23 land.
