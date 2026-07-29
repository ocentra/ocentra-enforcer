//! Canonical values for rule-mechanization decisions.
//!
//! The mechanization crate owns scaffolding behavior. This dependency-leaf
//! module owns the closed value sets that may cross that crate's public API.

use crate::boundary::decode_error::DecodeError;

macro_rules! non_blank_text {
    ($name:ident, $field:literal, $doc:literal, $allow_controls:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Validate text, rejecting invalid blank or disallowed control characters.
            pub fn try_new(value: String) -> Result<Self, DecodeError> {
                if value.trim().is_empty()
                    || (!$allow_controls && value.chars().any(char::is_control))
                {
                    return Err(DecodeError::new($field, "must be non-blank printable text"));
                }
                Ok(Self(value))
            }
            #[must_use]
            #[doc = "The as_str operation for this canonical domain value."]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl TryFrom<String> for $name {
            type Error = DecodeError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::try_new(value)
            }
        }
        impl std::str::FromStr for $name {
            type Err = DecodeError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                // ALLOC-JUSTIFICATION: the canonical domain value owns this text beyond the caller lifetime.
                Self::try_new(value.to_owned())
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

non_blank_text!(
    FeedbackToolName,
    "feedbackToolName",
    "A harness tool name retained by feedback processing.",
    false
);
non_blank_text!(
    ExternalDiagnosticCode,
    "externalDiagnosticCode",
    "A diagnostic code from an external tool.",
    false
);
non_blank_text!(
    GeneratedValidatorSource,
    "generatedValidatorSource",
    "Generated Rust validator source awaiting human implementation.",
    true
);
non_blank_text!(
    FixtureSlotContent,
    "fixtureSlotContent",
    "Generated initial content for a rule fixture slot.",
    true
);

/// Positive schema version of a feedback-decision domain record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for FeedbackDecisionSchemaVersion."]
#[doc = "BRAND-INVARIANT: validated canonical value; raw storage remains private."]
pub struct FeedbackDecisionSchemaVersion(u32);

impl FeedbackDecisionSchemaVersion {
    #[must_use]
    pub const fn initial() -> Self {
        Self(1)
    }

    /// Brand an already validated positive schema version.
    #[must_use]
    pub const fn try_new(value: std::num::NonZeroU32) -> Self {
        Self(value.get())
    }
}

/// Outcome of classifying a harness diagnostic for rule mechanization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for MechanizationClassification."]
pub enum MechanizationClassification {
    /// A deterministic source validator could have prevented the failure.
    Prevent,
    /// The signal is harness-only and is not a validator candidate.
    Detect,
}

/// Lifecycle state of a scaffolded rule before registry promotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for RuleLifecycleStatus."]
pub enum RuleLifecycleStatus {
    /// The rule was scaffolded from evidence and still needs review.
    Proposed,
}

/// Whether a feedback decision produced a candidate rule artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[doc = "Canonical domain representation for FeedbackScaffoldState."]
pub enum FeedbackScaffoldState {
    /// A candidate rule was created for a preventable diagnostic.
    Proposed,
    /// No candidate was created for a detect-only diagnostic.
    NotProposed,
}
