//! Common-family prefix `ARCH-1` (16 rules).
//! Validator id(s) dispatched per `checks.mjs`: common/architecture
//! (`ARCH-1.1`-`ARCH-1.15`) plus `common/ui-logic-coupling-scan`
//! (`ARCH-1.16`, its own dedicated legacy tool per `rules/rules.json`).
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/arch-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::pattern::PatternValidator;
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
    // ARCH-1.16 is registered directly (not via `reg`) because it needs
    // several fail markers, and `reg`'s signature only takes one. Unlike its
    // siblings above (synthetic `ENFORCER_ARCH_1_*_MARKER` placeholders),
    // these markers are literal substrings lifted from the legacy
    // `src/ui-logic-coupling-scan.mjs` scanner's
    // `DEFAULT_BUSINESS_LOGIC_IMPORT_PATTERNS` / `DEFAULT_EVENT_SOURCE_IMPORT_PATTERNS`
    // regexes (`/\/services\//i`, `/\/lib\/api(["'/]|$)/i`, `/\/api\/client/i`,
    // `/\bapi-client\b/i`, `/\/lib\/ws(["'/]|$)/i`, `/\/lib\/socket/i`,
    // `/\/realtime/i`) — the closest a line/substring detector can get to
    // "a presentation file imports a business-logic or event-source module
    // directly" without the original's import-parsing + presentation-path
    // classification (see the module doc for why: this engine is a literal
    // substring scan, not an AST parser, matching the ported `.mjs`'s own
    // "mechanical, signal-based" caveat).
    if let Ok(id) = "ARCH-1.16".parse::<RuleId>() {
        v.push(Box::new(PatternValidator::new(
            id,
            "Presentation/UI cannot call business logic directly",
            Severity::Error,
            [
                "/services/",
                "/lib/api",
                "/api/client",
                "api-client",
                "/lib/ws",
                "/lib/socket",
                "/realtime",
            ],
        )));
    }
    v
}
