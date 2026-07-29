//! Proves this crate's rule records actually load into the
//! `enforcer-rules` registry (arc-04 linkage), and that every registered
//! `Validator` (`enforcer_lang_cfml::all_validators`) has a matching
//! record whose `tags` carry the `coldfusion`/`cfml` markers -- the
//! closest honest proxy this crate can assert for "the `coldfusion`
//! language / `.cfc`/`.cfm` extensions are registered", since the current
//! `enforcer-rules::RuleRecord` schema (arc-04) does not yet carry an
//! `appliesTo`/language field of its own (that is a documentation-era
//! `.mjs` concept the Rust registry has not ported). Additionally, every
//! rule whose validator inspects CFML SOURCE (as opposed to a JSON/YAML
//! toolchain manifest such as `box.json`/`.cflintrc`/CI config) must have
//! at least one `.cfc`/`.cfm` fixture -- this crate's actual evidence
//! that real CFML source is covered.
//!
//! This test file also proves the T3 advisory row (`CF-ARCH-6.1`) carries
//! the verbatim `advisory, no mechanization possible + <reason>` label
//! the d01 parity oracle checks for.

use std::path::{Path, PathBuf};

use enforcer_lang_cfml::all_validators;
use enforcer_rules::loader::load_registry_from_files;

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // CARGO_MANIFEST_DIR is `<repo>/crates/enforcer-lang-cfml`.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("CARGO_MANIFEST_DIR has no crates/ parent")?
        .parent()
        .ok_or("CARGO_MANIFEST_DIR has no repo root")?
        .to_path_buf();
    Ok(root)
}

fn cfml_catalog_paths(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join("crates/enforcer-rules/rules/cfml.json"),
        root.join("crates/enforcer-rules/rules/cfml-advisory.json"),
    ]
}

#[test]
fn cfml_rule_catalogs_load_into_the_registry() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let paths = cfml_catalog_paths(&root);
    let path_refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    let registry = load_registry_from_files(&path_refs)?;
    let record_count = registry.iter().count();
    assert!(
        record_count >= 22,
        "expected every CF-*/CFML-* rule record (T1+T2+T3) to load, got {}",
        record_count
    );
    Ok(())
}

/// Rule ids whose validator legitimately inspects a JSON/YAML TOOLCHAIN
/// manifest (`box.json`, `.cflintrc`, CI config, TestBox coverage config)
/// rather than CFML source text -- these are exempt from the `.cfc`/`.cfm`
/// fixture-extension check below, mirroring
/// `enforcer-lang-dart::tests::fixture_parity`'s own YAML-manifest rules.
const MANIFEST_SCOPED_RULE_IDS: &[&str] =
    &["CF-TOOL-1.1", "CF-DEP-1.1", "CF-TOOL-2.1", "CF-CI-2.1"];

#[test]
fn every_fixture_provable_validator_has_a_matching_coldfusion_tagged_record(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let paths = cfml_catalog_paths(&root);
    let path_refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    let registry = load_registry_from_files(&path_refs)?;

    let validators = all_validators()?;
    for validator in &validators {
        let rule_id = validator.rule_id().to_string();
        let record = registry
            .get(validator.rule_id())
            .ok_or_else(|| format!("no registry record for `{rule_id}`"))?;
        assert!(
            record.tags.iter().any(|tag| tag.as_str() == "coldfusion"),
            "record `{rule_id}` is missing the `coldfusion` language tag"
        );
        if MANIFEST_SCOPED_RULE_IDS.contains(&rule_id.as_str()) {
            continue;
        }
        let fixtures_are_cfml = record.fixtures.fail.as_str().ends_with(".cfc")
            || record.fixtures.fail.as_str().ends_with(".cfm")
            || record.fixtures.pass.as_str().ends_with(".cfc")
            || record.fixtures.pass.as_str().ends_with(".cfm");
        assert!(
            fixtures_are_cfml,
            "record `{rule_id}` fixtures are not `.cfc`/`.cfm`: {:?}",
            record.fixtures
        );
    }
    Ok(())
}

#[test]
fn t3_advisory_row_carries_the_no_mechanization_label() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let advisory_path = root.join("crates/enforcer-rules/rules/cfml-advisory.json");
    let registry = load_registry_from_files(&[advisory_path.as_path()])?;
    let rule_id: enforcer_domain::ids::RuleId = "CF-ARCH-6.1".parse()?;
    let record = registry
        .get(&rule_id)
        .ok_or("CF-ARCH-6.1 must be registered")?;
    assert!(
        record.tags.iter().any(|tag| tag
            .as_str()
            .starts_with("advisory, no mechanization possible")),
        "T3 row `CF-ARCH-6.1` must carry the verbatim advisory label"
    );
    Ok(())
}
