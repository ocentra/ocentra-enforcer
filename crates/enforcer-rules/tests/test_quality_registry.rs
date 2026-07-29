//! d23 acceptance proof (registry leg): the `rules/test-quality.json`
//! catalog loads into the registry, every TEST-* ruleId resolves with
//! non-empty validator/fixture/doc-anchor linkage, and the tier split
//! matches the workpack (3 T1 blocking rules, 5 T2 scored rules).

use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::loader::parse_catalog;

const TEST_QUALITY_JSON: &str = include_str!("../rules/test-quality.json");

fn test_quality_records(
) -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    let raw = RuleCatalogJson::try_from(TEST_QUALITY_JSON.to_owned())?;
    let source = RuleCatalogSource::try_from("rules/test-quality.json".to_owned())?;
    Ok(parse_catalog(&raw, &source)?)
}

#[test]
fn test_quality_catalog_loads_eight_records() -> Result<(), Box<dyn std::error::Error>> {
    let records = test_quality_records()?;
    assert_eq!(records.len(), 8);
    Ok(())
}

#[test]
fn test_quality_catalog_loads_into_registry_with_no_duplicates(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = test_quality_records()?;
    let registry = enforcer_rules::loader::load_registry_from_records(records)?;
    assert_eq!(registry.iter().count(), 8);
    Ok(())
}

#[test]
fn every_test_quality_record_links_to_enforcer_lang_common_with_full_linkage(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = test_quality_records()?;
    for record in &records {
        assert_eq!(record.validator.crate_name.as_str(), "enforcer-lang-common");
        assert!(record
            .validator
            .path
            .as_str()
            .starts_with("rules::test_quality::"));
        assert!(record
            .fixtures
            .fail
            .as_str()
            .starts_with("crates/enforcer-lang-common/tests/fixtures/test_quality/"));
        assert!(record
            .fixtures
            .pass
            .as_str()
            .starts_with("crates/enforcer-lang-common/tests/fixtures/test_quality/"));
        assert!(record.doc_anchor.as_str().starts_with(
            "docs/plans/enforcer-selfhost-plan/workpacks/d23-test-companion-and-quality.md"
        ));
    }
    Ok(())
}

#[test]
fn test_quality_tier_split_matches_workpack_three_t1_five_t2(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = test_quality_records()?;
    let t1 = records
        .iter()
        .filter(|r| r.tier == enforcer_domain::severity::Tier::T1)
        .count();
    let t2 = records
        .iter()
        .filter(|r| r.tier == enforcer_domain::severity::Tier::T2)
        .count();
    assert_eq!(t1, 3, "expected 3 T1 blocking test-quality rules");
    assert_eq!(t2, 5, "expected 5 T2 scored test-quality rules");
    Ok(())
}

#[test]
fn test_quality_catalog_has_no_t3_rules() -> Result<(), Box<dyn std::error::Error>> {
    let records = test_quality_records()?;
    assert!(records
        .iter()
        .all(|r| r.tier != enforcer_domain::severity::Tier::T3));
    Ok(())
}
