//! Common-family prefix `CFG-1` (12 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/config-lockdown.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/cfg-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `CFG-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "CFG-1.1".parse::<RuleId>()?,
        "Strict profiles must fail on errors".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_1_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.2".parse::<RuleId>()?,
        "Immutable rules cannot be disabled".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_2_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.3".parse::<RuleId>()?,
        "Immutable rules cannot be downgraded".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_3_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.4".parse::<RuleId>()?,
        "Unsafe code requires governed waiver".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_4_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.5".parse::<RuleId>()?,
        "Public re-export allow mode is forbidden in strict profiles".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_5_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.6".parse::<RuleId>()?,
        "Build scripts and non-registry dependencies require waiver".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_6_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.7".parse::<RuleId>()?,
        "Boundary glob changes require owner note".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_7_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.8".parse::<RuleId>()?,
        "Rule disable requires expiry".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_8_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.9".parse::<RuleId>()?,
        "Unknown config keys are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_9_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.10".parse::<RuleId>()?,
        "Config precedence must be explicit".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_10_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.11".parse::<RuleId>()?,
        "Profile name must be known".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_11_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.12".parse::<RuleId>()?,
        "Config changes require policy self-check".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CFG_1_12_MARKER",
    );
    Ok(v)
}
