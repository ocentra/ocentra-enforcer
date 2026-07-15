//! `enforcer-lang-dart` -- e-pack-dart: the Dart/Flutter-family
//! `Validator` crate.
//!
//! # Charter
//!
//! Dart/Flutter is a greenfield gap the plan's arc-* tracks never
//! covered: before this pack there was no `enforcer-lang-dart` crate, no
//! `dart` language / `.dart` extension in any structured rule record,
//! and no Dart `Validator` impls. This pack stands up the crate
//! skeleton itself (no arc-* pack pre-built it) and implements every
//! `DART-*` rule from the ADBP `rules-flutter` gap rows against
//! [`enforcer_validator::validator::Validator`] (arc-05), proven through
//! [`enforcer_validator::harness::run_fixture_parity`] exactly like every
//! other `enforcer-lang-*` crate.
//!
//! Every rule here is a lightweight line/keyword-oriented text detector
//! (mirroring `enforcer-lang-common::rules::fsm`/`rules::size_shape`'s
//! dominant shape) rather than a full tree-sitter/AST parse — this
//! workspace has no tree-sitter dependency for ANY language target
//! (Python/Dart/CFML/TS all use the same line/keyword-scan posture), so
//! this crate follows the established convention rather than
//! introducing the only AST dependency in the workspace.
//!
//! This crate does NOT own:
//! - the `Validator` trait or fixture/parity harness (`enforcer-validator`,
//!   arc-05);
//! - the rule registry shape (`enforcer-rules`, arc-04) — it SHIPS its own
//!   `dart-*.json` rule records under `crates/enforcer-rules/rules/` the
//!   same way every other lang pack does, but does not own the registry
//!   skeleton itself;
//! - the d16 FSM/enum-parse mechanism or d22 size/shape-cap mechanism
//!   (both live in `enforcer-lang-common`) — this crate only supplies
//!   Dart `appliesTo` + Dart fixtures for rows those mechanisms already
//!   cover (`FSM-ENUMPARSE.1`, `FSM-COVERAGE.1`, `SIZE-*`), never a second
//!   copy of either mechanism;
//! - the `.dart` extension entry in the `enforcer-literal-scan` (arc-13)
//!   language registry — e01 already registered it
//!   (`crates/enforcer-literal-scan/src/language-registry.rs`); this
//!   crate only depends on that registration existing, it does not
//!   re-declare it.
//!
//! # Structure
//!
//! One module per checklist group under [`rules`]:
//! - [`rules::arch`] — layer/import boundaries, null-safety discipline,
//!   immutable entities (`DART-ARCH-1.*`, `DART-DOMAIN-1.1`,
//!   `DART-BANG-1.1`, `DART-FREEZED-1.1`).
//! - [`rules::types`] — typed DTOs, silent-fallback, form-state shape
//!   (`DART-TYPE-1.*`, `DART-FALLBACK-1.1`, `DART-FORMMAP-1.1`).
//! - [`rules::security`] — secrets, storage, transport, diagnostics
//!   (`DART-SEC-1.1..1.6`).
//! - [`rules::state`] — Riverpod/state-management discipline
//!   (`DART-STATE-1.1..1.3`, `DART-RIVERPOD-1.1`, `DART-INITSTATE-1.1`).
//! - [`rules::widget`] — widget composition, list perf, theming,
//!   navigation, l10n (`DART-COMP-1.*`, `DART-PERF-1.1`/`2.1`,
//!   `DART-COLOR-1.1`, `DART-NAV-2.1`, `DART-L10N-2.1`).
//! - [`rules::naming`] — filename/widget parity, import grouping, string
//!   style (`DART-NAME-1.1`, `DART-IMP-1.1`, `DART-STYLE-2.1`).
//! - [`rules::err`] — typed failure hierarchy, no raw exception to user
//!   (`DART-ERR-1.1`/`2.1`).
//! - [`rules::toolchain`] — analyzer/CI/dependency/codegen hygiene
//!   (`DART-TOOL-1.1..1.3`, `DART-DEP-1.1`, `DART-GEN-1.1`).
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_lang_dart::rules::arch::all`.

pub mod rules;

/// Build every validator this crate registers, across all eight rule
/// modules. Order is not significant to any consumer — `enforcer-scan`
/// dispatches by `RuleId`, not by vec position.
pub fn all_validators() -> Result<
    Vec<Box<dyn enforcer_validator::validator::Validator>>,
    enforcer_domain::boundary::decode_error::DecodeError,
> {
    let mut validators = Vec::new();
    validators.extend(rules::arch::all()?);
    validators.extend(rules::types::all()?);
    validators.extend(rules::security::all()?);
    validators.extend(rules::state::all()?);
    validators.extend(rules::widget::all()?);
    validators.extend(rules::naming::all()?);
    validators.extend(rules::err::all()?);
    validators.extend(rules::toolchain::all()?);
    Ok(validators)
}

#[cfg(test)]
mod registry_smoke {
    //! Smoke test: every validator this crate builds must expose a
    //! `DART-*`-prefixed `RuleId` and the set must be duplicate-free —
    //! catches a copy/paste rule id collision immediately, before the
    //! heavier per-rule fixture/parity suite in `tests/fixture_parity.rs`
    //! runs.

    use std::collections::BTreeSet;

    use crate::all_validators;

    #[test]
    fn every_validator_has_a_unique_dart_prefixed_rule_id() -> Result<(), Box<dyn std::error::Error>>
    {
        let validators = all_validators()?;
        assert!(
            !validators.is_empty(),
            "expected at least one registered validator"
        );

        let mut seen = BTreeSet::new();
        for validator in &validators {
            let rule_id = validator.rule_id().to_string();
            assert!(
                rule_id.starts_with("DART-"),
                "rule id `{rule_id}` is not DART-prefixed"
            );
            assert!(
                seen.insert(rule_id.clone()),
                "duplicate rule id `{rule_id}` registered twice"
            );
        }
        Ok(())
    }
}
