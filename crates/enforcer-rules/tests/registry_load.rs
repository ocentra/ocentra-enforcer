//! arc-04 acceptance proof: the shipped baseline rule catalogs
//! (`rules/deny-wall.json`, `rules/no-reexports.json`,
//! `rules/ocentra-parent-posture.json`, `rules/deferred-work-gate.json`)
//! load into one registry, every record's linkage resolves, a
//! malformed/duplicate record is rejected, and a seeded d13 version-drift
//! fails closed.

use enforcer_rules::loader::{load_registry_from_records, parse_catalog};
use enforcer_rules::version_drift::{check_drift, has_drift, DriftOutcome};

const DENY_WALL_JSON: &str = include_str!("../rules/deny-wall.json");
const NO_REEXPORTS_JSON: &str = include_str!("../rules/no-reexports.json");
const OCENTRA_PARENT_POSTURE_JSON: &str = include_str!("../rules/ocentra-parent-posture.json");
const DEFERRED_WORK_GATE_JSON: &str = include_str!("../rules/deferred-work-gate.json");

fn all_baseline_records(
) -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    let mut records = parse_catalog(DENY_WALL_JSON, "rules/deny-wall.json")?;
    records.extend(parse_catalog(NO_REEXPORTS_JSON, "rules/no-reexports.json")?);
    records.extend(parse_catalog(
        OCENTRA_PARENT_POSTURE_JSON,
        "rules/ocentra-parent-posture.json",
    )?);
    records.extend(parse_catalog(
        DEFERRED_WORK_GATE_JSON,
        "rules/deferred-work-gate.json",
    )?);
    Ok(records)
}

#[test]
fn baseline_catalogs_load_into_one_registry() -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let registry = load_registry_from_records(records)?;
    // 1 deny-wall + 1 no-reexports + 4 ocentra-parent posture + 1
    // deferred-work-gate records.
    assert_eq!(registry.len(), 7);
    Ok(())
}

#[test]
fn deferred_work_gate_record_loads_and_is_linked_to_lang_common_validator(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let registry = load_registry_from_records(records)?;
    let rule_id = "DEFER-1.1".parse()?;
    let record = registry.get(&rule_id).ok_or("expected DEFER-1.1 to load")?;
    assert_eq!(record.validator.crate_name, "enforcer-lang-common");
    assert!(record.validator.path.contains("DeferredWorkValidator"));
    assert!(!record.fixtures.fail.is_empty());
    assert!(!record.fixtures.pass.is_empty());
    assert!(!record.doc_anchor.is_empty());
    Ok(())
}

#[test]
fn deny_wall_record_loads_and_resolves() -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let registry = load_registry_from_records(records)?;
    let rule_id = "T1-DENYWALL.1".parse()?;
    let record = registry
        .get(&rule_id)
        .ok_or("expected T1-DENYWALL.1 to load")?;
    assert_eq!(record.validator.crate_name, "enforcer-lang-rust");
    assert!(!record.fixtures.fail.is_empty());
    assert!(!record.fixtures.pass.is_empty());
    assert!(!record.doc_anchor.is_empty());
    assert_eq!(record.tier, enforcer_domain::severity::Tier::T1);
    Ok(())
}

#[test]
fn no_reexports_record_loads_and_is_linked_to_lang_rust_validator(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let registry = load_registry_from_records(records)?;
    let rule_id = "T1-NOREEXPORT.1".parse()?;
    let record = registry
        .get(&rule_id)
        .ok_or("expected T1-NOREEXPORT.1 to load")?;
    assert_eq!(record.validator.crate_name, "enforcer-lang-rust");
    assert!(record.validator.path.contains("NoReexportsValidator"));
    Ok(())
}

