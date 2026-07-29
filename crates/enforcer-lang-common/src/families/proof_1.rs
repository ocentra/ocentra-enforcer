//! Common-family prefix `PROOF-1` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/proof-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/proof-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `PROOF-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "PROOF-1.1".parse::<RuleId>()?,
        "PR-ready claims require fresh proof".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_1_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.2".parse::<RuleId>()?,
        "Proof freshness must bind commit and scope".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_2_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.3".parse::<RuleId>()?,
        "Manual-required proof cannot auto-pass".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_3_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.4".parse::<RuleId>()?,
        "Required proof artifacts must exist".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_4_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.5".parse::<RuleId>()?,
        "Proof artifacts must hash-match".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_5_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.6".parse::<RuleId>()?,
        "Dirty worktrees invalidate PR-ready proof".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_6_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.7".parse::<RuleId>()?,
        "Waived proof must remain visible".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_7_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.8".parse::<RuleId>()?,
        "Proof command cannot be empty".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_8_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.9".parse::<RuleId>()?,
        "Proof command cannot be a shell string".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_9_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.10".parse::<RuleId>()?,
        "Proof registry docs paths must exist".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_10_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.11".parse::<RuleId>()?,
        "Proof capabilities must match environment".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_11_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.12".parse::<RuleId>()?,
        "Device proof cannot auto-pass on desktop".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_12_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.13".parse::<RuleId>()?,
        "Proof claims must list proved and unproved claims".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_13_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.14".parse::<RuleId>()?,
        "Proof output must be compact by default".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_14_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.15".parse::<RuleId>()?,
        "Proof exports must redact secrets".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_PROOF_1_15_MARKER",
    );
    Ok(v)
}
