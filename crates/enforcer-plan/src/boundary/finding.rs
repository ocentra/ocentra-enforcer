//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
use crate::boundary::values::{finding_detail, finding_title};
use enforcer_domain::findings::{Finding, FindingLine};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

pub(crate) fn build_error_finding(
    rule_id: &RuleId,
    title: &str,
    detail: impl Into<String>,
    file: &RelPath,
) -> Finding {
    crate::domain::finding::build_error_finding(
        rule_id,
        finding_title(title.to_owned()),
        // ALLOC-JUSTIFICATION: `findings` detail is caller-owned output text
        // and must remain owned before it is passed through domain validation.
        finding_detail(detail.into()),
        file,
        line_to_finding_line(1),
    )
}

pub(crate) fn build_error_finding_at(
    rule_id: &RuleId,
    title: &str,
    detail: impl Into<String>,
    file: &RelPath,
    line: u32,
) -> Finding {
    crate::domain::finding::build_error_finding_at(
        rule_id,
        finding_title(title.to_owned()),
        finding_detail(detail.into()),
        file,
        line_to_finding_line(line),
    )
}

pub(crate) fn build_lesson_finding(
    rule_id: &RuleId,
    severity: Severity,
    detail: impl Into<String>,
    file: &RelPath,
) -> Finding {
    crate::domain::finding::build_lesson_finding(
        rule_id,
        severity,
        finding_title("lesson-capture doctor".to_owned()),
        finding_detail(detail.into()),
        file,
    )
}

fn line_to_finding_line(line: u32) -> FindingLine {
    match std::num::NonZeroU32::new(line) {
        Some(line) => FindingLine::Known(SourceLine::try_new(line)),
        None => FindingLine::Unspecified,
    }
}
