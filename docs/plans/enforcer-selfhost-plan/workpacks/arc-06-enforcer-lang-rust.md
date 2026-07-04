# arc-06 Crate enforcer-lang-rust

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-lang-rust`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-lang-rust/Cargo.toml`, `crates/enforcer-lang-rust/src/lib.rs`, `crates/enforcer-lang-rust/src/rules/mod.rs`, `crates/enforcer-lang-rust/src/rules/no_reexports.rs`, `crates/enforcer-lang-rust/src/rules/error_handling.rs`, `crates/enforcer-lang-rust/tests/fixtures/no_reexports/**`, `crates/enforcer-lang-rust/tests/fixtures/error_handling/**`
- deps: `arc-01`, `arc-02`, `arc-04`, `arc-05`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
Rust-family rule detection lives in `scripts/rust-rules-source-*.mjs` / `rust-rules-scan-*.mjs` as ad hoc JS. There is no crate implementing the Rust language family against the `Validator` trait. OcentraParent's `no-reexports` discipline exists only as its own standalone println-style AST check (not a structured `Validator`), and Rust error-handling discipline (the `unwrap`/`expect`/`panic` family, d17) has no `syn`-based validator.

## Where We Want To Be
`enforcer-lang-rust` is the per-family validator crate for Rust: a set of `syn`-AST `Validator` impls (built on `enforcer-validator`) covering the Rust rule family, each keyed to a `RuleId` in `enforcer-rules`, each with fail/pass fixtures and a `cargo test` detection test. It hosts two OcentraParent-borrowed disciplines as structured validators: (1) the **`no-reexports`** Validator (`src/rules/no_reexports.rs`) — a `syn`-AST check that bans `pub use` / `pub(crate) use` barrels and rejects the `const _ = size_of` keep-alive idiom, emitting structured `Finding`s + a `Fix:` hint (NOT a println/exit binary), linked by `RuleId` to the no-reexports rule record arc-04 ships; and (2) **d17 rust-error-handling** (`src/rules/error_handling.rs`) — a `syn` Validator for the `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg!` family in first-party (non-`cfg(test)`) code, complementing the clippy deny-wall with structured, doc-anchored, fixable Findings on consumer repos.

## Requirement Checklist
- [ ] Implement the Rust-family `syn`-AST `Validator` impls per RUST_ARCHITECTURE.md, keyed to their `RuleId`s in `enforcer-rules` and consuming `enforcer-domain` types.
- [ ] Implement the **`no-reexports`** Validator (`src/rules/no_reexports.rs`): a `syn`-AST check banning `pub use` / `pub(crate) use` barrels and the `const _ = size_of` keep-alive idiom, emitting structured `Finding`s + a `Fix:` hint (never a println/exit binary), linked by `RuleId` to arc-04's no-reexports rule record.
- [ ] Implement **d17 rust-error-handling** (`src/rules/error_handling.rs`): a `syn` Validator flagging `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`dbg!` in first-party code (skip `cfg(test)`), with structured Findings + `Fix:` hints.
- [ ] Port the corresponding `.mjs` detection logic (`scripts/rust-rules-source-*.mjs`, `rust-rules-scan-*.mjs`, signature/pattern/late-* rules) to Rust validators.
- [ ] Provide fail/pass fixtures per rule under `crates/enforcer-lang-rust/tests/fixtures/<rule>/{bad,good}/`; wire them through the `enforcer-validator` parity harness.
- [ ] `cargo test -p enforcer-lang-rust` passes: every validator (including `no_reexports` and `error_handling`) fires on its fail fixture and is silent on its pass fixture.
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-lang-rust` exits 0 with fail/pass fixture coverage per rule (Rust-native parity), including `no_reexports` (a `pub use` barrel fixture flags; a concrete-path fixture is silent) and `error_handling`/d17 (an `unwrap()` in first-party code flags; the same in `cfg(test)` is silent). Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
arc-06 owns the crate SKELETON + baseline of `enforcer-lang-rust`: `Cargo.toml` (`[lints] workspace = true`), `src/lib.rs`, the `src/rules/mod.rs` module root + `Validator` registration, and the two hosted baseline validators `src/rules/no_reexports.rs` (linked to arc-04's rule record) and `src/rules/error_handling.rs` (d17), plus their fixtures. Deps arc-01/02/05 (validator base) + arc-04 (rule records it keys into by `RuleId`). Parallel-safe with all sibling lang crates (arc-07..12) and arc-13/arc-19 — disjoint crate trees, all built on the shared validator base.

Parallel-ownership boundary (disjoint-owns model): additional Rust-rule feature packs own SPECIFIC files under this crate — each owns `crates/enforcer-lang-rust/src/rules/<name>.rs` (+ a `src/rules/<name>/` module dir if needed) and `crates/enforcer-lang-rust/tests/fixtures/<name>/**`, and `deps: arc-06` so they land after this skeleton + module root exist. They do NOT own the whole crate. Keep owns DISJOINT by file; sequence by `deps:`.