#[test]
fn ocentra_parent_posture_yields_all_four_expected_records(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let registry = load_registry_from_records(records)?;

    let expected_ids = [
        "T1-PARENTPOSTURE.1",
        "T1-PARENTPOSTURE.2",
        "T1-PARENTPOSTURE.3",
        "T1-PARENTPOSTURE.4",
    ];
    for id in expected_ids {
        let rule_id = id.parse()?;
        let record = registry
            .get(&rule_id)
            .ok_or_else(|| format!("expected {id} to load"))?;
        assert!(!record.validator.path.is_empty());
        assert!(!record.fixtures.fail.is_empty());
        assert!(!record.fixtures.pass.is_empty());
        assert!(!record.doc_anchor.is_empty());
    }

    // publicReexportPolicy: forbid params resolve on the reexport posture
    // record specifically.
    let reexport_posture = registry
        .get(&"T1-PARENTPOSTURE.1".parse()?)
        .ok_or("expected T1-PARENTPOSTURE.1")?;
    assert_eq!(
        reexport_posture.params["publicReexportPolicy"],
        serde_json::json!("forbid")
    );

    // blockedProtocolDependencies posture record references the
    // enforcer-config substrate rather than redefining it.
    let dependency_posture = registry
        .get(&"T1-PARENTPOSTURE.4".parse()?)
        .ok_or("expected T1-PARENTPOSTURE.4")?;
    assert!(dependency_posture.params["configSubstrate"]
        .as_str()
        .ok_or("expected configSubstrate to be a string")?
        .contains("CargoDependencyPolicy"));

    Ok(())
}

#[test]
fn a_missing_posture_record_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let registry = load_registry_from_records(records)?;
    let missing = "T1-PARENTPOSTURE.99".parse()?;
    assert!(registry.get(&missing).is_none());
    Ok(())
}

#[test]
fn a_renamed_posture_record_id_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate a renamed rule id: the old id no longer resolves.
    let records = all_baseline_records()?;
    let registry = load_registry_from_records(records)?;
    let renamed = "T1-PARENT-POSTURE-ONE".parse()?;
    assert!(registry.get(&renamed).is_none());
    Ok(())
}

#[test]
fn malformed_record_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let malformed = r#"[
        {
            "ruleId": "RR-BAD.1",
            "version": 1,
            "title": "",
            "tier": "T1",
            "validator": { "crateName": "", "path": "" },
            "fixtures": { "fail": "", "pass": "" },
            "docAnchor": ""
        }
    ]"#;
    let records = parse_catalog(malformed, "<inline>")?;
    assert!(load_registry_from_records(records).is_err());
    Ok(())
}

#[test]
fn duplicate_rule_id_across_catalogs_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = all_baseline_records()?;
    let clone_of_first = records[0].clone();
    records.push(clone_of_first);
    assert!(load_registry_from_records(records).is_err());
    Ok(())
}

#[test]
fn seeded_version_drift_fails_closed_on_content_change_without_bump(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let baseline = records
        .iter()
        .find(|r| r.rule_id.as_str() == "T1-NOREEXPORT.1")
        .ok_or("expected baseline record")?
        .clone();

    // Seed a drift: bump the doc anchor (a parity artifact) without
    // bumping `version`.
    let mut candidate = baseline.clone();
    candidate.doc_anchor = format!("{}-moved", baseline.doc_anchor);

    assert_eq!(
        check_drift(&baseline, &candidate),
        DriftOutcome::ContentChangedVersionNotBumped
    );
    assert!(has_drift(&baseline, &candidate));
    Ok(())
}

#[test]
fn seeded_version_drift_fails_closed_on_hollow_version_bump(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let baseline = records
        .iter()
        .find(|r| r.rule_id.as_str() == "T1-DENYWALL.1")
        .ok_or("expected baseline record")?
        .clone();

    // Seed a drift: bump `version` with no matching fixture/anchor change.
    let mut candidate = baseline.clone();
    candidate.version += 1;

    assert_eq!(
        check_drift(&baseline, &candidate),
        DriftOutcome::VersionBumpedContentUnchanged
    );
    assert!(has_drift(&baseline, &candidate));
    Ok(())
}

#[test]
fn legitimate_version_bump_matching_content_change_is_clean(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let baseline = records
        .iter()
        .find(|r| r.rule_id.as_str() == "T1-PARENTPOSTURE.2")
        .ok_or("expected baseline record")?
        .clone();

    let mut candidate = baseline.clone();
    candidate.fixtures.fail = format!("{}.v2", baseline.fixtures.fail);
    candidate.version += 1;

    assert_eq!(
        check_drift(&baseline, &candidate),
        DriftOutcome::ContentChangedVersionBumped
    );
    assert!(!has_drift(&baseline, &candidate));
    Ok(())
}
