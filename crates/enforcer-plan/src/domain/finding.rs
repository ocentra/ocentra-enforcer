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
    title: FindingTitle,
    detail: FindingDetail,
    file: &RelPath,
    line: FindingLine,
) -> Finding {
    build_finding(
        FindingContext {
            rule_id,
            severity: Severity::Error,
            file,
            line,
        },
        FindingContent { title, detail },
    )
}

pub(crate) fn build_error_finding_at(
    rule_id: &RuleId,
    title: FindingTitle,
    detail: FindingDetail,
    file: &RelPath,
    line: FindingLine,
) -> Finding {
    build_finding(
        FindingContext {
            rule_id,
            severity: Severity::Error,
            file,
            line,
        },
        FindingContent { title, detail },
    )
}

pub(crate) fn build_lesson_finding(
    rule_id: &RuleId,
    severity: Severity,
    title: FindingTitle,
    detail: FindingDetail,
    file: &RelPath,
) -> Finding {
    build_finding(
        FindingContext {
            rule_id,
            severity,
            file,
            line: default_line(),
        },
        FindingContent { title, detail },
    )
}

fn default_line() -> FindingLine {
    match std::num::NonZeroU32::new(1) {
        Some(line) => FindingLine::Known(SourceLine::try_new(line)),
        None => FindingLine::Unspecified,
    }
}
