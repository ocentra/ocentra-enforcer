//! Common-family prefix `DOC-1` (1 rule).
//! Validator id(s) dispatched per `checks.mjs`: common/documentation.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/doc-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `DOC-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "DOC-1.1",
        "Public API documentation is recommended",
        Severity::Warning,
        "ENFORCER_DOC_1_1_MARKER",
    );
    v
}
