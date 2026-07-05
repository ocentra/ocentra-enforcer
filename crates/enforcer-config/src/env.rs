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
/// fake environment instead of mutating the real process environment
/// (`std::env::set_var` is process-global and racy under parallel tests).
pub trait EnvLookup {
    /// Return the variable's value if set.
    fn lookup(&self, name: &str) -> Option<String>;
}

/// The real process environment.
pub struct ProcessEnv;

impl EnvLookup for ProcessEnv {
    fn lookup(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

impl ConfigEnv {
    /// Read and decode every declared variable from the real process
    /// environment. This is the one call site downstream code should use;
    /// everything else in this crate treats the result as already
    /// parsed-at-boundary.
    ///
    /// # Errors
    /// Returns [`ConfigLoadError::InvalidEnvVar`] if `ENFORCER_PROFILE` is
    /// set to a name outside [`KNOWN_PROFILE_NAMES`].
    pub fn read() -> ConfigResult<Self> {
        Self::read_from(&ProcessEnv)
    }

    /// Read and decode every declared variable from an arbitrary
    /// [`EnvLookup`] (used by tests to avoid touching the real process
    /// environment).
    ///
    /// # Errors
    /// Returns [`ConfigLoadError::InvalidEnvVar`] if `ENFORCER_PROFILE` is
    /// set to a name outside [`KNOWN_PROFILE_NAMES`].
    pub fn read_from(env: &dyn EnvLookup) -> ConfigResult<Self> {
        let config_path = env.lookup(ENFORCER_CONFIG_PATH_VAR).map(PathBuf::from);

        let profile_name = match env.lookup(ENFORCER_PROFILE_VAR) {
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

#[cfg(test)]
mod tests {
    use super::{ConfigEnv, EnvLookup, ENFORCER_CONFIG_PATH_VAR, ENFORCER_PROFILE_VAR};
    use crate::error::ConfigLoadError;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// A fake environment for deterministic, non-racy tests: no
    /// `std::env::set_var` anywhere in this module.
    struct FakeEnv(BTreeMap<&'static str, String>);

    impl EnvLookup for FakeEnv {
        fn lookup(&self, name: &str) -> Option<String> {
            self.0.get(name).cloned()
        }
    }

    #[test]
    fn absent_vars_decode_to_no_overrides() -> Result<(), Box<dyn std::error::Error>> {
        let env = FakeEnv(BTreeMap::new());
        let decoded = ConfigEnv::read_from(&env)?;
        assert_eq!(decoded, ConfigEnv::default());
        Ok(())
    }

    #[test]
    fn config_path_var_decodes_to_typed_path_buf() -> Result<(), Box<dyn std::error::Error>> {
        let mut vars = BTreeMap::new();
        vars.insert(ENFORCER_CONFIG_PATH_VAR, "custom/cfg.json".to_owned());
        let env = FakeEnv(vars);
        let decoded = ConfigEnv::read_from(&env)?;
        assert_eq!(decoded.config_path, Some(PathBuf::from("custom/cfg.json")));
        assert_eq!(decoded.profile_name, None);
        Ok(())
    }

    #[test]
    fn profile_var_with_known_name_decodes() -> Result<(), Box<dyn std::error::Error>> {
        let mut vars = BTreeMap::new();
        vars.insert(ENFORCER_PROFILE_VAR, "strict".to_owned());
        let env = FakeEnv(vars);
        let decoded = ConfigEnv::read_from(&env)?;
        assert_eq!(decoded.profile_name, Some("strict".to_owned()));
        Ok(())
    }

    #[test]
    fn profile_var_with_unknown_name_fails_closed_not_silently_default(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut vars = BTreeMap::new();
        vars.insert(ENFORCER_PROFILE_VAR, "bogus-profile".to_owned());
        let env = FakeEnv(vars);
        let outcome = ConfigEnv::read_from(&env);
        let Err(err) = outcome else {
            return Err("expected Err for unknown ENFORCER_PROFILE value, got Ok".into());
        };
        match err {
            ConfigLoadError::InvalidEnvVar { var, value, .. } => {
                assert_eq!(var, ENFORCER_PROFILE_VAR);
                assert_eq!(value, "bogus-profile");
                Ok(())
            }
            other => Err(format!("expected InvalidEnvVar, got {other:?}").into()),
        }
    }

    #[test]
    fn both_vars_set_decode_independently() -> Result<(), Box<dyn std::error::Error>> {
        let mut vars = BTreeMap::new();
        vars.insert(ENFORCER_CONFIG_PATH_VAR, "a/b.json".to_owned());
        vars.insert(ENFORCER_PROFILE_VAR, "ocentra-parent".to_owned());
        let env = FakeEnv(vars);
        let decoded = ConfigEnv::read_from(&env)?;
        assert_eq!(decoded.config_path, Some(PathBuf::from("a/b.json")));
        assert_eq!(decoded.profile_name, Some("ocentra-parent".to_owned()));
        Ok(())
    }
}
