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

### Rule inventory (per-prefix)
Every `language: "rust"` ruleId prefix present in `rules/rules.json` (166 rules total, grouped by `RR-N` prefix), each named and OWNED here as a provable row — nothing rides the generic "port the .mjs" bullet. Each prefix group ships fail/pass fixtures wired through the arc-05 parity harness (`crates/enforcer-lang-rust/tests/fixtures/RR-N/{bad,good}/`) and a `cargo test -p enforcer-lang-rust` detection test. Counts are exact as of the 2026-07-04 audit (WAVE 3); the completeness assertion below binds them.

| Prefix | Count | `validator`(s) in rules.json | Family / doc | Backing `.mjs` source | Owned row |
|---|---|---|---|---|---|
| RR-1 | 5 | `rust/workspace-files` (4), `rust/cargo-manifest` (1) | toolchain-cargo (`rules/rust/toolchain-cargo.md`) | `rust-rules-cargo-scan.mjs` (+ workspace-file presence checks) | [ ] Toolchain/workspace-file presence Validator(s): `rust-toolchain.toml` pin + required workspace files + `Cargo.toml` manifest shape. |
| RR-2 | 2 | `rust/source-scan` (2) | source (`rules/rust/source.md`) | `rust-rules-source-scan.mjs` + `rust-rules-source-patterns.mjs` | [ ] Source-scan Validator: base source-pattern rows. |
| RR-3 | 22 | `rust/source-scan` (19), `rust/scanner` (3) | source (`rules/rust/source.md`) | `rust-rules-source-scan.mjs` / `-signatures*` / `-late-*` + `rust-rules-scan-core.mjs`/`-scan-engine.mjs` | [ ] Source-scan + generic-scanner Validators: signature/pattern/late-rule source family (largest source group). |
| RR-4 | 20 | `rust/source-scan` (20) | source (`rules/rust/source.md`) | `rust-rules-source-scan.mjs` + `rust-rules-source-signature-rules.mjs`/`-signature-text.mjs` | [ ] Source-scan Validator: signature-rule / signature-text source family. |
| RR-5 | 4 | `rust/source-scan` (4) | source (`rules/rust/source.md`) | `rust-rules-source-scan.mjs` + `rust-rules-source-names.mjs` | [ ] Source-scan Validator: naming/source-name rows. |
| RR-6 | 33 | `rust/domain-types` (11), `rust/source-scan` (18), `rust/scanner` (3), `rust/serialized-domain-types` (1) | domain (`rules/rust/domain.md`) | `rust-rules-source-scan.mjs` + `-late-domain-debug.mjs` + `-classification.mjs`/`-helpers.mjs`; scan-engine | [ ] Domain-types + serialized-domain-types + source-scan Validators: domain-primitive discipline (largest overall group). |
| RR-7 | 5 | `rust/imports-modules` (4), `rust/cargo-manifest` (1) | imports-modules (`rules/rust/imports-modules.md`) | `rust-rules-source-scan.mjs` (imports/modules) + `rust-rules-cargo-scan.mjs` | [ ] Imports-modules + cargo-manifest Validators: module/import discipline (kin to the hosted `no_reexports`). |
| RR-8 | 17 | `rust/async-runtime` (4), `rust/source-scan` (9), `rust/scanner` (4) | async-runtime (`rules/rust/async-runtime.md`) | `rust-rules-source-scan.mjs` + `rust-rules-scan-core.mjs`/`-scan-engine.mjs` (async patterns) | [ ] Async-runtime + source-scan + scanner Validators: async/runtime discipline family. |
| RR-9 | 20 | `rust/dependencies` (9), `rust/scanner` (11) | dependencies (`rules/rust/dependencies.md`) | `rust-rules-cargo-scan.mjs` + `rust-rules-scan-core.mjs`/`-scan-engine.mjs` | [ ] Dependencies + scanner Validators: dependency-policy discipline (blocked-protocol/git/path/build.rs etc). |
| RR-10 | 4 | `rust/cargo-gates` (4) | toolchain-cargo (`rules/rust/toolchain-cargo.md`) | `rust-rules-cargo-scan.mjs` (cargo-deny/audit/doc gates) | [ ] Cargo-gates Validator: `cargo deny`/`cargo audit`/`cargo doc` gate rows. |
| RR-11 | 3 | `rust/dependencies` (3) | dependencies (`rules/rust/dependencies.md`) | `rust-rules-cargo-scan.mjs` | [ ] Dependencies Validator: additional dependency-policy rows. |
| RR-12 | 15 | `rust/scanner` (13), `rust/source-scan` (2) | source (`rules/rust/source.md`) | `rust-rules-scan-core.mjs`/`-scan-engine.mjs` + `rust-rules-source-scan.mjs` | [ ] Scanner + source-scan Validators: generic-scanner source rows. |
| RR-14 | 15 | `rust/scanner` (13), `rust/source-scan` (2) | domain + source (`rules/rust/domain.md`, `rules/rust/source.md`) | `rust-rules-scan-core.mjs`/`-scan-engine.mjs` + `rust-rules-source-scan.mjs` | [ ] Scanner + source-scan Validators: cross domain/source scanner rows. |
| RR-18 | 1 | `rust/runtime-strings` (1) | source (`rules/rust/source.md`) | `rust-rules-source-scan.mjs` (runtime-string literals) | [ ] Runtime-strings Validator: runtime string-literal ban row (arc-03 `enforceRuntimeStringLiterals` policy). |

Note: the two hosted baseline validators above (`no_reexports`, d17 `error_handling`) are NEW structured ports (OcentraParent disciplines), not rows in the RR-* rules.json inventory; they are keyed to their own rule records (arc-04) and are additive to the 166.

- [ ] **Completeness assertion (count parity):** `cargo test -p enforcer-lang-rust` MUST cover every `language: "rust"` ruleId in `rules/rules.json`. Add a parity test that loads the rust ruleId set from `enforcer-rules`, asserts each has a registered `Validator` + at least the required fail/pass fixture per rules.json `requiresFailFixture`/`requiresPassFixture`, and asserts `covered_count == 166` (13 prefixes: RR-1..RR-12, RR-14, RR-18). The test fails if rules.json grows a rust rule with no owning validator/fixture, so no rule silently drops.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-lang-rust` exits 0 with fail/pass fixture coverage per rule (Rust-native parity), including `no_reexports` (a `pub use` barrel fixture flags; a concrete-path fixture is silent) and `error_handling`/d17 (an `unwrap()` in first-party code flags; the same in `cfg(test)` is silent). Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
arc-06 owns the crate SKELETON + baseline of `enforcer-lang-rust`: `Cargo.toml` (`[lints] workspace = true`), `src/lib.rs`, the `src/rules/mod.rs` module root + `Validator` registration, and the two hosted baseline validators `src/rules/no_reexports.rs` (linked to arc-04's rule record) and `src/rules/error_handling.rs` (d17), plus their fixtures. Deps arc-01/02/05 (validator base) + arc-04 (rule records it keys into by `RuleId`). Parallel-safe with all sibling lang crates (arc-07..12) and arc-13/arc-19 — disjoint crate trees, all built on the shared validator base.

Parallel-ownership boundary (disjoint-owns model): additional Rust-rule feature packs own SPECIFIC files under this crate — each owns `crates/enforcer-lang-rust/src/rules/<name>.rs` (+ a `src/rules/<name>/` module dir if needed) and `crates/enforcer-lang-rust/tests/fixtures/<name>/**`, and `deps: arc-06` so they land after this skeleton + module root exist. They do NOT own the whole crate. Keep owns DISJOINT by file; sequence by `deps:`.
