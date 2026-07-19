//! Common-family prefix `SCAN-1` (20 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/scanner-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/scan-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `SCAN-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "SCAN-1.1".parse::<RuleId>()?,
        "Scanner must mask string literals where appropriate".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_1_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.2".parse::<RuleId>()?,
        "Scanner must mask comments where appropriate".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_2_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.3".parse::<RuleId>()?,
        "Scanner must still detect suppression comments".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_3_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.4".parse::<RuleId>()?,
        "Scanner must handle CRLF and LF identically".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_4_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.5".parse::<RuleId>()?,
        "Scanner must support Unicode paths".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_5_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.6".parse::<RuleId>()?,
        "Scanner must support spaces in paths".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_6_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.7".parse::<RuleId>()?,
        "Scanner must support Windows drive paths".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_7_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.8".parse::<RuleId>()?,
        "Scanner symlink policy must be explicit".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_8_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.9".parse::<RuleId>()?,
        "Scanner must avoid symlink loops".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_9_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.10".parse::<RuleId>()?,
        "Scanner output must be sorted".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_10_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.11".parse::<RuleId>()?,
        "Scanner must bound file reads".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_11_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.12".parse::<RuleId>()?,
        "Scanner must skip binary files safely".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_12_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.13".parse::<RuleId>()?,
        "Invalid UTF-8 must produce bounded diagnostics".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_13_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.14".parse::<RuleId>()?,
        "Unknown extensions must not trigger false language scans".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_14_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.15".parse::<RuleId>()?,
        "Diff scope must handle deleted and renamed files".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_15_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.16".parse::<RuleId>()?,
        "File scope must not scan the whole repo".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_16_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.17".parse::<RuleId>()?,
        "Workspace scope must scan configured roots".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_17_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.18".parse::<RuleId>()?,
        "Crate/package scope must resolve manifests".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_18_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.19".parse::<RuleId>()?,
        "Scope reports must include included/excluded counts".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_19_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.20".parse::<RuleId>()?,
        "Doctor must expose ignore globs".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SCAN_1_20_MARKER",
    );
    Ok(v)
}
