//! Common-family prefix `CI-1` (21 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/ci-integrity, ci-integrity.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/ci-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `CI-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "CI-1.1".parse::<RuleId>()?,
        "CI must use npm ci".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_1_MARKER",
    );
    reg(
        &mut v,
        "CI-1.2".parse::<RuleId>()?,
        "CI must run npm test".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_2_MARKER",
    );
    reg(
        &mut v,
        "CI-1.3".parse::<RuleId>()?,
        "CI must run rule and policy tests".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_3_MARKER",
    );
    reg(
        &mut v,
        "CI-1.4".parse::<RuleId>()?,
        "CI must run multi-language tests".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_4_MARKER",
    );
    reg(
        &mut v,
        "CI-1.5".parse::<RuleId>()?,
        "CI must run MCP tests".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_5_MARKER",
    );
    reg(
        &mut v,
        "CI-1.6".parse::<RuleId>()?,
        "CI must run Enforcer self-scan".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_6_MARKER",
    );
    reg(
        &mut v,
        "CI-1.7".parse::<RuleId>()?,
        "CI must validate schemas".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_7_MARKER",
    );
    reg(
        &mut v,
        "CI-1.8".parse::<RuleId>()?,
        "CI must run secret scan".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_8_MARKER",
    );
    reg(
        &mut v,
        "CI-1.9".parse::<RuleId>()?,
        "CI must run dependency policy".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_9_MARKER",
    );
    reg(
        &mut v,
        "CI-1.10".parse::<RuleId>()?,
        "CI must run SBOM check".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_10_MARKER",
    );
    reg(
        &mut v,
        "CI-1.11".parse::<RuleId>()?,
        "Hard CI gates cannot continue on error".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_11_MARKER",
    );
    reg(
        &mut v,
        "CI-1.12".parse::<RuleId>()?,
        "CI must not hide failing hard gates".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_12_MARKER",
    );
    reg(
        &mut v,
        "CI-1.13".parse::<RuleId>()?,
        "CI action versions must be pinned".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_13_MARKER",
    );
    reg(
        &mut v,
        "CI-1.14".parse::<RuleId>()?,
        "CI workflows must declare least-privilege permissions".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_14_MARKER",
    );
    reg(
        &mut v,
        "CI-1.15".parse::<RuleId>()?,
        "CI must run on pull requests and main".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_15_MARKER",
    );
    reg(
        &mut v,
        "CI-1.16".parse::<RuleId>()?,
        "CI must cover Linux, Windows, and macOS".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_16_MARKER",
    );
    reg(
        &mut v,
        "CI-1.17".parse::<RuleId>()?,
        "CI workflow must match Enforcer adapter contract".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_17_MARKER",
    );
    reg(
        &mut v,
        "CI-1.18".parse::<RuleId>()?,
        "CI cannot call legacy weaker commands".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_18_MARKER",
    );
    reg(
        &mut v,
        "CI-1.19".parse::<RuleId>()?,
        "Branch protection policy is required".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_19_MARKER",
    );
    reg(
        &mut v,
        "CI-1.20".parse::<RuleId>()?,
        "Required checks must include Enforcer".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_20_MARKER",
    );
    reg(
        &mut v,
        "CI-1.21".parse::<RuleId>()?,
        "Subprocess JSON capture must be CI-safe".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_CI_1_21_MARKER",
    );
    Ok(v)
}
