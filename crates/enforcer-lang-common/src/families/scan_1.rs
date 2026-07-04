//! Common-family prefix `SCAN-1` (20 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/scanner-contracts.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/scan-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `SCAN-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "SCAN-1.1",
        "Scanner must mask string literals where appropriate",
        Severity::Error,
        "ENFORCER_SCAN_1_1_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.2",
        "Scanner must mask comments where appropriate",
        Severity::Error,
        "ENFORCER_SCAN_1_2_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.3",
        "Scanner must still detect suppression comments",
        Severity::Error,
        "ENFORCER_SCAN_1_3_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.4",
        "Scanner must handle CRLF and LF identically",
        Severity::Error,
        "ENFORCER_SCAN_1_4_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.5",
        "Scanner must support Unicode paths",
        Severity::Error,
        "ENFORCER_SCAN_1_5_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.6",
        "Scanner must support spaces in paths",
        Severity::Error,
        "ENFORCER_SCAN_1_6_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.7",
        "Scanner must support Windows drive paths",
        Severity::Error,
        "ENFORCER_SCAN_1_7_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.8",
        "Scanner symlink policy must be explicit",
        Severity::Error,
        "ENFORCER_SCAN_1_8_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.9",
        "Scanner must avoid symlink loops",
        Severity::Error,
        "ENFORCER_SCAN_1_9_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.10",
        "Scanner output must be sorted",
        Severity::Error,
        "ENFORCER_SCAN_1_10_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.11",
        "Scanner must bound file reads",
        Severity::Error,
        "ENFORCER_SCAN_1_11_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.12",
        "Scanner must skip binary files safely",
        Severity::Error,
        "ENFORCER_SCAN_1_12_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.13",
        "Invalid UTF-8 must produce bounded diagnostics",
        Severity::Error,
        "ENFORCER_SCAN_1_13_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.14",
        "Unknown extensions must not trigger false language scans",
        Severity::Error,
        "ENFORCER_SCAN_1_14_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.15",
        "Diff scope must handle deleted and renamed files",
        Severity::Error,
        "ENFORCER_SCAN_1_15_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.16",
        "File scope must not scan the whole repo",
        Severity::Error,
        "ENFORCER_SCAN_1_16_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.17",
        "Workspace scope must scan configured roots",
        Severity::Error,
        "ENFORCER_SCAN_1_17_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.18",
        "Crate/package scope must resolve manifests",
        Severity::Error,
        "ENFORCER_SCAN_1_18_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.19",
        "Scope reports must include included/excluded counts",
        Severity::Error,
        "ENFORCER_SCAN_1_19_MARKER",
    );
    reg(
        &mut v,
        "SCAN-1.20",
        "Doctor must expose ignore globs",
        Severity::Error,
        "ENFORCER_SCAN_1_20_MARKER",
    );
    v
}
