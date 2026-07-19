//! `enforcer-mechanization` — the d01 crate: the rule scaffolder plus the
//! fail-closed parity oracle (arc-14).
//!
//! # Charter
//!
//! Track D (d01) rule mechanization used to be spread across `.mjs`
//! check/contract scripts (`scripts/check-source-core-contract-*.mjs`) that
//! enforced rule/fixture completeness ad hoc. This crate is the Rust
//! replacement:
//!
//! - [`scaffold`] — given a minimal spec for a NEW rule, emit a well-formed
//!   `enforcer_rules::registry::RuleRecord`, a bare `Validator` skeleton
//!   (source text), and starter content for both fixture slots. The
//!   scaffolder never silently produces an already-passing rule: the
//!   generated validator skeleton always returns zero findings, so a
//!   freshly-scaffolded rule fails the oracle below until a human
//!   implements real detection logic.
//! - [`oracle`] — the fail-closed parity oracle: a candidate rule is only
//!   ACCEPTED if its record shape is well-formed, a validator
//!   implementation is supplied, that validator's `rule_id()` matches the
//!   record, it fires on the declared fail fixture, and it stays silent on
//!   the declared pass fixture. Built on
//!   [`enforcer_validator::harness::run_fixture_parity`] — this crate does
//!   not reimplement fixture I/O or pass/fail assertions, it composes the
//!   reusable base with the record-shape re-check.
//!
//! - [`feedback`] — the d08 harness-feedback pipeline: classifies a parsed
//!   `enforcer-harness` diagnostic as `prevent` vs `detect` (mechanical
//!   field matching, never an LLM judgment call) and, for `prevent`,
//!   drives [`scaffold::scaffold_rule`] to emit a PROPOSED candidate rule.
//!
//! This crate does NOT own: the rule registry itself (`enforcer-rules`),
//! the `Validator` trait or harness (`enforcer-validator`), or any
//! language-specific detection logic (the `enforcer-lang-*` crates).
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_mechanization::oracle::accept_rule`.

macro_rules! mechanization_finding {
    ($rule_id:expr, $title:expr, $detail:expr, $file:expr $(,)?) => {{
        match (
            $title.parse::<enforcer_domain::findings::FindingTitle>(),
            $detail.parse::<enforcer_domain::findings::FindingDetail>(),
        ) {
            (Ok(title), Ok(detail)) => Some(enforcer_domain::findings::Finding {
                rule_id: $rule_id,
                severity: enforcer_domain::severity::Severity::Error,
                title,
                detail,
                file: $file,
                line: enforcer_domain::findings::FindingLine::known(
                    enforcer_domain::telemetry_types::SourceLine::try_new(
                        std::num::NonZeroU32::MIN,
                    ),
                ),
                snippet: None,
            }),
            _ => None,
        }
    }};
}

pub mod error;
pub mod feedback;
pub mod oracle;
pub mod parity;
pub mod scaffold;
