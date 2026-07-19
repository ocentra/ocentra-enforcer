//! Common-family prefix `ENF-2` (1 rule).
//! Validator id(s) dispatched per `checks.mjs`: common/mutation-risk.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/enf-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `ENF-2` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "ENF-2.1".parse::<RuleId>()?,
        "Policy-critical mutations require stronger proof".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_2_1_MARKER",
    );
    Ok(v)
}
