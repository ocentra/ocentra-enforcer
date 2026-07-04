//! Common-family prefix `DEP-1` (2 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/dependency-policy.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/dep-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `DEP-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "DEP-1.1",
        "Dependency security audit must pass",
        Severity::Error,
        "ENFORCER_DEP_1_1_MARKER",
    );
    reg(
        &mut v,
        "DEP-1.2",
        "External npm package licenses must match policy",
        Severity::Error,
        "ENFORCER_DEP_1_2_MARKER",
    );
    v
}
