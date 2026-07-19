//! `enforcer-lang-py` -- arc-08: the Python-family `Validator` crate.
//!
//! # Charter
//!
//! Implements every `language == "python"` rule from `rules/rules.json`
//! (61 rules across the PY-1..PY-6 prefixes) against
//! `enforcer-validator`'s [`enforcer_validator::validator::Validator`]
//! trait. Each rule is a boundary line-marker validator entry or a
//! small dedicated struct (toolchain-diagnostics / manifest-shape /
//! required-coverage validators), grouped by the `rules/rules.json`
//! `validator` field into modules:
//!
//! - [`source_scan`] -- `python/source-scan` (44 rules: PY-1 x3, PY-4 x35,
//!   PY-6 x6).
//! - [`test_scan`] -- `python/test-scan` (2 rules: PY-2.1, PY-6.2).
//! - [`toolchain`] -- `python/ruff-json` + `python/typecheck` +
//!   `python/toolchain` (4 rules: PY-3.1, PY-3.2, PY-5.5, PY-5.6).
//! - [`generic_scanner_slice`] -- the PY SLICE of the shared
//!   `generic-scanner` engine (8 rules: PY-5.1/5.2/5.3/5.4/5.7/5.8/5.9/
//!   5.10). Per the workpack's shared-engine boundary note, this crate
//!   owns ONLY these PY-keyed rules -- the `generic-scanner` engine itself
//!   and the cross-language partition spec belong to arc-09.
//! - [`tests_required`] -- `python/tests` (3 rules: PY-6.8, PY-6.9,
//!   PY-6.10).
//! - [`rules::fastapi_layered`] -- e-pack-python: the FastAPI layered/
//!   clean-architecture + Python-security rule family (`PYFA-*`,
//!   legacy-labeled `py-fastapi-*`). Disjoint from `rules/rules.json`'s
//!   `language == "python"` catalog -- registered via its own
//!   `crates/enforcer-rules/rules/fastapi-layered.json` catalog, NOT via
//!   [`all_validators`]/the 61-count registry-coverage test below.
//!
//! The boundary line-marker scanner is the shared line-scan engine [`source_scan`] and
//! [`test_scan`] build their entries on; it exists specifically to dodge
//! the mem-arc-06-0002 double-dispatch gotcha (an AST/line visitor fires
//! for every occurrence of a marker regardless of syntactic position) by
//! guarding each marker match to the required source position.
//! rule's intent actually requires.
//!
//! This crate does NOT own: the `Validator` trait or fixture/parity
//! harness (`enforcer-validator`), the rule registry (`enforcer-rules`),
//! or the shared `generic-scanner` engine / cross-language partition
//! (arc-09).
//!
//! No `pub use` barrels (workspace doctrine): consumers path through the
//! modules directly, e.g. `enforcer_lang_py::source_scan::all`.

pub mod generic_scanner_slice;
pub mod rules;
pub mod source_scan;
pub mod test_scan;
pub mod tests_required;
pub mod toolchain;

/// Build every validator this crate registers, across all six modules.
/// Order is not significant to any consumer -- `enforcer-scan`
/// (arc-14/f05) dispatches by `RuleId`, not by vec position.
pub fn all_validators() -> Result<
    Vec<Box<dyn enforcer_validator::validator::Validator>>,
    enforcer_domain::boundary::decode_error::DecodeError,
> {
    let mut validators = Vec::new();
    validators.extend(source_scan::all()?);
    validators.extend(test_scan::all()?);
    validators.extend(toolchain::all()?);
    validators.extend(generic_scanner_slice::all()?);
    validators.extend(tests_required::all()?);
    Ok(validators)
}

#[cfg(test)]
mod registry_coverage {
    //! Count-parity assertion (workpack requirement): every
    //! `language == "python"` `ruleId` in `rules/rules.json` MUST have a
    //! registered [`enforcer_validator::validator::Validator`] in this
    //! crate, and the loaded count MUST equal 61. A new PY rule added to
    //! `rules.json` without a matching validator+fixtures fails this test,
    //! not silently passing.

    use std::collections::BTreeSet;

    use crate::all_validators;

    #[test]
    fn every_python_rule_id_has_a_registered_validator() -> Result<(), Box<dyn std::error::Error>> {
        let catalog_ids = crate::boundary::fixture::python_catalog_rule_ids()?;
        assert_eq!(
            catalog_ids.len(),
            61,
            "rules/rules.json language==python count drifted from the workpack's declared 61"
        );

        let validators = all_validators()?;
        let registered_ids: BTreeSet<_> = validators
            .iter()
            .map(|validator| validator.rule_id().clone())
            .collect();

        assert_eq!(
            registered_ids.len(),
            validators.len(),
            "a RuleId was registered by more than one validator in this crate"
        );

        let missing: Vec<_> = catalog_ids.difference(&registered_ids).collect();
        assert!(
            missing.is_empty(),
            "python ruleIds with no registered enforcer-lang-py validator: {missing:?}"
        );

        let extra: Vec<_> = registered_ids.difference(&catalog_ids).collect();
        assert!(
            extra.is_empty(),
            "enforcer-lang-py registered a RuleId not present as language==python in rules.json: {extra:?}"
        );

        assert_eq!(
            registered_ids.len(),
            61,
            "enforcer-lang-py must register exactly 61 python validators"
        );
        Ok(())
    }
}
pub(crate) mod boundary;
