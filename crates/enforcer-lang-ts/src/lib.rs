//! `enforcer-lang-ts` — the per-family `Validator` implementations for the
//! TypeScript/JavaScript rule family (`TS-1` .. `TS-8`, 73 rules total).
//!
//! # Charter
//!
//! This crate hosts the Rust-native replacement for the ad-hoc
//! `src/source-policy-typescript-*.mjs` detection logic. It builds on
//! [`enforcer_validator::validator::Validator`] and proves every rule
//! through [`enforcer_validator::harness::run_fixture_parity`].
//!
//! The enforcer VALIDATES TypeScript/JavaScript source from Rust — this
//! crate does not run in TypeScript and does not execute `tsc`/`eslint`
//! itself (the `typescript/toolchain` and `typescript/eslint-json`
//! validators inspect config files and CI wiring text, not live compiler
//! output; that live-tool integration is a `enforcer-harness` concern).
//!
//! Seven validator families cover the 73 rules (see
//! `docs/plans/enforcer-selfhost-plan/workpacks/arc-07-enforcer-lang-ts.md`
//! for the authoritative per-prefix table):
//!
//! - [`rules::source_scan`] — `typescript/source-scan` (17 rules: TS-1..3,
//!   TS-6 hand-authored subset).
//! - [`rules::test_scan`] — `typescript/test-scan` (TS-3.1).
//! - [`rules::import_boundaries`] — `typescript/import-boundaries` (TS-4.1).
//! - [`rules::toolchain`] — `typescript/toolchain` (4 rules: TS-5.1,
//!   TS-7.1/12/13).
//! - [`rules::eslint_json`] — `typescript/eslint-json` (TS-5.2).
//! - [`rules::tests_family`] — `typescript/tests` (TS-8.10).
//! - [`rules::generic_scanner`] — the TS slice (48 rules) of the
//!   cross-family `generic-scanner` engine. This crate owns only the TS
//!   rows of that shared engine's trigger table, not the engine itself
//!   (arc-09 owns the engine and its common/python/typescript partition).
//!
//! [`rules::registry`] enumerates every one of the 73 `TS-*` rule ids with
//! its owning validator and fixture pair, and is the single source the
//! count-parity completeness test in `tests/completeness.rs` walks.

pub mod rules;
