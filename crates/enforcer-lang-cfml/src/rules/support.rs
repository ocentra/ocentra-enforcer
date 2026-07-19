//! Typed, crate-private helpers shared by CFML validators.

use enforcer_domain::boundary::validation::ValidationSource;
use enforcer_domain::ids::{BuiltInCfmlRule, RuleId};
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

/// Fixed canonical identity and severity for one CFML finding.
pub(crate) struct FindingSpec<'a> {
    pub(crate) rule_id: &'a RuleId,
    pub(crate) rule: BuiltInCfmlRule,
    pub(crate) severity: Severity,
}

/// Convert supported line representations into the canonical source-line brand.
pub(crate) trait IntoSourceLine {
    fn into_source_line(self) -> Option<SourceLine>;
}

impl IntoSourceLine for u32 {
    fn into_source_line(self) -> Option<SourceLine> {
        std::num::NonZeroU32::new(self).map(SourceLine::try_new)
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

/// Convert a supported line representation into a canonical source line.
pub(crate) fn into_source_line(line: impl IntoSourceLine) -> Option<SourceLine> {
    line.into_source_line()
}

/// Find the one-based source line of the first matching CFML source marker.
pub(crate) fn first_line_containing(
    source: ValidationSource<'_>,
    marker: ValidationSource<'_>,
) -> Option<SourceLine> {
    source
        .as_str()
        .lines()
        .zip(1u32..)
        .find(|(line, _)| line.contains(marker.as_str()))
        .and_then(|(_, line)| std::num::NonZeroU32::new(line).map(SourceLine::try_new))
}

/// Find the one-based source line of the first matching marker.
pub(crate) fn first_line_containing_any(
    source: ValidationSource<'_>,
    markers: &[ValidationSource<'static>],
) -> Option<SourceLine> {
    source
        .as_str()
        .lines()
        .zip(1u32..)
        .find(|(line, _)| markers.iter().any(|marker| line.contains(marker.as_str())))
        .and_then(|(_, line)| std::num::NonZeroU32::new(line).map(SourceLine::try_new))
}
