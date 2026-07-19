//! Common-family prefix `GEN-1` (2 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/generated-artifacts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/gen-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `GEN-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "GEN-1.1".parse::<RuleId>()?,
        "Generated artifacts must not be committed as source".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_1_1_MARKER",
    );
    reg(
        &mut v,
        "GEN-1.2".parse::<RuleId>()?,
        "Generated output folders must not be committed as source".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_1_2_MARKER",
    );
    Ok(v)
}
