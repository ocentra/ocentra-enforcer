# arc-07 Crate enforcer-lang-ts

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-lang-ts`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-ts/**`
- deps: `arc-01`, `arc-02`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
TypeScript/JS-family rule detection lives in `src/source-policy-typescript-*.mjs` (source-domain, package-manifest, tsconfig, tests, boundaries) and the eslint-rule logic, as ad hoc JS. No crate implements the TS family against the `Validator` trait. Note: the enforcer VALIDATES TS from Rust — it does not run in TS.

## Where We Want To Be
`enforcer-lang-ts` is the per-family validator crate for TypeScript: `Validator` impls (built on `enforcer-validator`) covering the TS rule family (source domain, package manifest, tsconfig, boundaries, tests), each with fail/pass fixtures and a `cargo test` detection test.

## Requirement Checklist
- [ ] Implement the TS-family `Validator` impls per RUST_ARCHITECTURE.md, keyed to their `RuleId`s in `enforcer-rules`.
- [ ] Port the corresponding `.mjs` detection logic (`src/source-policy-typescript-*.mjs`, package-manifest/tsconfig/boundaries/tests, and the eslint-rule detection) to Rust validators.
- [ ] Provide fail/pass fixtures per rule; wire them through the `enforcer-validator` parity harness.
- [ ] Cover every `TS-*` prefix in the Rule inventory below (73 rules across TS-1..TS-8), not just the matrix subset; TS-6/7/8 must be row-level provable, not a generic bullet.
- [ ] `cargo test -p enforcer-lang-ts` passes: every validator fires on its fail fixture and is silent on its pass fixture.
- [ ] Count-parity/completeness test: `cargo test -p enforcer-lang-ts` asserts every `language == typescript` ruleId has a validator + fixtures and the total equals `rules/rules.json` (73).
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Rule inventory (per-prefix)
Authoritative source: `rules/rules.json`, `"language": "typescript"` → **73 rules** across 8 `TS-*` prefixes. The matrix today enumerates only the small single-source prefixes (TS-1..TS-5); TS-6/TS-7/TS-8 (65 rules) previously rode a single generic bullet. Every row below is provable via the arc-05 parity harness (fail fixture fires, pass fixture is silent) and is covered by the count-parity assertion.

`SHARED-ENGINE NOTE:` the `generic-scanner` validator/engine is SHARED across `common` + `python` + `typescript` (81 rules total; 48 of them TS). arc-07 owns ONLY the TypeScript SLICE of `generic-scanner` rules (`language: typescript`, 48 rows below), NOT the engine itself. The shared `generic-scanner` engine and its cross-family partition spec (common/py/ts) are owned by **arc-09** (see AUDIT_FINDINGS WAVE 3 MIS-MAP: `generic-scanner` 81-rule partition). arc-07 consumes the engine and asserts only its TS slice.

| Prefix | Count | Validator(s) (count) | Backing source |
| --- | --- | --- | --- |
| TS-1 | 3 | `typescript/source-scan` (3) | `src/source-policy-typescript-source*.mjs`, `-source-domain*.mjs` |
| TS-2 | 1 | `typescript/source-scan` (1) | `src/source-policy-typescript-source*.mjs` |
| TS-3 | 1 | `typescript/test-scan` (1) | `src/source-policy-typescript-test-*.mjs` |
| TS-4 | 1 | `typescript/import-boundaries` (1) | `src/source-policy-typescript-source-domain-domain-boundaries*.mjs` |
| TS-5 | 2 | `typescript/toolchain` (1), `typescript/eslint-json` (1) | `src/source-policy-typescript-manifest-tsconfig.mjs`, eslint-rule detection |
| TS-6 | 40 | `typescript/source-scan` (13), `generic-scanner` (27, TS slice) | `src/source-policy-typescript-source*.mjs`, `src/generic-typescript-scanner.mjs` (+ `generic-scanner-shared.mjs`) |
| TS-7 | 15 | `typescript/toolchain` (3), `generic-scanner` (12, TS slice) | `src/source-policy-typescript-manifest*.mjs`, `-package-manifest*.mjs`, `src/generic-typescript-scanner.mjs` |
| TS-8 | 10 | `typescript/tests` (1), `generic-scanner` (9, TS slice) | `src/source-policy-typescript-tests*.mjs`, `src/generic-typescript-scanner.mjs` |

Validator totals (TS slice): `generic-scanner` 48, `typescript/source-scan` 17, `typescript/toolchain` 4, `typescript/eslint-json` 1, `typescript/import-boundaries` 1, `typescript/test-scan` 1, `typescript/tests` 1 → **73** (Σ = rules.json typescript count).

`COMPLETENESS / COUNT-PARITY ASSERTION:` `cargo test -p enforcer-lang-ts` includes a completeness test that loads every `enforcer-rules` ruleId with `language == typescript`, asserts each has a registered `Validator` impl (no orphan ruleId), and asserts the enumerated total equals the count in `rules/rules.json` (73). The test FAILS if rules.json gains/loses a typescript rule without a matching validator + fixtures — closing the "generic bullet" hole. TS-slice `generic-scanner` rows are asserted against the arc-09-owned shared engine via the partition manifest; arc-07 does not re-implement the engine.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-lang-ts` exits 0 with fail/pass fixture coverage per rule. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-lang-ts/**`. Deps arc-01/02/05. Parallel-safe with all sibling lang crates (arc-06, arc-08..12) and arc-13/arc-19 — disjoint crate trees.
