//! Common-family prefix `LIT-1` (9 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/literal-risk.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/lit-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `LIT-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "LIT-1.1",
        "Low-confidence literals require review",
        Severity::Warning,
        "ENFORCER_LIT_1_1_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.2",
        "Event and command-name literals require review",
        Severity::Warning,
        "ENFORCER_LIT_1_2_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.3",
        "Route and URL literals require review",
        Severity::Warning,
        "ENFORCER_LIT_1_3_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.4",
        "Magic string comparisons require review",
        Severity::Warning,
        "ENFORCER_LIT_1_4_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.5",
        "Protocol header and media literals require review",
        Severity::Warning,
        "ENFORCER_LIT_1_5_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.6",
        "Raw JSON blob literals require review",
        Severity::Warning,
        "ENFORCER_LIT_1_6_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.7",
        "SQL fragment literals require review",
        Severity::Warning,
        "ENFORCER_LIT_1_7_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.8",
        "Shell fragment literals require review",
        Severity::Warning,
        "ENFORCER_LIT_1_8_MARKER",
    );
    reg(
        &mut v,
        "LIT-1.9",
        "Repeated literals require review",
        Severity::Warning,
        "ENFORCER_LIT_1_9_MARKER",
    );
    v
}
