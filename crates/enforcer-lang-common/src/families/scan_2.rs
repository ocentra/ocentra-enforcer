//! Common-family prefix `SCAN-2` (10 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/scanner-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/scan-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `SCAN-2` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "SCAN-2.1".parse::<RuleId>()?,
        "Rust strict mode must use cargo metadata".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_2_1_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.2".parse::<RuleId>()?,
        "Rust strict mode must use parser-backed checks".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_2_2_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.3".parse::<RuleId>()?,
        "Rust strict mode must ingest Clippy JSON".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_2_3_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.4".parse::<RuleId>()?,
        "Rust strict mode must ingest rustdoc warnings".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_2_4_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.5".parse::<RuleId>()?,
        "TypeScript strict mode must use compiler or ESLint JSON".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_2_5_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.6".parse::<RuleId>()?,
        "Python strict mode must ingest Ruff JSON".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_2_6_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.7".parse::<RuleId>()?,
        "Python strict mode must ingest Pyright or mypy output".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_2_7_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.8".parse::<RuleId>()?,
        "Security strict mode must ingest SARIF".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_2_8_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.9".parse::<RuleId>()?,
        "Regex scanner must remain fast preflight".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_2_9_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.10".parse::<RuleId>()?,
        "Native and regex reports must merge without duplicate spam".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_2_10_MARKER",
    );
    Ok(v)
}
