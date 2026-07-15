//! arc-04 acceptance proof: the shipped baseline rule catalogs
//! (`rules/deny-wall.json`, `rules/no-reexports.json`,
//! `rules/ocentra-parent-posture.json`, `rules/deferred-work-gate.json`)
//! load into one registry, every record's linkage resolves, a
//! malformed/duplicate record is rejected, and a seeded d13 version-drift
//! fails closed.

use enforcer_domain::rules_types::VersionDriftOutcome;
use enforcer_domain::rules_types::{RuleCatalogJson, RuleCatalogSource};
use enforcer_rules::loader::{load_registry_from_records, parse_catalog};
use enforcer_rules::version_drift::{check_drift, has_drift};
use enforcer_rules::RuleLoadError;

const DENY_WALL_JSON: &str = include_str!("../rules/deny-wall.json");
const NO_REEXPORTS_JSON: &str = include_str!("../rules/no-reexports.json");
const OCENTRA_PARENT_POSTURE_JSON: &str = include_str!("../rules/ocentra-parent-posture.json");
const DEFERRED_WORK_GATE_JSON: &str = include_str!("../rules/deferred-work-gate.json");

fn catalog(
    raw: &str,
    source: &str,
) -> enforcer_rules::RuleResult<Vec<enforcer_rules::registry::RuleRecord>> {
    let raw =
        RuleCatalogJson::try_from(raw.to_owned()).map_err(|error| RuleLoadError::Boundary {
            reason: enforcer_rules::boundary_reason(error),
        })?;
    let source = RuleCatalogSource::try_from(source.to_owned()).map_err(|error| {
        RuleLoadError::Boundary {
            reason: enforcer_rules::boundary_reason(error),
        }
    })?;
    parse_catalog(&raw, &source)
}

fn all_baseline_records(
) -> Result<Vec<enforcer_rules::registry::RuleRecord>, Box<dyn std::error::Error>> {
    let mut records = catalog(DENY_WALL_JSON, "rules/deny-wall.json")?;
    records.extend(catalog(NO_REEXPORTS_JSON, "rules/no-reexports.json")?);
    records.extend(catalog(
        OCENTRA_PARENT_POSTURE_JSON,
        "rules/ocentra-parent-posture.json",
    )?);
    records.extend(catalog(
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
    assert_eq!(
        record.validator.path,
        "rules::deferred_work::DeferredWorkValidator"
    );
    assert_eq!(
        record.fixtures.fail,
        "crates/enforcer-lang-common/tests/fixtures/deferred_work/bad/fail.rs"
    );
    assert_eq!(
        record.fixtures.pass,
        "crates/enforcer-lang-common/tests/fixtures/deferred_work/good/pass.rs"
    );
    assert_eq!(
        record.doc_anchor,
        "docs/plans/enforcer-selfhost-plan/workpacks/d03-deferred-work-gate.md#requirement-checklist"
    );
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
    assert_eq!(record.validator.path, "workspace_lints::DenyWallValidator");
    assert_eq!(
        record.fixtures.fail,
        "crates/enforcer-lang-rust/fixtures/deny-wall/fail_missing_lints.rs"
    );
    assert_eq!(
        record.fixtures.pass,
        "crates/enforcer-lang-rust/fixtures/deny-wall/pass_opts_in.rs"
    );
    assert_eq!(
        record.doc_anchor,
        "docs/plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md#borrows-from-ocentraparent"
    );
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
    assert_eq!(record.validator.path, "no_reexports::NoReexportsValidator");
    Ok(())
}

#[test]
fn ocentra_parent_posture_yields_all_four_expected_records(
) -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let registry = load_registry_from_records(records)?;

    let expected_records = [
        (
            "T1-PARENTPOSTURE.1",
            "no_reexports::NoReexportsValidator",
            "crates/enforcer-lang-rust/fixtures/ocentra-parent-posture/fail_reexport.rs",
            "crates/enforcer-lang-rust/fixtures/ocentra-parent-posture/pass_no_reexport.rs",
        ),
        (
            "T1-PARENTPOSTURE.2",
            "runtime_literal::RuntimeLiteralValidator",
            "crates/enforcer-lang-rust/fixtures/ocentra-parent-posture/fail_runtime_literal.rs",
            "crates/enforcer-lang-rust/fixtures/ocentra-parent-posture/pass_runtime_literal.rs",
        ),
        (
            "T1-PARENTPOSTURE.3",
            "domain_typed_fields::SerializedDomainPrimitiveValidator",
            "crates/enforcer-lang-rust/fixtures/ocentra-parent-posture/fail_bare_primitive_field.rs",
            "crates/enforcer-lang-rust/fixtures/ocentra-parent-posture/pass_branded_field.rs",
        ),
        (
            "T1-PARENTPOSTURE.4",
            "cargo_dependency::BlockedProtocolDependencyValidator",
            "crates/enforcer-lang-rust/fixtures/ocentra-parent-posture/fail_blocked_dependency_manifest.json",
            "crates/enforcer-lang-rust/fixtures/ocentra-parent-posture/pass_clean_dependency_manifest.json",
        ),
    ];
    for (id, validator_path, fail_fixture, pass_fixture) in expected_records {
        let rule_id = id.parse()?;
        let record = registry
            .get(&rule_id)
            .ok_or_else(|| format!("expected {id} to load"))?;
        assert_eq!(record.validator.path, validator_path, "{id} validator");
        assert_eq!(record.fixtures.fail, fail_fixture, "{id} fail fixture");
        assert_eq!(record.fixtures.pass, pass_fixture, "{id} pass fixture");
        assert_eq!(
            record.doc_anchor,
            "docs/plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md#borrows-from-ocentraparent",
            "{id} documentation anchor"
        );
    }

    // publicReexportPolicy: forbid params resolve on the reexport posture
    // record specifically.
    let reexport_posture = registry
        .get(&"T1-PARENTPOSTURE.1".parse()?)
        .ok_or("expected T1-PARENTPOSTURE.1")?;
    assert!(
        matches!(reexport_posture.params.get("publicReexportPolicy"), Some(enforcer_domain::rules_types::RuleParameter::Text(value)) if value == "forbid")
    );

    // blockedProtocolDependencies posture record references the
    // enforcer-config substrate rather than redefining it.
    let dependency_posture = registry
        .get(&"T1-PARENTPOSTURE.4".parse()?)
        .ok_or("expected T1-PARENTPOSTURE.4")?;
    assert!(
        matches!(dependency_posture.params.get("configSubstrate"), Some(enforcer_domain::rules_types::RuleParameter::Text(value)) if value.contains("CargoDependencyPolicy"))
    );

    Ok(())
}

#[test]
fn a_missing_posture_record_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let records = all_baseline_records()?;
    let registry = load_registry_from_records(records)?;
    let missing = "T1-PARENTPOSTURE.99".parse()?;
    if registry.get(&missing).is_some() {
        return Err("unknown posture rule id unexpectedly resolved".into());
    }
    Ok(())
}

