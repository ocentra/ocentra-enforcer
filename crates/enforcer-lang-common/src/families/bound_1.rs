//! Common-family prefix `BOUND-1` (10 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/architecture.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/bound-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `BOUND-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "BOUND-1.1",
        "Boundary modules require invariant documentation",
        Severity::Error,
        "ENFORCER_BOUND_1_1_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.2",
        "Raw boundary input must be converted",
        Severity::Error,
        "ENFORCER_BOUND_1_2_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.3",
        "Boundary modules cannot contain domain decisions",
        Severity::Error,
        "ENFORCER_BOUND_1_3_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.4",
        "Domain modules cannot import boundary modules",
        Severity::Error,
        "ENFORCER_BOUND_1_4_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.5",
        "Boundary modules require negative tests",
        Severity::Error,
        "ENFORCER_BOUND_1_5_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.6",
        "Boundary raw type count is budgeted",
        Severity::Error,
        "ENFORCER_BOUND_1_6_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.7",
        "Boundary glob additions require waiver",
        Severity::Error,
        "ENFORCER_BOUND_1_7_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.8",
        "Boundary utility filenames are forbidden",
        Severity::Error,
        "ENFORCER_BOUND_1_8_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.9",
        "Boundary DTOs cannot leak into domain signatures",
        Severity::Error,
        "ENFORCER_BOUND_1_9_MARKER",
    );
    reg(
        &mut v,
        "BOUND-1.10",
        "Boundary conversion functions return typed errors",
        Severity::Error,
        "ENFORCER_BOUND_1_10_MARKER",
    );
    v
}
