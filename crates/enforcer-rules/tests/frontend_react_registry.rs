//! e-pack-frontend-react acceptance proof (registry leg): the
//! `rules/frontend-react.json` catalog loads into the registry, every
//! `FE-*` ruleId resolves with non-empty validator/fixture/doc-anchor
//! linkage, and the tier split matches the workpack (12 T1 blocking rules,
//! 1 T2 scored layer-inversion advisory).

use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::loader::parse_catalog;

const FRONTEND_REACT_JSON: &str = include_str!("../rules/frontend-react.json");

fn frontend_react_records(
) -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    let raw = RuleCatalogJson::try_from(FRONTEND_REACT_JSON.to_owned())?;
    let source = RuleCatalogSource::try_from("rules/frontend-react.json".to_owned())?;
    Ok(parse_catalog(&raw, &source)?)
}

#[test]
fn frontend_react_catalog_loads_thirteen_records() -> Result<(), Box<dyn std::error::Error>> {
    let records = frontend_react_records()?;
    assert_eq!(records.len(), 13);
    Ok(())
}

#[test]
fn frontend_react_catalog_loads_into_registry_with_no_duplicates(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = frontend_react_records()?;
    let registry = enforcer_rules::loader::load_registry_from_records(records)?;
    assert_eq!(registry.iter().count(), 13);
    Ok(())
}

#[test]
fn every_frontend_react_record_links_to_enforcer_lang_ts_with_full_linkage(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = frontend_react_records()?;
    for record in &records {
        assert_eq!(record.validator.crate_name.as_str(), "enforcer-lang-ts");
        assert!(record
            .validator
            .path
            .as_str()
            .starts_with("rules::frontend_react::"));
        assert!(record
            .fixtures
            .fail
            .as_str()
            .starts_with("crates/enforcer-lang-ts/tests/fixtures/frontend_react/"));
        assert!(record
            .fixtures
            .pass
            .as_str()
            .starts_with("crates/enforcer-lang-ts/tests/fixtures/frontend_react/"));
        assert!(record
            .doc_anchor
            .as_str()
            .starts_with("docs/plans/enforcer-selfhost-plan/workpacks/e-pack-frontend-react.md"));
    }
    Ok(())
}

#[test]
fn frontend_react_tier_split_matches_workpack_twelve_t1_one_t2(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = frontend_react_records()?;
    let t1 = records
        .iter()
        .filter(|r| r.tier == enforcer_domain::severity::Tier::T1)
        .count();
    let t2 = records
        .iter()
        .filter(|r| r.tier == enforcer_domain::severity::Tier::T2)
        .count();
    assert_eq!(t1, 12, "expected 12 T1 blocking frontend-react rules");
    assert_eq!(
        t2, 1,
        "expected 1 T2 scored frontend-react rule (FE-ARCH-1.4)"
    );
    Ok(())
}

#[test]
fn frontend_react_catalog_has_no_t3_rules() -> Result<(), Box<dyn std::error::Error>> {
    let records = frontend_react_records()?;
    assert!(records
        .iter()
        .all(|r| r.tier != enforcer_domain::severity::Tier::T3));
    Ok(())
}

#[test]
fn fe_arch_1_4_is_the_sole_t2_scored_advisory() -> Result<(), Box<dyn std::error::Error>> {
    let records = frontend_react_records()?;
    let record = records
        .iter()
        .find(|r| r.rule_id.as_str() == "FE-ARCH-1.4")
        .ok_or("expected FE-ARCH-1.4 to load")?;
    assert_eq!(record.tier, enforcer_domain::severity::Tier::T2);
    Ok(())
}

/// Pins the doctrine-divergence rule mechanically: `FE-EFFECT-1.1` must be
/// present, T1, and linked to the `EffectNotZodValidator`.
#[test]
fn fe_effect_1_1_divergence_rule_is_present_and_linked() -> Result<(), Box<dyn std::error::Error>> {
    let records = frontend_react_records()?;
    let record = records
        .iter()
        .find(|r| r.rule_id.as_str() == "FE-EFFECT-1.1")
        .ok_or("expected FE-EFFECT-1.1 to load")?;
    assert_eq!(record.tier, enforcer_domain::severity::Tier::T1);
    assert!(record
        .validator
        .path
        .as_str()
        .ends_with("EffectNotZodValidator"));
    Ok(())
}
