//! Common-family prefix `SEC-1` (2 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/secret-scan.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/sec-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `SEC-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "SEC-1.1".parse::<RuleId>()?,
        "Inline secrets are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SEC_1_1_MARKER",
    );
    reg(
        &mut v,
        "SEC-1.2".parse::<RuleId>()?,
        "Sensitive files are forbidden in source scope".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SEC_1_2_MARKER",
    );
    Ok(v)
}
