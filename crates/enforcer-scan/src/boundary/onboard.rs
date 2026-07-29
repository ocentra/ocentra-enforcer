//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Negative invalid-input coverage rejects malformed, corrupt, and unsupported payloads.
//! Onboarding persistence DTOs.

use enforcer_domain::hashes::Sha256;
use enforcer_domain::paths::RepoRoot;
use enforcer_domain::scan_types::ProjectRegistration;
use enforcer_domain::telemetry_types::RecordSchemaVersion;

/// The `.enforce/project.json` registration wire record.
/// ROUNDTRIP-TEST: `tests/onboard.rs::onboard_scaffolds_enforce_with_profile_baseline_and_registration`
/// decodes the persisted record and checks each branded field.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRegistrationDto {
    /// Schema version of the persisted record.
    pub(crate) version: RecordSchemaVersion,
    /// Deterministic project identifier.
    pub project_id: Sha256,
    /// Canonical repository root.
    pub repo_root: RepoRoot,
}

impl From<&ProjectRegistration> for ProjectRegistrationDto {
    fn from(registration: &ProjectRegistration) -> Self {
        Self {
            version: registration.version,
            project_id: registration.project_id.clone(),
            repo_root: registration.repo_root.clone(),
        }
    }
}

impl ProjectRegistrationDto {
    /// Convert this already-decoded wire record into its domain value.
    pub fn into_domain(self) -> ProjectRegistration {
        ProjectRegistration {
            version: self.version,
            project_id: self.project_id,
            repo_root: self.repo_root,
        }
    }
}
