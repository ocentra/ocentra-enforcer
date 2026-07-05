//! d21 acceptance proof (registry leg): the `rules/ownerset.json` catalog
//! loads into the registry and `OWNERSET-1.1`'s record links fully to its
//! `enforcer-lang-common` validator, fixtures, and L39 doc anchor.

use enforcer_rules::loader::parse_catalog;

const OWNERSET_JSON: &str = include_str!("../rules/ownerset.json");

fn ownerset_records() -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>>
{
    Ok(parse_catalog(OWNERSET_JSON, "rules/ownerset.json")?)
}

#[test]
fn ownerset_catalog_loads_one_record() -> Result<(), Box<dyn std::error::Error>> {
    let records = ownerset_records()?;
    assert_eq!(records.len(), 1);
    Ok(())
}

#[test]
fn ownerset_catalog_loads_into_registry_with_no_duplicates() -> Result<(), Box<dyn std::error::Error>>
{
    let records = ownerset_records()?;
    let registry = enforcer_rules::loader::load_registry_from_records(records)?;
    assert_eq!(registry.len(), 1);
    Ok(())
}

#[test]
fn ownerset_record_links_to_enforcer_lang_common_with_full_linkage(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = ownerset_records()?;
    let record = &records[0];
    assert_eq!(record.rule_id.as_str(), "OWNERSET-1.1");
    assert_eq!(record.validator.crate_name, "enforcer-lang-common");
    assert!(record
        .validator
        .path
        .starts_with("rules::change_discipline::"));
    assert!(!record.fixtures.fail.is_empty());
    assert!(!record.fixtures.pass.is_empty());
    assert!(record
        .fixtures
        .fail
        .starts_with("crates/enforcer-lang-common/tests/fixtures/change_discipline/"));
    assert!(record
        .fixtures
        .pass
        .starts_with("crates/enforcer-lang-common/tests/fixtures/change_discipline/"));
    assert!(record.doc_anchor.contains("orchestration-lessons.md"));
    assert!(record.doc_anchor.contains("L39"));
    Ok(())
}

#[test]
fn ownerset_record_is_tier_t1_with_change_discipline_and_owner_set_tags(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = ownerset_records()?;
    let record = &records[0];
    assert_eq!(record.tier, enforcer_domain::severity::Tier::T1);
    assert!(record.tags.iter().any(|t| t == "change-discipline"));
    assert!(record.tags.iter().any(|t| t == "owner-set"));
    Ok(())
}
