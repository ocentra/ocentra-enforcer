//! Common-family prefix `WAIVER-1` (10 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/waiver-policy.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/waiver-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `WAIVER-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "WAIVER-1.1".parse::<RuleId>()?,
        "Waivers must include required metadata".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_WAIVER_1_1_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.2".parse::<RuleId>()?,
        "Waiver scope must be narrow".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_WAIVER_1_2_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.3".parse::<RuleId>()?,
        "Expired waivers fail".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_WAIVER_1_3_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.4".parse::<RuleId>()?,
        "Immutable rules cannot be waived unless marked waivable".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_WAIVER_1_4_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.5".parse::<RuleId>()?,
        "CI waiver behavior must be explicit".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_WAIVER_1_5_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.6".parse::<RuleId>()?,
        "Waivers must remain visible in output".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_WAIVER_1_6_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.7".parse::<RuleId>()?,
        "Active waiver count is budgeted".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_WAIVER_1_7_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.8".parse::<RuleId>()?,
        "Permanent waiver grandfathering is forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_WAIVER_1_8_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.9".parse::<RuleId>()?,
        "Waiver owner must be a human or team".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_WAIVER_1_9_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.10".parse::<RuleId>()?,
        "Waivers require remediation plans".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_WAIVER_1_10_MARKER",
    );
    Ok(v)
}
