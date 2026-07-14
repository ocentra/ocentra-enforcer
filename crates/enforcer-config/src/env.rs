//! The sole reader of `std::env` for `enforcer-config`'s own overrides
//! (a07 parse-at-boundary requirement). Every var this crate consumes is
//! declared here, once, with its type, required/default, and fail-closed
//! decode behavior — there is no scattered `std::env::var(...)` elsewhere
//! in this crate.
//!
//! # Declared variables
//! - `ENFORCER_CONFIG_PATH` (optional `PathBuf`): overrides the project
//!   config file path that [`crate::load_project_config`] would otherwise
//!   use. Absent -> caller's default path is used unchanged.
//! - `ENFORCER_PROFILE` (optional profile name, validated against
//!   [`crate::profiles::KNOWN_PROFILE_NAMES`] at read time): forces the
//!   profile layer, bypassing whatever `profileName` a project config
//!   declares. Absent -> the config file's own `profileName` (or
//!   `default` when there is no project config) is used unchanged.
//!
//! Both are optional; neither var being unset is ever an error — it just
//! means "no override". A var that IS set but decodes to an invalid value
//! (e.g. `ENFORCER_PROFILE=bogus`) is a typed
//! [`crate::error::ConfigLoadError::InvalidEnvVar`], never a silent
//! fallback to the default, matching the file-load boundary contract.

use std::path::PathBuf;

use crate::error::{ConfigLoadError, ConfigResult};
use crate::profiles::KNOWN_PROFILE_NAMES;

/// Name of the config-path override variable.
pub const ENFORCER_CONFIG_PATH_VAR: &str = "ENFORCER_CONFIG_PATH";

/// Name of the profile-name override variable.
pub const ENFORCER_PROFILE_VAR: &str = "ENFORCER_PROFILE";

/// Typed, decoded view of every environment variable `enforcer-config`
/// consumes. Constructed only via [`ConfigEnv::read`] /
/// [`ConfigEnv::read_from`] — never assembled field-by-field from ad hoc
/// `std::env::var` calls elsewhere.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigEnv {
    /// `ENFORCER_CONFIG_PATH` override, if set.
    pub config_path: Option<PathBuf>,
    /// `ENFORCER_PROFILE` override, if set and valid.
    pub profile_name: Option<String>,
}

/// Abstraction over "look up an env var by name", so tests can supply a
/// controlled environment instead of mutating the real process environment
/// (`std::env::set_var` is process-global and racy under parallel tests).
pub trait EnvLookup {
    /// Return the variable's value if set. An unreadable value is an error;
    /// only an absent value means that no override was supplied.
    fn lookup(&self, name: &str) -> ConfigResult<Option<String>>;
}

/// The real process environment.
#[derive(Debug)]
pub struct ProcessEnv;

impl EnvLookup for ProcessEnv {
    fn lookup(&self, name: &str) -> ConfigResult<Option<String>> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(ConfigLoadError::EnvVarRead {
                var: name.to_owned(),
                reason: "value is not valid Unicode".to_owned(),
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
        let config_path = match env.lookup(ENFORCER_CONFIG_PATH_VAR)? {
            None => None,
            Some(value) if value.trim().is_empty() => {
                return Err(ConfigLoadError::InvalidEnvVar {
                    var: ENFORCER_CONFIG_PATH_VAR,
                    value,
                    reason: "path override must not be empty".to_owned(),
                });
            }
            Some(value) => Some(PathBuf::from(value)),
        };

        let profile_name = match env.lookup(ENFORCER_PROFILE_VAR)? {
            None => None,
            Some(value) if KNOWN_PROFILE_NAMES.contains(&value.as_str()) => Some(value),
            Some(value) => {
                return Err(ConfigLoadError::InvalidEnvVar {
                    var: ENFORCER_PROFILE_VAR,
                    value,
                    reason: format!(
                        "unknown profile name (known profiles: {})",
                        KNOWN_PROFILE_NAMES.join(", ")
                    ),
                })
            }
        };

        Ok(ConfigEnv {
            config_path,
            profile_name,
        })
    }
}
