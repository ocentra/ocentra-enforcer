# b01 Plan Scaffolder

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Plan Scaffolder`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-plan/src/scaffolder.rs, crates/enforcer-plan/tests/fixtures/scaffolder/**`
- deps: `arc-20-enforcer-plan`
- tier: `P1 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
Plans today are hand-assembled: someone copies a prior plan dir, hand-edits the capsule, and hopes the skeleton matches. There is no `enforcer plan new` command and no single source of the OcentraParent skeleton, so drift between plans is guaranteed and unprovable. arc-20 stands up the `enforcer-plan` crate skeleton (`Cargo.toml`/`lib.rs`/module root/`Validator` registration) but nothing inside it deterministically emits a plan directory. This pack owns the `src/scaffolder.rs` module plus its `cargo test` fixtures — it does NOT own the whole `enforcer-plan` crate.

## Where We Want To Be
A deterministic emitter in `enforcer-plan` (arc-20): `enforcer plan new <name>` (a `scaffolder` module driven by the `enforcer-cli`/`enforcer-mcp` surface) writes a complete, byte-stable plan skeleton (PLAN_STATE, PLAN_EXECUTION_BLUEPRINT, TEST_PROOF_EXPECTATIONS, WORKPACK_INDEX, capsule-stamped workpack stub) rendered from b03's template assets under `crates/enforcer-plan/templates/`, and that b02's PLAN-* `Validator` validates green. Emitted names route through `enforcer-domain` branded newtypes (parse-at-boundary), never bare `String`.

## Requirement Checklist
- [ ] `enforcer plan new <name>` emits the full directory tree under `docs/plans/<name>/` via the `scaffolder` module in `enforcer-plan`.
- [ ] Every emitted file carries the exact agent-capsule block and required frontmatter (owns/deps/tier), rendered from b03's `crates/enforcer-plan/templates/` assets (no inline duplicate of the capsule literal).
- [ ] Emission is deterministic: same `<name>` yields byte-identical output (golden fixture under `crates/enforcer-plan/tests/fixtures/scaffolder/`).
- [ ] Refuses to overwrite an existing plan dir (fail-closed) unless `--force`; the plan name parses into an `enforcer-domain` newtype before any I/O.
- [ ] Emitted skeleton passes b02's PLAN-* `Validator` with zero `Finding`s.
- [ ] Emits live RESUME-STATE artifacts in every new plan skeleton (owner req 2026-07-04, AUDIT_FINDINGS WAVE 5): a dedicated `RESUME_STATE.md` (rendered from b03's `crates/enforcer-plan/templates/` assets) carrying a `Where We Are` block PLUS `CHECKLIST` / `TASKLIST` / `PROGRESS` lists PLUS `PREV` and `NEXT` records (done / in-progress / next), so a token-out/crash resumes cheaply without re-deriving state. The scaffolder seeds these with the plan name and empty-but-well-formed records; b02's PLAN-RESUME-STATE rule (this pack's cross-check) reports zero `Finding`s over the emission.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier T1 / P1, Rust-native (`cargo test`). Prove via `cargo test -p enforcer-plan` over `crates/enforcer-plan/tests/fixtures/scaffolder/**`: an emit test diffs emitter output against a checked-in golden tree fixture; a determinism test runs the emitter twice and asserts identical bytes; a cross-check test feeds emitted output to b02's validator entrypoint (`#[test]` invoking the PLAN-* `Validator`) and asserts zero `Finding`s; a resume-state fixture test asserts the scaffolded plan contains `RESUME_STATE.md` with the `Where We Are` block plus the `CHECKLIST`/`TASKLIST`/`PROGRESS` lists and `PREV`/`NEXT` records (golden-tree diff). Name these detection tests in TEST_PROOF_EXPECTATIONS.md proof rows before DONE.

## Parallel Ownership Notes
Deps `arc-20-enforcer-plan` (which owns the crate skeleton — `Cargo.toml`/`lib.rs`/module root/`Validator` registration), sequenced after that skeleton exists. Blocks nothing directly, but b02 and b05 consume its golden fixture and validator cross-check, and it renders from b03's template assets read-only. Owns only `crates/enforcer-plan/src/scaffolder.rs` + `crates/enforcer-plan/tests/fixtures/scaffolder/**`, disjoint by file from b02 (validator module), b03 (`templates/` assets), b04 (orchestrator module), b05 (skill/command), and d25 (verify-gates module) inside the same arc-20 crate — the only shared artifact is the golden fixture, produced here and read-only elsewhere, so all run concurrently once arc-20 lands.
