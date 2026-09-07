//! Validator input decoded at filesystem and transport boundaries.

use super::decode_error::DecodeError;

// BOUNDARY-INVARIANT: source text is wrapped immediately before validator
// execution and never becomes durable domain state.
// boundaryOwnerNote: enforcer-domain owns shared validator-input decoding.
// Negative invalid-input coverage is not applicable because validators must
// accept every text sequence, including empty and malformed source files.

/// Borrowed validator input text after the caller's I/O boundary.
#[derive(Debug, Clone, Copy)]
#[doc = "Borrowed source text accepted by one validator invocation."]
pub struct ValidationSource<'a>(&'a str);

impl<'a> ValidationSource<'a> {
    /// Wrap source text for one validator invocation.
    #[must_use]
    pub const fn from_text(source: &'a str) -> Self {
        Self(source)
    }

    /// View the borrowed source text.
    #[must_use]
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

/// Borrowed lexical marker searched for by a source validator.
///
/// The marker is intentionally distinct from [`ValidationSource`]: one is
/// the untrusted document being inspected, while the other is validator-owned
/// rule vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc = "Canonical borrowed search marker used by validation adapters."]
pub struct ValidationMarker<'a>(&'a str);

impl ValidationMarker<'static> {
    /// Wrap a validator-owned static search marker.
    #[must_use]
    pub const fn from_static(marker: &'static str) -> Self {
        Self(marker)
    }
}

impl<'a> ValidationMarker<'a> {
    /// View the marker at the lexical matching boundary.
    #[must_use]
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

/// Owned validator input text read at a filesystem or transport boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc = "BRAND-INVARIANT: source text is owned only by the validation boundary adapter."]
pub struct ValidationSourceText(String);

impl ValidationSourceText {
    /// Retain source text for repeated validator calls.
    #[must_use]
    #[doc = "Retain source text for repeated validation calls."]
    pub fn try_new(source: String) -> Self {
        Self(source)
    }

    /// Borrow the retained text for one validator call.
    #[must_use]
    #[doc = "Borrow the retained text for one validator invocation."]
    pub fn as_source(&self) -> ValidationSource<'_> {
        ValidationSource::from_text(&self.0)
    }
}

impl From<&'static str> for ValidationSourceText {
    fn from(source: &'static str) -> Self {
        Self::try_new(source.to_owned())
    }
}

/// Opaque text decoded from an MCP validation report before it enters
/// process-local compatibility history.
///
/// BRAND-INVARIANT: labels are non-blank, contain no control characters, and
/// cross the transport boundary through this type rather than raw strings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct McpReportLabelText(String);

impl McpReportLabelText {
    /// Canonical valid timestamp used only when a platform timestamp cannot
    /// cross the MCP boundary. This is a domain-owned invariant, not a router
    /// fallback string.
    #[must_use]
    pub fn epoch_fallback() -> Self {
        Self("1970-01-01T00:00:00.000Z".to_owned())
    }

    /// Reject blank and control-bearing report labels at the MCP boundary.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        if value.trim().is_empty() {
            return Err(DecodeError::new("mcpReportLabel", "label is blank"));
        }
        if value.chars().any(char::is_control) {
            return Err(DecodeError::new(
                "mcpReportLabel",
                "label contains a control character",
            ));
        }
        Ok(Self(value))
    }

    /// Move validated label text into a typed consumer without cloning.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// Canonical Dart widget class name extracted from a source boundary.
///
/// BRAND-INVARIANT: values are accepted only when they are non-empty Dart
/// identifiers beginning with an ASCII uppercase letter; callers therefore do
/// not carry an unvalidated class-name string through rule logic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DartWidgetName(String);

impl DartWidgetName {
    /// Decode one public Dart widget class name.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        let valid = !value.is_empty()
            && value
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
            && value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
        valid
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("dartWidgetName", "must be a public Dart identifier"))
    }

    /// View the validated widget name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Canonical snake-case Dart filename stem produced by the naming rule.
///
/// BRAND-INVARIANT: the stem is owned after conversion and contains only
/// lowercase ASCII letters, digits, and underscores.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DartFilenameStem(String);

impl DartFilenameStem {
    /// Construct a validated Dart filename stem.
    pub fn try_new(value: String) -> Result<Self, DecodeError> {
        let valid = !value.is_empty()
            && value
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_');
        valid
            .then_some(Self(value))
            .ok_or_else(|| DecodeError::new("dartFilenameStem", "must be snake_case"))
    }

    /// View the validated filename stem.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Import ordering groups understood by the Dart import validator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DartImportGroup {
    Dart,
    Package,
    Relative,
}

/// Cardinality of public Flutter widget declarations in one source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DartWidgetMultiplicity {
    None,
    One,
    Multiple,
}
