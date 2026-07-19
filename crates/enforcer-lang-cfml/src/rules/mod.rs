//! CFML/ColdFusion rule families, one module per checklist group. See each
//! module's doc comment for its `RuleId` set; [`crate::all_validators`]
//! composes every module's `all()` into the single vec this crate
//! registers.
//!
//! The `T3` (advisory, no-mechanization-possible) row carries no
//! `Validator` at all by design -- the d01 parity oracle
//! (`enforcer_mechanization::parity`) checks only that its registry
//! record's `tags` carries the verbatim
//! `advisory, no mechanization possible + <reason>` label, never a
//! fixture/detection pair. That row (`CF-ARCH-6.1`) exists only as a
//! record in `crates/enforcer-rules/rules/cfml-advisory.json`, not as Rust
//! code in this module tree.

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
mod boundary;
pub mod cflint_adapter;
pub mod err;
pub mod security;
pub mod style;
mod support;
pub mod toolchain;
