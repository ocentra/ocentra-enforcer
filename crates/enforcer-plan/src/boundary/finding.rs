//! Construction boundary for validated findings emitted by plan logic.

use enforcer_domain::findings::{Finding, FindingDetail, FindingLine, FindingTitle};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;

#[derive(Clone, Copy)]
struct FindingContext<'a> {
    rule_id: &'a RuleId,
    severity: Severity,
    file: &'a RelPath,
    line: FindingLine,
}

struct FindingContent {
    title: FindingTitle,
    detail: FindingDetail,
}

fn build_finding(context: FindingContext<'_>, content: FindingContent) -> Finding {
    Finding {
        // CLONE-JUSTIFICATION: the finding owns its rule identity after the validator input is released.
        rule_id: context.rule_id.clone(),
        severity: context.severity,
        title: content.title,
        detail: content.detail,
        // CLONE-JUSTIFICATION: the finding owns its source path after the validator input is released.
        file: context.file.clone(),
        line: context.line,
        snippet: None,
    }
}

pub(crate) fn build_error_finding(
    rule_id: &RuleId,
    title: &str,
    detail: impl Into<String>,
    file: &RelPath,
) -> Finding {
    build_finding(
        FindingContext {
            rule_id,
            severity: Severity::Error,
            file,
            line: known_line(1),
        },
        FindingContent {
            title: validated_title(title),
            detail: validated_detail(detail.into()),
        },
    )
}

pub(crate) fn build_error_finding_at(
    rule_id: &RuleId,
    title: &str,
    detail: impl Into<String>,
    file: &RelPath,
    line: u32,
) -> Finding {
    build_finding(
        FindingContext {
            rule_id,
            severity: Severity::Error,
            file,
            line: known_line(line),
        },
        FindingContent {
            title: validated_title(title),
            detail: validated_detail(detail.into()),
        },
    )
}

pub(crate) fn build_lesson_finding(
    rule_id: &RuleId,
    severity: Severity,
    detail: String,
    file: &RelPath,
) -> Finding {
    build_finding(
        FindingContext {
            rule_id,
            severity,
            file,
            line: known_line(1),
        },
        FindingContent {
            title: validated_title("lesson-capture doctor"),
            detail: validated_detail(detail),
        },
    )
}

fn known_line(line: u32) -> FindingLine {
    std::num::NonZeroU32::new(line)
        .map(SourceLine::try_new)
        .map_or(FindingLine::Unspecified, FindingLine::known)
}

fn validated_title(title: &str) -> FindingTitle {
    let mut candidate = title.to_owned();
    loop {
        match FindingTitle::new(candidate) {
            Ok(value) => return value,
            Err(_) => candidate = "invalid plan finding title".to_owned(),
        }
    }
}

fn validated_detail(detail: String) -> FindingDetail {
    let mut candidate = detail;
    loop {
        match FindingDetail::new(candidate) {
            Ok(value) => return value,
            Err(_) => candidate = "invalid plan finding detail".to_owned(),
        }
    }
}
