//! Common-family prefix `NPM-1` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/package-determinism, package-determinism, dependency-policy, sbom.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/npm-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `NPM-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "NPM-1.1",
        "package-lock.json is required",
        Severity::Error,
        "ENFORCER_NPM_1_1_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.2",
        "npm ci is required in CI",
        Severity::Error,
        "ENFORCER_NPM_1_2_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.3",
        "Enforcer dependencies must be pinned",
        Severity::Error,
        "ENFORCER_NPM_1_3_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.4",
        "packageManager must pin npm",
        Severity::Error,
        "ENFORCER_NPM_1_4_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.5",
        "Node engine must be bounded",
        Severity::Error,
        "ENFORCER_NPM_1_5_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.6",
        "Dependency install scripts require approval",
        Severity::Error,
        "ENFORCER_NPM_1_6_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.7",
        "Git dependencies are forbidden",
        Severity::Error,
        "ENFORCER_NPM_1_7_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.8",
        "File and path dependencies are forbidden",
        Severity::Error,
        "ENFORCER_NPM_1_8_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.9",
        "npm audit high and critical findings must fail",
        Severity::Error,
        "ENFORCER_NPM_1_9_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.10",
        "Dependency licenses must match policy",
        Severity::Error,
        "ENFORCER_NPM_1_10_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.11",
        "Suspicious dependency names are forbidden",
        Severity::Error,
        "ENFORCER_NPM_1_11_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.12",
        "SBOM must be generated for release",
        Severity::Error,
        "ENFORCER_NPM_1_12_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.13",
        "Published package files must be explicit",
        Severity::Error,
        "ENFORCER_NPM_1_13_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.14",
        "Package bin paths must exist",
        Severity::Error,
        "ENFORCER_NPM_1_14_MARKER",
    );
    reg(
        &mut v,
        "NPM-1.15",
        "Package export paths must exist",
        Severity::Error,
        "ENFORCER_NPM_1_15_MARKER",
    );
    v
}
