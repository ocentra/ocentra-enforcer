# b04 Parallel Orchestrator Binding

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Parallel Orchestrator Binding`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-plan/src/orchestrator.rs, crates/enforcer-plan/tests/fixtures/orchestrator/**`
- deps: `arc-20-enforcer-plan, arc-16-enforcer-coordination, b02-plan-structure-validator`
- tier: `P1 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
A validated plan tells you which workpacks may run in parallel (disjoint owns, no dep edge) but nothing turns that into an execution frontier. The `enforcer-coordination` crate (arc-16: hub/lane/claim/guard/closeout/ledger/presence) exists but is not bound to plan structure, so lane assignment is manual. arc-20 stands up the `enforcer-plan` crate skeleton but ships no orchestrator. This pack owns the `src/orchestrator.rs` module in `enforcer-plan` plus its `cargo test` fixtures — it does NOT own either whole crate.

## Where We Want To Be
A deterministic binding in `enforcer-plan` (arc-20): `src/orchestrator.rs` computes the parallel frontier from the validated plan graph and drives it through the `enforcer-coordination` (arc-16) crate API: frontier -> hub lanes -> claim/guard/closeout, with an intent-queue for overlap resolution. It reuses b02's PLAN-PARALLEL-SAFETY predicate (imported, not reimplemented) to keep concurrent lanes disjoint.

## Requirement Checklist
- [ ] Build the dep DAG from workpack `deps:` fields and compute the ready frontier (deps satisfied), over `enforcer-domain` newtypes.
- [ ] Assign frontier workpacks to hub lanes so no two concurrent lanes share owns globs (reuse b02's PLAN-PARALLEL-SAFETY predicate, do not reimplement).
- [ ] Bind lane lifecycle to the `enforcer-coordination` (arc-16) claim -> guard -> closeout API.
- [ ] Intent-queue serializes any residual owns overlap that slips past static checks (fail-closed: refuse concurrent claim on overlapping owns).
- [ ] Reuse the existing `enforcer-coordination` crate; add no parallel coordination store.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier P1, T1, Rust-native (`cargo test`). Prove via `cargo test -p enforcer-plan` over `crates/enforcer-plan/tests/fixtures/orchestrator/**`: a frontier test asserts the ready frontier for a fixture plan graph; a lanes test asserts disjoint-owns lane assignment via the reused PLAN-PARALLEL-SAFETY predicate; a claim/guard test uses an `enforcer-coordination` fake/in-memory harness to assert claim/guard/closeout are invoked in order and that overlapping-owns claims are rejected fail-closed. Name these detection tests in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Deps `arc-20-enforcer-plan` (crate skeleton), `arc-16-enforcer-coordination` (the hub/lane/claim/guard/closeout API this binds to), and `b02-plan-structure-validator` (imports its PLAN-PARALLEL-SAFETY predicate and consumes a validated plan graph), so it starts after those land. Owns only `crates/enforcer-plan/src/orchestrator.rs` + `crates/enforcer-plan/tests/fixtures/orchestrator/**`, disjoint by file from b01 (scaffolder module), b02 (validator module), b03 (`templates/` assets), b05 (skill/command), and d25 (verify-gates module) — d25 hosts its own `src/verify_gates.rs` in the same arc-20 crate, a different file, so the two run concurrently. It blocks nothing else in Track B.
