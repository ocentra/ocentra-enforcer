//! Common-family prefix `SRC-1` (2 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/source-shape, common/source-scan.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/src-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `SRC-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "SRC-1.1",
        "Source files must stay within shape limits",
        Severity::Error,
        "ENFORCER_SRC_1_1_MARKER",
    );
    reg(
        &mut v,
        "SRC-1.2",
        "Placeholder implementation markers are forbidden",
        Severity::Error,
        "ENFORCER_SRC_1_2_MARKER",
    );
    v
}
