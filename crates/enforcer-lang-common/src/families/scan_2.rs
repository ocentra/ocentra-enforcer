//! Common-family prefix `SCAN-2` (10 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/scanner-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/scan-2/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `SCAN-2` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "SCAN-2.1",
        "Rust strict mode must use cargo metadata",
        Severity::Error,
        "ENFORCER_SCAN_2_1_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.2",
        "Rust strict mode must use parser-backed checks",
        Severity::Error,
        "ENFORCER_SCAN_2_2_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.3",
        "Rust strict mode must ingest Clippy JSON",
        Severity::Error,
        "ENFORCER_SCAN_2_3_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.4",
        "Rust strict mode must ingest rustdoc warnings",
        Severity::Error,
        "ENFORCER_SCAN_2_4_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.5",
        "TypeScript strict mode must use compiler or ESLint JSON",
        Severity::Error,
        "ENFORCER_SCAN_2_5_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.6",
        "Python strict mode must ingest Ruff JSON",
        Severity::Error,
        "ENFORCER_SCAN_2_6_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.7",
        "Python strict mode must ingest Pyright or mypy output",
        Severity::Error,
        "ENFORCER_SCAN_2_7_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.8",
        "Security strict mode must ingest SARIF",
        Severity::Error,
        "ENFORCER_SCAN_2_8_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.9",
        "Regex scanner must remain fast preflight",
        Severity::Error,
        "ENFORCER_SCAN_2_9_MARKER",
    );
    reg(
        &mut v,
        "SCAN-2.10",
        "Native and regex reports must merge without duplicate spam",
        Severity::Error,
        "ENFORCER_SCAN_2_10_MARKER",
    );
    v
}
