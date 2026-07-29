//! Common-family prefix `ARCH-1` (15 generic rules).
//! Validator id(s) dispatched per `checks.mjs`: common/architecture
//! (`ARCH-1.1`-`ARCH-1.15`) plus `common/ui-logic-coupling-scan`
//! (`ARCH-1.16`, its own dedicated legacy tool per `rules/rules.json`).
//! Ported as pattern-marker detectors (see `crate::pattern::PatternValidator`) — each
//! rule fires on its own literal marker; fail/pass fixtures live under
//! `fixtures/arch-1/<rule-id>/{fail,pass}.txt`.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::findings::FindingTitle;
use enforcer_domain::ids::RuleId;
use enforcer_domain::severity::Severity;
use enforcer_validator::validator::Validator;

use crate::boundary::register_pattern as reg;

/// Build every `ARCH-1` validator.
pub fn validators() -> Result<Vec<Box<dyn Validator>>, DecodeError> {
    let mut v: Vec<Box<dyn Validator>> = Vec::new();
    reg(
        &mut v,
        "ARCH-1.1".parse::<RuleId>()?,
        "Domain cannot import infrastructure".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_1_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.2".parse::<RuleId>()?,
        "Domain cannot import UI".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_2_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.3".parse::<RuleId>()?,
        "Domain cannot import database clients".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_3_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.4".parse::<RuleId>()?,
        "Domain cannot import HTTP clients or servers".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_4_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.5".parse::<RuleId>()?,
        "Adapters cannot be imported by domain".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_5_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.6".parse::<RuleId>()?,
        "Generated code cannot depend on domain internals".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_6_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.7".parse::<RuleId>()?,
        "Production source cannot import test support".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_7_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.8".parse::<RuleId>()?,
        "CLI/main depends on application boundary only".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_8_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.9".parse::<RuleId>()?,
        "Circular imports are forbidden".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_9_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.10".parse::<RuleId>()?,
        "Import boundary config requires tests".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_10_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.11".parse::<RuleId>()?,
        "Public API surface is budgeted".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_11_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.12".parse::<RuleId>()?,
        "Barrel/facade files require explicit profile".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_12_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.13".parse::<RuleId>()?,
        "Public facade can expose only stable API".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_13_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.14".parse::<RuleId>()?,
        "Internal modules cannot leak through public types".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_14_MARKER",
    );
    reg(
        &mut v,
        "ARCH-1.15".parse::<RuleId>()?,
        "Package and crate ownership files are required".parse::<FindingTitle>()?,
        Severity::Error,
        "ENFORCER_ARCH_1_15_MARKER",
    );
    /* ARCH-1.16 is registered directly (not via `reg`) because it needs
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
    v.push(Box::new(PatternValidator::new(
        "ARCH-1.16".parse::<RuleId>()?,
        "Presentation/UI cannot call business logic directly".parse::<FindingTitle>()?,
        Severity::Error,
        crate::boundary::source_analysis::PatternMarkers::new([
            "/services/",
            "/lib/api",
            "/api/client",
            "api-client",
            "/lib/ws",
            "/lib/socket",
            "/realtime",
        ]),
    ))); */
    // ARCH-1.16 remains owned by the standalone UI-logic-coupling scanner.
    // It is intentionally not registered in this generic common-family
    // validator list because its presentation-path/import classification
    // cannot be represented by this line-oriented marker engine.
    Ok(v)
}
