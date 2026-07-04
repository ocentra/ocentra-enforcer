# arc-04 Crate enforcer-rules

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-rules`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-rules/Cargo.toml`, `crates/enforcer-rules/src/lib.rs`, `crates/enforcer-rules/src/registry.rs`, `crates/enforcer-rules/src/loader.rs`, `crates/enforcer-rules/src/version_drift.rs`, `crates/enforcer-rules/rules/**`, `crates/enforcer-rules/tests/**`
- deps: `arc-01`, `arc-02`, `arc-03`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Rules today live as prose `.md` plus scattered detection logic in `scripts/rust-rules-*.mjs` and `src/*.mjs`. There is no structured rule registry: the id <-> validator <-> fixtures <-> doc-anchor <-> tier linkage is implicit and unenforced.

## Where We Want To Be
`enforcer-rules` is the rules-as-data registry per doctrine: typed rule records (in `enforcer-domain` types / `rules.json` / RON) each carrying `ruleId <-> validator <-> {fail+pass fixtures} <-> doc-anchor <-> tier`, with a typed loader + parity metadata. The AI consumes the structured rule, never prose. This crate also SHIPS the OcentraParent-borrowed discipline as typed **T1 rule records** so the enforcer governs CONSUMER repos (not just its own workspace): (1) the `[workspace.lints]` deny-wall as a rule record (the same deny set a01 hard-codes into this workspace's `Cargo.toml`, re-expressed as rules-as-data validated in a target repo's manifest); (2) the `no-reexports` / no-`pub use`-barrel discipline as a rule record whose Rust-target `Validator` lives in `enforcer-lang-rust` (arc-06); (3) the `version_drift` module (d13) that detects when a rule record's declared version drifts from its validator/fixtures/doc-anchor. `.md` may stay as optional human-canonical text; the engine consumes the structured record.

## Requirement Checklist
- [ ] Implement the structured rule registry per RUST_ARCHITECTURE.md: typed rule records (built on `enforcer-domain` newtypes), loaded from `rules.json`/RON, exposing the id/validator/fixtures/doc-anchor/tier linkage.
- [ ] Provide the typed loader (parse-at-boundary via `enforcer-config` conventions) + parity metadata each rule carries (fail+pass fixture references, tier, doc-anchor).
- [ ] Ship the `[workspace.lints]` deny-wall as a typed **T1 rule record** (rules-as-data) for consumer repos: the deny set (`unsafe_code=forbid`, `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `dbg_macro`, `print_stdout`, `print_stderr`, `await_holding_lock`, `future_not_send`, `clone_on_ref_ptr`, `redundant_clone`, `needless_pass_by_value`, `map_err_ignore`, `large_enum_variant`) encoded as a rule record with fail/pass fixture references and a doc-anchor. (a01 owns the manifest keys that enforce it on THIS workspace; arc-04 owns the rule DATA that lets the enforcer check a target repo.)
- [ ] Ship the `no-reexports` (no `pub use` / `pub(crate) use` barrel) discipline as a typed **T1 rule record**; its Rust-target `Validator` (syn) is owned by arc-06 (`enforcer-lang-rust`), linked here by `RuleId`.
- [ ] Implement d13 **rule-version-drift** (`src/version_drift.rs`): detect when a rule record's declared version is out of sync with its validator/fixtures/doc-anchor (fail-closed), so a rule cannot silently drift from its parity artifacts.
- [ ] Port the current rule catalog / detection metadata scattered in `scripts/rust-rules-*.mjs` and the source-policy `.mjs` into structured rule records (data, not prose).
- [ ] `cargo test -p enforcer-rules` passes: registry loads, every rule record is well-formed, and fail/pass fixture references resolve; a malformed/duplicate rule record is rejected; the deny-wall + no-reexports T1 records load and resolve; a seeded version-drift (rule version bumped without a matching fixture/anchor) fails closed.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-rules` exits 0 — registry loads, all rule records validate (fail fixture for a malformed/duplicate rule), fixture/doc-anchor references resolve, the deny-wall + no-reexports T1 rule records load, and d13 version-drift fails closed on a seeded drift (rule version bumped without matching fixture/anchor). Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
arc-04 owns the crate SKELETON + baseline of `enforcer-rules`: `Cargo.toml` (with `[lints] workspace = true` opting into a01's deny-wall), `src/lib.rs`, the rule `registry`/`loader` module roots, `src/version_drift.rs` (d13, hosted here), the shipped baseline rule records under `rules/**` (including the deny-wall + no-reexports T1 records), and `tests/**`. Deps arc-01/02/03. The validator crate (arc-05) and every lang crate (arc-06..12) consume this registry, so it precedes them. Parallel-safe with arc-03 once foundation lands.

Parallel-ownership boundary (disjoint-owns model): feature packs that ADD rule records own their OWN specific files under this crate — a rule-family pack owns `crates/enforcer-rules/rules/<name>.{json,ron}` (its record) and its fixtures under the owning lang/security crate, NOT the whole `enforcer-rules` crate, and `deps: arc-04` so they are sequenced after this skeleton exists. Keep owns DISJOINT by file. The `no-reexports` rule record here is paired with its Rust `Validator` in arc-06 (`enforcer-lang-rust`) by `RuleId` — arc-06 owns the validator `.rs`; arc-04 owns the record data. d13 version-drift is part of THIS skeleton (not a feature pack).
