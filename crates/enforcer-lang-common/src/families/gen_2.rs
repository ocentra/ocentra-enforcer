//! Common-family prefix `GEN-2` (10 rules).
//! Validator id(s) dispatched per `checks.mjs`: generic-scanner, common/generated-artifacts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/gen-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `GEN-2` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "GEN-2.1",
        "Generated directories require ignore policy",
        Severity::Error,
        "ENFORCER_GEN_2_1_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.2",
        "Generated files require source owner provenance",
        Severity::Error,
        "ENFORCER_GEN_2_2_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.3",
        "Generated files cannot be edited manually",
        Severity::Error,
        "ENFORCER_GEN_2_3_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.4",
        "Generated contract artifacts require source hash",
        Severity::Error,
        "ENFORCER_GEN_2_4_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.5",
        "Generated schema files must be reproducible",
        Severity::Error,
        "ENFORCER_GEN_2_5_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.6",
        "Runtime output directories cannot be tracked",
        Severity::Error,
        "ENFORCER_GEN_2_6_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.7",
        "Generated files cannot be single source of truth",
        Severity::Error,
        "ENFORCER_GEN_2_7_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.8",
        "Generated code cannot contain suppressions",
        Severity::Error,
        "ENFORCER_GEN_2_8_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.9",
        "Generated code cannot live in domain modules",
        Severity::Error,
        "ENFORCER_GEN_2_9_MARKER",
    );
    reg(
        &mut v,
        "GEN-2.10",
        "Generated snapshots must be stable",
        Severity::Error,
        "ENFORCER_GEN_2_10_MARKER",
    );
    v
}
