//! Common-family prefix `HAR-2` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/harness-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/har-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `HAR-2` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "HAR-2.1".parse::<RuleId>()?,
        "Harness runs must identify command lifecycle".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_1_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.2".parse::<RuleId>()?,
        "Raw harness logs must be bounded and redacted".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_2_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.3".parse::<RuleId>()?,
        "Harness diagnostics must be sorted deterministically".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_3_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.4".parse::<RuleId>()?,
        "Harness parsers must not throw on malformed output".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_4_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.5".parse::<RuleId>()?,
        "Cargo JSON diagnostics must normalize".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_5_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.6".parse::<RuleId>()?,
        "ESLint JSON diagnostics must normalize".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_6_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.7".parse::<RuleId>()?,
        "Python tool diagnostics must normalize".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_7_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.8".parse::<RuleId>()?,
        "SARIF diagnostics must normalize".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_8_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.9".parse::<RuleId>()?,
        "Last failure must avoid raw terminal dumps".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_9_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.10".parse::<RuleId>()?,
        "Harness artifacts cannot escape storage".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_10_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.11".parse::<RuleId>()?,
        "Pinned proof runs must survive pruning".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_11_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.12".parse::<RuleId>()?,
        "Failed harness commands must fail process gates".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_12_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.13".parse::<RuleId>()?,
        "Harness JSON output must have schema artifacts".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_13_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.14".parse::<RuleId>()?,
        "Harness human output must redact secrets".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_14_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.15".parse::<RuleId>()?,
        "Harness commands must avoid shell by default".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_HAR_2_15_MARKER",
    );
    Ok(v)
}
