//! Shared, non-public helpers every Dart rule module in this crate builds
//! its [`enforcer_validator::validator::Validator`] impls on: a
//! [`Finding`] builder (mirrors `enforcer-lang-common`'s
//! `rules::fsm`/`rules::size_shape` `finding()` helper) and small
//! line-oriented text helpers (`first_line_containing`,
//! `contains_any`) that keep each rule's `validate` body a short,
//! auditable scan over `input.source` rather than a hand-rolled loop
//! repeated in every module.
//!
//! Not `pub` at the crate root (workspace doctrine: no `pub use`
//! barrels) — sibling rule modules reach these via
//! `crate::rules::support::*`.

use enforcer_domain::findings::Finding;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::ValidationInput;

/// The fixed parts of one rule's finding: id, severity, and title.
/// Bundled so the per-call-site `finding()` helper stays under clippy's
/// `too_many_arguments` limit (mirrors
/// `enforcer_lang_common::rules::fsm::FindingSpec`).
pub struct FindingSpec<'a> {
    pub rule_id: &'a RuleId,
    pub severity: Severity,
    pub title: &'a str,
}

/// Build a [`Finding`] for one of this crate's validators.
pub fn finding(
    spec: &FindingSpec<'_>,
    detail: String,
    input: &ValidationInput<'_>,
    line: u32,
) -> Finding {
    Finding {
        rule_id: spec.rule_id.clone(),
        severity: spec.severity,
        title: spec.title.to_owned(),
        detail,
        file: input.file.clone(),
        line,
        snippet: None,
    }
}

/// Find the 1-based line number of the first line containing `marker`, or
/// `None` if absent.
pub fn first_line_containing(source: &str, marker: &str) -> Option<u32> {
    source
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(marker))
        .map(|(idx, _)| (idx as u32).saturating_add(1))
}

/// Find the 1-based line number of the first line containing any of
/// `markers`, or `None` if none is present.
pub fn first_line_containing_any(source: &str, markers: &[&str]) -> Option<u32> {
    source
        .lines()
        .enumerate()
        .find(|(_, line)| markers.iter().any(|marker| line.contains(marker)))
        .map(|(idx, _)| (idx as u32).saturating_add(1))
}
