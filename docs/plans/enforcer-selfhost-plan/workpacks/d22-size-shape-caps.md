# d22 Size Shape Caps

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Size Shape Caps`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-common/src/rules/size_shape.rs, crates/enforcer-lang-common/tests/fixtures/size_shape/**`
- deps: `arc-09-enforcer-lang-common, arc-05-enforcer-validator, arc-04-enforcer-rules, d01-rule-mechanization-engine, d02-baseline-grandfather-ratchet`
- tier: `P1 (T1 hard caps + T2 scored complexity)`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Every ADBP agent stack and the CLAUDE.TEMPLATE mandate a family of hard size caps (ADBP_GAPS rows 91-94), but the `enforcer-rules` registry has only `SRC-2.1`, a generic file-line budget. There is NO length family for: files (≤200 lines; Rust ≤400), functions (≤30 lines), classes (≤150 lines, ≤10-12 public methods), params (≤5), line-length (≤120, INCLUDING trailing pragmas/comments so a trailing suppression tail cannot smuggle a 200-char line past the cap), and test files (≤300 lines). Cyclomatic complexity (<10) and nesting depth (≤3) are mentioned only generically, with no first-class scored family. Package/path nesting depth (≤3 levels) is uncovered. And there is no grandfather-ratchet baseline mode so an existing oversized file can warn-at-baseline / fail-if-grown rather than block the whole repo on day one. No `Validator` in `enforcer-lang-common` covers the size family. These extend and complement the existing `SRC-*` shape rules — they do not replace them.

## Where We Want To Be
A `size_shape` rule module in `enforcer-lang-common` (arc-09) — a `crates/enforcer-lang-common/src/rules/size_shape.rs` implementing the `Validator` trait (from `enforcer-validator`, arc-05), emitting structured `Finding`s, with each `RuleId` carried as a typed rule record in `enforcer-rules` (arc-04), scaffolded through d01 — giving the deterministic length family plus a scored complexity family, composed with the d02 baseline-grandfather-ratchet. Length metrics count physical lines/columns; structural counts (functions, classes, params, complexity, nesting) come from the AST (`syn` for Rust targets, `tree-sitter` for TS/JS/Python/Dart/etc. targets):
- T1 (deterministic, blocks): `SIZE-FILE-1.1` (file ≤200, per-language override Rust ≤400), `SIZE-FUNC-1.1` (function ≤30), `SIZE-CLASS-1.1` (class ≤150 / ≤10-12 public methods), `SIZE-PARAMS-1.1` (≤5 params), `SIZE-LINE-1.1` (line ≤120, measured over the raw line including any trailing pragma), `SIZE-TESTFILE-1.1` (test file ≤300).
- T2 (scored, non-blocking): `SIZE-CX-1.1` cyclomatic complexity <10 / nesting ≤3 as a first-class scored family (AST-derived score+confidence); `py-fastapi-package-nesting-depth` path nesting ≤3 levels.
- Baseline mode: `FE-LEN-2.1` grandfather ratchet — a baselined oversized file warns at/below its recorded baseline and fails only if it grows; this composes with the existing d02 ratchet in `enforcer-scan` (d22 supplies the per-file length metric, d02 owns the `baseline_ratchet.rs` baseline store + ratchet mechanics).

