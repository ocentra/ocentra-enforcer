# d23 Test Companion And Quality

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Test Companion And Quality`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `rules/common/test-quality.md, src/test-quality.ts, tests/test-quality.test.mjs, tests/fixtures/test-quality/**`
- deps: `d01, d16`
- tier: `P1 (T1 presence checks + T2 naming/quality heuristics)`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
ADBP mandates a test-quality family (ADBP_GAPS rows 95-100) that the registry backs only partially. `TEST-2.1` gives a crate-level scaffold but there is no per-file / per-symbol test companion; no check for assertion-free tests; no "assert on error variant/type not message string"; no behavioral test-name discipline; no factories / injected-clock (no-wall-clock-assertion) rule; and no target-project coverage floor set as a *failing* threshold. Concretely: a new `services/foo.py` or a new `pub fn` in a watched layer can ship with no matching test companion; an `it("x", () => { svc.do() })` with no `expect` passes; tests assert on message strings instead of error variants; test names like `test_order_1` survive; time tests assert on wall-clock deltas. FSM transition coverage — every transition tested, valid AND invalid — is the row this pack shares with d16 (d16 owns FSM semantics; d23 owns the *test-coverage-of-transitions* obligation).

## Where We Want To Be
A `rules/common/test-quality.md` doc plus validators scaffolded through d01, split by tier:
- T1 (deterministic, blocks): `TEST-COMPANION-1.1` / `COMP-1.1` / `RUST-TEST-1.1` / `py.tests.companion-required` — every source file / new public symbol in a watched layer has a matching test companion (source + test file both present); `CF-TEST-1.5` / `FE-TEST-1.4` — an `it`/`test` block with zero assertions is a violation.
- T2 (scored, non-blocking, Rust literal-scan model): `py-fastapi-behavioral-test-names` / `FE-TEST-1.1` behavioral naming (`test_<action>_<scenario>_<outcome>` / `should ... when ...`) and query-by-role-not-testid; `CF-TEST-1.4` / `TEST-VARIANT-1.1` assert-on-variant-not-message; `py-fastapi-test-data-factories` factories over inline dicts and `py-fastapi-no-wallclock-assert` injected clock over wall-clock delta; `py-fastapi-fsm-transition-coverage` / `TEST-FSM-1.1` FSM valid+invalid transition coverage (deps d16); `py-fastapi-coverage-fail-under` / `CIGATE-1.1/1.2` target-project coverage floor as a failing threshold (T1 where the presence of the `fail_under` gate is deterministic, T2 where the floor value is scored).

## Requirement Checklist
- [ ] `rules/common/test-quality.md` created with one anchored section per ruleId, each carrying its tier.
- [ ] T1 companion presence: a watched source file / new public symbol with no matching test companion is flagged; source + matching test present is clean.
- [ ] T1 assertion-free: an `it`/`test` with no `expect`/assert is flagged; a block with an assertion on visible output is clean.
- [ ] T2 behavioral names, assert-on-variant, factories, no-wall-clock, FSM transition coverage, coverage-floor — each emits `score`+`confidence` against a threshold (or, for the coverage-gate *presence*, a deterministic check).
- [ ] FSM transition-coverage row consumes d16's transition model; d23 owns only the test-coverage obligation, not the FSM semantics.
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> doc <-> validator <-> {fail,pass} fixtures <-> detection-test.

## Acceptance And Proof
5-way parity per rule. Fixtures live under `tests/fixtures/test-quality/`.

- `TEST-COMPANION-1.1` (T1): fail-fixture `orphan-source/services/foo.py` (or `orphan_mod.rs` with a `pub fn`) present with NO companion test — must be flagged; pass-fixture `paired/services/foo.py` + `paired/tests/test_foo.py` (source + matching companion) — must stay clean; detection test in `tests/test-quality.test.mjs`.
- `CF-TEST-1.5` / `FE-TEST-1.4` assertion-free (T1): fail `no-assert.fail.test.ts` (`it("x", () => { svc.do(); })`, no expect — flagged); pass `has-assert.pass.test.ts` (`expect(result).toEqual(...)` — clean).
- `py-fastapi-behavioral-test-names` / `FE-TEST-1.1` (T2): fail `bad-names.fail.test.ts` (`test_order_1`, `getByTestId(...)` — score crosses); pass `good-names.pass.test.ts` (`test_cancel_order_already_shipped_raises`, `getByRole(...)` — under).
- `CF-TEST-1.4` / `TEST-VARIANT-1.1` (T2): fail `assert-on-message.fail.test.ts` (`.toThrow(message=...)` / `assert err.message == "..."` — crosses); pass `assert-on-variant.pass.test.ts` (`.toThrow(type=...)` / `matches!(e, Err(X))` — under).
- `py-fastapi-test-data-factories` + `py-fastapi-no-wallclock-assert` (T2): fail `inline-and-wallclock.fail.test.py` (inline `{"email": ...}` dict; `assert monotonic() - start <= 0.6` — crosses); pass `factory-and-fakeclock.pass.test.py` (factory fixture; injected `FakeClock` asserting on the decision — under).
- `py-fastapi-fsm-transition-coverage` / `TEST-FSM-1.1` (T2, deps d16): fail `fsm-no-invalid-test.fail` (FSM defined, no test hitting an invalid transition — crosses); pass `fsm-full-coverage.pass` (test asserts `InvalidTransitionError` on an illegal transition plus a valid path — under).
- `py-fastapi-coverage-fail-under` / `CIGATE-1.1/1.2` (T1 presence / T2 floor): fail `coverage-no-floor.fail` (`[tool.coverage]` with no `fail_under`; `vitest run` with no `--coverage`); pass `coverage-floor-set.pass` (`fail_under=70`; `vitest run --coverage`).

Prove via `tests/test-quality.test.mjs` (all fixtures) and the d01 `rule-scaffold-parity` oracle. Update the corresponding rows in TEST_PROOF_EXPECTATIONS.md before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `rules/common/test-quality.md`, `src/test-quality.ts`, and `tests/fixtures/test-quality/**`. Depends on `d01` (scaffolder + parity) and `d16` (FSM transition model, consumed only by the transition-coverage row — d23 does not define FSM semantics or edit d16's files). The `SIZE-TESTFILE-1.1` test-file length cap belongs to d22, not here; d23 governs test *content* quality (companion presence, assertions, naming, coverage floor). Does not touch d21's change-discipline family. Can start once d01 and d16 land.
