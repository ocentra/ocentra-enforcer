# b02 Plan Structure Validator

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Plan Structure Validator`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-plan/src/validator.rs, crates/enforcer-rules/src/rules/plan.rs, crates/enforcer-plan/tests/fixtures/plan-validator/**`
- deps: `arc-20-enforcer-plan, arc-05-enforcer-validator, arc-04-enforcer-rules`
- tier: `P4 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The skeleton and capsule rules exist only as prose in this plan's authoring instructions. Nothing mechanically checks that a workpack has the capsule, owns/deps/tier, or that no-dep-edge workpacks have disjoint owns globs. Prose without a backing check is hope, not proof. arc-20 stands up the `enforcer-plan` crate skeleton and arc-04 the rule registry, but neither carries the PLAN-* rules. This pack owns the `src/validator.rs` module in `enforcer-plan` plus the typed `PLAN-*` rule records in `enforcer-rules` (`src/rules/plan.rs`) and its `cargo test` fixtures — it does NOT own either whole crate.

## Where We Want To Be
A hard, fail-closed validator in `enforcer-plan` (arc-20): `src/validator.rs` implementing the `Validator` trait (from `enforcer-validator`, arc-05) for the PLAN-* rules, emitting structured `Finding`s (from `enforcer-domain`) that can be pointed at any plan dir — including THIS one — and deterministically report every structural violation. Each PLAN-* rule is a typed rule RECORD in `enforcer-rules` (arc-04) carrying `ruleId <-> validator <-> {fail,pass fixtures} <-> doc-anchor <-> tier`.

## Requirement Checklist
- [ ] PLAN-CAPSULE: every workpack contains the exact agent-capsule marker block, unmodified fields — a `Validator` impl emitting a `Finding` on any deviation.
- [ ] PLAN-SKELETON: required headings present in order (Where We Are, Where We Want To Be, Requirement Checklist, Acceptance And Proof, Parallel Ownership Notes).
- [ ] PLAN-FRONTMATTER: owns/deps/tier lines present and well-formed; tier in the P0-P5 set; ids parse into `enforcer-domain` newtypes.
- [ ] PLAN-PARALLEL-SAFETY: for any two workpacks with no dep edge between them, their owns globs MUST be disjoint (overlap => `Finding`). This is the reusable predicate b04's orchestrator imports.
- [ ] Each PLAN-* rule is a typed record in `enforcer-rules` with `ruleId <-> validator <-> doc-anchor <-> {fail,pass fixtures}` parity and fails closed on missing input.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier T1 / P4 (self-enforce green), Rust-native (`Validator` impls + fail/pass fixtures + `cargo test` detection). Prove via `cargo test -p enforcer-plan` (and `-p enforcer-rules` for record parity): a rules detection test drives each PLAN-* rule against pass/fail fixtures in `crates/enforcer-plan/tests/fixtures/plan-validator/**`; a self-host test runs the validator against `docs/plans/enforcer-selfhost-plan/` and asserts zero `Finding`s (self-enforce green); a parity test asserts every PLAN-* `ruleId` maps to a rule record, a `Validator`, a doc anchor, and both fixtures. Name these detection tests in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Deps `arc-20-enforcer-plan` (crate skeleton + `Validator` registration), `arc-05-enforcer-validator` (the `Validator` trait + parity harness), and `arc-04-enforcer-rules` (the rule registry the PLAN-* records live in), sequenced after those skeletons exist. Consumes b01's golden fixture read-only for a positive case; otherwise independent. Owns `crates/enforcer-plan/src/validator.rs` + `crates/enforcer-rules/src/rules/plan.rs` + its own `tests/fixtures/plan-validator/**` — disjoint by file from b01 (scaffolder module), b03 (`templates/` assets), b04 (orchestrator module), b05 (skill/command), and d25 (verify-gates module). b04 imports this pack's PLAN-PARALLEL-SAFETY predicate and b05 invokes the validator against the live plan dir, but neither adds source here.
