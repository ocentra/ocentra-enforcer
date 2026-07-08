# d12 Layered And Frontend RuleIds

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Layered And Frontend RuleIds`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-ts/src/rules/layered_frontend.rs, crates/enforcer-lang-ts/tests/fixtures/layered_frontend/**`
- deps: `d01, arc-07, arc-05, arc-04`
- tier: `P0 contract/schema`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The `enforcer-lang-ts` crate (arc-07) carries the baseline TS-family `Validator` impls (source-domain, package-manifest, tsconfig, boundaries, tests) built on the `enforcer-validator` trait, but ADBP's layered/frontend AST linters (no-repo-in-router, no-fetch-in-useEffect, feature-boundaries, StrEnum-only, symbol-level-DI) are not yet backed as `Validator`s. They exist only as separate prose guidance. Note: the enforcer VALIDATES a user's TS/JS from Rust — it is not itself TS. TS/JS targets are parsed with `tree-sitter` (or `swc`), never by shelling out to a TS toolchain.

## Where We Want To Be
Fold ADBP's five layered/frontend AST linters into `enforcer-lang-ts` as first-class `Validator` impls in `crates/enforcer-lang-ts/src/rules/layered_frontend.rs`, each keyed to a `RuleId` record in `enforcer-rules` (arc-04) and scaffolded via the d01 mechanization engine (arc-14). Each rule parses the target TS/JS with `tree-sitter`/`swc`, walks the AST, and emits structured `Finding`s (from `enforcer-domain`) plus a terse `Fix:` hint — never a println/exit binary — mirroring the existing baseline TS validators the arc-07 skeleton registers.

## Requirement Checklist
- [ ] Implement each of the five as a `Validator` impl (tree-sitter/swc AST walk) in `src/rules/layered_frontend.rs`, registered in the `enforcer-lang-ts` rule set exposed by the arc-07 skeleton.
- [ ] Mint a registry rule record per rule via d01 (`enforcer-rules` typed record: `ruleId <-> validator <-> {fail+pass fixtures} <-> doc-anchor <-> tier`).
- [ ] Each rule passes the d01 `rule-scaffold-parity` oracle (id <-> validator <-> doc-anchor <-> fixtures).
- [ ] The optional human-canonical doc anchor for each rule is carried in the rule record; the engine consumes the structured record, not prose.
- [ ] Rules are AST-based (T1 deterministic), not text-heuristic; obey `[workspace.lints]` (no `unwrap/expect/panic/print_*`) and no `pub use` barrels.

## Acceptance And Proof
Tier T1, P0 contract/schema. Prove via `cargo test -p enforcer-lang-ts` over `crates/enforcer-lang-ts/tests/fixtures/layered_frontend/<rule>/{bad,good}/` (a fail fixture per rule that must trip and a pass fixture that must stay clean), plus the d01 parity oracle. Mechanism: five tree-sitter/swc AST `Validator`s emitting registry-backed `Finding`s, each with a fail-fixture that must flag and a pass-fixture that must not. Record the detection-test artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Depends on d01 (scaffold + parity oracle) and on arc-07 for the `enforcer-lang-ts` crate skeleton (Cargo.toml, lib.rs, rule-set registration) — this pack lands only `src/rules/layered_frontend.rs` + its fixtures and must not edit the crate skeleton or the baseline TS validators the arc-07 pack owns. Deps arc-05 (Validator trait) and arc-04 (rule records). `owns:` is disjoint by file from the arc-07 skeleton, from e-pack-frontend-react (`src/rules/frontend_react.rs`), and from every sibling.
