//! Serialized manifest DTOs for the CP00 persistence boundary.
//!
//! BOUNDARY-INVARIANT: manifest DTOs carry only decoded persisted row data.
//! NEGATIVE-TEST: crates/enforcer-rules/tests/cyberskills_disposition/manifest.rs
//! rejects malformed manifest, record, and component values.
//! ROUNDTRIP-TEST: crates/enforcer-rules/tests/cyberskills_disposition/manifest.rs
//! contains manifest, record, and component codec cycles.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::cp08::{Cp08ProjectionDto, SourceIdentityDto};
use super::implementation::ImplementationTruthDto;
use crate::cyberskills_disposition::types::{
    DecompositionState, LegacyDisposition, SourceAvailability,
};

/// Decoded CP00 CyberSkills disposition manifest.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CyberSkillsDispositionManifestDto {
    pub schema_version: u32,
    pub source_catalog: String,
    pub source_vendor_root: String,
    pub mapping_policy: String,
    pub records: Vec<CyberSkillDispositionRecordDto>,
}

/// Decoded per-skill identity and decomposition record.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CyberSkillDispositionRecordDto {
    pub catalog_id: String,
    pub source_path: String,
    pub source_availability: SourceAvailability,
    pub decomposition_state: DecompositionState,
    // DEFAULT-JUSTIFICATION: absent source hash/anchors are permitted only for sourceUnavailable rows and rejected by validation.
    #[serde(default)]
    pub source_sha256: Option<String>,
    // DEFAULT-JUSTIFICATION: absent source hash/anchors are permitted only for sourceUnavailable rows and rejected by validation.
    #[serde(default)]
    pub source_anchors: Vec<String>,
    pub attribution: Value,
    // DEFAULT-JUSTIFICATION: legacy v1 fields remain optional during typed migration; record validation requires disposition/rationale.
    #[serde(default)]
    pub legacy_disposition: Option<LegacyDisposition>,
    // DEFAULT-JUSTIFICATION: legacy v1 fields remain optional during typed migration; record validation requires disposition/rationale.
    #[serde(default)]
    pub legacy_rationale: Option<String>,
    // DEFAULT-JUSTIFICATION: legacy v1 fields remain optional during typed migration; record validation requires disposition/rationale.
    #[serde(default)]
    pub legacy: Option<Value>,
    // DEFAULT-JUSTIFICATION: only sourceUnavailable rows may omit this object; validation enforces the protected identity contract.
    #[serde(default)]
    pub unavailable_source: Option<Value>,
    // DEFAULT-JUSTIFICATION: unreviewed rows intentionally carry no guessed components.
    #[serde(default)]
    pub components: Vec<CyberSkillComponentDto>,
    /// V3 source projection, validated against the flat compatibility fields.
    // DEFAULT-JUSTIFICATION: v2 manifests omit the v3 source object; v3 validation rejects the omission.
    #[serde(default)]
    pub source: Option<SourceIdentityDto>,
    /// Minimal verified projection of immutable CP08 evidence.
    // DEFAULT-JUSTIFICATION: v2 manifests omit the v3 projection; v3 validation rejects the omission.
    #[serde(default)]
    pub cp08_projection: Option<Cp08ProjectionDto>,
    /// Independent implementation and executable-proof truth.
    // DEFAULT-JUSTIFICATION: v2 manifests omit the v3 implementation truth; v3 validation rejects the omission.
    #[serde(default)]
    pub implementation: Option<ImplementationTruthDto>,
}

/// Decoded independently evidenced disposition component.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CyberSkillComponentDto {
    pub component_id: String,
    pub kind: crate::cyberskills_disposition::types::ComponentKind,
    pub tier: crate::cyberskills_disposition::types::ComponentTier,
    pub status: crate::cyberskills_disposition::types::ComponentStatus,
    pub coverage_kind: crate::cyberskills_disposition::types::CoverageKind,
    // DEFAULT-JUSTIFICATION: component predicate is required for mechanical kinds and absent for retained guidance.
    #[serde(default)]
    pub predicate: Option<String>,
    // DEFAULT-JUSTIFICATION: retained advisory/manual components require purpose by validation.
    #[serde(default)]
    pub purpose: Option<String>,
    // DEFAULT-JUSTIFICATION: blocked components may defer implementation until a named dependency is resolved.
    #[serde(default)]
    pub implementation_ref: Option<Value>,
    // DEFAULT-JUSTIFICATION: evidence is accumulated as each component proof closes.
    #[serde(default)]
    pub evidence_refs: Vec<Value>,
    pub not_proved: Vec<String>,
}
