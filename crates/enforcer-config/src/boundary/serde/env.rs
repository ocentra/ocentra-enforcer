//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Environment transport boundary for `enforcer-config`'s own overrides.
//! (a07 parse-at-boundary requirement). Every var this crate consumes is
//! declared here, once, with its type, required/default, and fail-closed
//! decode behavior â€” there is no scattered `std::env::var(...)` elsewhere
//! in this crate.
//!
//! # Declared variables
//! - `ENFORCER_CONFIG_PATH` (optional `PathBuf`): overrides the project
//!   config file path that [`crate::load_project_config`] would otherwise
//!   use. Absent -> caller's default path is used unchanged.
//! - `ENFORCER_PROFILE` (optional profile name, validated against
//!   [`crate::profiles::known_profile_names`] registry at read time): forces the
//!   profile layer, bypassing whatever `profileName` a project config
//!   declares. Absent -> the config file's own `profileName` (or
//!   `default` when there is no project config) is used unchanged.
//!
//! Both are optional; neither var being unset is ever an error â€” it just
//! means "no override". A var that IS set but decodes to an invalid value
//! (e.g. `ENFORCER_PROFILE=bogus`) is a typed
//! [`crate::error::ConfigLoadError::InvalidEnvVar`], never a silent
//! fallback to the default, matching the file-load boundary contract.

use std::path::PathBuf;

use crate::error::{ConfigLoadError, ConfigResult};
use crate::profiles::KNOWN_PROFILE_NAMES;
use enforcer_domain::config_types::{
    ConfigEnvironmentValue, ConfigEnvironmentVariable, ConfigErrorReason, ConfigProfileName,
};

/// Name of the config-path override variable.
pub const ENFORCER_CONFIG_PATH_VAR: &str = "ENFORCER_CONFIG_PATH";

/// Name of the profile-name override variable.
pub const ENFORCER_PROFILE_VAR: &str = "ENFORCER_PROFILE";

/// Typed, decoded view of every environment variable `enforcer-config`
/// consumes. Constructed only via [`ConfigEnv::read`] /
/// [`ConfigEnv::read_from`] â€” never assembled field-by-field from ad hoc
/// `std::env::var` calls elsewhere.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigEnv {
    /// `ENFORCER_CONFIG_PATH` override, if set.
    pub config_path: Option<PathBuf>,
    /// `ENFORCER_PROFILE` override, if set and valid.
    pub profile_name: Option<ConfigProfileName>,
}

/// Abstraction over "look up an env var by name", so tests can supply a
/// controlled environment instead of mutating the real process environment
/// (`std::env::set_var` is process-global and racy under parallel tests).
pub trait EnvLookup {
    /// Return the variable's value if set. An unreadable value is an error;
    /// only an absent value means that no override was supplied.
    fn lookup(
        &self,
        name: &ConfigEnvironmentVariable,
    ) -> ConfigResult<Option<ConfigEnvironmentValue>>;
}

/// The real process environment.
#[derive(Debug)]
pub struct ProcessEnv;

impl EnvLookup for ProcessEnv {
    fn lookup(
        &self,
        name: &ConfigEnvironmentVariable,
    ) -> ConfigResult<Option<ConfigEnvironmentValue>> {
        match std::env::var(name.as_str()) {
            Ok(value) => Ok(Some(ConfigEnvironmentValue::from_owned(value))),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(ConfigLoadError::EnvVarRead {
                var: name.clone(),
                reason: ConfigErrorReason::from_owned("value is not valid Unicode".to_owned()),
            }),
        }
    }
}

impl ConfigEnv {
    /// Read and decode every declared variable from the real process
    /// environment. This is the one call site downstream code should use;
    /// everything else in this crate treats the result as already
    /// parsed-at-boundary.
    ///
    /// # Errors
    /// Returns [`ConfigLoadError::InvalidEnvVar`] if a declared override is
    /// malformed, or [`ConfigLoadError::EnvVarRead`] if a process value is
    /// unreadable.
    pub fn read() -> ConfigResult<Self> {
        Self::read_from(&ProcessEnv)
    }

    /// Read and decode every declared variable from an arbitrary
    /// [`EnvLookup`] (used by tests to avoid touching the real process
    /// environment).
    ///
    /// # Errors
    /// Returns [`ConfigLoadError::InvalidEnvVar`] if a declared override is
    /// malformed, or an error returned by [`EnvLookup`] if its source cannot
    /// read a requested value.
    pub fn read_from(env: &dyn EnvLookup) -> ConfigResult<Self> {
        let config_path_variable =
            ConfigEnvironmentVariable::new(ENFORCER_CONFIG_PATH_VAR.to_owned())
                .map_err(ConfigLoadError::Parse)?;
        let config_path = match env.lookup(&config_path_variable)? {
            None => None,
            Some(value) if value.as_str().trim().is_empty() => {
                return Err(ConfigLoadError::InvalidEnvVar {
                    var: config_path_variable.clone(),
                    value,
                    reason: ConfigErrorReason::from_owned(
                        "path override must not be empty".to_owned(),
                    ),
                });
            }
            Some(value) if value.as_str().contains('\0') => {
                return Err(ConfigLoadError::InvalidEnvVar {
                    var: config_path_variable.clone(),
                    value,
                    reason: ConfigErrorReason::from_owned(
                        "path override must not contain NUL bytes".to_owned(),
                    ),
                });
            }
            Some(value) => Some(PathBuf::from(value.into_string())),
        };

        let profile_variable = ConfigEnvironmentVariable::new(ENFORCER_PROFILE_VAR.to_owned())
            .map_err(ConfigLoadError::Parse)?;
        let profile_name = match env.lookup(&profile_variable)? {
            None => None,
            Some(value) if KNOWN_PROFILE_NAMES.contains(&value.as_str()) => Some({
                let raw_value = value.into_string();
                ConfigProfileName::new(raw_value.clone()).map_err(|reason| {
                    ConfigLoadError::InvalidEnvVar {
                        var: profile_variable.clone(),
                        value: ConfigEnvironmentValue::from_owned(raw_value),
                        reason: ConfigErrorReason::from_owned(reason.to_string()),
                    }
                })?
            }),
            Some(value) => {
                return Err(ConfigLoadError::InvalidEnvVar {
                    var: profile_variable,
                    value,
                    reason: ConfigErrorReason::from_owned(format!(
                        "unknown profile name (known profiles: {})",
                        KNOWN_PROFILE_NAMES.join(", ")
                    )),
                })
            }
        };

        Ok(ConfigEnv {
            config_path,
            profile_name,
        })
    }
}
