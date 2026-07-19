//! Common-family prefix `REPO-1` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/repo-governance, repo-governance.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/repo-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `REPO-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "REPO-1.1".parse::<RuleId>()?,
        "CODEOWNERS is required".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_1_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.2".parse::<RuleId>()?,
        "CODEOWNERS must protect rules".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_2_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.3".parse::<RuleId>()?,
        "CODEOWNERS must protect enforcement source".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_3_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.4".parse::<RuleId>()?,
        "CODEOWNERS must protect schemas and adapters".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_4_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.5".parse::<RuleId>()?,
        "CODEOWNERS must protect workflows".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_5_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.6".parse::<RuleId>()?,
        "Package lockfile is required".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_6_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.7".parse::<RuleId>()?,
        "packageManager is required".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_7_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.8".parse::<RuleId>()?,
        "Node version policy must be bounded".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_8_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.9".parse::<RuleId>()?,
        "Dependency versions must be deterministic".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_9_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.10".parse::<RuleId>()?,
        "License file is required".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_10_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.11".parse::<RuleId>()?,
        "Security policy is required".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_11_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.12".parse::<RuleId>()?,
        "Contributing guide must explain rule changes".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_12_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.13".parse::<RuleId>()?,
        "Changelog is required for rule behavior changes".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_13_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.14".parse::<RuleId>()?,
        "Release policy must be documented".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_14_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.15".parse::<RuleId>()?,
        "Generated schema artifacts must not drift".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_REPO_1_15_MARKER",
    );
    Ok(v)
}
