//! Public validated CyberSkills vocabulary and role-specific domain wrappers.
//!
//! BOUNDARY-INVARIANT: each public wrapper has one semantic role and is
//! validated by its corresponding serde implementation before use.
//! NEGATIVE-TEST: the CP00 negative fixture matrix rejects invalid values,
//! duplicate collections, and contradictory component statuses.

use super::source::ValidatedText;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Canonical vendor source path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcePathEnvelope(pub(super) ValidatedText);
/// Lowercase SHA-256 value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sha256ValueEnvelope(pub(super) ValidatedText);
/// License label asserted by a source record.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LicenseNameEnvelope(pub(super) ValidatedText);
/// Source-local content or heading anchor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceAnchorEnvelope(pub(super) ValidatedText);
/// Artifact-local heading and line anchor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArtifactAnchorEnvelope(pub(super) ValidatedText);
/// Accepted CP08 batch identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BatchNameEnvelope(pub(super) ValidatedText);
/// Repository-relative immutable artifact path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArtifactPathEnvelope(pub(super) ValidatedText);
/// Additive correction identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CorrectionIdEnvelope(pub(super) ValidatedText);
/// Native implementation component identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComponentIdEnvelope(pub(super) ValidatedText);

// BRAND-INVARIANT: only role validators construct this message-bearing error;
// callers cannot treat it as an unvalidated string boundary.
/// Typed validation error emitted by role-specific boundary wrappers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError(String);

impl ValidationError {
    fn new(label: &str) -> Self {
        Self(format!("{label} has an invalid value"))
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ValidationError {}

fn nonempty_trimmed(value: &str) -> bool {
    !value.trim().is_empty() && value == value.trim()
}

fn lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn artifact_heading(value: &str) -> bool {
    let Some((heading, line)) = value.rsplit_once(":L") else {
        return false;
    };
    heading.trim_start().starts_with('#') && line.parse::<u32>().is_ok_and(|line| line > 0)
}

macro_rules! validated_text_role {
    ($name:ident, $validator:ident, $label:literal) => {
        impl $name {
            fn parse(value: String) -> Result<Self, ValidationError> {
                $validator(&value)
                    .then_some(Self(ValidatedText::new(value)))
                    .ok_or_else(|| ValidationError::new($label))
            }

            /// Borrow this role-specific validated value.
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

validated_text_role!(SourcePathEnvelope, nonempty_trimmed, "source path");
validated_text_role!(Sha256ValueEnvelope, lower_sha256, "SHA-256 value");
validated_text_role!(LicenseNameEnvelope, nonempty_trimmed, "license name");
validated_text_role!(SourceAnchorEnvelope, nonempty_trimmed, "source anchor");
validated_text_role!(ArtifactAnchorEnvelope, artifact_heading, "artifact anchor");
validated_text_role!(BatchNameEnvelope, nonempty_trimmed, "batch name");
validated_text_role!(ArtifactPathEnvelope, nonempty_trimmed, "artifact path");
validated_text_role!(CorrectionIdEnvelope, nonempty_trimmed, "correction ID");
validated_text_role!(ComponentIdEnvelope, nonempty_trimmed, "component ID");

fn validate_unique<T: AsRef<str>>(items: &[T], label: &str) -> Result<(), ValidationError> {
    let mut seen = BTreeSet::new();
    items.iter().try_for_each(|item| {
        seen.insert(item.as_ref())
            .then_some(())
            .ok_or_else(|| ValidationError::new(label))
    })
}

macro_rules! validated_list_role {
    ($name:ident, $item:ty, $label:literal, $wire:literal) => {
        #[doc = "Validated role-specific collection."]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(try_from = $wire)]
        pub struct $name(pub(super) Vec<$item>);

        impl $name {
            /// Borrow this validated role-specific collection.
            pub fn as_slice(&self) -> &[$item] {
                &self.0
            }

            /// Iterate over validated role-specific values.
            pub fn iter(&self) -> std::slice::Iter<'_, $item> {
                self.0.iter()
            }

            /// Report whether the collection is empty.
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
        }

        impl TryFrom<Vec<$item>> for $name {
            type Error = ValidationError;

            fn try_from(items: Vec<$item>) -> Result<Self, Self::Error> {
                validate_unique(&items, $label)?;
                Ok(Self(items))
            }
        }
    };
}

validated_list_role!(
    SourceAnchorListEnvelope,
    SourceAnchorEnvelope,
    "source anchors",
    "Vec<SourceAnchorEnvelope>"
);
validated_list_role!(
    ArtifactAnchorListEnvelope,
    ArtifactAnchorEnvelope,
    "artifact anchors",
    "Vec<ArtifactAnchorEnvelope>"
);
validated_list_role!(
    ComponentIdListEnvelope,
    ComponentIdEnvelope,
    "component IDs",
    "Vec<ComponentIdEnvelope>"
);

/// Availability state of a catalog source.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SourceAvailability {
    Available,
    SourceUnavailable,
}

/// Verified CP08 projection lifecycle state.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed unit enum uses its stable scalar wire value by design."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectionStatus {
    Absent,
    Partial,
    Complete,
}

/// Relationship of a CP08 provenance entry to its predecessor.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed unit enum uses its stable scalar wire value by design."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum ProvenanceRelation {
    Accepted,
    AdditiveCorrection,
}

/// Independent implementation or executable-proof coverage level.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed unit enum uses its stable scalar wire value by design."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum CoverageLevel {
    None,
    Partial,
    Complete,
}

/// Review state for a catalog identity.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DecompositionState {
    Unreviewed,
    Reviewed,
    Unavailable,
}

/// Legacy triage label retained for migration.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum LegacyDisposition {
    Native,
    Unported,
    AdapterDeferred,
    AdvisoryProse,
}

/// Closed decomposition component vocabulary.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentKind {
    NativePredicate,
    ExternalEngine,
    Advisory,
    Manual,
}

/// Catalog planning tier associated with a component.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentTier {
    #[serde(rename = "T1")]
    T1,
    #[serde(rename = "T2")]
    T2,
    #[serde(rename = "T3")]
    T3,
}

/// Evidence lifecycle state for a component.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentStatus {
    Proposed,
    Implemented,
    Proved,
    Retained,
    Blocked,
}

/// Scope of what a component actually proves.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CoverageKind {
    NarrowedPredicate,
    Component,
}

/// Legacy conversion estimate retained for compatibility.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ConversionDifficulty {
    Easy,
    Medium,
    Hard,
}

/// Closed evidence artifact vocabulary.
#[doc = "SERDE-TAG-JUSTIFICATION: this closed scalar vocabulary intentionally uses its stable string representation."]
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKind {
    SourceAttribution,
    Validator,
    FailFixture,
    PassFixture,
    MalformedFixture,
    BoundaryFixture,
    Cli,
    Mcp,
    Ci,
    AdapterRecorded,
    AdapterLive,
    ManualRetention,
}
