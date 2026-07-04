//! Common-family prefix `PROOF-1` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/proof-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/proof-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `PROOF-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "PROOF-1.1",
        "PR-ready claims require fresh proof",
        Severity::Error,
        "ENFORCER_PROOF_1_1_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.2",
        "Proof freshness must bind commit and scope",
        Severity::Error,
        "ENFORCER_PROOF_1_2_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.3",
        "Manual-required proof cannot auto-pass",
        Severity::Error,
        "ENFORCER_PROOF_1_3_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.4",
        "Required proof artifacts must exist",
        Severity::Error,
        "ENFORCER_PROOF_1_4_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.5",
        "Proof artifacts must hash-match",
        Severity::Error,
        "ENFORCER_PROOF_1_5_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.6",
        "Dirty worktrees invalidate PR-ready proof",
        Severity::Error,
        "ENFORCER_PROOF_1_6_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.7",
        "Waived proof must remain visible",
        Severity::Error,
        "ENFORCER_PROOF_1_7_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.8",
        "Proof command cannot be empty",
        Severity::Error,
        "ENFORCER_PROOF_1_8_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.9",
        "Proof command cannot be a shell string",
        Severity::Error,
        "ENFORCER_PROOF_1_9_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.10",
        "Proof registry docs paths must exist",
        Severity::Error,
        "ENFORCER_PROOF_1_10_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.11",
        "Proof capabilities must match environment",
        Severity::Error,
        "ENFORCER_PROOF_1_11_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.12",
        "Device proof cannot auto-pass on desktop",
        Severity::Error,
        "ENFORCER_PROOF_1_12_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.13",
        "Proof claims must list proved and unproved claims",
        Severity::Error,
        "ENFORCER_PROOF_1_13_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.14",
        "Proof output must be compact by default",
        Severity::Error,
        "ENFORCER_PROOF_1_14_MARKER",
    );
    reg(
        &mut v,
        "PROOF-1.15",
        "Proof exports must redact secrets",
        Severity::Error,
        "ENFORCER_PROOF_1_15_MARKER",
    );
    v
}
