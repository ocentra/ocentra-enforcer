//! `enforcer-lang-rust` — the Rust-family `Validator` crate (arc-06
//! skeleton + baseline).
//!
//! # Charter
//!
//! This crate is the per-family validator crate for Rust: a set of
//! `syn`-AST [`enforcer_validator::validator::Validator`] impls covering the
//! Rust rule family, each keyed to a [`enforcer_domain::ids::RuleId`] in
//! `enforcer-rules`, each with fail/pass fixtures proven by
//! `enforcer_validator::harness::run_fixture_parity`.
//!
//! arc-06 owns the crate SKELETON plus two hosted baseline validators:
//! - [`rules::no_reexports`] — bans `pub use` / `pub(crate) use` barrels and
//!   the `const _ = size_of` keep-alive idiom, keyed to arc-04's
//!   `T1-NOREEXPORT.1` rule record.
//! - [`rules::error_handling`] — d17 rust-error-handling: flags
//!   `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`dbg!` in
//!   first-party (non-`cfg(test)`) code.
//!
//! Additional Rust-rule feature packs (the `RR-*` prefix inventory in
//! `TEST_PROOF_EXPECTATIONS.md`) own SPECIFIC files under
//! `src/rules/<name>.rs` + `fixtures/<name>/**`, disjoint from this
//! skeleton's owned files, sequenced after it by `deps: arc-06`.
//!
//! No `pub use` barrels (workspace doctrine, and the very rule this crate
//! hosts): consumers path through the modules directly, e.g.
//! `enforcer_lang_rust::rules::no_reexports::NoReexportsValidator`.

pub mod rules;
