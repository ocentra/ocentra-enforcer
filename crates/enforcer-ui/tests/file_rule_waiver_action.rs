use std::str::FromStr;

use enforcer_domain::ids::RuleId;
use enforcer_domain::paths::RelPath;
use enforcer_domain::rules_types::{
    RuleCatalogJson, RuleCatalogSource, WaiverExpiryDate, WaiverOwner, WaiverReason,
};
use enforcer_rules::loader::{load_registry_from_records, parse_catalog};
use enforcer_rules::registry::RuleRegistry;
use enforcer_rules::waiver::{ExpiryPolicy, Waiver, WaiverRegistry};
use enforcer_ui::actions::file_rule_waiver::{
    project_waiver_registry_path, upsert_file_rule_waiver, FileRuleWaiverInput,
    FileRuleWaiverWriteError,
};

const RULES: &str = r#"[
  {
    "ruleId": "SRC-1.1",
    "version": 1,
    "title": "Source shape",
    "tier": "T1",
    "validator": { "crateName": "enforcer-lang-common", "path": "source_shape::Validator" },
    "fixtures": { "fail": "fixtures/fail", "pass": "fixtures/pass" },
    "docAnchor": "docs/rules/SRC-1.md#SRC-1.1"
  }
]"#;

fn rules() -> Result<RuleRegistry, Box<dyn std::error::Error>> {
    let raw = RuleCatalogJson::try_from(RULES.to_owned())?;
    let source = RuleCatalogSource::try_from("<inline rules>".to_owned())?;
    load_registry_from_records(parse_catalog(&raw, &source)?).map_err(Into::into)
}

fn today() -> Result<WaiverExpiryDate, Box<dyn std::error::Error>> {
    Ok("2026-07-10".parse()?)
}

fn request(
    path: &str,
    owner: &str,
    reason: &str,
) -> Result<FileRuleWaiverInput, Box<dyn std::error::Error>> {
    Ok(FileRuleWaiverInput {
        path: RelPath::try_from(path.to_owned())?,
        rule_id: RuleId::from_str("SRC-1.1")?,
        owner: WaiverOwner::try_from(owner.to_owned())?,
        reason: WaiverReason::try_from(reason.to_owned())?,
        expires: Some("2026-07-11".parse()?),
    })
}

#[test]
fn invalid_input_writes_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let project = tempfile::tempdir()?;
    let outcome = upsert_file_rule_waiver(
        project.path(),
        &rules()?,
        &today()?,
        &request("src/**", "platform-team", "tracked migration")?,
    );

    assert!(matches!(outcome, Err(FileRuleWaiverWriteError::Waiver(_))));
    assert!(
        !project_waiver_registry_path(project.path()).exists(),
        "invalid input must not create a waiver file"
    );
    assert!(
        !project.path().join(".enforce").exists(),
        "invalid input must not create the waiver directory"
    );
    Ok(())
}

#[test]
fn request_to_waiver_roundtrip_preserves_all_boundary_fields(
) -> Result<(), Box<dyn std::error::Error>> {
    let request = request("src/legacy.rs", "platform-team", "tracked migration")?;
    let waiver = Waiver::from(&request);

    assert_eq!(waiver.path, request.path);
    assert_eq!(waiver.rule_id, request.rule_id);
    assert_eq!(waiver.owner, request.owner);
    assert_eq!(waiver.reason, request.reason);
    assert_eq!(waiver.expires, request.expires);
    Ok(())
}

#[test]
fn valid_write_round_trips_through_the_strict_registry() -> Result<(), Box<dyn std::error::Error>> {
    let project = tempfile::tempdir()?;
    let rule_registry = rules()?;
    let expected = request("src/legacy.rs", "platform-team", "tracked migration")?;

    upsert_file_rule_waiver(project.path(), &rule_registry, &today()?, &expected)?;

    let registry_path = project_waiver_registry_path(project.path());
    let loaded = WaiverRegistry::load_file(
        &registry_path,
        &rule_registry,
        &today()?,
        ExpiryPolicy::RejectExpired,
    )?;
    let waivers: Vec<_> = loaded.iter().collect();
    assert_eq!(waivers.len(), 1);
    assert_eq!(waivers[0].path, expected.path);
    assert_eq!(waivers[0].rule_id, expected.rule_id);
    assert_eq!(waivers[0].owner, expected.owner);
    assert_eq!(waivers[0].reason, expected.reason);
    assert_eq!(waivers[0].expires, expected.expires);
    Ok(())
}

#[test]
fn repeated_upsert_is_byte_identical_and_preserves_other_valid_waivers(
) -> Result<(), Box<dyn std::error::Error>> {
    let project = tempfile::tempdir()?;
    let rule_registry = rules()?;
    let first = request("src/first.rs", "platform-team", "first exception")?;
    let second = request("src/second.rs", "platform-team", "second exception")?;

    upsert_file_rule_waiver(project.path(), &rule_registry, &today()?, &first)?;
    upsert_file_rule_waiver(project.path(), &rule_registry, &today()?, &second)?;
    let registry_path = project_waiver_registry_path(project.path());
    let first_bytes = std::fs::read(&registry_path)?;

    upsert_file_rule_waiver(project.path(), &rule_registry, &today()?, &second)?;
    let second_bytes = std::fs::read(&registry_path)?;
    let loaded = WaiverRegistry::load_file(
        &registry_path,
        &rule_registry,
        &today()?,
        ExpiryPolicy::RejectExpired,
    )?;

    assert_eq!(first_bytes, second_bytes);
    let waivers: Vec<_> = loaded.iter().collect();
    assert_eq!(waivers.len(), 2);
    assert!(waivers
        .iter()
        .any(|waiver| waiver.path.as_str() == "src/first.rs"));
    assert_eq!(
        waivers
            .iter()
            .filter(|waiver| {
                waiver.path.as_str() == "src/second.rs" && waiver.rule_id == second.rule_id
            })
            .count(),
        1
    );
    Ok(())
}
