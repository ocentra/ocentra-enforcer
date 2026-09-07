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

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::findings::{Finding, ScanScope};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::syntax_types::{CapabilitySet, FactCapability};

use crate::analysis::{CapabilityMatch, PreparedAnalysis};

/// One file's validated path, source text, and scan scope as seen by a validator.
#[derive(Debug, Clone, Copy)]
#[doc = "One file's validated path, source text, and scan scope as seen by a validator."]
pub struct ValidationInput<'a> {
    /// Repo-relative path of the file under validation.
    pub file: &'a RelPath,
    /// Raw source text of the file.
    pub source: ValidationSource<'a>,
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
pub trait Validator: Send + Sync {
    /// The rule this validator implements. Every finding this validator
    /// produces MUST carry this same [`RuleId`] — the harness asserts that
    /// invariant, not just "some finding fired".
    fn rule_id(&self) -> &RuleId;

    /// Declare the closed fact capability required by this validator.
    fn required_capabilities(&self) -> CapabilitySet {
        CapabilitySet::empty()
    }

    /// Inspect one file, returning every finding it trips. An empty vec
    /// means the file is clean with respect to this validator's rule.
    fn validate(&self, input: ValidationInput<'_>) -> Vec<Finding>;

    /// Dispatch with prepared analysis while preserving legacy validators.
    fn validate_with_analysis(
        &self,
        input: ValidationInput<'_>,
        analysis: Option<&PreparedAnalysis>,
    ) -> ValidationDispatch {
        let required = self.required_capabilities();
        if required.contains(FactCapability::FunctionFacts) {
            let Some(prepared) = analysis else {
                return ValidationDispatch::Skipped(AnalysisSkip::NotPrepared);
            };
            if prepared.capability_match(FactCapability::FunctionFacts)
                == CapabilityMatch::NotSatisfied
            {
                return ValidationDispatch::Skipped(AnalysisSkip::RequirementUnavailable);
            }
        }
        ValidationDispatch::Ran(self.validate(input))
    }
}

/// Explicit result of the analysis-aware dispatch seam.
#[derive(Debug, PartialEq, Eq)]
pub enum ValidationDispatch {
    /// The validator ran and produced its ordinary findings.
    Ran(Vec<Finding>),
    /// The validator did not run because its declared fact contract was absent.
    Skipped(AnalysisSkip),
}

/// Typed reason a fact-backed validator did not run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisSkip {
    /// No prepared analysis was supplied.
    NotPrepared,
    /// Prepared analysis was unavailable, malformed, unsafe, or incomplete.
    RequirementUnavailable,
}