#[test]
fn a_renamed_posture_record_id_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate a renamed rule id: the old id no longer resolves.
    let records = all_baseline_records()?;
    let registry = load_registry_from_records(records)?;
    let renamed = "T1-PARENT-POSTURE-ONE".parse()?;
    if registry.get(&renamed).is_some() {
        return Err("renamed posture rule id unexpectedly resolved".into());
    }
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
    let outcome = catalog(malformed, "<inline>");
    assert!(matches!(outcome, Err(RuleLoadError::Boundary { .. })));
    Ok(())
}

#[test]
fn duplicate_rule_id_across_catalogs_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = all_baseline_records()?;
    let clone_of_first = records[0].clone();
    records.push(clone_of_first);
    match load_registry_from_records(records) {
        Err(RuleLoadError::DuplicateRuleId { rule_id }) => {
            assert_eq!(rule_id.as_str(), "T1-DENYWALL.1");
        }
        Err(other) => return Err(format!("unexpected duplicate-id error: {other:?}").into()),
        Ok(_) => return Err("duplicate rule id unexpectedly loaded".into()),
    }
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
    candidate.doc_anchor = format!("{}-moved", baseline.doc_anchor).parse()?;

    assert_eq!(
        check_drift(&baseline, &candidate),
        VersionDriftOutcome::ContentChangedVersionNotBumped
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
    candidate.version =
        enforcer_domain::rules_types::RuleVersion::new(baseline.version.value() + 1)?;

    assert_eq!(
        check_drift(&baseline, &candidate),
        VersionDriftOutcome::VersionBumpedContentUnchanged
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
    candidate.fixtures.fail = format!("{}.v2", baseline.fixtures.fail).parse()?;
    candidate.version =
        enforcer_domain::rules_types::RuleVersion::new(baseline.version.value() + 1)?;

    assert_eq!(
        check_drift(&baseline, &candidate),
        VersionDriftOutcome::ContentChangedVersionBumped
    );
    assert!(!has_drift(&baseline, &candidate));
    Ok(())
}
