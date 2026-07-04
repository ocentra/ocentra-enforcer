//! Common-family prefix `CONTRACT-1` (1 rule).
//! Validator id(s) dispatched per `checks.mjs`: common/single-source-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/contract-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `CONTRACT-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "CONTRACT-1.1",
        "Single-source contract values must not be copied",
        Severity::Error,
        "ENFORCER_CONTRACT_1_1_MARKER",
    );
    v
}
