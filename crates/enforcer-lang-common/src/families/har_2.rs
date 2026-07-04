//! Common-family prefix `HAR-2` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/harness-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/har-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `HAR-2` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "HAR-2.1",
        "Harness runs must identify command lifecycle",
        Severity::Error,
        "ENFORCER_HAR_2_1_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.2",
        "Raw harness logs must be bounded and redacted",
        Severity::Error,
        "ENFORCER_HAR_2_2_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.3",
        "Harness diagnostics must be sorted deterministically",
        Severity::Error,
        "ENFORCER_HAR_2_3_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.4",
        "Harness parsers must not throw on malformed output",
        Severity::Error,
        "ENFORCER_HAR_2_4_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.5",
        "Cargo JSON diagnostics must normalize",
        Severity::Error,
        "ENFORCER_HAR_2_5_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.6",
        "ESLint JSON diagnostics must normalize",
        Severity::Error,
        "ENFORCER_HAR_2_6_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.7",
        "Python tool diagnostics must normalize",
        Severity::Error,
        "ENFORCER_HAR_2_7_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.8",
        "SARIF diagnostics must normalize",
        Severity::Error,
        "ENFORCER_HAR_2_8_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.9",
        "Last failure must avoid raw terminal dumps",
        Severity::Error,
        "ENFORCER_HAR_2_9_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.10",
        "Harness artifacts cannot escape storage",
        Severity::Error,
        "ENFORCER_HAR_2_10_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.11",
        "Pinned proof runs must survive pruning",
        Severity::Error,
        "ENFORCER_HAR_2_11_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.12",
        "Failed harness commands must fail process gates",
        Severity::Error,
        "ENFORCER_HAR_2_12_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.13",
        "Harness JSON output must have schema artifacts",
        Severity::Error,
        "ENFORCER_HAR_2_13_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.14",
        "Harness human output must redact secrets",
        Severity::Error,
        "ENFORCER_HAR_2_14_MARKER",
    );
    reg(
        &mut v,
        "HAR-2.15",
        "Harness commands must avoid shell by default",
        Severity::Error,
        "ENFORCER_HAR_2_15_MARKER",
    );
    v
}
