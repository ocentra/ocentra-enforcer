//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Conversion from security-rule observations into canonical findings.

use enforcer_domain::findings::{
    Finding, FindingDetail, FindingLine, FindingSnippet, FindingTitle,
};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use std::num::NonZeroU32;

/// Decode one source observation into the canonical finding model.
pub(crate) fn from_source(
    identity: (&RuleId, Severity),
    title: impl Into<String>,
    detail: impl Into<String>,
    file: &RelPath,
    source: (u32, Option<&str>),
) -> Option<Finding> {
    let (rule_id, severity) = identity;
    let (line, snippet) = source;
    let line = if line == 0 {
        FindingLine::Unspecified
    } else {
        FindingLine::known(SourceLine::try_new(NonZeroU32::new(line)?))
    };
    Some(Finding {
        rule_id: rule_id.clone(),
        severity,
        title: FindingTitle::new(title.into()).ok()?,
        detail: FindingDetail::new(detail.into()).ok()?,
        file: file.clone(),
        line,
        snippet: snippet
            .map(|value| {
                // ALLOC-JUSTIFICATION: a finding retains its redacted snippet after source scanning returns.
                FindingSnippet::new(value.to_owned())
            })
            .transpose()
            .ok()?,
    })
}
