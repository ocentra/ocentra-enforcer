//! Resolver boundary integration coverage.

use enforcer_config::resolve::{resolve, resolve_profile_only};

use enforcer_config::error::ConfigLoadError;
use enforcer_domain::config_types::{
    ConfigJson, ConfigProfileName, ConfigSource, Platform, PublicReexportPolicy,
};
use serde_json::json;

#[test]
fn zero_config_falls_back_to_default_profile_alone() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = resolve(None, &ConfigSource::from_owned("<none>".to_owned()))?;
    assert_eq!(cfg.profile_name.as_str(), "default");
    assert_eq!(cfg.supported_platforms, Platform::all());
    Ok(())
}

#[test]
fn valid_project_config_merges_overrides_over_profile_defaults(
) -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({
        "schemaVersion": 2,
        "profileName": "strict",
        "failFast": true
    })
    .to_string();
    let cfg = resolve(
        Some(&ConfigJson::from_owned(raw)),
        &ConfigSource::from_owned("ocentra-enforcer.config.json".to_owned()),
    )?;
    assert_eq!(cfg.profile_name.as_str(), "strict");
    assert!(
        matches!(
            cfg.rust_scan_scope.fail_fast,
            enforcer_domain::config_types::RuleEnabled::Enabled
        ),
        "override should win over profile default (false)"
    );
    assert_eq!(
        cfg.cargo_dependency_policy.public_reexport_policy,
        PublicReexportPolicy::Forbid
    );
    Ok(())
}

#[test]
fn missing_schema_version_fails_typed() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({ "profileName": "strict" }).to_string();
    let outcome = resolve(
        Some(&ConfigJson::from_owned(raw)),
        &ConfigSource::from_owned("bad.json".to_owned()),
    );
    let Err(err) = outcome else {
        return Err("expected Err for missing schemaVersion".into());
    };
    let ConfigLoadError::MissingRequiredField { field, .. } = err else {
        return Err("expected MissingRequiredField".into());
    };
    assert_eq!(field.as_str(), "schemaVersion");
    Ok(())
}

#[test]
fn missing_profile_name_fails_typed() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({ "schemaVersion": 2 }).to_string();
    let outcome = resolve(
        Some(&ConfigJson::from_owned(raw)),
        &ConfigSource::from_owned("bad.json".to_owned()),
    );
    let Err(err) = outcome else {
        return Err("expected Err for missing profileName".into());
    };
    let ConfigLoadError::MissingRequiredField { field, .. } = err else {
        return Err("expected MissingRequiredField".into());
    };
    assert_eq!(field.as_str(), "profileName");
    Ok(())
}

#[test]
fn unknown_profile_name_fails_typed_naming_it() -> Result<(), Box<dyn std::error::Error>> {
    let raw = json!({ "schemaVersion": 2, "profileName": "totally-made-up" }).to_string();
    let outcome = resolve(
        Some(&ConfigJson::from_owned(raw)),
        &ConfigSource::from_owned("bad.json".to_owned()),
    );
    let Err(err) = outcome else {
        return Err("expected Err for unknown profileName".into());
    };
    match err {
        ConfigLoadError::UnknownProfile { profile_name, .. } => {
            assert_eq!(profile_name.as_str(), "totally-made-up");
            Ok(())
        }
        other => Err(format!("expected UnknownProfile, got {other:?}").into()),
    }
}

#[test]
fn supported_platforms_present_vs_absent_defaults_to_all_three(
) -> Result<(), Box<dyn std::error::Error>> {
    let absent = resolve_profile_only(&ConfigProfileName::new("strict".to_owned())?)?;
    assert_eq!(absent.supported_platforms, Platform::all());

    let raw = json!({
        "schemaVersion": 2,
        "profileName": "strict",
        "supportedPlatforms": ["windows"]
    })
    .to_string();
    let present = resolve(
        Some(&ConfigJson::from_owned(raw)),
        &ConfigSource::from_owned("cfg.json".to_owned()),
    )?;
    assert_eq!(present.supported_platforms, vec![Platform::Windows]);
    Ok(())
}
