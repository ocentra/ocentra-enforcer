//! d21 acceptance proof (registry leg): the `rules/ownerset.json` catalog
//! loads into the registry and `OWNERSET-1.1`'s record links fully to its
//! `enforcer-lang-common` validator, fixtures, and L39 doc anchor.

use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::loader::parse_catalog;

const OWNERSET_JSON: &str = include_str!("../rules/ownerset.json");

fn ownerset_records(
) -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    let raw = RuleCatalogJson::try_from(OWNERSET_JSON.to_owned())?;
    let source = RuleCatalogSource::try_from("rules/ownerset.json".to_owned())?;
    Ok(parse_catalog(&raw, &source)?)
}

#[test]
fn ownerset_catalog_loads_one_record() -> Result<(), Box<dyn std::error::Error>> {
    let records = ownerset_records()?;
    assert_eq!(records.len(), 1);
    Ok(())
}

#[test]
fn ownerset_catalog_loads_into_registry_with_no_duplicates(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = ownerset_records()?;
    let registry = enforcer_rules::loader::load_registry_from_records(records)?;
    assert_eq!(registry.iter().count(), 1);
    Ok(())
}

#[test]
fn ownerset_record_links_to_enforcer_lang_common_with_full_linkage(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = ownerset_records()?;
    let record = &records[0];
    assert_eq!(record.rule_id.as_str(), "OWNERSET-1.1");
    assert_eq!(record.validator.crate_name.as_str(), "enforcer-lang-common");
    assert!(record
        .validator
        .path
        .as_str()
        .starts_with("rules::change_discipline::"));
    assert!(record
        .fixtures
        .fail
        .as_str()
        .starts_with("crates/enforcer-lang-common/tests/fixtures/change_discipline/"));
    assert!(record
        .fixtures
        .pass
        .as_str()
        .starts_with("crates/enforcer-lang-common/tests/fixtures/change_discipline/"));
    assert_eq!(
        record.doc_anchor.as_str(),
        "docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md#L39"
    );
    Ok(())
}

#[test]
fn ownerset_record_is_tier_t1_with_change_discipline_and_owner_set_tags(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = ownerset_records()?;
    let record = &records[0];
    assert_eq!(record.tier, enforcer_domain::severity::Tier::T1);
    assert!(record
        .tags
        .iter()
        .any(|t| t.as_str() == "change-discipline"));
    assert!(record.tags.iter().any(|t| t.as_str() == "owner-set"));
    Ok(())
}
