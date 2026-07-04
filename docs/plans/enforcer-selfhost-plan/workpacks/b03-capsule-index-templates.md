# b03 Capsule Index Templates

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Capsule Index Templates`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-plan/templates/capsule.tpl, crates/enforcer-plan/templates/workpack-index.tpl, crates/enforcer-plan/templates/plan-readme.tpl, crates/enforcer-plan/src/templates.rs, crates/enforcer-plan/tests/fixtures/plan-templates/**`
- deps: `arc-20-enforcer-plan`
- tier: `P0 T1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md).

## Where We Are
The agent-capsule block, WORKPACK_INDEX, and the plan README no-read routing lists live only in authoring prose. b01 would hand-inline the capsule text and b02 hand-encode the expected block. Two copies of the same contract are a drift bug waiting to happen. arc-20 stands up the `enforcer-plan` crate skeleton but ships no template assets. This pack owns the `templates/*.tpl` assets, the `src/templates.rs` loader/renderer module, and its `cargo test` fixtures — it does NOT own the whole `enforcer-plan` crate.

## Where We Want To Be
One canonical set of verbatim template assets under `crates/enforcer-plan/templates/` (capsule, WORKPACK_INDEX, plan README with token-routing / no-read lists) plus a `src/templates.rs` Rust module that loads and renders them via deterministic string substitution, so both the scaffolder (b01) and validator (b02) consume them as the single source of truth (no inline capsule duplication).

## Requirement Checklist
- [ ] `templates/capsule.tpl` holds the exact agent-capsule block with named placeholders (`{{plan}}`, `{{doc}}`).
- [ ] `templates/workpack-index.tpl` defines the WORKPACK_INDEX table columns and selection semantics.
- [ ] `templates/plan-readme.tpl` encodes token-routing guidance and the explicit no-read lists (which docs an agent must NOT open unless routed).
- [ ] Template files are pure data (no logic); `src/templates.rs` performs deterministic string substitution (embedded via `include_str!` or a compile-time load), returning typed errors on missing placeholders — never a bare panic.
- [ ] A frozen snapshot fixture pins each template so accidental edits surface as a diff.
- [ ] Obeys `[workspace.lints]` (no `unwrap`/`expect`/`panic`/`print_*`); no `pub use` barrels.

## Acceptance And Proof
Tier P0 (contract/schema), T1, Rust-native (`cargo test`). Prove via `cargo test -p enforcer-plan` over `crates/enforcer-plan/tests/fixtures/plan-templates/**`: a snapshot test asserts each rendered template equals its frozen fixture; a contract test asserts b01 (scaffolder) and b02 (validator) both consume `src/templates.rs` (no inline duplicates) — a source check that no capsule literal appears outside `crates/enforcer-plan/templates/` and `src/templates.rs`. Name these detection tests in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Deps `arc-20-enforcer-plan` (crate skeleton), sequenced after that skeleton exists. This is the shared contract layer, authored as its own workpack to keep b01 and b02 disjoint. Owns `crates/enforcer-plan/templates/*.tpl` + `crates/enforcer-plan/src/templates.rs` + `crates/enforcer-plan/tests/fixtures/plan-templates/**` — disjoint by file from b01 (scaffolder module), b02 (validator module + `enforcer-rules` records), b04 (orchestrator module), b05 (skill/command), and d25 (verify-gates module) inside the same arc-20 crate; b01/b02/b04/b05 import these read-only. The contract test enforces adoption.
