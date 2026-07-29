//! Common-family prefix `DEP-1` (2 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/dependency-policy.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/dep-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `DEP-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "DEP-1.1".parse::<RuleId>()?,
        "Dependency security audit must pass".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DEP_1_1_MARKER",
    );
    reg(
        &mut v,
        "DEP-1.2".parse::<RuleId>()?,
        "External npm package licenses must match policy".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DEP_1_2_MARKER",
    );
    Ok(v)
}
