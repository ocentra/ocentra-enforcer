//! Common-family prefix `HAR-1` (1 rule).
//! Validator id(s) dispatched per `checks.mjs`: harness/run-capture.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/har-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `HAR-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "HAR-1.1",
        "Harnessed command failed",
        Severity::Error,
        "ENFORCER_HAR_1_1_MARKER",
    );
    v
}
