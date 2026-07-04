//! Common-family prefix `WAIVER-1` (10 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/waiver-policy.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/waiver-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `WAIVER-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "WAIVER-1.1",
        "Waivers must include required metadata",
        Severity::Error,
        "ENFORCER_WAIVER_1_1_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.2",
        "Waiver scope must be narrow",
        Severity::Error,
        "ENFORCER_WAIVER_1_2_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.3",
        "Expired waivers fail",
        Severity::Error,
        "ENFORCER_WAIVER_1_3_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.4",
        "Immutable rules cannot be waived unless marked waivable",
        Severity::Error,
        "ENFORCER_WAIVER_1_4_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.5",
        "CI waiver behavior must be explicit",
        Severity::Error,
        "ENFORCER_WAIVER_1_5_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.6",
        "Waivers must remain visible in output",
        Severity::Error,
        "ENFORCER_WAIVER_1_6_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.7",
        "Active waiver count is budgeted",
        Severity::Error,
        "ENFORCER_WAIVER_1_7_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.8",
        "Permanent waiver grandfathering is forbidden",
        Severity::Error,
        "ENFORCER_WAIVER_1_8_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.9",
        "Waiver owner must be a human or team",
        Severity::Error,
        "ENFORCER_WAIVER_1_9_MARKER",
    );
    reg(
        &mut v,
        "WAIVER-1.10",
        "Waivers require remediation plans",
        Severity::Error,
        "ENFORCER_WAIVER_1_10_MARKER",
    );
    v
}
