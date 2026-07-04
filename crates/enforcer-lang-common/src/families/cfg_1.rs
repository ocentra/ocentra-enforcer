//! Common-family prefix `CFG-1` (12 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/config-lockdown.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/cfg-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `CFG-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "CFG-1.1",
        "Strict profiles must fail on errors",
        Severity::Error,
        "ENFORCER_CFG_1_1_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.2",
        "Immutable rules cannot be disabled",
        Severity::Error,
        "ENFORCER_CFG_1_2_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.3",
        "Immutable rules cannot be downgraded",
        Severity::Error,
        "ENFORCER_CFG_1_3_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.4",
        "Unsafe code requires governed waiver",
        Severity::Error,
        "ENFORCER_CFG_1_4_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.5",
        "Public re-export allow mode is forbidden in strict profiles",
        Severity::Error,
        "ENFORCER_CFG_1_5_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.6",
        "Build scripts and non-registry dependencies require waiver",
        Severity::Error,
        "ENFORCER_CFG_1_6_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.7",
        "Boundary glob changes require owner note",
        Severity::Error,
        "ENFORCER_CFG_1_7_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.8",
        "Rule disable requires expiry",
        Severity::Error,
        "ENFORCER_CFG_1_8_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.9",
        "Unknown config keys are forbidden",
        Severity::Error,
        "ENFORCER_CFG_1_9_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.10",
        "Config precedence must be explicit",
        Severity::Error,
        "ENFORCER_CFG_1_10_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.11",
        "Profile name must be known",
        Severity::Error,
        "ENFORCER_CFG_1_11_MARKER",
    );
    reg(
        &mut v,
        "CFG-1.12",
        "Config changes require policy self-check",
        Severity::Error,
        "ENFORCER_CFG_1_12_MARKER",
    );
    v
}
