//! Mechanical classifier: `prevent` vs `detect` over a parsed harness
//! failure's fields.
//!
//! "Mechanical" per the workpack means an explicit, exhaustively-matched
//! Rust function over structured fields — never an LLM/vibe judgment call.
//! A failure is `Prevent` only when its `ruleId`/`tool` shape is one this
//! crate has an explicit static-analysis-shaped signal for (a rustc/tsc/
//! eslint/pyright/bandit/SARIF diagnostic tied to a *specific* rule code —
//! the kind of thing a `Validator` could have caught before the harness
//! ever ran). Everything else (a runtime `pytest` assertion failure, a
//! graceful-skip `HAR-2.4` marker, or a diagnostic whose rule id is empty/
//! generic) is `Detect`: harness-only, not preventable by a static rule.

use enforcer_harness::parsers::HarnessDiagnostic;

/// The two classification outcomes. Closed set — a third bucket must be a
/// deliberate new variant, not a smuggled bool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Classification {
    /// Could have been caught by a static `Validator` ahead of the harness
    /// run — a candidate for auto-scaffolding a PROPOSED rule.
    Prevent,
    /// Harness-only signal (e.g. a runtime test assertion, a graceful-skip
    /// marker). Not a candidate for mechanization; no rule is scaffolded.
    Detect,
}

/// Tool families whose diagnostics carry a STATIC-ANALYSIS-shaped rule id
/// (a specific lint/compiler/SARIF code tied to source text a `Validator`
/// could inspect) — as opposed to a runtime test-framework failure
/// (`pytest`) which has no such stable, source-inspectable code.
const STATIC_ANALYSIS_TOOLS: &[&str] = &[
    "cargo", "rustc", "tsc", "eslint", "pyright", "bandit", "cflint", "sarif",
];

/// Rule ids that mark a harness-internal graceful-skip record
/// ([`enforcer_harness::parsers::skip_diagnostic`]) rather than a real
/// finding — never preventable, since nothing was actually inspected.
const SKIP_RULE_IDS: &[&str] = &["HAR-2.4"];

/// Classify one parsed harness diagnostic. Pure function over the
/// diagnostic's own fields — no I/O, no LLM call, exhaustively matched so
/// every branch is auditable.
pub fn classify(diagnostic: &HarnessDiagnostic) -> Classification {
    let rule_id = diagnostic.rule_id.trim();
    if rule_id.is_empty() || SKIP_RULE_IDS.contains(&rule_id) {
        return Classification::Detect;
    }

    let tool = diagnostic.tool.trim().to_ascii_lowercase();
    let is_static_analysis_tool = STATIC_ANALYSIS_TOOLS
        .iter()
        .any(|known| tool.contains(known));
    if !is_static_analysis_tool {
        // e.g. `pytest`: a runtime assertion failure, not a static-analysis
        // signal — nothing a source-inspecting `Validator` could pre-empt.
        return Classification::Detect;
    }

    // `pytest`-shaped rule ids ("pytest" itself, used as the generic rule
    // id for every FAILED line) carry no specific lint code even when the
    // tool string happens to match — defensive re-check kept explicit
    // rather than folded into `STATIC_ANALYSIS_TOOLS` so the "no specific
    // code" reasoning stays visible at the call site.
    if rule_id.eq_ignore_ascii_case("pytest") {
        return Classification::Detect;
    }

    Classification::Prevent
}
