//! Common-family prefix `REPO-1` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/repo-governance, repo-governance.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/repo-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `REPO-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "REPO-1.1",
        "CODEOWNERS is required",
        Severity::Error,
        "ENFORCER_REPO_1_1_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.2",
        "CODEOWNERS must protect rules",
        Severity::Error,
        "ENFORCER_REPO_1_2_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.3",
        "CODEOWNERS must protect enforcement source",
        Severity::Error,
        "ENFORCER_REPO_1_3_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.4",
        "CODEOWNERS must protect schemas and adapters",
        Severity::Error,
        "ENFORCER_REPO_1_4_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.5",
        "CODEOWNERS must protect workflows",
        Severity::Error,
        "ENFORCER_REPO_1_5_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.6",
        "Package lockfile is required",
        Severity::Error,
        "ENFORCER_REPO_1_6_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.7",
        "packageManager is required",
        Severity::Error,
        "ENFORCER_REPO_1_7_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.8",
        "Node version policy must be bounded",
        Severity::Error,
        "ENFORCER_REPO_1_8_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.9",
        "Dependency versions must be deterministic",
        Severity::Error,
        "ENFORCER_REPO_1_9_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.10",
        "License file is required",
        Severity::Error,
        "ENFORCER_REPO_1_10_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.11",
        "Security policy is required",
        Severity::Error,
        "ENFORCER_REPO_1_11_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.12",
        "Contributing guide must explain rule changes",
        Severity::Error,
        "ENFORCER_REPO_1_12_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.13",
        "Changelog is required for rule behavior changes",
        Severity::Error,
        "ENFORCER_REPO_1_13_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.14",
        "Release policy must be documented",
        Severity::Error,
        "ENFORCER_REPO_1_14_MARKER",
    );
    reg(
        &mut v,
        "REPO-1.15",
        "Generated schema artifacts must not drift",
        Severity::Error,
        "ENFORCER_REPO_1_15_MARKER",
    );
    v
}
