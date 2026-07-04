//! `enforcer-validator` — the `Validator` trait plus the reusable
//! fixture/parity harness (arc-05).
//!
//! # Charter
//!
//! This crate is the BASE every lang/security/literal-scan validator
//! family (`enforcer-lang-rust`, `enforcer-lang-ts`, `enforcer-lang-py`,
//! `enforcer-lang-common`, `enforcer-lang-security`, `enforcer-lang-iac`,
//! `enforcer-lang-k8s`, `enforcer-literal-scan`, ...) builds on. It owns:
//!
//! - The [`validator::Validator`] trait: the minimal per-file detection
//!   contract every rule implementation satisfies.
//! - The [`harness::run_fixture_parity`] oracle: given a validator and its
//!   fail/pass fixture paths, assert it fires on the fail fixture and
//!   stays silent on the pass fixture. This is the Rust-native replacement
//!   for the ad-hoc `.mjs` detection-check/parity plumbing — every lang
//!   crate calls this from its own `cargo test` rather than reimplementing
//!   fixture I/O and pass/fail assertions itself.
//!
//! This crate does NOT own: rule records or registry lookup
//! (`enforcer-rules`), filesystem walking or scan orchestration
//! (`enforcer-scan`), or any language-specific detection logic (the
//! `enforcer-lang-*` crates depend on this one, never the reverse).

pub mod error;
pub mod harness;
pub mod validator;
