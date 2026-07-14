//! Public environment-boundary coverage for `enforcer-config`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use enforcer_config::env::{ConfigEnv, EnvLookup, ENFORCER_CONFIG_PATH_VAR, ENFORCER_PROFILE_VAR};
use enforcer_config::error::{ConfigLoadError, ConfigResult};

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
    fn lookup(&self, name: &str) -> ConfigResult<Option<String>> {
        match &self.failure {
            Some(error) => Err(error.clone()),
            None => Ok(self.values.get(name).cloned()),
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
    assert_eq!(decoded.config_path.as_ref().map(|path| path.as_path()), Some(PathBuf::from("custom/cfg.json").as_path()));
    assert_eq!(decoded.profile_name, None);
    Ok(())
}

#[test]
fn empty_config_path_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_CONFIG_PATH_VAR, "   ".to_owned());
    let error = ConfigEnv::read_from(&ControlledEnv::with_values(values))
        .expect_err("empty path must fail");
    assert!(matches!(
        error,
        ConfigLoadError::InvalidEnvVar {
            var: ENFORCER_CONFIG_PATH_VAR,
            ..
        }
    ));
    Ok(())
}

#[test]
fn config_path_with_nul_byte_fails_at_the_environment_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_CONFIG_PATH_VAR, "config\0override.json".to_owned());
    let error = ConfigEnv::read_from(&ControlledEnv::with_values(values))
        .expect_err("NUL path must fail");
    assert!(matches!(
        error,
        ConfigLoadError::InvalidEnvVar {
            var: ENFORCER_CONFIG_PATH_VAR,
            reason,
            ..
        } if reason == "path override must not contain NUL bytes"
    ));
    Ok(())
}

#[test]
fn profile_var_with_known_name_decodes() -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_PROFILE_VAR, "strict".to_owned());
    let decoded = ConfigEnv::read_from(&ControlledEnv::with_values(values))?;
    assert_eq!(decoded.profile_name.as_ref().map(|profile| profile.as_str()), Some("strict"));
    Ok(())
}

#[test]
fn profile_var_with_unknown_name_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_PROFILE_VAR, "bogus-profile".to_owned());
    let error = ConfigEnv::read_from(&ControlledEnv::with_values(values))
        .expect_err("unknown profile must fail");
    assert!(matches!(
        error,
        ConfigLoadError::InvalidEnvVar {
            var: ENFORCER_PROFILE_VAR,
            value,
            ..
        } if value == "bogus-profile"
    ));
    Ok(())
}

#[test]
fn lookup_errors_are_not_treated_as_absent_overrides() -> Result<(), Box<dyn std::error::Error>> {
    let env = ControlledEnv::failing_with(ConfigLoadError::EnvVarRead {
        var: ENFORCER_PROFILE_VAR.to_owned(),
        reason: "environment value is not valid Unicode".to_owned(),
    });
    let error = ConfigEnv::read_from(&env).expect_err("lookup error must propagate");
    assert!(matches!(
        error,
        ConfigLoadError::EnvVarRead { var, reason }
            if var == ENFORCER_PROFILE_VAR && reason == "environment value is not valid Unicode"
    ));
    Ok(())
}

#[test]
fn both_vars_set_decode_independently() -> Result<(), Box<dyn std::error::Error>> {
    let mut values = BTreeMap::new();
    values.insert(ENFORCER_CONFIG_PATH_VAR, "a/b.json".to_owned());
    values.insert(ENFORCER_PROFILE_VAR, "ocentra-parent".to_owned());
    let decoded = ConfigEnv::read_from(&ControlledEnv::with_values(values))?;
    assert_eq!(decoded.config_path.as_ref().map(|path| path.as_path()), Some(PathBuf::from("a/b.json").as_path()));
    assert_eq!(decoded.profile_name.as_ref().map(|profile| profile.as_str()), Some("ocentra-parent"));
    Ok(())
}
