//! d22 acceptance proof (registry leg): the `rules/size-shape.json` catalog
//! loads into the registry, every SIZE/FE-LEN ruleId resolves with
//! non-empty validator/fixture/doc-anchor linkage, and the tier split
//! matches the workpack (6 T1 blocking rules, 3 T2 scored rules).

use enforcer_rules::loader::parse_catalog;

const SIZE_SHAPE_JSON: &str = include_str!("../rules/size-shape.json");

fn size_shape_records(
) -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    Ok(parse_catalog(SIZE_SHAPE_JSON, "rules/size-shape.json")?)
}

#[test]
fn size_shape_catalog_loads_nine_records() -> Result<(), Box<dyn std::error::Error>> {
    let records = size_shape_records()?;
    assert_eq!(records.len(), 9);
    Ok(())
}

#[test]
fn size_shape_catalog_loads_into_registry_with_no_duplicates(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = size_shape_records()?;
    let registry = enforcer_rules::loader::load_registry_from_records(records)?;
    assert_eq!(registry.len(), 9);
    Ok(())
}

#[test]
fn every_size_shape_record_links_to_enforcer_lang_common_with_full_linkage(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = size_shape_records()?;
    for record in &records {
        assert_eq!(record.validator.crate_name, "enforcer-lang-common");
        assert!(record.validator.path.starts_with("rules::size_shape::"));
        assert!(!record.fixtures.fail.is_empty());
        assert!(!record.fixtures.pass.is_empty());
        assert!(record
            .fixtures
            .fail
            .starts_with("crates/enforcer-lang-common/tests/fixtures/size_shape/"));
        assert!(record
            .fixtures
            .pass
            .starts_with("crates/enforcer-lang-common/tests/fixtures/size_shape/"));
        assert!(record.doc_anchor.contains("d22-size-shape-caps.md"));
    }
    Ok(())
}

#[test]
fn size_shape_tier_split_matches_workpack_six_t1_three_t2() -> Result<(), Box<dyn std::error::Error>>
{
    let records = size_shape_records()?;
    let t1 = records
        .iter()
        .filter(|r| r.tier == enforcer_domain::severity::Tier::T1)
        .count();
    let t2 = records
        .iter()
        .filter(|r| r.tier == enforcer_domain::severity::Tier::T2)
        .count();
    assert_eq!(t1, 6, "expected 6 T1 blocking size/shape rules");
    assert_eq!(t2, 3, "expected 3 T2 scored size/shape rules");
    Ok(())
}

#[test]
fn size_shape_catalog_has_no_t3_rules() -> Result<(), Box<dyn std::error::Error>> {
    let records = size_shape_records()?;
    assert!(records
        .iter()
        .all(|r| r.tier != enforcer_domain::severity::Tier::T3));
    Ok(())
}
