//! Common-family prefix `ENF-1` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/rule-coverage, common/report-shape.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/enf-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `ENF-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "ENF-1.1".parse::<RuleId>()?,
        "Rule docs and registry must stay in sync".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_1_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.2".parse::<RuleId>()?,
        "Registry docs must point to stable anchors".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_2_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.3".parse::<RuleId>()?,
        "Scanner-emitted rule IDs must be registered".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_3_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.4".parse::<RuleId>()?,
        "Enforced rules must have fixture evidence".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_4_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.5".parse::<RuleId>()?,
        "Rule IDs must be locked".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_5_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.6".parse::<RuleId>()?,
        "Rule IDs must be unique".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_6_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.7".parse::<RuleId>()?,
        "Rule metadata must not drift".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_7_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.8".parse::<RuleId>()?,
        "Violation reports must be complete".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_8_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.9".parse::<RuleId>()?,
        "JSON output must be deterministic".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_9_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.10".parse::<RuleId>()?,
        "Human output must be deterministic".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_10_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.11".parse::<RuleId>()?,
        "Validators must not use undeclared network access".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_11_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.12".parse::<RuleId>()?,
        "Validator source must be self-scanned".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_12_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.13".parse::<RuleId>()?,
        "Enforcer source cannot carry temporary bypasses".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_13_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.14".parse::<RuleId>()?,
        "Generated JSON schemas must match Effect schemas".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_14_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.15".parse::<RuleId>()?,
        "CLI and MCP behavior must match".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ENF_1_15_MARKER",
    );
    Ok(v)
}
