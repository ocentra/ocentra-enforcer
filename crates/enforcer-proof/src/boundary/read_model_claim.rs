//! BOUNDARY-INVARIANT: this boundary module validates raw wire values and converts only through typed domain contracts.
//! Claim DTO conversion boundary for the project proof read model.
//! Invalid branded claim values are rejected before conversion, with
//! negative claim coverage in the proof boundary tests.
//!
//! ROUNDTRIP-TEST: `tests/boundary_round_trip.rs` serializes and decodes
//! `ProjectClaimSummaryDto`, preserving its branded proof identifiers.

use enforcer_domain::proof_types::{ProjectClaimState, ProofId};

use crate::claim::ClaimEnvelope;
use crate::read_model::ProjectClaimSummary;

/// Serialized result of evaluating the project-local PR-ready claim.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectClaimSummaryDto {
    pub registry_path: String,
    pub state: ProjectClaimState,
    pub required_proof_ids: Vec<ProofId>,
    pub claim: Option<ClaimEnvelope>,
    pub error: Option<String>,
}

impl From<ProjectClaimSummary> for ProjectClaimSummaryDto {
    fn from(value: ProjectClaimSummary) -> Self {
        Self {
            registry_path: value.registry_path.as_str().to_owned(),
            state: value.state,
            required_proof_ids: value.required_proof_ids,
            claim: value.claim,
            error: value.error,
        }
    }
}

impl TryFrom<ProjectClaimSummaryDto> for ProjectClaimSummary {
    type Error = enforcer_core::error::Error;

    fn try_from(value: ProjectClaimSummaryDto) -> Result<Self, Self::Error> {
        Ok(Self {
            registry_path: enforcer_domain::paths::RelPath::try_from(value.registry_path)
                .map_err(enforcer_core::error::Error::Decode)?,
            state: value.state,
            required_proof_ids: value.required_proof_ids,
            claim: value.claim,
            error: value.error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectClaimSummary, ProjectClaimSummaryDto};

    #[test]
    fn project_claim_summary_dto_rejects_an_invalid_registry_path() -> serde_json::Result<()> {
        let dto: ProjectClaimSummaryDto = serde_json::from_value(serde_json::json!({
            "registryPath":"../escape", "state":"blocked", "requiredProofIds":[], "claim":null, "error":null
        }))?;
        assert!(matches!(
            ProjectClaimSummary::try_from(dto),
            Err(enforcer_core::error::Error::Decode(_))
        ));
        Ok(())
    }
}
