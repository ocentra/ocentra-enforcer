//! Dart/Flutter rule families, one module per checklist group. See each
//! module's doc comment for its `RuleId` set; [`all_validators`] in the
//! crate root composes every module's `all()` into the single vec this
//! crate registers.
//!
//! `T3` (advisory, no-mechanization-possible) rows carry no `Validator`
//! at all by design — the d01 parity oracle
//! (`enforcer_mechanization::parity`) checks only that their registry
//! record's `tags` carries the verbatim
//! `advisory, no mechanization possible + <reason>` label, never a
//! fixture/detection pair. Those three rows
//! (`DART-NAME-3.1`/`DART-IMP-2.1`/`DART-STATE-2.1`) exist only as
//! records in `crates/enforcer-rules/rules/dart-advisory.json`, not as
//! Rust code in this module tree.

macro_rules! finding {
    ($spec:expr, $detail:literal, $input:expr, $line:expr $(,)?) => {{
        let spec = $spec;
        let Ok(title) = spec.rule.finding_title() else {
            return Vec::new();
        };
        let Ok(detail) = $detail.parse::<enforcer_domain::findings::FindingDetail>() else {
            return Vec::new();
        };
        let Some(source_line) = $crate::rules::support::into_source_line($line) else {
            return Vec::new();
        };
        enforcer_domain::findings::Finding {
            rule_id: Clone::clone(spec.rule_id),
            severity: spec.severity,
            title,
            detail,
            file: Clone::clone($input.file),
            line: enforcer_domain::findings::FindingLine::known(source_line),
            snippet: None,
        }
    }};
    ($spec:expr, $detail:expr, $input:expr, $line:expr $(,)?) => {{
        let spec = $spec;
        let Ok(title) = spec.rule.finding_title() else {
            return Vec::new();
        };
        let Ok(detail) = enforcer_domain::findings::FindingDetail::new($detail) else {
            return Vec::new();
        };
        let Some(source_line) = $crate::rules::support::into_source_line($line) else {
            return Vec::new();
        };
        enforcer_domain::findings::Finding {
            rule_id: Clone::clone(spec.rule_id),
            severity: spec.severity,
            title,
            detail,
            file: Clone::clone($input.file),
            line: enforcer_domain::findings::FindingLine::known(source_line),
            snippet: None,
        }
    }};
}

pub mod arch;
pub mod err;
pub mod naming;
pub mod security;
pub mod state;
mod support;
pub mod toolchain;
pub mod types;
pub mod widget;
