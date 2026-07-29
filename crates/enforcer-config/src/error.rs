//! Typed load errors for `enforcer-config`. Every failure mode that the
//! legacy `.mjs` config loader (`src/check-policy.mjs` `CFG-1.10`/`CFG-1.11`)
//! reported as a runtime *finding* is promoted here to a typed error raised
//! at load time — invalid config never reaches the engine as a live value.

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::config_types::{
    ConfigEnvironmentValue, ConfigEnvironmentVariable, ConfigErrorReason, ConfigField,
    ConfigProfileName, ConfigSource,
};

/// Load-time failure for a project config or profile.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ConfigLoadError {
    /// The config file's bytes were not valid JSON, or a field did not
    /// decode into its typed shape.
    #[error("config parse failed: {0}")]
    Parse(#[from] DecodeError),

    /// A decoded policy violated a cross-field invariant.
    #[error(transparent)]
    Policy(#[from] crate::policy::PolicyValidationError),

    /// `schemaVersion` or `profileName` is missing (mechanical mirror of
    /// `CFG-1.10`).
    #[error("config at `{}` is missing required field `{}` (schemaVersion and profileName must both be declared for unambiguous layering)", .path.as_str(), .field.as_str())]
    MissingRequiredField {
        /// Source path of the offending config (project config or profile).
        path: ConfigSource,
        /// The missing field name.
        field: ConfigField,
    },

    /// `profileName` does not name a known profile (mechanical mirror of
    /// `CFG-1.11`).
    #[error("config at `{}` declares unknown profileName `{}` (known profiles: strict, default, ocentra-enforcer, ocentra-parent)", .path.as_str(), .profile_name.as_str())]
    UnknownProfile {
        /// Source path of the offending config.
        path: ConfigSource,
        /// The unrecognized profile name value, verbatim.
        profile_name: ConfigProfileName,
    },

    /// The config/profile file could not be read from disk.
    #[error("failed to read config file `{}`: {}", .path.as_str(), .reason.as_str())]
    Io {
        /// Path that failed to read.
        path: ConfigSource,
        /// Underlying I/O failure description.
        reason: ConfigErrorReason,
    },

    /// An `enforcer-config` environment variable could not be decoded from
    /// the process environment. This is distinct from an unset variable:
    /// treating an unreadable override as absent would silently select a
    /// different configuration layer.
    #[error("failed to read environment variable `{}`: {}", .var.as_str(), .reason.as_str())]
    EnvVarRead {
        /// Name of the environment variable whose value was unreadable.
        var: ConfigEnvironmentVariable,
        /// Safe description of the read failure.
        reason: ConfigErrorReason,
    },

    /// An `enforcer-config`-owned environment variable was set but its
    /// value did not decode into the declared typed shape (mirrors the
    /// file-load fail-closed contract: a bad env override is a typed
    /// error, never a silent fallback to the default). See
    /// [`crate::env`].
    #[error("environment variable `{}` is set to an invalid value: {}", .var.as_str(), .reason.as_str())]
    InvalidEnvVar {
        /// The environment variable name (always one of [`crate::env`]'s
        /// declared vars).
        var: ConfigEnvironmentVariable,
        /// The raw value that failed to decode, verbatim (not
        /// user-secret-bearing: these are config overrides, not
        /// credentials).
        value: ConfigEnvironmentValue,
        /// Why the value was rejected.
        reason: ConfigErrorReason,
    },
}

/// Result alias for `enforcer-config` load operations.
pub type ConfigResult<T> = std::result::Result<T, ConfigLoadError>;
