//! Typed decode failure emitted by external-input boundaries.

// BOUNDARY-INVARIANT: parser and deserializer failures retain raw diagnostic
// text only at the transport boundary; typed values never carry this text as
// domain state.
// boundaryOwnerNote: this module owns the shared decode-failure transport
// contract for all crates that accept external text or serialized input.
// Negative malformed/invalid boundary input is covered by the branded ID,
// path, hash, and record boundary tests in this crate.

/// Structured failure returned when a boundary rejects external input.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
#[error("decode/validation failed at `{path}`: {reason}")]
#[doc = "A structured rejection produced while decoding untrusted boundary input."]
pub struct DecodeError {
    /// Boundary field or logical location that rejected the input.
    pub path: String,
    /// Human-readable reason the boundary rejected the input.
    pub reason: String,
    /// Optional diagnostic hint; never a trusted domain value.
    pub input_hint: Option<String>,
}

impl DecodeError {
    /// Create a failure with the boundary field path and rejection reason.
    pub fn new(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: reason.into(),
            input_hint: None,
        }
    }

    /// Attach a non-authoritative input hint for diagnostics.
    pub fn with_input_hint(mut self, hint: impl Into<String>) -> Self {
        self.input_hint = Some(hint.into());
        self
    }
}
