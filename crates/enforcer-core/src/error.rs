//! Shared `Result`/`Error` types for the whole workspace, plus the
//! structured decode/validation error that boundary parsers
//! (`enforcer-domain`, `enforcer-events`, `enforcer-config`) return instead
//! of stringly-typed errors.

use enforcer_domain::boundary::decode_error::DecodeError;

/// The shared workspace error type. Every crate's fallible API funnels into
/// this (or a crate-local error that converts into it).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Underlying I/O failure.
    #[error("io failure: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization failure.
    #[error("json failure: {0}")]
    Json(#[from] serde_json::Error),

    /// Structured decode/validation failure at a parse boundary.
    #[error(transparent)]
    Decode(#[from] DecodeError),

    /// Environment variable missing or malformed.
    #[error("environment variable `{name}` unavailable: {reason}")]
    Env {
        /// Variable name that failed to resolve.
        /// BRAND-INVARIANT: exact process environment key supplied at the I/O boundary.
        name: String,
        /// Why it failed (missing, not unicode, ...).
        /// BRAND-INVARIANT: diagnostic text supplied by `std::env::VarError`.
        reason: String,
    },

    /// Clock/time acquisition failure.
    #[error("time acquisition failed: {0}")]
    Time(String),

    /// `tracing` subscriber initialization failure.
    #[error("tracing init failed: {0}")]
    TracingInit(String),

    /// Invalid configuration handed to a core primitive (e.g. a bad
    /// redaction pattern).
    #[error("invalid core configuration: {0}")]
    InvalidConfig(String),
}

/// Shared workspace result alias.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::{DecodeError, Error};

    #[test]
    fn io_error_converts_into_shared_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: Error = io.into();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn json_error_converts_into_shared_error() -> super::Result<()> {
        let json_err = serde_json::Error::io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid JSON fixture",
        ));
        let err: Error = json_err.into();
        assert!(matches!(err, Error::Json(_)));
        Ok(())
    }

    #[test]
    fn decode_error_carries_path_and_reason() {
        let err = DecodeError::new("config.rules[3].id", "not a known rule id")
            .with_input_hint("RULE-???");
        assert_eq!(err.path, "config.rules[3].id");
        assert_eq!(err.reason, "not a known rule id");
        assert_eq!(err.input_hint.as_deref(), Some("RULE-???"));
        let shared: Error = err.into();
        assert_eq!(
            shared.to_string(),
            "decode/validation failed at `config.rules[3].id`: not a known rule id"
        );
    }

    #[test]
    fn decode_error_display_is_structured() {
        let err = DecodeError::new("root.field", "bad value");
        assert_eq!(
            err.to_string(),
            "decode/validation failed at `root.field`: bad value"
        );
    }
}
