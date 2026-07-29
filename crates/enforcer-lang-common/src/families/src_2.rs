//! Common-family prefix `SRC-2` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: source-shape-check, generic-scanner, common/source-shape.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/src-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `SRC-2` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "SRC-2.1".parse::<RuleId>()?,
        "File line budget must be respected".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_1_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.2".parse::<RuleId>()?,
        "Function line budget must be respected".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_2_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.3".parse::<RuleId>()?,
        "Export count budget must be respected".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_3_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.4".parse::<RuleId>()?,
        "Type count budget must be respected".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_4_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.5".parse::<RuleId>()?,
        "Class/struct count budget must be respected".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_5_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.6".parse::<RuleId>()?,
        "Nesting depth budget must be respected".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_6_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.7".parse::<RuleId>()?,
        "Branch budget must be respected".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_7_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.8".parse::<RuleId>()?,
        "Dumping-ground source filenames are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_8_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.9".parse::<RuleId>()?,
        "Temporary code comments are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_9_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.10".parse::<RuleId>()?,
        "Placeholder implementation markers are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_10_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.11".parse::<RuleId>()?,
        "Copied huge source blocks are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_11_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.12".parse::<RuleId>()?,
        "Duplicate function names in one module are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_12_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.13".parse::<RuleId>()?,
        "Mixed responsibility source files are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_13_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.14".parse::<RuleId>()?,
        "Internal modules cannot expose public API".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_14_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.15".parse::<RuleId>()?,
        "Dependency direction violations are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_SRC_2_15_MARKER",
    );
    Ok(v)
}
