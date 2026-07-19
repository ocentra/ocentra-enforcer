//! Embedded-profile boundary coverage.

use enforcer_config::error::ConfigLoadError;
use enforcer_config::resolve::resolve_profile_only;
use enforcer_config::serde::{decode_json, embedded_profile_json, embedded_profile_names};
use enforcer_domain::config_types::{ConfigProfileName, ConfigSource};

#[test]
fn every_known_profile_decodes_at_the_json_boundary() -> Result<(), Box<dyn std::error::Error>> {
    for name in embedded_profile_names()? {
        let raw = embedded_profile_json(&name)?;
        let value: serde_json::Value = decode_json(
            &raw,
            &ConfigSource::from_owned("embedded profile test".to_owned()),
            "embedded profile must be JSON",
        )?;
        assert_eq!(
            value.get("profileName").and_then(serde_json::Value::as_str),
            Some(name.as_str())
        );
    }
    Ok(())
}

#[test]
fn unknown_embedded_profile_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let outcome = embedded_profile_json(&ConfigProfileName::new("nonexistent".to_owned())?);
    let Err(error) = outcome else {
        return Err(enforcer_domain::boundary::decode_error::DecodeError::new(
            "profileName",
            "unknown profile was accepted",
        )
        .into());
    };
    let ConfigLoadError::UnknownProfile { profile_name, .. } = error else {
        return Err(error.into());
    };
    assert_eq!(profile_name.as_str(), "nonexistent");
    Ok(())
}

#[test]
fn ocentra_enforcer_profile_allows_only_the_tauri_build_script_path(
) -> Result<(), Box<dyn std::error::Error>> {
    let profile = resolve_profile_only(&ConfigProfileName::new("ocentra-enforcer".to_owned())?)?;
    let allowed_paths = profile
        .cargo_dependency_policy
        .allowed_build_rs_paths
        .iter()
        .map(|path| path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        allowed_paths,
        vec!["crates/enforcer-ui/frontend/src-tauri/build.rs"]
    );
    assert_eq!(
        profile.cargo_dependency_policy.allow_build_rs,
        enforcer_domain::config_types::RuleEnabled::Disabled
    );
    Ok(())
}
