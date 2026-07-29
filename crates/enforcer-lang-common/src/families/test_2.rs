//! Common-family prefix `TEST-2` (2 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/required-tests.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/test-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `TEST-2` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "TEST-2.1".parse::<RuleId>()?,
        "Source workspaces must have test scaffolds".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_TEST_2_1_MARKER",
    );
    reg(
        &mut v,
        "TEST-2.2".parse::<RuleId>()?,
        "Tests must live in organized test roots".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_TEST_2_2_MARKER",
    );
    Ok(v)
}
