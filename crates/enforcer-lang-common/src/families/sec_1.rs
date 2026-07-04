//! Common-family prefix `SEC-1` (2 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/secret-scan.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/sec-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `SEC-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "SEC-1.1",
        "Inline secrets are forbidden",
        Severity::Error,
        "ENFORCER_SEC_1_1_MARKER",
    );
    reg(
        &mut v,
        "SEC-1.2",
        "Sensitive files are forbidden in source scope",
        Severity::Error,
        "ENFORCER_SEC_1_2_MARKER",
    );
    v
}
