//! Typed CyberSkills disposition ledger boundary.
//!
//! This module deliberately models source identity and decomposition state
//! separately.  An available but unreviewed source is not given a guessed
//! component kind, and the protected source-unavailable identity is never
//! treated as reviewed content.
//! This module owns the serialized wire DTO family for the persisted CP00
//! ledger and validates it before deriving any runtime projection.
//!
//! BOUNDARY-INVARIANT: raw CyberSkills JSON is decoded here, then validated
//! into the closed disposition vocabulary before any derived count is used.
//! NEGATIVE-TEST: `tests/cyberskills_disposition.rs` rejects malformed,
//! duplicate, unavailable-source, and incomplete component cases.
//! ROUNDTRIP-TEST: `tests/cyberskills_disposition.rs::manifest_round_trips_without_count_drift`
//! serializes and reparses the complete typed contract before validation.
// ROUNDTRIP-TEST: crates/enforcer-rules/tests/cyberskills_disposition.rs

#[path = "cyberskills_disposition/components.rs"]
mod components;
#[path = "cyberskills_disposition/cp08_chain.rs"]
mod cp08_chain;
#[path = "cyberskills_disposition/implementation_truth.rs"]
mod implementation_truth;
#[path = "cyberskills_disposition/manifest.rs"]
mod manifest;
#[path = "cyberskills_disposition/projection.rs"]
mod projection;
#[path = "cyberskills_disposition/provenance.rs"]
mod provenance;
#[path = "cyberskills_disposition/source.rs"]
mod source;
/// Validated semantic vocabulary and role-specific domain wrappers.
#[path = "cyberskills_disposition/types.rs"]
pub mod types;
/// Serde-owned DTOs used at the CP00 persistence boundary.
#[path = "cyberskills_disposition/wire.rs"]
pub mod wire;

pub(super) fn require(condition: bool, error: impl FnOnce() -> String) -> Result<(), String> {
    condition.then_some(()).ok_or_else(error)
}

pub(super) fn ensure(condition: bool, error: String) -> Result<(), String> {
    condition.then_some(()).ok_or(error)
}

pub const PROTECTED_CATALOG_ID: &str = "detecting-fileless-malware-techniques";
pub const PROTECTED_SOURCE_PATH: &str =
    "vendor/anthropic-cybersecurity-skills/skills/detecting-fileless-malware-techniques/SKILL.md";
pub const PROTECTED_TRACKED_BLOB: &str = "df48fa4149dd25956e730443d3582693a3f825a8";

/// Decode the CP00 manifest JSON at the boundary.
pub fn parse_manifest(
    raw: &str,
) -> Result<wire::manifest::CyberSkillsDispositionManifestDto, serde_json::Error> {
    serde_json::from_str(raw)
}

#[derive(Debug, Default, PartialEq, Eq)]
/// Counts derived from validated rows and components.
pub struct DerivedDispositionCounts {
    pub identity_rows: usize,
    pub readable_sources: usize,
    pub source_unavailable: usize,
    pub reviewed_rows: usize,
    pub decomposed_rows: usize,
    pub implemented_components: usize,
    pub proved_components: usize,
    pub advisory_retained: usize,
    pub manual_retained: usize,
    pub unexplained_rows: usize,
    pub cp08_complete_rows: usize,
    pub cp08_partial_rows: usize,
    pub cp08_component_count: usize,
    pub cp08_missing_native: usize,
    pub cp08_missing_external: usize,
}

/// Validate the semantic CP00 contract after serde has checked the closed
/// vocabularies and unknown-field boundary.
/// Validate identity, source, decomposition, and evidence invariants.
pub fn validate_manifest(
    manifest: &wire::manifest::CyberSkillsDispositionManifestDto,
) -> Result<DerivedDispositionCounts, String> {
    manifest::validate_manifest(manifest)
}
