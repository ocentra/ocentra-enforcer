//! Typed load errors for `enforcer-config`. Every failure mode that the
//! legacy `.mjs` config loader (`src/check-policy.mjs` `CFG-1.10`/`CFG-1.11`)
//! reported as a runtime *finding* is promoted here to a typed error raised
//! at load time — invalid config never reaches the engine as a live value.

use enforcer_core::error::DecodeError;

/// Load-time failure for a project config or profile.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ConfigLoadError {
    /// The config file's bytes were not valid JSON, or a field did not
    /// decode into its typed shape.
    #[error("config parse failed: {0}")]
    Parse(#[from] DecodeError),

    /// `schemaVersion` or `profileName` is missing (mechanical mirror of
    /// `CFG-1.10`).
    #[error("config at `{path}` is missing required field `{field}` (schemaVersion and profileName must both be declared for unambiguous layering)")]
    MissingRequiredField {
        /// Source path of the offending config (project config or profile).
        path: String,
        /// The missing field name.
        field: &'static str,
    },

    /// `profileName` does not name a known profile (mechanical mirror of
    /// `CFG-1.11`).
    #[error("config at `{path}` declares unknown profileName `{profile_name}` (known profiles: strict, default, ocentra-enforcer, ocentra-parent)")]
    UnknownProfile {
        /// Source path of the offending config.
        path: String,
        /// The unrecognized profile name value, verbatim.
        profile_name: String,
    },

    /// The config/profile file could not be read from disk.
    #[error("failed to read config file `{path}`: {reason}")]
    Io {
        /// Path that failed to read.
        path: String,
        /// Underlying I/O failure description.
        reason: String,
    },

    /// An `enforcer-config`-owned environment variable was set but its
    /// value did not decode into the declared typed shape (mirrors the
    /// file-load fail-closed contract: a bad env override is a typed
    /// error, never a silent fallback to the default). See
    /// [`crate::env`].
    #[error("environment variable `{var}` is set to an invalid value `{value}`: {reason}")]
    InvalidEnvVar {
        /// The environment variable name (always one of [`crate::env`]'s
        /// declared vars).
        var: &'static str,
        /// The raw value that failed to decode, verbatim (not
        /// user-secret-bearing: these are config overrides, not
        /// credentials).
        value: String,
        /// Why the value was rejected.
        reason: String,
    },
}

/// Result alias for `enforcer-config` load operations.
pub type ConfigResult<T> = std::result::Result<T, ConfigLoadError>;
