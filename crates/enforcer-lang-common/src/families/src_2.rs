//! Common-family prefix `SRC-2` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: source-shape-check, generic-scanner, common/source-shape.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/src-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `SRC-2` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "SRC-2.1",
        "File line budget must be respected",
        Severity::Error,
        "ENFORCER_SRC_2_1_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.2",
        "Function line budget must be respected",
        Severity::Error,
        "ENFORCER_SRC_2_2_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.3",
        "Export count budget must be respected",
        Severity::Error,
        "ENFORCER_SRC_2_3_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.4",
        "Type count budget must be respected",
        Severity::Error,
        "ENFORCER_SRC_2_4_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.5",
        "Class/struct count budget must be respected",
        Severity::Error,
        "ENFORCER_SRC_2_5_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.6",
        "Nesting depth budget must be respected",
        Severity::Error,
        "ENFORCER_SRC_2_6_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.7",
        "Branch budget must be respected",
        Severity::Error,
        "ENFORCER_SRC_2_7_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.8",
        "Dumping-ground source filenames are forbidden",
        Severity::Error,
        "ENFORCER_SRC_2_8_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.9",
        "Temporary code comments are forbidden",
        Severity::Error,
        "ENFORCER_SRC_2_9_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.10",
        "Placeholder implementation markers are forbidden",
        Severity::Error,
        "ENFORCER_SRC_2_10_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.11",
        "Copied huge source blocks are forbidden",
        Severity::Error,
        "ENFORCER_SRC_2_11_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.12",
        "Duplicate function names in one module are forbidden",
        Severity::Error,
        "ENFORCER_SRC_2_12_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.13",
        "Mixed responsibility source files are forbidden",
        Severity::Error,
        "ENFORCER_SRC_2_13_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.14",
        "Internal modules cannot expose public API",
        Severity::Error,
        "ENFORCER_SRC_2_14_MARKER",
    );
    reg(
        &mut v,
        "SRC-2.15",
        "Dependency direction violations are forbidden",
        Severity::Error,
        "ENFORCER_SRC_2_15_MARKER",
    );
    v
}
