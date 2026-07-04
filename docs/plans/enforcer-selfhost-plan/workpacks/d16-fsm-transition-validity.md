# d16 FSM Transition Validity

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `FSM Transition Validity`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `rules/common/fsm.md, src/validators/fsm-*.ts, tests/fixtures/fsm/**`
- deps: `d01`
- tier: `P0/P1 mixed`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
The registry has no FSM rule family. Stateful entities across every stack (Python/ColdFusion/Flutter/React/Rust) mutate `status`/`role`/`type` fields with raw `entity.status = x` writes, compare against bare string literals, and parse enums with silent `orElse`/`?? default` fallbacks. A generic `magic-string-comparison` rule exists but there is no "fixed-set literal must be an enum member" gate, no transition-map requirement, and no FSM-transition-coverage test gate. This is the marquee ADBP miss (rows 41-50 of ADBP_GAPS.md), enforced by ADBP across five source files and collapsed here to one cross-stack family.

## Where We Want To Be
An `enforcer`-shipped FSM rule family (docs in `rules/common/fsm.md`, validators `src/validators/fsm-*.ts`, fixtures `tests/fixtures/fsm/**`), all scaffolded via the d01 mechanization engine so every ruleId carries the 5-way parity (ruleId <-> doc <-> validator <-> fail+pass fixture <-> detection test). Structural presence rules block (T1); design-intent rules are scored (T2); genuinely unmechanizable intent is labeled T3.

## Requirement Checklist
Each rule below is scaffolded with `enforcer rule new <ID>` (d01) and lands a doc anchor in `rules/common/fsm.md`, an `fsm-*.ts` validator, and a fail+pass fixture pair under `tests/fixtures/fsm/`.

- [ ] **T1 FSM-1.1 / py-fastapi-fsm-required-for-status-mutation / CF-FSM-2.1 / FE-FSM-1.2 — mandatory FSM for stateful entity.** A `status`/`role`/`type` field mutation must route through a transition call, not a raw assignment.
- [ ] **T1 CF-FSM-2.3 / FE-FSM-1.2 — explicit transitions map.** Allowed transitions declared as a states->transitions map (`as const` / `transitions()`); no ad-hoc `setStatus(string)`.
- [ ] **T1 py-fastapi-canonical-layout / py-fastapi-fsm-location — FSM canonical layout.** Transition map lives in `state_machines/`, enums in `enums/`; a transition map in `models/` is a violation.
- [ ] **T1/T2 py-fastapi-status-string-literal-forbidden / DART-TYPE-2.1 / ENUM-STATE-1.2 / CF-FSM-1.1 — fixed-set literal must be enum.** `if status == "pending":` on a status/role/type field must compare an enum member. (T1 where the field is a known status/role/type symbol; T2 where the field binding is inferred.)
- [ ] **T1 py.enum.strenum-only / py-fastapi-enum-location — enums in `enums/`, StrEnum base.** Every class in `enums/` inherits `StrEnum`/typed enum base; `class Status(Enum)` outside `enums/` or non-StrEnum inside is a violation.
- [ ] **T1 DART-TYPE-1.7 — enum parse no silent fallback.** No `firstWhere(..., orElse: () => X.item)` / `?? default` variant fallback on enum parse; throw or return nullable.
- [ ] **T2 CF-FSM-2.2 — validate-before-mutate / raise-on-invalid.** `canTransition` returning bool while mutation happens regardless is a violation; illegal transition must `assertTransition`-raise `InvalidTransition`.
- [ ] **T2 CF-FSM-2.4 — terminal-state no outgoing.** Terminal states (CLOSED/CANCELLED) map to `[]`; giving a terminal state an outgoing edge is a violation.
- [ ] **T2 CF-FSM-2.5 — FSM singleton + stateless.** Pure from/to -> decision; writing per-request instance/`variables` state inside an FSM method is a violation.
- [ ] **T2 py-fastapi-fsm-transition-coverage / FE-TEST-1.6 / TEST-FSM-1.1 / DART-TEST-3.1 — transition-coverage test.** An FSM with no test hitting an invalid transition scores over threshold; a test asserting `InvalidTransitionError`/`InvalidTransition` on an illegal edge stays clean.

## Acceptance And Proof
Tier P0 for the T1 blocking rules (mandatory-FSM, explicit map, canonical layout, enum-location/StrEnum, enum-parse-no-fallback, fixed-set-literal where the field is a known symbol); tier P1 for the T2 scored rules (validate-before-mutate, terminal-no-outgoing, singleton-stateless, transition-coverage). For every ruleId the fail-fixture must be flagged and the pass-fixture must stay clean under its `fsm-*.ts` validator; the detection test asserts both directions. T2 fixtures assert the score crosses the fail threshold and stays under it on pass (Rust literal-scan scored model). No rule in this pack is T3 — every point is at least scored. Re-run the d01 `rule-scaffold-parity` oracle so all new ruleIds show 5-way parity, and record the detection-test artifact paths in TEST_PROOF_EXPECTATIONS.md.

Representative fail/pass/test triples:
- FSM-1.1: fail `tests/fixtures/fsm/mandatory/fail_raw_status_assign.py` (`order.status = "shipped"`), pass `.../pass_transition_call.py` (`fsm.assert_transition(order.status, Target)`), test `fsm-required.test`.
- DART-TYPE-1.7: fail `tests/fixtures/fsm/enum-parse/fail_orelse_default.dart`, pass `.../pass_try_from_nullable.dart`, test `fsm-enum-parse.test`.
- py-fastapi-fsm-transition-coverage (T2): fail `tests/fixtures/fsm/coverage/fail_no_invalid_test/` (FSM defined, no invalid-transition test — score over threshold), pass `.../pass_invalid_covered/` (test asserts raise — score under), test `fsm-coverage.test`.

## Parallel Ownership Notes
Owns `rules/common/fsm.md`, `src/validators/fsm-*.ts`, and `tests/fixtures/fsm/**` exclusively; disjoint from all siblings. Depends on d01 for scaffolding and the parity oracle. The CFML `CF-FSM-*` and Dart FSM semantics referenced by e-pack-cfml / e-pack-dart reuse this family's transition-validity core — those packs register the language appliesTo, this pack owns the shared FSM mechanism and must not edit language-registration files.
