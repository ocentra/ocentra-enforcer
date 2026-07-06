//! d10 acceptance proof (registry leg): the `rules/resilience.json`
//! catalog loads into the registry, every resilience ruleId resolves with
//! non-empty validator/fixture/doc-anchor linkage, and the tier split
//! matches the workpack (1 T1 blocking required-test obligation, 2 T2
//! scored smells).

use enforcer_rules::loader::parse_catalog;

const RESILIENCE_JSON: &str = include_str!("../rules/resilience.json");

fn resilience_records(
) -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    Ok(parse_catalog(RESILIENCE_JSON, "rules/resilience.json")?)
}

#[test]
fn resilience_catalog_loads_three_records() -> Result<(), Box<dyn std::error::Error>> {
    let records = resilience_records()?;
    assert_eq!(records.len(), 3);
    Ok(())
}

#[test]
fn resilience_catalog_loads_into_registry_with_no_duplicates(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = resilience_records()?;
    let registry = enforcer_rules::loader::load_registry_from_records(records)?;
    assert_eq!(registry.len(), 3);
    Ok(())
}

#[test]
fn every_resilience_record_links_to_enforcer_lang_common_with_full_linkage(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = resilience_records()?;
    for record in &records {
        assert_eq!(record.validator.crate_name, "enforcer-lang-common");
        assert!(record.validator.path.starts_with("rules::resilience::"));
        assert!(!record.fixtures.fail.is_empty());
        assert!(!record.fixtures.pass.is_empty());
        assert!(record
            .fixtures
            .fail
            .starts_with("crates/enforcer-lang-common/tests/fixtures/resilience/"));
        assert!(record
            .fixtures
            .pass
            .starts_with("crates/enforcer-lang-common/tests/fixtures/resilience/"));
        assert!(record.doc_anchor.contains("d10-resilience-auditor.md"));
    }
    Ok(())
}

#[test]
fn resilience_tier_split_matches_workpack_one_t1_two_t2() -> Result<(), Box<dyn std::error::Error>>
{
    let records = resilience_records()?;
    let t1 = records
        .iter()
        .filter(|r| r.tier == enforcer_domain::severity::Tier::T1)
        .count();
    let t2 = records
        .iter()
        .filter(|r| r.tier == enforcer_domain::severity::Tier::T2)
        .count();
    assert_eq!(t1, 1, "expected 1 T1 blocking required-test obligation");
    assert_eq!(t2, 2, "expected 2 T2 scored smells");
    Ok(())
}

#[test]
fn resilience_catalog_has_no_t3_rules() -> Result<(), Box<dyn std::error::Error>> {
    let records = resilience_records()?;
    assert!(records
        .iter()
        .all(|r| r.tier != enforcer_domain::severity::Tier::T3));
    Ok(())
}
