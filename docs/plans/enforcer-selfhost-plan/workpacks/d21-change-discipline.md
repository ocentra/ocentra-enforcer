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

- owns: `crates/enforcer-lang-common/src/rules/change_discipline.rs, crates/enforcer-lang-common/tests/fixtures/change_discipline/**`
- deps: `arc-09-enforcer-lang-common, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine`
- tier: `P2 (mostly T3-labeled + T1-enforced labels; some T2)`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
ADBP enforces a family of change-hygiene obligations (ADBP_GAPS rows 86-90) that the `enforcer-rules` registry lacks entirely or backs only generically: TODO/FIXME/HACK markers are banned outright but the tracker-ref form (`(#1234)`) is not enforced; structural changes (3+ files / new dir) are never required to explain themselves or update `ARCHITECTURE.md`; deviations from established patterns are never recorded as an ADR in `decisions.md`; refactors are never checked for isolation from feature work; and new-dependency additions carry only a generic `dependency` keyword with no justification gate. No `Validator` in `enforcer-lang-common` covers the change family. Most of these are irreducibly judgment calls — whether a change is "structural", whether a pattern was "deviated from", whether a commit "mixes" refactor and feature — so they cannot be mechanized as blocking deterministic checks. What CAN be mechanized is the *presence and well-formedness of the required marker/artifact*, and the detection of a *new dependency being added*.

## Where We Want To Be
A `change_discipline` rule module in `enforcer-lang-common` (arc-09) — a `crates/enforcer-lang-common/src/rules/change_discipline.rs` implementing the `Validator` trait (from `enforcer-validator`, arc-05), emitting structured `Finding`s (from `enforcer-domain`), with each `RuleId` carried as a typed rule record in `enforcer-rules` (arc-04), scaffolded through the d01 mechanization engine. Split by tier honestly:
- T1 (deterministic, blocks): the LABEL/marker grammar — a `// TODO`/`FIXME`/`HACK` marker MUST carry a tracker ref `(#NNNN)`; a malformed or bare marker is a violation. This is the one deterministic, tested lane. Since this scans the target's comment text across languages, the validator matches markers over source lines (tree-sitter comment nodes where a target grammar is available, line-regex fallback otherwise).
- T2 (scored, non-blocking): `new-dependency-added` detection — a diff/manifest scan that flags a manifest gaining a dependency line, emitting `score`+`confidence` (on the Rust literal-scan scored model) so CI can nudge (not block) toward a recorded rationale.
- T3 (advisory, no mechanization possible + reason): `isolated-refactor` (refactor in dedicated commits, not mixed with feature work), `ADR-on-deviation` (deviation recorded as an ADR in `decisions.md`), `structural-change-explain` (3+ file / new-dir change documents WHY / updates `ARCHITECTURE.md`), `dependency-justification` (rationale for a new package). Each of these ships a T3 label `advisory, no mechanization possible + <reason>` on its `enforcer-rules` record, and the *presence of that label on the rule record* is itself T1-enforced by the d01 parity oracle.

## Requirement Checklist
- [ ] `change_discipline` rule records registered in `enforcer-rules`, one per `RuleId`, each carrying its tier and (for T3 rows) the verbatim `advisory, no mechanization possible + <reason>` label. (Optional human-canonical `.md` may live in the g08 rules explorer surface; the engine consumes the typed record.)
- [ ] T1 ruleId `CHG-TODO-TRACKER-REF` / `LITTODO-1.1`: a `Validator` in `change_discipline.rs` flags a bare `// TODO: fix later`; `// TODO(#1234): reason` stays clean.
- [ ] T2 ruleId `CHG-DEP-JUSTIFY` (new-dependency-added detection): a manifest gaining a dep line scores over threshold; a manifest with a documented rationale (or unchanged) stays under.
- [ ] T3 labeled records `CHG-REFACTOR-ISOLATED`, `CHG-ADR-DEVIATION` / `DOCGATE-1.2`, `CHG-STRUCTURAL-EXPLAIN` / `COMP-1.4`, `CHG-DEP-JUSTIFY` narrative — each carries the advisory label; d01 parity confirms the label is present on the record.
- [ ] All rows registered via d01 `rule new`; parity oracle green (ruleId <-> rule-record <-> validator <-> {fail,pass} fixtures <-> `cargo test` detection for T1/T2; ruleId <-> record-with-label for T3).
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
5-way parity per rule, Rust-native (`Validator` impl + fail/pass fixtures + a `cargo test` detection test).

- `CHG-TODO-TRACKER-REF` (T1): fail-fixture `crates/enforcer-lang-common/tests/fixtures/change_discipline/todo_no_ref/bad.rs` (`// TODO: clean this up`, must be flagged); pass-fixture `crates/enforcer-lang-common/tests/fixtures/change_discipline/todo_no_ref/good.rs` (`// TODO(#1234): drop after v2`, must stay clean); `#[test]` detection in `change_discipline.rs` (`#[cfg(test)]`) asserting fail flags exactly this and pass is clean.
- `CHG-DEP-JUSTIFY` new-dependency-added (T2): fail-fixture `crates/enforcer-lang-common/tests/fixtures/change_discipline/new_dep/bad.toml` (a manifest with an added dep, no recorded rationale — score crosses threshold); pass-fixture `.../new_dep/good.toml` (rationale present / no added dep — score under threshold); detection test asserts the score crosses on fail and stays under on pass (the Rust literal-scan scored model).
- T3 rows (`CHG-REFACTOR-ISOLATED`, `CHG-ADR-DEVIATION`, `CHG-STRUCTURAL-EXPLAIN`): labeled `advisory, no mechanization possible` with reasons — refactor/feature mixing, "structural" and "deviation" are semantic judgments over intent that no regex/AST decides. Proof is the d01 label-presence parity check on the `enforcer-rules` record, not a behavior fixture.

Prove via `cargo test -p enforcer-lang-common` (T1 + T2 fixtures) and the d01 `rule-scaffold-parity` oracle (all rows, including T3 label presence). Update the corresponding rows in TEST_PROOF_EXPECTATIONS.md before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint from siblings: it exclusively creates `crates/enforcer-lang-common/src/rules/change_discipline.rs` and `crates/enforcer-lang-common/tests/fixtures/change_discipline/**` — a specific rule module inside the arc-09 crate, NOT the crate itself. Depends on `arc-09` (which stands up the `enforcer-lang-common` crate skeleton + module root + `Validator` registration), `arc-05` (the `Validator` trait + parity harness), `arc-04` (the rule registry), and `d01` (scaffolder + parity oracle). Sequenced after arc-09's skeleton exists so the module root can register this validator. Does not touch d22's `size_shape.rs` or d23's `test_quality.rs` sibling modules in the same crate; the marker grammar here is unrelated to d03's deferred-work grammar (this is tracker-ref for TODO/FIXME/HACK, not stub deferral).
