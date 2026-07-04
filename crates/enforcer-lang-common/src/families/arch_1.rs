//! Common-family prefix `ARCH-1` (15 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/architecture.
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/arch-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::registry::reg;

/// Build every `ARCH-1` validator.
pub fn validators() -> Vec<Box<dyn Validator>> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "ARCH-1.1",
        "Domain cannot import infrastructure",
        Severity::Error,
        "ENFORCER_ARCH_1_1_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.2",
        "Domain cannot import UI",
        Severity::Error,
        "ENFORCER_ARCH_1_2_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.3",
        "Domain cannot import database clients",
        Severity::Error,
        "ENFORCER_ARCH_1_3_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.4",
        "Domain cannot import HTTP clients or servers",
        Severity::Error,
        "ENFORCER_ARCH_1_4_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.5",
        "Adapters cannot be imported by domain",
        Severity::Error,
        "ENFORCER_ARCH_1_5_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.6",
        "Generated code cannot depend on domain internals",
        Severity::Error,
        "ENFORCER_ARCH_1_6_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.7",
        "Production source cannot import test support",
        Severity::Error,
        "ENFORCER_ARCH_1_7_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.8",
        "CLI/main depends on application boundary only",
        Severity::Error,
        "ENFORCER_ARCH_1_8_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.9",
        "Circular imports are forbidden",
        Severity::Error,
        "ENFORCER_ARCH_1_9_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.10",
        "Import boundary config requires tests",
        Severity::Error,
        "ENFORCER_ARCH_1_10_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.11",
        "Public API surface is budgeted",
        Severity::Error,
        "ENFORCER_ARCH_1_11_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.12",
        "Barrel/facade files require explicit profile",
        Severity::Error,
        "ENFORCER_ARCH_1_12_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.13",
        "Public facade can expose only stable API",
        Severity::Error,
        "ENFORCER_ARCH_1_13_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.14",
        "Internal modules cannot leak through public types",
        Severity::Error,
        "ENFORCER_ARCH_1_14_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.15",
        "Package and crate ownership files are required",
        Severity::Error,
        "ENFORCER_ARCH_1_15_MARKER",
    );
    v
}
