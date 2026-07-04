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

- owns: `crates/enforcer-lang-common/src/rules/test_quality.rs, crates/enforcer-lang-common/tests/fixtures/test_quality/**`
- deps: `arc-09-enforcer-lang-common, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine, d16-fsm-transition-validity`
- tier: `P1 (T1 presence checks + T2 naming/quality heuristics)`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
ADBP mandates a test-quality family (ADBP_GAPS rows 95-100) that the `enforcer-rules` registry backs only partially. `TEST-2.1` gives a crate-level scaffold but there is no per-file / per-symbol test companion; no check for assertion-free tests; no "assert on error variant/type not message string"; no behavioral test-name discipline; no factories / injected-clock (no-wall-clock-assertion) rule; and no target-project coverage floor set as a *failing* threshold. No `Validator` in `enforcer-lang-common` covers the test-content family. Concretely: a new `services/foo.py` or a new `pub fn` in a watched layer can ship with no matching test companion; a zero-assertion `it("x", () => { svc.do() })` passes; tests assert on message strings instead of error variants; test names like `test_order_1` survive; time tests assert on wall-clock deltas. FSM transition coverage — every transition tested, valid AND invalid — is the row this pack shares with d16 (d16 owns FSM semantics in `fsm.rs`; d23 owns the *test-coverage-of-transitions* obligation).

## Where We Want To Be
A `test_quality` rule module in `enforcer-lang-common` (arc-09) — a `crates/enforcer-lang-common/src/rules/test_quality.rs` implementing the `Validator` trait (from `enforcer-validator`, arc-05), emitting structured `Finding`s, with each `RuleId` carried as a typed rule record in `enforcer-rules` (arc-04), scaffolded through d01. Test bodies are inspected via `syn` (Rust targets) / `tree-sitter` (TS/JS/Python targets); companion presence is a filesystem/module check. Split by tier:
- T1 (deterministic, blocks): `TEST-COMPANION-1.1` / `COMP-1.1` / `RUST-TEST-1.1` / `py.tests.companion-required` — every source file / new public symbol in a watched layer has a matching test companion (source + test present; for Rust, a `#[cfg(test)]` module or `tests/` companion); `CF-TEST-1.5` / `FE-TEST-1.4` — a test block with zero assertions is a violation.
- T2 (scored, non-blocking, Rust literal-scan model): `py-fastapi-behavioral-test-names` / `FE-TEST-1.1` behavioral naming (`test_<action>_<scenario>_<outcome>` / `should ... when ...`) and query-by-role-not-testid; `CF-TEST-1.4` / `TEST-VARIANT-1.1` assert-on-variant-not-message; `py-fastapi-test-data-factories` factories over inline dicts and `py-fastapi-no-wallclock-assert` injected clock over wall-clock delta; `py-fastapi-fsm-transition-coverage` / `TEST-FSM-1.1` FSM valid+invalid transition coverage (deps d16's `fsm.rs` transition model); `py-fastapi-coverage-fail-under` / `CIGATE-1.1/1.2` target-project coverage floor as a failing threshold (T1 where the presence of the `fail_under` gate is deterministic, T2 where the floor value is scored).

## Requirement Checklist
- [ ] `test_quality` rule records registered in `enforcer-rules`, one per `RuleId`, each carrying its tier. (Optional human-canonical `.md` may live in the g08 rules explorer surface.)
- [ ] T1 companion presence: a watched source file / new public symbol with no matching test companion is flagged; source + matching test present is clean.
- [ ] T1 assertion-free: a test block with no `expect`/`assert!` is flagged; a block with an assertion on visible output is clean.
- [ ] T2 behavioral names, assert-on-variant, factories, no-wall-clock, FSM transition coverage, coverage-floor — each emits `score`+`confidence` against a threshold (or, for the coverage-gate *presence*, a deterministic check).
- [ ] FSM transition-coverage row consumes d16's `fsm.rs` transition model; d23 owns only the test-coverage obligation, not the FSM semantics.
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> rule-record <-> validator <-> {fail,pass} fixtures <-> `cargo test` detection.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
5-way parity per rule, Rust-native (`Validator` impl + fail/pass fixtures + a `cargo test` detection test). Fixtures live under `crates/enforcer-lang-common/tests/fixtures/test_quality/`.

- `TEST-COMPANION-1.1` (T1): fail-fixture `test_quality/companion/bad/services/foo.py` (or `bad/orphan_mod.rs` with a `pub fn`) present with NO companion test — must be flagged; pass-fixture `test_quality/companion/good/services/foo.py` + `test_quality/companion/good/tests/test_foo.py` (source + matching companion) — must stay clean; `#[test]` detection in `test_quality.rs`.
- `CF-TEST-1.5` / `FE-TEST-1.4` assertion-free (T1): fail `test_quality/no_assert/bad.test.ts` (`it("x", () => { svc.do(); })`, no expect — flagged); pass `test_quality/no_assert/good.test.ts` (`expect(result).toEqual(...)` — clean).
- `py-fastapi-behavioral-test-names` / `FE-TEST-1.1` (T2): fail `test_quality/names/bad.test.ts` (`test_order_1`, `getByTestId(...)` — score crosses); pass `test_quality/names/good.test.ts` (`test_cancel_order_already_shipped_raises`, `getByRole(...)` — under).
- `CF-TEST-1.4` / `TEST-VARIANT-1.1` (T2): fail `test_quality/variant/bad.test.ts` (`.toThrow(message=...)` / `assert err.message == "..."` — crosses); pass `test_quality/variant/good.rs` (`matches!(e, Err(X))` / `.toThrow(type=...)` — under).
- `py-fastapi-test-data-factories` + `py-fastapi-no-wallclock-assert` (T2): fail `test_quality/factories/bad.test.py` (inline `{"email": ...}` dict; `assert monotonic() - start <= 0.6` — crosses); pass `test_quality/factories/good.test.py` (factory fixture; injected `FakeClock` asserting on the decision — under).
- `py-fastapi-fsm-transition-coverage` / `TEST-FSM-1.1` (T2, deps d16): fail `test_quality/fsm/bad` (FSM defined, no test hitting an invalid transition — crosses); pass `test_quality/fsm/good` (test asserts `InvalidTransitionError` on an illegal transition plus a valid path — under).
- `py-fastapi-coverage-fail-under` / `CIGATE-1.1/1.2` (T1 presence / T2 floor): fail `test_quality/coverage/bad` (`[tool.coverage]` with no `fail_under`; `vitest run` with no `--coverage`); pass `test_quality/coverage/good` (`fail_under=70`; `vitest run --coverage`).

Prove via `cargo test -p enforcer-lang-common` (all fixtures) and the d01 `rule-scaffold-parity` oracle. Update the corresponding rows in TEST_PROOF_EXPECTATIONS.md before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `crates/enforcer-lang-common/src/rules/test_quality.rs` and `crates/enforcer-lang-common/tests/fixtures/test_quality/**` — a specific rule module inside the arc-09 crate, NOT the crate itself. Depends on `arc-09` (crate skeleton + module root + `Validator` registration), `arc-05` (the `Validator` trait + parity harness), `arc-04` (the rule registry), `d01` (scaffolder + parity), and `d16` (the `fsm.rs` transition model, consumed only by the transition-coverage row — d23 does not define FSM semantics or edit d16's module). Sequenced after arc-09's skeleton exists. The `SIZE-TESTFILE-1.1` test-file length cap belongs to d22's `size_shape.rs`, not here; d23 governs test *content* quality (companion presence, assertions, naming, coverage floor). Does not touch d21's `change_discipline.rs` sibling module. Can start once arc-09, d01 and d16 land.
