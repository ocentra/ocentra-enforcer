//! Common-family prefix `CI-1` (21 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/ci-integrity, ci-integrity.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/ci-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `CI-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "CI-1.1",
        "CI must use npm ci",
        Severity::Error,
        "ENFORCER_CI_1_1_MARKER",
    );
    reg(
        &mut v,
        "CI-1.2",
        "CI must run npm test",
        Severity::Error,
        "ENFORCER_CI_1_2_MARKER",
    );
    reg(
        &mut v,
        "CI-1.3",
        "CI must run rule and policy tests",
        Severity::Error,
        "ENFORCER_CI_1_3_MARKER",
    );
    reg(
        &mut v,
        "CI-1.4",
        "CI must run multi-language tests",
        Severity::Error,
        "ENFORCER_CI_1_4_MARKER",
    );
    reg(
        &mut v,
        "CI-1.5",
        "CI must run MCP tests",
        Severity::Error,
        "ENFORCER_CI_1_5_MARKER",
    );
    reg(
        &mut v,
        "CI-1.6",
        "CI must run Enforcer self-scan",
        Severity::Error,
        "ENFORCER_CI_1_6_MARKER",
    );
    reg(
        &mut v,
        "CI-1.7",
        "CI must validate schemas",
        Severity::Error,
        "ENFORCER_CI_1_7_MARKER",
    );
    reg(
        &mut v,
        "CI-1.8",
        "CI must run secret scan",
        Severity::Error,
        "ENFORCER_CI_1_8_MARKER",
    );
    reg(
        &mut v,
        "CI-1.9",
        "CI must run dependency policy",
        Severity::Error,
        "ENFORCER_CI_1_9_MARKER",
    );
    reg(
        &mut v,
        "CI-1.10",
        "CI must run SBOM check",
        Severity::Error,
        "ENFORCER_CI_1_10_MARKER",
    );
    reg(
        &mut v,
        "CI-1.11",
        "Hard CI gates cannot continue on error",
        Severity::Error,
        "ENFORCER_CI_1_11_MARKER",
    );
    reg(
        &mut v,
        "CI-1.12",
        "CI must not hide failing hard gates",
        Severity::Error,
        "ENFORCER_CI_1_12_MARKER",
    );
    reg(
        &mut v,
        "CI-1.13",
        "CI action versions must be pinned",
        Severity::Error,
        "ENFORCER_CI_1_13_MARKER",
    );
    reg(
        &mut v,
        "CI-1.14",
        "CI workflows must declare least-privilege permissions",
        Severity::Error,
        "ENFORCER_CI_1_14_MARKER",
    );
    reg(
        &mut v,
        "CI-1.15",
        "CI must run on pull requests and main",
        Severity::Error,
        "ENFORCER_CI_1_15_MARKER",
    );
    reg(
        &mut v,
        "CI-1.16",
        "CI must cover Linux, Windows, and macOS",
        Severity::Error,
        "ENFORCER_CI_1_16_MARKER",
    );
    reg(
        &mut v,
        "CI-1.17",
        "CI workflow must match Enforcer adapter contract",
        Severity::Error,
        "ENFORCER_CI_1_17_MARKER",
    );
    reg(
        &mut v,
        "CI-1.18",
        "CI cannot call legacy weaker commands",
        Severity::Error,
        "ENFORCER_CI_1_18_MARKER",
    );
    reg(
        &mut v,
        "CI-1.19",
        "Branch protection policy is required",
        Severity::Error,
        "ENFORCER_CI_1_19_MARKER",
    );
    reg(
        &mut v,
        "CI-1.20",
        "Required checks must include Enforcer",
        Severity::Error,
        "ENFORCER_CI_1_20_MARKER",
    );
    reg(
        &mut v,
        "CI-1.21",
        "Subprocess JSON capture must be CI-safe",
        Severity::Error,
        "ENFORCER_CI_1_21_MARKER",
    );
    v
}
