//! Serialization boundary for the project proof read model.
//!
//! These DTOs are the Tauri/API response contract. Raw strings remain here
//! because JSON is the explicit external representation; proof-domain code
//! uses the canonical values before constructing this response.
//! Invalid external artifact paths are rejected before DTO construction, with
//! negative coverage in `tests/read_model_boundary.rs`.

use std::path::Path;

use crate::boundary::read_model_claim::ProjectClaimSummaryDto;
use crate::boundary::read_model_journal::ProjectJournalSummaryDto;
use crate::boundary::read_model_run::ProjectProofRunSummaryDto;
use enforcer_core::error::Result;

use crate::envelope::GitStateEnvelope;
use crate::read_model::ProjectProofSnapshot;

// ROUNDTRIP-TEST: the integration boundary tests serialize snapshot fixtures and decode them back.

/// Serialized project proof snapshot returned to desktop/API consumers.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProofSnapshotDto {
    pub proof_root: String,
    pub current_git: GitStateEnvelope,
    pub journal: ProjectJournalSummaryDto,
    pub runs: Vec<ProjectProofRunSummaryDto>,
    pub claim: ProjectClaimSummaryDto,
}

impl From<ProjectProofSnapshot> for ProjectProofSnapshotDto {
    fn from(value: ProjectProofSnapshot) -> Self {
        Self {
            proof_root: value.proof_root.as_str().to_owned(),
            current_git: value.current_git,
            journal: value.journal.into(),
            runs: value.runs.into_iter().map(Into::into).collect(),
            claim: value.claim.into(),
        }
    }
}

impl TryFrom<ProjectProofSnapshotDto> for ProjectProofSnapshot {
    type Error = enforcer_core::error::Error;

    fn try_from(value: ProjectProofSnapshotDto) -> Result<Self> {
        Ok(Self {
            proof_root: enforcer_domain::paths::RelPath::try_from(value.proof_root)
                .map_err(enforcer_core::error::Error::Decode)?,
            current_git: value.current_git,
            journal: value.journal.try_into()?,
            runs: value
                .runs
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>>>()?,
            claim: value.claim.try_into()?,
        })
    }
}

/// Read the internal typed snapshot and serialize only at the public API edge.
pub fn read_project_proof_snapshot(root: &Path) -> Result<ProjectProofSnapshotDto> {
    crate::read_model::read_project_proof_snapshot(root).map(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{ProjectProofSnapshot, ProjectProofSnapshotDto};

    #[test]
    fn project_proof_snapshot_dto_rejects_an_invalid_proof_root() -> serde_json::Result<()> {
        let dto: ProjectProofSnapshotDto = serde_json::from_value(serde_json::json!({
            "proofRoot":"../escape", "currentGit":{"commit":"abcdef0","branch":"rust-build","dirty":false},
            "journal":{"path":".enforce/proofs/events.ndjson","state":"verified","recordCount":0,"latestEventType":null,"latestProofId":null,"latestTimestamp":null,"error":null},
            "runs":[], "claim":{"registryPath":"proofs.json","state":"blocked","requiredProofIds":[],"claim":null,"error":null}
        }))?;
        assert!(matches!(
            ProjectProofSnapshot::try_from(dto),
            Err(enforcer_core::error::Error::Decode(_))
        ));
        Ok(())
    }
}
