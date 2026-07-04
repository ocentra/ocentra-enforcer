//! Common-family prefix `ENF-1` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/rule-coverage, common/report-shape.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/enf-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `ENF-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "ENF-1.1",
        "Rule docs and registry must stay in sync",
        Severity::Error,
        "ENFORCER_ENF_1_1_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.2",
        "Registry docs must point to stable anchors",
        Severity::Error,
        "ENFORCER_ENF_1_2_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.3",
        "Scanner-emitted rule IDs must be registered",
        Severity::Error,
        "ENFORCER_ENF_1_3_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.4",
        "Enforced rules must have fixture evidence",
        Severity::Error,
        "ENFORCER_ENF_1_4_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.5",
        "Rule IDs must be locked",
        Severity::Error,
        "ENFORCER_ENF_1_5_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.6",
        "Rule IDs must be unique",
        Severity::Error,
        "ENFORCER_ENF_1_6_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.7",
        "Rule metadata must not drift",
        Severity::Error,
        "ENFORCER_ENF_1_7_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.8",
        "Violation reports must be complete",
        Severity::Error,
        "ENFORCER_ENF_1_8_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.9",
        "JSON output must be deterministic",
        Severity::Error,
        "ENFORCER_ENF_1_9_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.10",
        "Human output must be deterministic",
        Severity::Error,
        "ENFORCER_ENF_1_10_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.11",
        "Validators must not use undeclared network access",
        Severity::Error,
        "ENFORCER_ENF_1_11_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.12",
        "Validator source must be self-scanned",
        Severity::Error,
        "ENFORCER_ENF_1_12_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.13",
        "Enforcer source cannot carry temporary bypasses",
        Severity::Error,
        "ENFORCER_ENF_1_13_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.14",
        "Generated JSON schemas must match Effect schemas",
        Severity::Error,
        "ENFORCER_ENF_1_14_MARKER",
    );
    reg(
        &mut v,
        "ENF-1.15",
        "CLI and MCP behavior must match",
        Severity::Error,
        "ENFORCER_ENF_1_15_MARKER",
    );
    v
}
