//! `enforcer-mechanization` — the d01 crate: the rule scaffolder plus the
//! fail-closed parity oracle (arc-14).
//!
//! # Charter
//!
//! Track D (d01) rule mechanization used to be spread across `.mjs`
//! check/contract scripts (`scripts/check-source-core-contract-*.mjs`) that
//! enforced rule/fixture completeness ad hoc. This crate is the Rust
//! replacement:
//!
//! - [`scaffold`] — given a minimal spec for a NEW rule, emit a well-formed
//!   `enforcer_rules::registry::RuleRecord`, a `Validator` implementation
//!   stub (source text), and starter content for both fixture slots. The
//!   scaffolder never silently produces an already-passing rule: the
//!   generated validator stub always returns zero findings, so a
//!   freshly-scaffolded rule fails the oracle below until a human
//!   implements real detection logic.
//! - [`oracle`] — the fail-closed parity oracle: a candidate rule is only
//!   ACCEPTED if its record shape is well-formed, a validator
//!   implementation is supplied, that validator's `rule_id()` matches the
//!   record, it fires on the declared fail fixture, and it stays silent on
//!   the declared pass fixture. Built on
//!   [`enforcer_validator::harness::run_fixture_parity`] — this crate does
//!   not reimplement fixture I/O or pass/fail assertions, it composes the
//!   reusable base with the record-shape re-check.
//!
//! This crate does NOT own: the rule registry itself (`enforcer-rules`),
//! the `Validator` trait or harness (`enforcer-validator`), or any
//! language-specific detection logic (the `enforcer-lang-*` crates).
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_mechanization::oracle::accept_rule`.

pub mod error;
pub mod oracle;
pub mod scaffold;
