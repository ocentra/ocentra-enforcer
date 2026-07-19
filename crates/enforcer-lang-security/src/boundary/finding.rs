//! Conversion from security source observations into canonical findings.

use std::num::NonZeroU32;

use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_validator::validator::ValidationInput;

pub(crate) struct ValidationFindingFactory<'a> {
    rule_id: &'a RuleId,
    title: FindingTitle,
}

impl<'a> ValidationFindingFactory<'a> {
    pub(crate) fn new(rule_id: &'a RuleId, title: &'static str) -> Option<Self> {
        Some(Self {
            rule_id,
            title: FindingTitle::new(String::from(title)).ok()?,
        })
    }

    pub(crate) fn finding(
        &self,
        input: &ValidationInput<'_>,
        line: u32,
        severity: Severity,
        detail: impl Into<String>,
    ) -> Option<Finding> {
        from_owned_source(
            (self.rule_id, severity),
            self.title.as_str(),
            detail,
            input.file,
            (line, None),
        )
    }

    pub(crate) fn at_start(
        &self,
        input: &ValidationInput<'_>,
        severity: Severity,
        detail: impl Into<String>,
    ) -> Option<Finding> {
        self.finding(input, 1, severity, detail)
    }
}

pub(crate) struct DynamicFindingFactory<'a> {
    rule_id: &'a RuleId,
}

impl<'a> DynamicFindingFactory<'a> {
    pub(crate) const fn new(rule_id: &'a RuleId) -> Self {
        Self { rule_id }
    }

    pub(crate) fn finding(
        &self,
        file: &RelPath,
        severity: Severity,
        title: &'static str,
        detail: impl Into<String>,
    ) -> Option<Finding> {
        from_owned_source((self.rule_id, severity), title, detail, file, (1, None))
    }
}

pub(crate) fn from_source(
    identity: (&RuleId, Severity),
    title: impl Into<String>,
    detail: impl Into<String>,
    file: &RelPath,
    source: (u32, Option<&str>),
) -> Option<Finding> {
    let (rule_id, severity) = identity;
    let (line, snippet) = source;
    from_owned_source(
        (rule_id, severity),
        title,
        detail,
        file,
        (line, snippet.map(str::to_owned)),
    )
}

pub(crate) fn from_owned_source(
    identity: (&RuleId, Severity),
    title: impl Into<String>,
    detail: impl Into<String>,
    file: &RelPath,
    source: (u32, Option<String>),
) -> Option<Finding> {
    let (rule_id, severity) = identity;
    let (line, snippet) = source;
    let line = if line == 0 {
        FindingLine::Unspecified
    } else {
        let line = NonZeroU32::new(line)?;
        FindingLine::known(SourceLine::try_new(line))
    };
    Some(Finding {
        rule_id: rule_id.clone(),
        severity,
        title: match FindingTitle::new(title.into()) {
            Ok(title) => title,
            Err(_) => return None,
        },
        detail: match FindingDetail::new(detail.into()) {
            Ok(detail) => detail,
            Err(_) => return None,
        },
        file: file.clone(),
        line,
        snippet: match snippet.map(FindingSnippet::new).transpose() {
            Ok(snippet) => snippet,
            Err(_) => return None,
        },
    })
}

/// Convert a validator observation into a canonical finding.
pub(crate) fn from_validation(
    rule: (&RuleId, Severity),
    title: &'static str,
    detail: String,
    input: &ValidationInput<'_>,
    line: u32,
) -> Option<Finding> {
    from_owned_source(rule, title, detail, input.file, (line, None))
}
