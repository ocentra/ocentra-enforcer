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

use enforcer_domain::boundary::validation::{ValidationMarker, ValidationSource};
use enforcer_domain::ids::{BuiltInDartRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

/// The fixed parts of one rule's finding: id, severity, and title.
/// Bundled so the per-call-site `finding()` helper stays under clippy's
/// `too_many_arguments` limit (mirrors
/// `enforcer_lang_common::rules::fsm::FindingSpec`).
#[derive(Debug)]
pub(crate) struct FindingSpec<'a> {
    pub(crate) rule_id: &'a RuleId,
    pub(crate) rule: BuiltInDartRule,
    pub(crate) severity: Severity,
}

pub(crate) trait IntoSourceLine {
    fn into_source_line(self) -> Option<SourceLine>;
}

impl IntoSourceLine for u32 {
    fn into_source_line(self) -> Option<SourceLine> {
        std::num::NonZeroU32::new(self).map(SourceLine::try_new)
    }
}

impl IntoSourceLine for usize {
    fn into_source_line(self) -> Option<SourceLine> {
        let Ok(line) = u32::try_from(self) else {
            return None;
        };
        std::num::NonZeroU32::new(line).map(SourceLine::try_new)
    }
}

impl IntoSourceLine for SourceLine {
    fn into_source_line(self) -> Option<SourceLine> {
        Some(self)
    }
}

impl IntoSourceLine for Option<SourceLine> {
    fn into_source_line(self) -> Option<SourceLine> {
        self
    }
}

pub(crate) fn into_source_line(line: impl IntoSourceLine) -> Option<SourceLine> {
    line.into_source_line()
}

/// Find the 1-based line number of the first line containing `marker`, or
/// `None` if absent.
pub(crate) fn first_line_containing(
    source: ValidationSource<'_>,
    marker: ValidationMarker<'_>,
) -> Option<SourceLine> {
    source
        .as_str()
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(marker.as_str()))
        .and_then(|(idx, _)| idx.saturating_add(1).into_source_line())
}

/// Find the 1-based line number of the first line containing any of
/// `markers`, or `None` if none is present.
pub(crate) fn first_line_containing_any(
    source: ValidationSource<'_>,
    markers: &[ValidationMarker<'_>],
) -> Option<SourceLine> {
    source
        .as_str()
        .lines()
        .enumerate()
        .find(|(_, line)| markers.iter().any(|marker| line.contains(marker.as_str())))
        .and_then(|(idx, _)| idx.saturating_add(1).into_source_line())
}
