//! Conversion from Python source observations into canonical findings.
//!
//! BOUNDARY-INVARIANT: raw Python scanner text and line numbers are validated
//! into canonical finding value types exactly once; invalid observations are
//! rejected instead of leaking partially valid findings.
//! NEGATIVE-TEST: invalid blank finding text is rejected below.

use std::num::NonZeroU32;

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use enforcer_validator::validator::ValidationInput;

pub(crate) struct PythonFindingMessage<'a> {
    title: &'a str,
    detail: String,
    snippet: Option<&'a str>,
}

pub(crate) struct PythonFindingSpec<'a> {
    pub(crate) rule_id: &'a RuleId,
    pub(crate) severity: Severity,
    pub(crate) title: &'a str,
}

pub(crate) fn static_title(value: &'static str) -> Result<FindingTitle, DecodeError> {
    value.parse()
}

impl<'a> PythonFindingMessage<'a> {
    pub(crate) fn new(title: &'a str, detail: impl Into<String>, snippet: Option<&'a str>) -> Self {
        Self {
            title,
            detail: detail.into(),
            snippet,
        }
    }
}

pub(crate) fn finding(
    spec: &PythonFindingSpec<'_>,
    detail: String,
    input: &ValidationInput<'_>,
    line: u32,
) -> Vec<Finding> {
    from_python_source(
        spec.rule_id,
        spec.severity,
        input.file,
        line,
        PythonFindingMessage::new(spec.title, detail, None),
    )
    .into_iter()
    .collect()
}

pub(crate) fn from_python_source(
    rule_id: &RuleId,
    severity: Severity,
    file: &RelPath,
    line: u32,
    message: PythonFindingMessage<'_>,
) -> Option<Finding> {
    let line = if line == 0 {
        FindingLine::Unspecified
    } else {
        FindingLine::known(SourceLine::try_new(NonZeroU32::new(line)?))
    };
    let snippet = match message
        .snippet
        .map(|value| FindingSnippet::new(value.to_owned()))
        .transpose()
    {
        Ok(snippet) => snippet,
        Err(_) => return None,
    };
    Some(Finding {
        rule_id: rule_id.clone(),
        severity,
        title: match FindingTitle::new(message.title.to_owned()) {
            Ok(title) => title,
            Err(_) => return None,
        },
        detail: match FindingDetail::new(message.detail) {
            Ok(detail) => detail,
            Err(_) => return None,
        },
        file: file.clone(),
        line,
        snippet,
    })
}

#[cfg(test)]
mod tests {
    use super::{from_python_source, PythonFindingMessage};
    use enforcer_domain::ids::BuiltInPythonRule;
    use enforcer_domain::paths::RelPath;
    use enforcer_domain::severity::Severity;

    #[test]
    fn blank_boundary_text_is_rejected_before_a_finding_is_emitted(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file: RelPath = "src/example.py".parse()?;
        let finding = from_python_source(
            &BuiltInPythonRule::Py1Rule1.id(),
            Severity::Error,
            &file,
            1,
            PythonFindingMessage::new("", "valid detail", None),
        );
        assert_eq!(finding, None);
        Ok(())
    }
}
