//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Conversion from raw Rust source-parser observations into canonical findings.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::{Finding, FindingDetail, FindingLine, FindingTitle};
use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::severity::Severity;
use enforcer_domain::telemetry_types::SourceLine;
use std::num::NonZeroU32;
use syn::spanned::Spanned;

/// Convert a syntax-tree span into a canonical positive source line.
pub(crate) fn source_line<S: Spanned>(spanned: &S) -> SourceLine {
    let raw = u32::try_from(spanned.span().start().line.max(1)).unwrap_or(u32::MAX);
    SourceLine::try_new(NonZeroU32::new(raw).unwrap_or(NonZeroU32::MIN))
}

/// Canonical first line for file-level findings without a narrower span.
pub(crate) const fn first_source_line() -> SourceLine {
    SourceLine::try_new(NonZeroU32::MIN)
}

/// Decode one source-parser observation into the canonical finding model.
pub(crate) fn from_source(
    rule: (&RuleId, Severity),
    title: impl Into<String>,
    detail: impl Into<String>,
    file: &RelPath,
    line: SourceLine,
) -> Result<Finding, DecodeError> {
    let (rule_id, severity) = rule;
    Ok(Finding {
        rule_id: rule_id.clone(),
        severity,
        title: FindingTitle::new(title.into())?,
        detail: FindingDetail::new(detail.into())?,
        file: file.clone(),
        line: FindingLine::known(line),
        snippet: None,
    })
}
