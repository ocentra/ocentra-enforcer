//! Convert borrowed TypeScript source observations into owned canonical findings.
//!
//! BOUNDARY-INVARIANT: parse raw analyzer observations and convert immediately into
//! canonical domain findings.
//! boundaryOwnerNote: enforcer-lang-ts owns this finding conversion boundary.
//! Negative invalid-input coverage rejects invalid titles, details, and lines.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use std::num::NonZeroU32;

/// Canonical first source line for findings that describe a whole file.
pub(crate) const FIRST_SOURCE_LINE: SourceLine = SourceLine::try_new(NonZeroU32::MIN);

/// Raw observation captured directly from one TypeScript source location.
pub(crate) struct SourceFinding<'a> {
    pub(crate) severity: Severity,
    pub(crate) title: &'a str,
    pub(crate) detail: String,
    pub(crate) line: SourceLine,
    pub(crate) snippet: Option<&'a str>,
}

/// Validate one raw source-scanner observation and own it at the finding boundary.
pub(crate) fn from_source(
    rule_id: &RuleId,
    file: &RelPath,
    spec: SourceFinding<'_>,
) -> Result<Finding, DecodeError> {
    let snippet = spec
        .snippet
        .map(|value| FindingSnippet::new(value.to_owned()))
        .transpose()?;
    Ok(Finding {
        rule_id: rule_id.clone(),
        severity: spec.severity,
        title: FindingTitle::new(spec.title.to_owned())?,
        detail: FindingDetail::new(spec.detail)?,
        file: file.clone(),
        line: FindingLine::known(spec.line),
        snippet,
    })
}
