//! Common-family prefix `GEN-2` (10 rules).
//! Validator id(s) dispatched per `checks.mjs`: generic-scanner, common/generated-artifacts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/gen-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `GEN-2` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "GEN-2.1".parse::<RuleId>()?,
        "Generated directories require ignore policy".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_2_1_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.2".parse::<RuleId>()?,
        "Generated files require source owner provenance".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_2_2_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.3".parse::<RuleId>()?,
        "Generated files cannot be edited manually".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_2_3_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.4".parse::<RuleId>()?,
        "Generated contract artifacts require source hash".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_2_4_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.5".parse::<RuleId>()?,
        "Generated schema files must be reproducible".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_2_5_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.6".parse::<RuleId>()?,
        "Runtime output directories cannot be tracked".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_2_6_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.7".parse::<RuleId>()?,
        "Generated files cannot be single source of truth".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_2_7_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.8".parse::<RuleId>()?,
        "Generated code cannot contain suppressions".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_2_8_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.9".parse::<RuleId>()?,
        "Generated code cannot live in domain modules".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_2_9_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.10".parse::<RuleId>()?,
        "Generated snapshots must be stable".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_GEN_2_10_MARKER",
    );
    Ok(v)
}
