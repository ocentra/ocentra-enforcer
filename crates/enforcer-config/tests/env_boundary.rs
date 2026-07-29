//! Public environment-boundary coverage for `enforcer-config`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use enforcer_config::env::{ConfigEnv, EnvLookup, ENFORCER_CONFIG_PATH_VAR, ENFORCER_PROFILE_VAR};
use enforcer_config::error::{ConfigLoadError, ConfigResult};
use enforcer_domain::config_types::{
    ConfigEnvironmentValue, ConfigEnvironmentVariable, ConfigErrorReason,
};

struct ControlledEnv {
    values: BTreeMap<&'static str, String>,
    failure: Option<ConfigLoadError>,
}

impl ControlledEnv {
    fn with_values(values: BTreeMap<&'static str, String>) -> Self {
        Self {
            values,
            failure: None,
        }
    }

    fn failing_with(error: ConfigLoadError) -> Self {
        Self {
            values: BTreeMap::new(),
            failure: Some(error),
        }
    }
}

impl EnvLookup for ControlledEnv {
    fn lookup(
        &self,
        name: &ConfigEnvironmentVariable,
    ) -> ConfigResult<Option<ConfigEnvironmentValue>> {
        match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(self
                .values
                .get(name.as_str())
                .cloned()
                .map(ConfigEnvironmentValue::from_owned)),
        }
    }
}

#[test]
fn absent_vars_decode_to_no_overrides() -> Result<(), Box<dyn std::error::Error>> {
    let env = ControlledEnv::with_values(BTreeMap::new());
    assert_eq!(ConfigEnv::read_from(&env)?, ConfigEnv::default());
    Ok(())
}

#[test]
fn config_path_var_decodes_to_typed_path_buf() -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_CONFIG_PATH_VAR, "custom/cfg.json".to_owned());
    let decoded = ConfigEnv::read_from(&ControlledEnv::with_values(values))?;
    assert_eq!(
        decoded.config_path.as_deref(),
        Some(PathBuf::from("custom/cfg.json").as_path())
    );
    assert_eq!(decoded.profile_name, None);
    Ok(())
}

#[test]
fn empty_config_path_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_CONFIG_PATH_VAR, "   ".to_owned());
    let error = ConfigEnv::read_from(&ControlledEnv::with_values(values))
        .err()
        .ok_or("empty path must fail")?;
    let ConfigLoadError::InvalidEnvVar { var, .. } = error else {
        return Err("expected InvalidEnvVar".into());
    };
    assert_eq!(var.as_str(), ENFORCER_CONFIG_PATH_VAR);
    Ok(())
}

#[test]
fn config_path_with_nul_byte_fails_at_the_environment_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_CONFIG_PATH_VAR, "config\0override.json".to_owned());
    let error = ConfigEnv::read_from(&ControlledEnv::with_values(values))
        .err()
        .ok_or("NUL path must fail")?;
    let ConfigLoadError::InvalidEnvVar { var, reason, .. } = error else {
        return Err("expected InvalidEnvVar".into());
    };
    assert_eq!(var.as_str(), ENFORCER_CONFIG_PATH_VAR);
    assert_eq!(reason.as_str(), "path override must not contain NUL bytes");
    Ok(())
}

#[test]
fn profile_var_with_known_name_decodes() -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_PROFILE_VAR, "strict".to_owned());
    let decoded = ConfigEnv::read_from(&ControlledEnv::with_values(values))?;
    assert_eq!(
        decoded
            .profile_name
            .as_ref()
            .map(|profile| profile.as_str()),
        Some("strict")
    );
    Ok(())
}

#[test]
fn profile_var_with_unknown_name_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_PROFILE_VAR, "bogus-profile".to_owned());
    let error = ConfigEnv::read_from(&ControlledEnv::with_values(values))
        .err()
        .ok_or("unknown profile must fail")?;
    let ConfigLoadError::InvalidEnvVar { var, value, .. } = error else {
        return Err("expected InvalidEnvVar".into());
    };
    assert_eq!(var.as_str(), ENFORCER_PROFILE_VAR);
    assert_eq!(value.as_str(), "bogus-profile");
    Ok(())
}

#[test]
fn lookup_errors_are_not_treated_as_absent_overrides() -> Result<(), Box<dyn std::error::Error>> {
    let env = ControlledEnv::failing_with(ConfigLoadError::EnvVarRead {
        var: ConfigEnvironmentVariable::new(ENFORCER_PROFILE_VAR.to_owned())?,
        reason: ConfigErrorReason::from_owned("environment value is not valid Unicode".to_owned()),
    });
    let error = ConfigEnv::read_from(&env)
        .err()
        .ok_or("lookup error must propagate")?;
    let ConfigLoadError::EnvVarRead { var, reason } = error else {
        return Err("expected EnvVarRead".into());
    };
    assert_eq!(var.as_str(), ENFORCER_PROFILE_VAR);
    assert_eq!(reason.as_str(), "environment value is not valid Unicode");
    Ok(())
}

#[test]
fn both_vars_set_decode_independently() -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_CONFIG_PATH_VAR, "a/b.json".to_owned());
    values.insert(ENFORCER_PROFILE_VAR, "ocentra-parent".to_owned());
    let decoded = ConfigEnv::read_from(&ControlledEnv::with_values(values))?;
    assert_eq!(
        decoded.config_path.as_deref(),
        Some(PathBuf::from("a/b.json").as_path())
    );
    assert_eq!(
        decoded
            .profile_name
            .as_ref()
            .map(|profile| profile.as_str()),
        Some("ocentra-parent")
    );
    Ok(())
}
