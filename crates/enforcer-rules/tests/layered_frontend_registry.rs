//! d12 acceptance proof (registry leg): the `rules/layered-frontend.json`
//! catalog loads into the registry, every `LFE-*` ruleId resolves with
//! non-empty validator/fixture/doc-anchor linkage, and the tier split
//! matches the workpack (5 T1 blocking rules, ADBP_PARITY_MATRIX rows
//! FRONT-01..FRONT-05).

use enforcer_rules::loader::parse_catalog;

const LAYERED_FRONTEND_JSON: &str = include_str!("../rules/layered-frontend.json");

fn layered_frontend_records(
) -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    Ok(parse_catalog(
        LAYERED_FRONTEND_JSON,
        "rules/layered-frontend.json",
    )?)
}

#[test]
fn layered_frontend_catalog_loads_five_records() -> Result<(), Box<dyn std::error::Error>> {
    let records = layered_frontend_records()?;
    assert_eq!(records.len(), 5);
    Ok(())
}

#[test]
fn layered_frontend_catalog_loads_into_registry_with_no_duplicates(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = layered_frontend_records()?;
    let registry = enforcer_rules::loader::load_registry_from_records(records)?;
    assert_eq!(registry.len(), 5);
    Ok(())
}

#[test]
fn every_layered_frontend_record_links_to_enforcer_lang_ts_with_full_linkage(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = layered_frontend_records()?;
    for record in &records {
        assert_eq!(record.validator.crate_name, "enforcer-lang-ts");
        assert!(record
            .validator
            .path
            .starts_with("rules::layered_frontend::"));
        assert!(!record.fixtures.fail.is_empty());
        assert!(!record.fixtures.pass.is_empty());
        assert!(record
            .fixtures
            .fail
            .starts_with("crates/enforcer-lang-ts/tests/fixtures/layered_frontend/"));
        assert!(record
            .fixtures
            .pass
            .starts_with("crates/enforcer-lang-ts/tests/fixtures/layered_frontend/"));
        assert!(record
            .doc_anchor
            .contains("d12-layered-and-frontend-ruleids.md"));
    }
    Ok(())
}

#[test]
fn layered_frontend_catalog_is_all_t1_no_t2_no_t3() -> Result<(), Box<dyn std::error::Error>> {
    let records = layered_frontend_records()?;
    assert_eq!(records.len(), 5);
    assert!(records
        .iter()
        .all(|r| r.tier == enforcer_domain::severity::Tier::T1));
    Ok(())
}

/// Pins the exact five ADBP_PARITY_MATRIX-named rule ids (FRONT-01..05)
/// mechanically, so a future edit can't silently drop or rename one.
#[test]
fn all_five_front_rule_ids_present() -> Result<(), Box<dyn std::error::Error>> {
    let records = layered_frontend_records()?;
    let ids: std::collections::BTreeSet<_> = records
        .iter()
        .map(|r| r.rule_id.as_str().to_owned())
        .collect();
    for expected in ["LFE-1.1", "LFE-1.2", "LFE-1.3", "LFE-1.4", "LFE-1.5"] {
        assert!(ids.contains(expected), "missing rule id {expected}");
    }
    Ok(())
}
