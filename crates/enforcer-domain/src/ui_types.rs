//! Canonical UI execution-context values.
//!
//! Rendering surfaces use this closed mode instead of maintaining local
//! `RunMode` copies. `Silent` means return an empty presentation payload
//! without reading the underlying UI data source; `Human` permits normal
//! rendering.

use crate::boundary::decode_error::DecodeError;

/// Validated host name accepted by the UI bind boundary.
///
/// The domain keeps host syntax intentionally narrow: non-empty host text
/// without whitespace or control characters. IP/DNS resolution remains the
/// transport owner's responsibility.
/// BRAND-INVARIANT: constructors reject empty, whitespace, and control text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UiBindHost(String);

impl UiBindHost {
    /// Validate and construct a host at an input boundary.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        Self::try_from(value)
    }

    /// Return the default loopback host used by local UI serving.
    #[must_use]
    pub fn loopback() -> Self {
        // ALLOC-JUSTIFICATION: the default host is owned by the domain value.
        Self("127.0.0.1".to_owned())
    }

    /// Borrow the validated host text for the transport boundary.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for UiBindHost {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(DecodeError::new("uiBindHost", "must not be empty"));
        }
        if value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
        {
            return Err(DecodeError::new(
                "uiBindHost",
                "must not contain whitespace or control characters",
            ));
        }
        Ok(Self(value))
    }
}

impl std::str::FromStr for UiBindHost {
    type Err = DecodeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        // ALLOC-JUSTIFICATION: parsing owns one validated host value so the
        // borrowed input cannot outlive the domain object.
        Self::try_from(value.to_owned())
    }
}

impl std::fmt::Display for UiBindHost {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Validated non-empty token used to authorize a non-loopback UI bind.
/// BRAND-INVARIANT: constructors reject empty and control-character text.
#[derive(Clone, PartialEq, Eq)]
pub struct UiAuthToken(String);

impl UiAuthToken {
    /// Validate and construct a token at an input boundary.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        Self::try_from(value)
    }

    /// Borrow the validated token for the bind gate without exposing a raw
    /// token constructor to callers.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for UiAuthToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("UiAuthToken")
            .field(&"[REDACTED]")
            .finish()
    }
}

impl TryFrom<String> for UiAuthToken {
    type Error = DecodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(DecodeError::new("uiAuthToken", "must not be empty"));
        }
        if value.chars().any(char::is_control) {
            return Err(DecodeError::new(
                "uiAuthToken",
                "must not contain control characters",
            ));
        }
        Ok(Self(value))
    }
}

/// Whether a UI rendering entry point may produce human-facing output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[doc = "The closed rendering mode accepted by UI entry points."]
pub enum UiRunMode {
    /// Render normal human-facing output.
    #[default]
    Human,
    /// Suppress output and avoid data-source reads where the caller supports it.
    Silent,
}
