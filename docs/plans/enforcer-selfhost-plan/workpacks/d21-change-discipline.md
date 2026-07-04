# d21 Change Discipline

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Change Discipline`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `rules/common/change-discipline.md, src/change-discipline.ts, tests/change-discipline.test.mjs, tests/fixtures/change-discipline/**`
- deps: `d01`
- tier: `P2 (mostly T3-labeled + T1-enforced labels; some T2)`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
ADBP enforces a family of change-hygiene obligations (ADBP_GAPS rows 86-90) that the registry lacks entirely or backs only generically: TODO/FIXME/HACK markers are banned outright but the tracker-ref form (`(#1234)`) is not enforced; structural changes (3+ files / new dir) are never required to explain themselves or update `ARCHITECTURE.md`; deviations from established patterns are never recorded as an ADR in `decisions.md`; refactors are never checked for isolation from feature work; and new-dependency additions carry only a generic `dependency` keyword with no justification gate. Most of these are irreducibly judgment calls — whether a change is "structural", whether a pattern was "deviated from", whether a commit "mixes" refactor and feature — so they cannot be mechanized as blocking deterministic checks. What CAN be mechanized is the *presence and well-formedness of the required marker/artifact*, and the detection of a *new dependency being added*.

## Where We Want To Be
A `rules/common/change-discipline.md` doc plus validators scaffolded through the d01 engine, split by tier honestly:
- T1 (deterministic, blocks): the LABEL/marker grammar — a `// TODO`/`FIXME`/`HACK` marker MUST carry a tracker ref `(#NNNN)`; a malformed or bare marker is a violation. This is the one deterministic, tested lane.
- T2 (scored, non-blocking): `new-dependency-added` detection — a diff/manifest scan that flags a manifest gaining a dependency line, emitting `score`+`confidence` so CI can nudge (not block) toward a recorded rationale.
- T3 (advisory, no mechanization possible + reason): `isolated-refactor` (refactor in dedicated commits, not mixed with feature work), `ADR-on-deviation` (deviation recorded as an ADR in `decisions.md`), `structural-change-explain` (3+ file / new-dir change documents WHY / updates `ARCHITECTURE.md`), `dependency-justification` (rationale for a new package). Each of these ships a T3 label `advisory, no mechanization possible + <reason>`, and the *presence of that label in the rule doc* is itself T1-enforced by the d01 parity oracle.

## Requirement Checklist
- [ ] `rules/common/change-discipline.md` created with one anchored section per ruleId, each carrying its tier and (for T3 rows) the verbatim `advisory, no mechanization possible + <reason>` label.
- [ ] T1 ruleId `CHG-TODO-TRACKER-REF` / `LITTODO-1.1`: bare `// TODO: fix later` flagged; `// TODO(#1234): reason` clean.
- [ ] T2 ruleId `CHG-DEP-JUSTIFY` (new-dependency-added detection): manifest gaining a dep line scores over threshold; manifest with a documented rationale (or unchanged) stays under.
- [ ] T3 labeled rows `CHG-REFACTOR-ISOLATED`, `CHG-ADR-DEVIATION` / `DOCGATE-1.2`, `CHG-STRUCTURAL-EXPLAIN` / `COMP-1.4`, `CHG-DEP-JUSTIFY` narrative — each carries the advisory label; d01 parity confirms the label is present.
- [ ] All rows registered via d01 `rule new`; parity oracle green (ruleId <-> doc <-> validator <-> fixtures <-> detection-test for T1/T2; ruleId <-> doc-with-label for T3).

## Acceptance And Proof
5-way parity per rule.

- `CHG-TODO-TRACKER-REF` (T1): fail-fixture `tests/fixtures/change-discipline/todo-no-ref.fail.ts` (`// TODO: clean this up`, must be flagged); pass-fixture `tests/fixtures/change-discipline/todo-with-ref.pass.ts` (`// TODO(#1234): drop after v2`, must stay clean); detection test in `tests/change-discipline.test.mjs` asserting fail flags exactly this and pass is clean.
- `CHG-DEP-JUSTIFY` new-dependency-added (T2): fail-fixture `tests/fixtures/change-discipline/new-dep-added.fail.json` (manifest with an added dep, no recorded rationale — score crosses threshold); pass-fixture `tests/fixtures/change-discipline/new-dep-justified.pass.json` (rationale present / no added dep — score under threshold); detection test asserts the score crosses on fail and stays under on pass (the Rust literal-scan scored model).
- T3 rows (`CHG-REFACTOR-ISOLATED`, `CHG-ADR-DEVIATION`, `CHG-STRUCTURAL-EXPLAIN`): labeled `advisory, no mechanization possible` with reasons — refactor/feature mixing, "structural" and "deviation" are semantic judgments over intent that no regex/AST decides. Proof is the d01 label-presence parity check, not a behavior fixture.

Prove via `tests/change-discipline.test.mjs` (T1 + T2 fixtures) and the d01 `rule-scaffold-parity` oracle (all rows, including T3 label presence). Update the corresponding rows in TEST_PROOF_EXPECTATIONS.md before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint from siblings: it exclusively creates `rules/common/change-discipline.md`, `src/change-discipline.ts`, and `tests/fixtures/change-discipline/**`. Depends only on `d01` for the scaffolder + parity oracle. Does not touch d22's size/shape families or d23's test-quality families; the marker grammar here is unrelated to d24's `DEFERRED(...)` deferred-work grammar (this is tracker-ref for TODO/FIXME/HACK, not stub deferral). Can start as soon as d01 lands.
