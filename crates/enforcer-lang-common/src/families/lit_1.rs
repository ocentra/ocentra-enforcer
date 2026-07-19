//! Common-family prefix `LIT-1` (9 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/literal-risk.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/lit-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `LIT-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "LIT-1.1".parse::<RuleId>()?,
        "Low-confidence literals require review".parse::<FindingTitle>()?,
        Severity::Warning,
        "ENFORCER_LIT_1_1_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.2".parse::<RuleId>()?,
        "Event and command-name literals require review".parse::<FindingTitle>()?,
        Severity::Warning,
        "ENFORCER_LIT_1_2_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.3".parse::<RuleId>()?,
        "Route and URL literals require review".parse::<FindingTitle>()?,
        Severity::Warning,
        "ENFORCER_LIT_1_3_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.4".parse::<RuleId>()?,
        "Magic string comparisons require review".parse::<FindingTitle>()?,
        Severity::Warning,
        "ENFORCER_LIT_1_4_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.5".parse::<RuleId>()?,
        "Protocol header and media literals require review".parse::<FindingTitle>()?,
        Severity::Warning,
        "ENFORCER_LIT_1_5_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.6".parse::<RuleId>()?,
        "Raw JSON blob literals require review".parse::<FindingTitle>()?,
        Severity::Warning,
        "ENFORCER_LIT_1_6_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.7".parse::<RuleId>()?,
        "SQL fragment literals require review".parse::<FindingTitle>()?,
        Severity::Warning,
        "ENFORCER_LIT_1_7_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.8".parse::<RuleId>()?,
        "Shell fragment literals require review".parse::<FindingTitle>()?,
        Severity::Warning,
        "ENFORCER_LIT_1_8_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.9".parse::<RuleId>()?,
        "Repeated literals require review".parse::<FindingTitle>()?,
        Severity::Warning,
        "ENFORCER_LIT_1_9_MARKER",
    );
    Ok(v)
}
