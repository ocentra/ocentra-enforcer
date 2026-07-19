//! Common-family prefix `TEST-1` (3 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/test-doubles, common/weak-assertions, common/skipped-focused-tests.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/test-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `TEST-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "TEST-1.1".parse::<RuleId>()?,
        "Test doubles are forbidden by default".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_TEST_1_1_MARKER",
    );
    reg(
        &mut v,
        "TEST-1.2".parse::<RuleId>()?,
        "Weak assertions are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_TEST_1_2_MARKER",
    );
    reg(
        &mut v,
        "TEST-1.3".parse::<RuleId>()?,
        "Hidden, focused, or ignored tests are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_TEST_1_3_MARKER",
    );
    Ok(v)
}
