//! h12 acceptance proof: the `rules/cyberskills-adapters.json` catalog (the
//! one T2 severity-gate record over recorded engine-adapter output) loads
//! into a registry, its 5-way linkage resolves (ruleId <-> validator <->
//! fixtures <-> doc-anchor <-> tier), and its fixture files referenced
//! actually exist on disk.

use std::path::{Path, PathBuf};

use enforcer_rules::loader::{load_registry_from_records, parse_catalog};

const CYBERSKILLS_ADAPTERS_JSON: &str = include_str!("../rules/cyberskills-adapters.json");

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // CARGO_MANIFEST_DIR is `<repo>/crates/enforcer-rules`.
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

#[test]
fn cyberskills_adapters_catalog_loads_and_every_record_resolves(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = parse_catalog(CYBERSKILLS_ADAPTERS_JSON, "rules/cyberskills-adapters.json")?;
    assert_eq!(
        records.len(),
        1,
        "expected 1 h12 cyberskills-adapter rule record"
    );

    let registry = load_registry_from_records(records)?;
    assert_eq!(registry.len(), 1);

    let root = repo_root()?;
    let rule_id = "CYBER-ADAPTER-SCA-SEVERITY.1".parse()?;
    let record = registry
        .get(&rule_id)
        .ok_or("expected CYBER-ADAPTER-SCA-SEVERITY.1 to load")?;
    assert!(!record.validator.crate_name.is_empty());
    assert!(!record.validator.path.is_empty());
    assert!(!record.doc_anchor.is_empty());
    assert_eq!(record.tier, enforcer_domain::severity::Tier::T2);

    let fail_path = root.join(&record.fixtures.fail);
    let pass_path = root.join(&record.fixtures.pass);
    assert!(
        Path::new(&fail_path).is_file(),
        "fail fixture missing on disk: {}",
        fail_path.display()
    );
    assert!(
        Path::new(&pass_path).is_file(),
        "pass fixture missing on disk: {}",
        pass_path.display()
    );

    Ok(())
}

#[test]
fn cyberskills_adapters_catalog_has_no_duplicate_or_malformed_records(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = parse_catalog(CYBERSKILLS_ADAPTERS_JSON, "rules/cyberskills-adapters.json")?;
    let mut clone_of_first = records.clone();
    clone_of_first.push(records[0].clone());
    assert!(load_registry_from_records(clone_of_first).is_err());
    Ok(())
}
