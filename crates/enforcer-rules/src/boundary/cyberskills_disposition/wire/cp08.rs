//! Serialized CP08 projection DTOs.
//!
//! BOUNDARY-INVARIANT: CP08 DTOs cache verified immutable artifact evidence.
//! NEGATIVE-TEST: crates/enforcer-rules/tests/cyberskills_disposition/cp08_validation.rs
//! rejects projection and provenance drift.
//! ROUNDTRIP-TEST: crates/enforcer-rules/tests/cyberskills_disposition/manifest.rs
//! contains source, projection, and provenance codec cycles.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::super::types::{
    ArtifactAnchorListEnvelope, ArtifactPathEnvelope, BatchNameEnvelope, ComponentKind,
    ComponentStatus, CorrectionIdEnvelope, LicenseNameEnvelope, ProvenanceRelation,
    Sha256ValueEnvelope, SourceAnchorListEnvelope, SourceAvailability, SourcePathEnvelope,
};

/// Source identity projection with role-specific validated fields.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceIdentityDto {
    pub path: SourcePathEnvelope,
    pub sha256: Option<Sha256ValueEnvelope>,
    pub availability: SourceAvailability,
    pub license: LicenseNameEnvelope,
    pub anchors: SourceAnchorListEnvelope,
}

/// Verified cache of the terminal CP08 decomposition state.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cp08ProjectionDto {
    pub status: super::super::types::ProjectionStatus,
    pub component_count: usize,
    pub present_kinds: Vec<ComponentKind>,
    pub missing_kinds: Vec<ComponentKind>,
    pub kind_status: BTreeMap<ComponentKind, ComponentStatus>,
    pub provenance_chain: Vec<Cp08ProvenanceEntryDto>,
}

/// One immutable root or additive correction in the CP08 chain.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cp08ProvenanceEntryDto {
    pub relation: ProvenanceRelation,
    pub batch: BatchNameEnvelope,
    pub artifact_path: ArtifactPathEnvelope,
    pub artifact_sha256: Sha256ValueEnvelope,
    pub source_sha256: Sha256ValueEnvelope,
    pub artifact_anchors: ArtifactAnchorListEnvelope,
    pub component_count: usize,
    pub present_kinds: Vec<ComponentKind>,
    pub missing_kinds: Vec<ComponentKind>,
    pub kind_status: BTreeMap<ComponentKind, ComponentStatus>,
    // DEFAULT-JUSTIFICATION: accepted roots omit correction identity fields; chain validation rejects invalid omissions on corrections.
    #[serde(default)]
    pub correction_id: Option<CorrectionIdEnvelope>,
    // DEFAULT-JUSTIFICATION: accepted roots omit prior hashes; additive corrections require a hash-linked predecessor.
    #[serde(default)]
    pub prior_artifact_sha256: Option<Sha256ValueEnvelope>,
    // DEFAULT-JUSTIFICATION: accepted roots add no component kinds; corrections must list newly supplied kinds.
    #[serde(default)]
    pub adds_kinds: Vec<ComponentKind>,
}
