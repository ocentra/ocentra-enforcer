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

- owns: `rules/common/size-shape.md, src/size-shape.ts, tests/size-shape.test.mjs, tests/fixtures/size-shape/**`
- deps: `d01, d02`
- tier: `P1 (T1 hard caps + T2 scored complexity)`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [ADBP_GAPS](../ADBP_GAPS.md).

## Where We Are
Every ADBP agent stack and the CLAUDE.TEMPLATE mandate a family of hard size caps (ADBP_GAPS rows 91-94), but the registry has only `SRC-2.1`, a generic file-line budget. There is NO length family for: files (≤200 lines; Rust ≤400), functions (≤30 lines), classes (≤150 lines, ≤10-12 public methods), params (≤5), line-length (≤120, INCLUDING trailing pragmas/comments so a `// eslint-disable` tail cannot smuggle a 200-char line past the cap), and test files (≤300 lines). Cyclomatic complexity (<10) and nesting depth (≤3) are mentioned only generically, with no first-class scored family. Package/path nesting depth (≤3 levels) is uncovered. And there is no grandfather-ratchet baseline mode so an existing oversized file can warn-at-baseline / fail-if-grown rather than block the whole repo on day one. These extend and complement the existing `SRC-*` shape rules — they do not replace them.

## Where We Want To Be
A `rules/common/size-shape.md` doc plus validators scaffolded through d01, giving the deterministic length family plus a scored complexity family, composed with the d02 baseline-grandfather-ratchet:
- T1 (deterministic, blocks): `SIZE-FILE-1.1` (file ≤200, per-language override Rust ≤400), `SIZE-FUNC-1.1` (function ≤30), `SIZE-CLASS-1.1` (class ≤150 / ≤10-12 public methods), `SIZE-PARAMS-1.1` (≤5 params), `SIZE-LINE-1.1` (line ≤120, measured over the raw line including any trailing pragma), `SIZE-TESTFILE-1.1` (test file ≤300).
- T2 (scored, non-blocking): `SIZE-CX-1.1` cyclomatic complexity <10 / nesting ≤3 as a first-class scored family (AST-derived score+confidence); `py-fastapi-package-nesting-depth` path nesting ≤3 levels.
- Baseline mode: `FE-LEN-2.1` grandfather ratchet — a baselined oversized file warns at/below its recorded baseline and fails only if it grows; this composes with the existing d02 ratchet (d22 supplies the per-file length metric, d02 owns the baseline store + ratchet mechanics).

## Requirement Checklist
- [ ] `rules/common/size-shape.md` created with one anchored section per ruleId, each carrying its tier; per-language cap overrides (Rust file ≤400) documented in the same doc.
- [ ] T1 hard caps ship deterministic validators: file, function, class (+ public-method count), params, line-length (trailing-pragma-inclusive), test-file.
- [ ] T2 scored: `SIZE-CX-1.1` complexity/nesting and `py-fastapi-package-nesting-depth` emit `score`+`confidence` against thresholds.
- [ ] Grandfather ratchet `FE-LEN-2.1`: baselined file at baseline count is clean; same file grown +1 line fails. Composes with d02 (uses d02's baseline store; does not duplicate it).
- [ ] Line-length counts trailing pragmas/comments (a `// eslint-disable-next-line ...` tail does not exempt the line from the cap).
- [ ] All rows registered via d01 `rule new`; parity oracle green across ruleId <-> doc <-> validator <-> {fail,pass} fixtures <-> detection-test.

## Acceptance And Proof
5-way parity per rule. Fixtures live under `tests/fixtures/size-shape/`.

- `SIZE-FILE-1.1` (T1): fail `file-201-lines.fail.ts` (201 lines, flagged); pass `file-200-lines.pass.ts` (200 lines, clean); plus a Rust override pair `file-401.fail.rs` / `file-400.pass.rs` asserting the ≤400 language cap.
- `SIZE-FUNC-1.1` (T1): fail `func-31-lines.fail.ts`; pass `func-30-lines.pass.ts`.
- `SIZE-CLASS-1.1` (T1): fail `class-151-lines.fail.ts` (and a `class-13-public-methods.fail.ts`); pass `class-150-lines.pass.ts`.
- `SIZE-PARAMS-1.1` (T1): fail `fn-6-params.fail.ts`; pass `fn-5-params.pass.ts`.
- `SIZE-LINE-1.1` (T1): fail `line-130-chars.fail.ts` and `line-with-trailing-pragma.fail.ts` (a >120 line whose overflow is a trailing pragma — must still flag); pass `line-120-chars.pass.ts`.
- `SIZE-TESTFILE-1.1` (T1): fail `testfile-301-lines.fail.test.ts`; pass `testfile-300-lines.pass.test.ts`.
- `SIZE-CX-1.1` (T2): fail `cx-12-branches.fail.ts` (cc=12, 5-deep nesting — score crosses); pass `cx-6-guarded.pass.ts` (cc=6, guard clauses — score under). Rust literal-scan scored model: fixtures assert the threshold, not a block.
- `py-fastapi-package-nesting-depth` (T2): fail `deep/app/core/services/domain/user/service.py` (>3 levels — score crosses); pass `shallow/app/user_service.py` (≤3 — under).
- `FE-LEN-2.1` grandfather ratchet (T2): fail `baselined-grown.fail` (baseline recorded via d02, file grown +1 line); pass `baselined-at-baseline.pass` (file at recorded baseline count).

Prove via `tests/size-shape.test.mjs` (all fixtures) and the d01 `rule-scaffold-parity` oracle. Update the corresponding rows in TEST_PROOF_EXPECTATIONS.md before DONE.

## Parallel Ownership Notes
`owns:` set is disjoint: exclusively creates `rules/common/size-shape.md`, `src/size-shape.ts`, and `tests/fixtures/size-shape/**`. Depends on `d01` (scaffolder + parity) and `d02` (baseline-grandfather-ratchet store, which the `FE-LEN-2.1` row composes with rather than reimplements). Extends/complements the existing `SRC-*` shape rules but must not edit them — the length family is additive. Does not touch d21's change-discipline or d23's test-quality families. The `SIZE-TESTFILE-1.1` cap here is a length metric only; test *content* quality (companion presence, assertion-free tests, naming) is d23's scope.
