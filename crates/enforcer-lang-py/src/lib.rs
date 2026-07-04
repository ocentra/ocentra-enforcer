//! `enforcer-lang-py` -- arc-08: the Python-family `Validator` crate.
//!
//! # Charter
//!
//! Implements every `language == "python"` rule from `rules/rules.json`
//! (61 rules across the PY-1..PY-6 prefixes) against
//! `enforcer-validator`'s [`enforcer_validator::validator::Validator`]
//! trait. Each rule is a [`line_marker::LineMarkerValidator`] entry or a
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
//!
//! [`line_marker`] is the shared line-scan engine [`source_scan`] and
//! [`test_scan`] build their entries on; it exists specifically to dodge
//! the mem-arc-06-0002 double-dispatch gotcha (an AST/line visitor fires
//! for every occurrence of a marker regardless of syntactic position) by
//! guarding each marker match to the position ([`line_marker::Guard`]) the
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
pub mod line_marker;
pub mod source_scan;
pub mod test_scan;
pub mod tests_required;
pub mod toolchain;

/// Build every validator this crate registers, across all six modules.
/// Order is not significant to any consumer -- `enforcer-scan`
/// (arc-14/f05) dispatches by `RuleId`, not by vec position.
pub fn all_validators(
) -> Result<Vec<Box<dyn enforcer_validator::validator::Validator>>, enforcer_core::error::DecodeError>
{
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
    use std::path::PathBuf;

    use crate::all_validators;

    /// Repo-relative path from this crate's manifest dir up to the
    /// workspace-root `rules/rules.json` catalog.
    fn rules_json_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("rules/rules.json")
    }

    /// Every `ruleId` in `rules/rules.json` whose `"language"` is
    /// `"python"`, parsed with plain `serde_json::Value` indexing (this
    /// crate does not depend on the legacy catalog's full typed shape --
    /// only `id`/`language` are read here).
    fn python_rule_ids_from_catalog() -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
        let raw = std::fs::read_to_string(rules_json_path())?;
        let parsed: serde_json::Value = serde_json::from_str(&raw)?;
        let rules = parsed
            .get("rules")
            .and_then(serde_json::Value::as_array)
            .ok_or("rules/rules.json missing top-level `rules` array")?;
        let mut ids = BTreeSet::new();
        for rule in rules {
            let language = rule.get("language").and_then(serde_json::Value::as_str);
            if language != Some("python") {
                continue;
            }
            let id = rule
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or("python rule record missing `id`")?;
            ids.insert(id.to_owned());
        }
        Ok(ids)
    }

    #[test]
    fn every_python_rule_id_has_a_registered_validator() -> Result<(), Box<dyn std::error::Error>> {
        let catalog_ids = python_rule_ids_from_catalog()?;
        assert_eq!(
            catalog_ids.len(),
            61,
            "rules/rules.json language==python count drifted from the workpack's declared 61"
        );

        let validators = all_validators()?;
        let registered_ids: BTreeSet<String> = validators
            .iter()
            .map(|validator| validator.rule_id().to_string())
            .collect();

        assert_eq!(
            registered_ids.len(),
            validators.len(),
            "a RuleId was registered by more than one validator in this crate"
        );

        let missing: Vec<&String> = catalog_ids.difference(&registered_ids).collect();
        assert!(
            missing.is_empty(),
            "python ruleIds with no registered enforcer-lang-py validator: {missing:?}"
        );

        let extra: Vec<&String> = registered_ids.difference(&catalog_ids).collect();
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
