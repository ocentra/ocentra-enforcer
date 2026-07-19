//! h12 acceptance proof: the `rules/cyberskills-adapters.json` catalog (the
//! one T2 severity-gate record over recorded engine-adapter output) loads
//! into a registry, its 5-way linkage resolves (ruleId <-> validator <->
//! fixtures <-> doc-anchor <-> tier), and its fixture files referenced
//! actually exist on disk.

use std::path::{Path, PathBuf};

use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::loader::{load_registry_from_records, parse_catalog};

const CYBERSKILLS_ADAPTERS_JSON: &str = include_str!("../rules/cyberskills-adapters.json");

fn load_catalog() -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    let raw = RuleCatalogJson::try_from(CYBERSKILLS_ADAPTERS_JSON.to_owned())?;
    let source = RuleCatalogSource::try_from("rules/cyberskills-adapters.json".to_owned())?;
    Ok(parse_catalog(&raw, &source)?)
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // CARGO_MANIFEST_DIR is `<repo>/crates/enforcer-rules`.
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

#[test]
fn cyberskills_adapters_catalog_loads_and_every_record_resolves(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = load_catalog()?;
    assert_eq!(
        records.len(),
        1,
        "expected 1 h12 cyberskills-adapter rule record"
    );

    let registry = load_registry_from_records(records)?;
    assert_eq!(registry.iter().count(), 1);

    let root = repo_root()?;
    let rule_id = "CYBER-ADAPTER-SCA-SEVERITY.1".parse()?;
    let record = registry
        .get(&rule_id)
        .ok_or("expected CYBER-ADAPTER-SCA-SEVERITY.1 to load")?;
    assert_eq!(record.validator.crate_name.as_str(), "enforcer-harness");
    assert_eq!(
        record.validator.path.as_str(),
        "adapters::cyberskills::gate::SeverityThresholdGate"
    );
    assert_eq!(record.doc_anchor.as_str(), "docs/plans/enforcer-selfhost-plan/workpacks/h12-cyberskills-python-adapters.md#requirement-checklist");
    assert_eq!(record.tier, enforcer_domain::severity::Tier::T2);

    let fail_path = root.join(record.fixtures.fail.as_str());
    let pass_path = root.join(record.fixtures.pass.as_str());
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
    let records = load_catalog()?;
    let mut clone_of_first = records.clone();
    clone_of_first.push(records[0].clone());
    assert!(
        matches!(load_registry_from_records(clone_of_first), Err(enforcer_rules::RuleLoadError::DuplicateRuleId { rule_id }) if rule_id.as_str() == "CYBER-ADAPTER-SCA-SEVERITY.1")
    );
    Ok(())
}
