//! Common-family prefix `DOCENF-1` (10 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/docs-completeness.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/docenf-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `DOCENF-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "DOCENF-1.1".parse::<RuleId>()?,
        "Rule docs must include required teaching sections".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DOCENF_1_1_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.2".parse::<RuleId>()?,
        "Source rule docs must include fail and pass code blocks".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DOCENF_1_2_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.3".parse::<RuleId>()?,
        "Tagged rule doc code blocks must stay parseable".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DOCENF_1_3_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.4".parse::<RuleId>()?,
        "Fix snippets must stay compact".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DOCENF_1_4_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.5".parse::<RuleId>()?,
        "Registry doc anchors must be stable lowercase anchors".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DOCENF_1_5_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.6".parse::<RuleId>()?,
        "Immutable rule docs must use mandatory language".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DOCENF_1_6_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.7".parse::<RuleId>()?,
        "Docs must not make legacy aliases canonical".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DOCENF_1_7_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.8".parse::<RuleId>()?,
        "Docs cannot describe the pack as Rust-only".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DOCENF_1_8_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.9".parse::<RuleId>()?,
        "Advisory rule docs must explain promotion".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DOCENF_1_9_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.10".parse::<RuleId>()?,
        "Review and proof rules must name proof evidence".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_DOCENF_1_10_MARKER",
    );
    Ok(v)
}
