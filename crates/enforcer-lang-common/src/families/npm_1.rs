//! Common-family prefix `NPM-1` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/package-determinism, package-determinism, dependency-policy, sbom.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/npm-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `NPM-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "NPM-1.1".parse::<RuleId>()?,
        "package-lock.json is required".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_1_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.2".parse::<RuleId>()?,
        "npm ci is required in CI".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_2_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.3".parse::<RuleId>()?,
        "Enforcer dependencies must be pinned".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_3_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.4".parse::<RuleId>()?,
        "packageManager must pin npm".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_4_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.5".parse::<RuleId>()?,
        "Node engine must be bounded".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_5_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.6".parse::<RuleId>()?,
        "Dependency install scripts require approval".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_6_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.7".parse::<RuleId>()?,
        "Git dependencies are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_7_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.8".parse::<RuleId>()?,
        "File and path dependencies are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_8_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.9".parse::<RuleId>()?,
        "npm audit high and critical findings must fail".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_9_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.10".parse::<RuleId>()?,
        "Dependency licenses must match policy".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_10_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.11".parse::<RuleId>()?,
        "Suspicious dependency names are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_11_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.12".parse::<RuleId>()?,
        "SBOM must be generated for release".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_12_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.13".parse::<RuleId>()?,
        "Published package files must be explicit".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_13_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.14".parse::<RuleId>()?,
        "Package bin paths must exist".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_14_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.15".parse::<RuleId>()?,
        "Package export paths must exist".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_NPM_1_15_MARKER",
    );
    Ok(v)
}
