//! Common-family prefix `TEST-2` (2 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/required-tests.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/test-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `TEST-2` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "TEST-2.1",
        "Source workspaces must have test scaffolds",
        Severity::Error,
        "ENFORCER_TEST_2_1_MARKER",
    );
    reg(
        &mut v,
        "TEST-2.2",
        "Tests must live in organized test roots",
        Severity::Error,
        "ENFORCER_TEST_2_2_MARKER",
    );
    v
}
