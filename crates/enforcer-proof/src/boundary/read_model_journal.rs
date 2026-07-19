//! Journal DTO conversion boundary for the project proof read model.
//! Invalid branded journal values are rejected before conversion, with
//! negative snapshot coverage in `tests/read_model_boundary.rs`.
//!
//! ROUNDTRIP-TEST: `tests/boundary_round_trip.rs` serializes and decodes
//! `ProjectJournalSummaryDto`, including the latest event and proof values.

use enforcer_domain::proof_types::{JournalEventType, JournalState, ProofId};

use crate::read_model::ProjectJournalSummary;

/// Serialized journal state returned to API consumers.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectJournalSummaryDto {
    pub path: String,
    pub state: JournalState,
    pub record_count: usize,
    pub latest_event_type: Option<JournalEventType>,
    pub latest_proof_id: Option<ProofId>,
    pub latest_timestamp: Option<String>,
    pub error: Option<String>,
}

impl From<ProjectJournalSummary> for ProjectJournalSummaryDto {
    fn from(value: ProjectJournalSummary) -> Self {
        Self {
            path: value.path.as_str().to_owned(),
            state: value.state,
            record_count: value.record_count,
            latest_event_type: value.latest_event_type,
            latest_proof_id: value.latest_proof_id,
            latest_timestamp: value.latest_timestamp,
            error: value.error,
        }
    }
}

impl TryFrom<ProjectJournalSummaryDto> for ProjectJournalSummary {
    type Error = enforcer_core::error::Error;

    fn try_from(value: ProjectJournalSummaryDto) -> Result<Self, Self::Error> {
        Ok(Self {
            path: enforcer_domain::paths::RelPath::try_from(value.path)
                .map_err(enforcer_core::error::Error::Decode)?,
            state: value.state,
            record_count: value.record_count,
            latest_event_type: value.latest_event_type,
            latest_proof_id: value.latest_proof_id,
            latest_timestamp: value.latest_timestamp,
            error: value.error,
        })
    }
}
