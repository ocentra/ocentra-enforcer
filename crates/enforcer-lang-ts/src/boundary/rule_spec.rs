//! Static rule-table spellings before decoding into canonical rule specs.
//!
//! BOUNDARY-INVARIANT: raw table spellings convert to canonical rule IDs and
//! finding titles before registry construction.
//! boundaryOwnerNote: enforcer-lang-ts owns its embedded rule-spec boundary.
//! Negative invalid-input coverage is exercised by rule-spec construction
//! tests that reject malformed identifiers and titles.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;

/// Decode a compile-time rule identifier at the static table boundary.
pub(crate) fn decode_rule_id(raw: &'static str) -> Result<RuleId, DecodeError> {
    raw.parse()
}

/// Decode a compile-time finding title at the static table boundary.
pub(crate) fn decode_finding_title(raw: &'static str) -> Result<FindingTitle, DecodeError> {
    raw.parse()
}

#[derive(Debug, Clone, Copy)]
/// How a static rule-table row recognizes its forbidden pattern.
pub(crate) enum TriggerKind {
    Word,
    Literal,
    NonNullAssertion,
    ExportedFunctionReturnType,
}

#[derive(Debug, Clone, Copy)]
/// Raw compile-time row at the embedded rule-table boundary.
pub(crate) struct RawRuleSpec {
    pub(crate) rule_id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) kind: TriggerKind,
    pub(crate) needles: &'static [&'static str],
    pub(crate) comment_guard: bool,
}