## Requirement Checklist
- [ ] `size_shape` rule records registered in `enforcer-rules`, one per `RuleId`, each carrying its tier; per-language cap overrides (Rust file ≤400) carried on the typed record. (Optional human-canonical `.md` may live in the g08 rules explorer surface.)
- [ ] T1 hard caps ship deterministic `Validator` impls in `size_shape.rs`: file, function, class (+ public-method count), params, line-length (trailing-pragma-inclusive), test-file.
- [ ] T2 scored: `SIZE-CX-1.1` complexity/nesting and `py-fastapi-package-nesting-depth` emit `score`+`confidence` against thresholds.
- [ ] Grandfather ratchet `FE-LEN-2.1`: a baselined file at baseline count is clean; the same file grown +1 line fails. Composes with d02 (uses d02's `baseline_ratchet.rs` store; does not duplicate it).
- [ ] Line-length counts trailing pragmas/comments (a trailing suppression tail does not exempt the line from the cap).
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> rule-record <-> validator <-> {fail,pass} fixtures <-> `cargo test` detection.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
5-way parity per rule, Rust-native (`Validator` impl + fail/pass fixtures + a `cargo test` detection test). Fixtures live under `crates/enforcer-lang-common/tests/fixtures/size_shape/`.

- `SIZE-FILE-1.1` (T1): fail `size_shape/file/bad.ts` (201 lines, flagged); pass `size_shape/file/good.ts` (200 lines, clean); plus a Rust override pair `size_shape/file_rust/bad.rs` (401 lines) / `size_shape/file_rust/good.rs` (400 lines) asserting the ≤400 language cap.
- `SIZE-FUNC-1.1` (T1): fail `size_shape/func/bad.ts` (31-line fn); pass `size_shape/func/good.ts` (30-line fn).
- `SIZE-CLASS-1.1` (T1): fail `size_shape/class/bad.ts` (151-line class) and `size_shape/class_methods/bad.ts` (13 public methods); pass `size_shape/class/good.ts` (150 lines).
- `SIZE-PARAMS-1.1` (T1): fail `size_shape/params/bad.ts` (6 params); pass `size_shape/params/good.ts` (5 params).
- `SIZE-LINE-1.1` (T1): fail `size_shape/line/bad.ts` (130 chars) and `size_shape/line_pragma/bad.ts` (a >120 line whose overflow is a trailing pragma — must still flag); pass `size_shape/line/good.ts` (120 chars).
- `SIZE-TESTFILE-1.1` (T1): fail `size_shape/testfile/bad.test.ts` (301 lines); pass `size_shape/testfile/good.test.ts` (300 lines).
- `SIZE-CX-1.1` (T2): fail `size_shape/cx/bad.ts` (cc=12, 5-deep nesting — score crosses); pass `size_shape/cx/good.ts` (cc=6, guard clauses — score under). Rust literal-scan scored model: fixtures assert the threshold, not a block.
- `py-fastapi-package-nesting-depth` (T2): fail `size_shape/nesting/bad/app/core/services/domain/user/service.py` (>3 levels — score crosses); pass `size_shape/nesting/good/app/user_service.py` (≤3 — under).
- `FE-LEN-2.1` grandfather ratchet (T2): fail `size_shape/ratchet/bad` (baseline recorded via d02, file grown +1 line); pass `size_shape/ratchet/good` (file at recorded baseline count).

Prove via `cargo test -p enforcer-lang-common` (all fixtures) and the d01 `rule-scaffold-parity` oracle. Update the corresponding rows in TEST_PROOF_EXPECTATIONS.md before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `crates/enforcer-lang-common/src/rules/size_shape.rs` and `crates/enforcer-lang-common/tests/fixtures/size_shape/**` — a specific rule module inside the arc-09 crate, NOT the crate itself. Depends on `arc-09` (crate skeleton + module root + `Validator` registration), `arc-05` (the `Validator` trait + parity harness), `arc-04` (the rule registry), `d01` (scaffolder + parity), and `d02` (the `enforcer-scan` `baseline_ratchet.rs` store, which the `FE-LEN-2.1` row composes with rather than reimplements). Sequenced after arc-09's skeleton exists. Extends/complements the existing `SRC-*` shape rules but must not edit them — the length family is additive. Does not touch d21's `change_discipline.rs` or d23's `test_quality.rs` sibling modules in the same crate. The `SIZE-TESTFILE-1.1` cap here is a length metric only; test *content* quality (companion presence, assertion-free tests, naming) is d23's scope.
