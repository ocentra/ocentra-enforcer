//! `enforcer-lang-cfml` -- e-pack-cfml: the CFML/ColdFusion `Validator`
//! crate.
//!
//! # Charter
//!
//! CFML/ColdFusion is a greenfield gap the plan's arc-* tracks never
//! covered: before this pack there was no `enforcer-lang-cfml` crate, no
//! `coldfusion` language / `.cfc`/`.cfm` extension in any structured rule
//! record, and no CFML `Validator` impls. This pack stands up the crate
//! skeleton itself (no arc-* pack pre-built it) and implements every
//! `CF-*`/`CFML-*` rule from the ADBP CFML gap rows against
//! [`enforcer_validator::validator::Validator`] (arc-05), proven through
//! [`enforcer_validator::harness::run_fixture_parity`] exactly like every
//! other `enforcer-lang-*` crate.
//!
//! Unlike Rust (`syn`) or TS/Dart (`tree-sitter`), this workspace has no
//! bundled CFML grammar/AST. Every structural rule here is a lightweight
//! line/keyword-oriented text detector (mirroring
//! `enforcer-lang-common::rules::fsm`/`rules::size_shape`'s dominant
//! shape). Where the ADBP source rule names a real CFLint rule code
//! (`MISSING_VAR`, `QUERYPARAM_REQ`, ...), [`rules::cflint_adapter`] shells
//! out to the `cflint` binary itself (T1 hard-gate codes) or maps its
//! output to a scored/advisory finding, and GRACEFULLY SKIPS -- with a
//! recorded "tool unavailable" diagnostic, never a silent pass -- when the
//! binary is absent from `PATH`.
//!
//! This crate does NOT own:
//! - the `Validator` trait or fixture/parity harness (`enforcer-validator`,
//!   arc-05);
//! - the rule registry shape (`enforcer-rules`, arc-04) -- it SHIPS its own
//!   `cfml*.json` rule records under `crates/enforcer-rules/rules/` the
//!   same way every other lang pack does, but does not own the registry
//!   skeleton itself;
//! - the d16 FSM/enum-parse mechanism or d22 size/shape-cap mechanism
//!   (both live in `enforcer-lang-common`) -- this crate only supplies
//!   `coldfusion` `appliesTo` + CFML fixtures for rows those mechanisms
//!   already cover (`FSM-*`, `SIZE-*`), never a second copy of either
//!   mechanism;
//! - the `.cfc`/`.cfm` extension entry in the `enforcer-literal-scan`
//!   (arc-13) language registry -- the arc-13/e01 owner records that
//!   additively; this crate only depends on that registration existing.
//!
//! # Structure
//!
//! One module per checklist group under [`rules`]:
//! - [`rules::arch`] -- layered architecture, WireBox DI shape
//!   (`CF-ARCH-*`, `CF-DI-1.1`).
//! - [`rules::security`] -- SQLi/XSS/secrets/info-disclosure
//!   (`CF-SEC-*`).
//! - [`rules::style`] -- `var` scoping, `arguments` scope, typed
//!   signatures, banned dynamic-eval, script-vs-tag syntax, naming,
//!   access modifiers (`CF-STYLE-*`, `CFML-*`).
//! - [`rules::err`] -- typed throws, no swallowed catch (`CF-ERR-*`).
//! - [`rules::toolchain`] -- `box.json`/`.cflintrc`/CI/TestBox hygiene
//!   (`CF-TOOL-*`, `CF-CI-*`, `CF-DEP-1.1`, `CF-TEST-1.1`).
//! - [`rules::cflint_adapter`] -- the CFLint shell-out advisory adapter
//!   with an honest graceful-skip when the binary is absent.
//!
//! FSM (`CF-FSM-*`) and size/complexity (`CF-SIZE-*`/`CFML-CPLX-2.1`)
//! mechanisms come from `enforcer-lang-common`'s d16/d22 families directly
//! -- this crate does not re-register them, it only supplies their CFML
//! fixtures under `tests/fixtures/`.
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_lang_cfml::rules::arch::all`.

pub mod rules;

/// Build every FIXTURE-PROVABLE validator this crate registers -- i.e.
/// every validator whose pass fixture is deterministically silent
/// regardless of host environment.
///
/// Deliberately EXCLUDES [`rules::cflint_adapter`]: that validator's
/// output legitimately depends on whether the `cflint` binary is
/// installed on the host (honest graceful-skip vs. a real advisory scan),
/// so it cannot satisfy the uniform "pass fixture => zero findings"
/// contract [`enforcer_validator::harness::run_fixture_parity`] enforces
/// on every machine. Callers that want the CFLint adapter call
/// [`rules::cflint_adapter::all`] directly and register it alongside this
/// crate's other validators; see that module's own tests for its
/// graceful-skip/report-mapping proof instead.
///
/// Also does NOT include the d16/d22 `enforcer-lang-common` validators
/// (`FSM-*`/`SIZE-*`) -- those are registered once, by
/// `enforcer-lang-common` itself, and this crate only supplies CFML
/// `appliesTo`/fixtures for them; re-registering them here would double
/// them up in any consumer that links both crates.
pub fn all_validators(
) -> Result<Vec<Box<dyn enforcer_validator::validator::Validator>>, enforcer_core::error::DecodeError>
{
    let mut validators = Vec::new();
    validators.extend(rules::arch::all()?);
    validators.extend(rules::security::all()?);
    validators.extend(rules::style::all()?);
    validators.extend(rules::err::all()?);
    validators.extend(rules::toolchain::all()?);
    Ok(validators)
}

#[cfg(test)]
mod registry_smoke {
    //! Smoke test: every validator this crate builds must expose a
    //! `CF-`/`CFML-`-prefixed `RuleId` and the set must be duplicate-free --
    //! catches a copy/paste rule id collision immediately, before the
    //! heavier per-rule fixture/parity suite in `tests/fixture_parity.rs`
    //! runs.

    use std::collections::BTreeSet;

    use crate::all_validators;

    #[test]
    fn every_validator_has_a_unique_cfml_prefixed_rule_id() -> Result<(), Box<dyn std::error::Error>>
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
                rule_id.starts_with("CF-") || rule_id.starts_with("CFML-"),
                "rule id `{rule_id}` is not CF-/CFML-prefixed"
            );
            assert!(
                seen.insert(rule_id.clone()),
                "duplicate rule id `{rule_id}` registered twice"
            );
        }
        Ok(())
    }
}
