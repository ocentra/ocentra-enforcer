//! The `Validator` trait: the base abstraction every lang/security/
//! literal-scan validator family implements.
//!
//! A `Validator` is keyed to exactly one [`RuleId`] (from `enforcer-rules`'
//! registry linkage) and inspects one file's source text, producing zero or
//! more [`Finding`]s (from `enforcer-domain`). It does not walk the
//! filesystem, resolve config, or know about [`ScanScope`] beyond accepting
//! it as context — that orchestration belongs to `enforcer-scan` (arc-14+).
//! This crate owns only the per-file detection contract and the harness
//! that proves it.

use enforcer_domain::findings::{Finding, ScanScope};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;

/// One file's source text, as a validator sees it.
///
/// Deliberately minimal: a validator gets the repo-relative path (for
/// [`Finding::file`]) and the raw source text. Anything richer (parsed AST,
/// config-derived parameters) is a concern for the lang-specific crates
/// that build on this trait, not this base.
#[derive(Debug, Clone, Copy)]
pub struct ValidationInput<'a> {
    /// Repo-relative path of the file under validation.
    pub file: &'a RelPath,
    /// Raw source text of the file.
    pub source: &'a str,
    /// What kind of run this validation is part of (workspace scan, single
    /// file, crate, diff). Most validators ignore this; it exists for the
    /// rare validator whose behavior legitimately depends on scan breadth.
    pub scope: ScanScope,
}

/// The base detection contract every rule's validator implements.
///
/// Implementors MUST be pure with respect to [`ValidationInput`]: same
/// input, same findings, no hidden I/O or global state. That purity is what
/// makes the fixture/parity harness in this crate ([`crate::harness`]) a
/// valid oracle — it re-runs the validator against fixed fixture text and
/// asserts a fixed outcome.
pub trait Validator {
    /// The rule this validator implements. Every finding this validator
    /// produces MUST carry this same [`RuleId`] — the harness asserts that
    /// invariant, not just "some finding fired".
    fn rule_id(&self) -> &RuleId;

    /// Inspect one file, returning every finding it trips. An empty vec
    /// means the file is clean with respect to this validator's rule.
    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding>;
}
