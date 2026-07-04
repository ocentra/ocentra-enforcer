//! Common-family prefix `SBOM-1` (1 rule).
//! Validator id(s) dispatched per `checks.mjs`: common/sbom.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/sbom-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `SBOM-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "SBOM-1.1",
        "SBOM generation must complete",
        Severity::Error,
        "ENFORCER_SBOM_1_1_MARKER",
    );
    v
}
