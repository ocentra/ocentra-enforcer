//! Proof-run DTO conversion boundary for the project proof read model.
//! Invalid artifact paths are rejected before conversion, with negative
//! snapshot coverage in `tests/read_model_boundary.rs`.
//!
//! ROUNDTRIP-TEST: `tests/boundary_round_trip.rs` serializes and decodes
//! the artifact counts and proof-run summary DTOs.

use enforcer_domain::proof_types::ProofFreshness;

use crate::envelope::ProofRunEnvelope;
use crate::read_model::{ProjectProofRunSummary, ProjectRunArtifacts};

/// Serialized artifact presence counts for one proof run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRunArtifactsDto {
    pub declared: usize,
    pub present: usize,
    pub missing: usize,
    pub total_bytes: u64,
}

/// Serialized summary of one discovered proof run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProofRunSummaryDto {
    pub path: String,
    pub proof_run: Option<ProofRunEnvelope>,
    pub freshness: ProofFreshness,
    pub artifacts: ProjectRunArtifactsDto,
    pub parse_error: Option<String>,
}

impl From<ProjectRunArtifacts> for ProjectRunArtifactsDto {
    fn from(value: ProjectRunArtifacts) -> Self {
        Self {
            declared: value.declared,
            present: value.present,
            missing: value.missing,
            total_bytes: value.total_bytes,
        }
    }
}

impl From<ProjectRunArtifactsDto> for ProjectRunArtifacts {
    fn from(value: ProjectRunArtifactsDto) -> Self {
        Self {
            declared: value.declared,
            present: value.present,
            missing: value.missing,
            total_bytes: value.total_bytes,
        }
    }
}

impl From<ProjectProofRunSummary> for ProjectProofRunSummaryDto {
    fn from(value: ProjectProofRunSummary) -> Self {
        Self {
            path: value.path.as_str().to_owned(),
            proof_run: value.proof_run,
            freshness: value.freshness,
            artifacts: value.artifacts.into(),
            parse_error: value.parse_error,
        }
    }
}

impl TryFrom<ProjectProofRunSummaryDto> for ProjectProofRunSummary {
    type Error = enforcer_core::error::Error;

    fn try_from(value: ProjectProofRunSummaryDto) -> Result<Self, Self::Error> {
        Ok(Self {
            path: enforcer_domain::paths::RelPath::try_from(value.path)
                .map_err(enforcer_core::error::Error::Decode)?,
            proof_run: value.proof_run,
            freshness: value.freshness,
            artifacts: value.artifacts.into(),
            parse_error: value.parse_error,
        })
    }
}
