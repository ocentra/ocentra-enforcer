//! d16 acceptance proof (registry leg): the `rules/fsm.json` catalog loads
//! into the registry, every FSM ruleId resolves with non-empty
//! validator/fixture/doc-anchor linkage, and the tier split matches the
//! workpack (6 T1 blocking rules, 4 T2 scored rules).

use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::loader::parse_catalog;

const FSM_JSON: &str = include_str!("../rules/fsm.json");

fn fsm_records() -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    let raw = RuleCatalogJson::try_from(FSM_JSON.to_owned())?;
    let source = RuleCatalogSource::try_from("rules/fsm.json".to_owned())?;
    Ok(parse_catalog(&raw, &source)?)
}

#[test]
fn fsm_catalog_loads_ten_records() -> Result<(), Box<dyn std::error::Error>> {
    let records = fsm_records()?;
    assert_eq!(records.len(), 10);
    Ok(())
}

#[test]
fn fsm_catalog_loads_into_registry_with_no_duplicates() -> Result<(), Box<dyn std::error::Error>> {
    let records = fsm_records()?;
    let registry = enforcer_rules::loader::load_registry_from_records(records)?;
    assert_eq!(registry.iter().count(), 10);
    Ok(())
}

#[test]
fn every_fsm_record_links_to_enforcer_lang_common_with_full_linkage(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = fsm_records()?;
    for record in &records {
        assert_eq!(record.validator.crate_name.as_str(), "enforcer-lang-common");
        assert!(record.validator.path.as_str().starts_with("rules::fsm::"));
        assert!(record
            .fixtures
            .fail
            .as_str()
            .starts_with("crates/enforcer-lang-common/tests/fixtures/fsm/"));
        assert!(record
            .fixtures
            .pass
            .as_str()
            .starts_with("crates/enforcer-lang-common/tests/fixtures/fsm/"));
        assert!(record.doc_anchor.as_str().starts_with(
            "docs/plans/enforcer-selfhost-plan/workpacks/d16-fsm-transition-validity.md"
        ));
    }
    Ok(())
}

#[test]
fn fsm_tier_split_matches_workpack_six_t1_four_t2() -> Result<(), Box<dyn std::error::Error>> {
    let records = fsm_records()?;
    let t1 = records
        .iter()
        .filter(|r| r.tier == enforcer_domain::severity::Tier::T1)
        .count();
    let t2 = records
        .iter()
        .filter(|r| r.tier == enforcer_domain::severity::Tier::T2)
        .count();
    assert_eq!(t1, 6, "expected 6 T1 blocking FSM rules");
    assert_eq!(t2, 4, "expected 4 T2 scored FSM rules");
    Ok(())
}

#[test]
fn fsm_catalog_has_no_t3_rules() -> Result<(), Box<dyn std::error::Error>> {
    let records = fsm_records()?;
    assert!(records
        .iter()
        .all(|r| r.tier != enforcer_domain::severity::Tier::T3));
    Ok(())
}
