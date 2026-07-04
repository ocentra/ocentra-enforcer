//! Common-family prefix `DOCENF-1` (10 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/docs-completeness.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/docenf-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `DOCENF-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "DOCENF-1.1",
        "Rule docs must include required teaching sections",
        Severity::Error,
        "ENFORCER_DOCENF_1_1_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.2",
        "Source rule docs must include fail and pass code blocks",
        Severity::Error,
        "ENFORCER_DOCENF_1_2_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.3",
        "Tagged rule doc code blocks must stay parseable",
        Severity::Error,
        "ENFORCER_DOCENF_1_3_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.4",
        "Fix snippets must stay compact",
        Severity::Error,
        "ENFORCER_DOCENF_1_4_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.5",
        "Registry doc anchors must be stable lowercase anchors",
        Severity::Error,
        "ENFORCER_DOCENF_1_5_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.6",
        "Immutable rule docs must use mandatory language",
        Severity::Error,
        "ENFORCER_DOCENF_1_6_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.7",
        "Docs must not make legacy aliases canonical",
        Severity::Error,
        "ENFORCER_DOCENF_1_7_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.8",
        "Docs cannot describe the pack as Rust-only",
        Severity::Error,
        "ENFORCER_DOCENF_1_8_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.9",
        "Advisory rule docs must explain promotion",
        Severity::Error,
        "ENFORCER_DOCENF_1_9_MARKER",
    );
    reg(
        &mut v,
        "DOCENF-1.10",
        "Review and proof rules must name proof evidence",
        Severity::Error,
        "ENFORCER_DOCENF_1_10_MARKER",
    );
    v
}
