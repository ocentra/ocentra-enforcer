//! X06 learning observation contracts for model runtime outcomes.
//!
//! This module defines typed, serializable observation candidates the
//! model-runtime/learning surface emits for load failures, provider
//! downgrade, hash/tokenizer mismatches, degraded fallback, successful
//! local loads, and RAG quality proofs.
//!
//! Store-backed callers append each candidate to the dedicated
//! model-observation log and bridge it into the generic observation log
//! so learning projections can replay durable evidence without a manual
//! capture step.
//!
//! ROUNDTRIP-TEST: tests/model_observations.rs::observation_dto_domain_boundary_conversions_round_trip

use enforcer_domain::memory_types::ModelRuntimeObservationKind;
use enforcer_domain::memory_types::RecurrenceNegativeKind;
use serde::{Deserialize, Serialize};

use crate::boundary::log_schema::ObservationLogEntryDto;
use crate::error::Result;
use crate::graph::MemoryGraph;
use crate::ingest::{ingest_observation_payload_into_store, Observation};
use crate::owned_boundary::Retained;
use crate::store::Store;
use enforcer_domain::memory_types::IngestIncidentId;
use enforcer_domain::memory_types::{ModelTask, ProviderKind};

pub const MODEL_RUNTIME_OBSERVATION_SCHEMA_VERSION: u32 = 1;

/// Durable model-runtime observation envelope stored in the native
/// model-observation log and mirrored into the generic observation log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeObservationRecordDto {
    pub schema_version: u32,
    pub observed_at: String,
    pub source: String,
    pub run_id: String,
    pub candidate: ModelRuntimeObservationCandidate,
}

impl ModelRuntimeObservationRecordDto {
    pub fn new(
        observed_at: impl Into<String>,
        source: impl Into<String>,
        run_id: impl Into<String>,
        candidate: ModelRuntimeObservationCandidate,
    ) -> Self {
        Self {
            schema_version: MODEL_RUNTIME_OBSERVATION_SCHEMA_VERSION,
            observed_at: observed_at.into(),
            source: source.into(),
            run_id: run_id.into(),
            candidate,
        }
    }
}

/// One typed event shape for a model runtime outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "observationKind", rename_all = "kebab-case")]
pub enum ModelRuntimeObservationCandidate {
    #[serde(rename = "model-load-failure")]
    ModelLoadFailureDto(ModelLoadFailureDto),
    #[serde(rename = "provider-downgrade")]
    ProviderDowngradeDto(ProviderDowngradeDto),
    ArtifactHashMismatch(HashMismatchDto),
    TokenizerHashMismatch(HashMismatchDto),
    #[serde(rename = "degraded-fallback")]
    DegradedFallbackDto(DegradedFallbackDto),
    SuccessfulLocalLoad(LocalLoadSucceededDto),
    #[serde(rename = "retrieval-quality-proof")]
    RetrievalQualityProofDto(RetrievalQualityProofDto),
    #[serde(rename = "reranker-lift-proof")]
    RerankerLiftProofDto(RerankerLiftProofDto),
    #[serde(rename = "token-reduction-proof")]
    TokenReductionProofDto(TokenReductionProofDto),
    #[serde(rename = "route-choice-improvement")]
    RouteChoiceImprovementDto(RouteChoiceImprovementDto),
    #[serde(rename = "recurrence-or-negative-evidence")]
    RecurrenceOrNegativeEvidenceDto(RecurrenceOrNegativeEvidenceDto),
}

impl ModelRuntimeObservationCandidate {
    pub fn kind(&self) -> ModelRuntimeObservationKind {
        match self {
            ModelRuntimeObservationCandidate::ModelLoadFailureDto(_) => {
                ModelRuntimeObservationKind::ModelLoadFailure
            }
            ModelRuntimeObservationCandidate::ProviderDowngradeDto(_) => {
                ModelRuntimeObservationKind::ProviderDowngrade
            }
            ModelRuntimeObservationCandidate::ArtifactHashMismatch(_) => {
                ModelRuntimeObservationKind::ArtifactHashMismatch
            }
            ModelRuntimeObservationCandidate::TokenizerHashMismatch(_) => {
                ModelRuntimeObservationKind::TokenizerHashMismatch
            }
            ModelRuntimeObservationCandidate::DegradedFallbackDto(_) => {
                ModelRuntimeObservationKind::DegradedFallback
            }
            ModelRuntimeObservationCandidate::SuccessfulLocalLoad(_) => {
                ModelRuntimeObservationKind::SuccessfulLocalLoad
            }
            ModelRuntimeObservationCandidate::RetrievalQualityProofDto(_) => {
                ModelRuntimeObservationKind::RetrievalQualityProof
            }
            ModelRuntimeObservationCandidate::RerankerLiftProofDto(_) => {
                ModelRuntimeObservationKind::RerankerLiftProof
            }
            ModelRuntimeObservationCandidate::TokenReductionProofDto(_) => {
                ModelRuntimeObservationKind::TokenReductionProof
            }
            ModelRuntimeObservationCandidate::RouteChoiceImprovementDto(_) => {
                ModelRuntimeObservationKind::RouteChoiceImprovement
            }
            ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidenceDto(_) => {
                ModelRuntimeObservationKind::RecurrenceOrNegativeEvidence
            }
        }
    }

    pub fn model_or_query_context(&self) -> String {
        match self {
            ModelRuntimeObservationCandidate::ModelLoadFailureDto(event) => {
                event.model_id.retained()
            }
            ModelRuntimeObservationCandidate::ProviderDowngradeDto(event) => {
                event.model_id.retained()
            }
            ModelRuntimeObservationCandidate::ArtifactHashMismatch(event)
            | ModelRuntimeObservationCandidate::TokenizerHashMismatch(event) => {
                event.model_id.retained()
            }
            ModelRuntimeObservationCandidate::DegradedFallbackDto(event) => {
                event.model_id.retained()
            }
            ModelRuntimeObservationCandidate::SuccessfulLocalLoad(event) => {
                event.model_id.retained()
            }
            ModelRuntimeObservationCandidate::RetrievalQualityProofDto(event) => {
                event.query.retained()
            }
            ModelRuntimeObservationCandidate::RerankerLiftProofDto(event) => event.query.retained(),
            ModelRuntimeObservationCandidate::TokenReductionProofDto(event) => {
                event.query.retained()
            }
            ModelRuntimeObservationCandidate::RouteChoiceImprovementDto(event) => {
                event.query.retained()
            }
            ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidenceDto(event) => {
                event.lesson_id.retained()
            }
        }
    }

    pub fn is_clean_evidence(&self) -> bool {
        matches!(
            self,
            ModelRuntimeObservationCandidate::SuccessfulLocalLoad(_)
                | ModelRuntimeObservationCandidate::RetrievalQualityProofDto(_)
                | ModelRuntimeObservationCandidate::RerankerLiftProofDto(_)
                | ModelRuntimeObservationCandidate::TokenReductionProofDto(_)
                | ModelRuntimeObservationCandidate::RouteChoiceImprovementDto(_)
                | ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidenceDto(
                    RecurrenceOrNegativeEvidenceDto {
                        clean_evidence: true,
                        ..
                    }
                )
        )
    }
}

pub fn ingest_model_runtime_observation(
    store: &mut Store,
    graph: &mut MemoryGraph,
    record: &ModelRuntimeObservationRecordDto,
) -> Result<IngestIncidentId> {
    store.append_model_observation(record.retained())?;
    let kind = record.candidate.kind();
    let observation = Observation {
        lesson_id: (match &record.candidate {
            ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidenceDto(event) => {
                event.lesson_id.retained()
            }
            _ => String::new(),
        })
        .into(),
        rule_id: None,
        fault_class: Some(kind.as_str().retained().into()),
        repo_context: record.candidate.model_or_query_context().into(),
        clean: record.candidate.is_clean_evidence().into(),
        source_surface: record.source.retained().into(),
        ts: record.observed_at.retained().into(),
    };
    ingest_observation_payload_into_store(
        store,
        graph,
        observation,
        Some(format!("model-runtime:{}", kind.as_str()).into()),
        Some(serde_json::to_string(&record)?.into()),
    )
}

/// Append a model-runtime observation to the canonical store without
/// mutating the in-memory projection.
pub fn record_model_runtime_observation_in_store(
    store: &mut Store,
    record: &ModelRuntimeObservationRecordDto,
) -> Result<String> {
    store.append_model_observation(record.retained())?;
    let kind = record.candidate.kind();
    let payload = serde_json::to_value(record)?;
    let payload_kind = format!("model-runtime:{}", kind.as_str());
    let observation = Observation {
        lesson_id: (match &record.candidate {
            ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidenceDto(event) => {
                event.lesson_id.retained()
            }
            _ => String::new(),
        })
        .into(),
        rule_id: None,
        fault_class: Some(kind.as_str().retained().into()),
        repo_context: record.candidate.model_or_query_context().into(),
        clean: record.candidate.is_clean_evidence().into(),
        source_surface: record.source.retained().into(),
        ts: record.observed_at.retained().into(),
    };
    let mut assigned_id = String::new();
    store.append_observation_entry(|seq| {
        let id = format!("obs-{seq:04}");
        assigned_id = id.retained();
        ObservationLogEntryDto {
            schema_version: crate::boundary::log_schema::SCHEMA_VERSION,
            seq: seq.into(),
            id,
            lesson_id: observation.lesson_id.retained().into(),
            rule_id: observation.rule_id.retained().map(Into::into),
            fault_class: observation.fault_class.retained().map(Into::into),
            repo_context: observation.repo_context.retained().into(),
            clean: observation.clean.is_clean(),
            source_surface: observation.source_surface.retained().into(),
            ts: observation.ts.retained().into(),
            supersedes_seq: None,
            payload_kind: Some(payload_kind),
            payload: Some(payload),
        }
    })?;
    Ok(assigned_id)
}

/// Rebuild a deterministic, read-only projection of model-runtime
/// observations from the canonical store log.
pub fn project_model_runtime_observations_from_store(
    store: &Store,
) -> Result<Vec<ModelRuntimeObservationRecordDto>> {
    let native = store.read_model_observation_entries()?;
    if !native.entries.is_empty() {
        return Ok(native
            .entries
            .into_iter()
            .map(|entry| ModelRuntimeObservationRecordDto {
                schema_version: entry.schema_version,
                observed_at: entry.observed_at,
                source: entry.source,
                run_id: entry.run_id,
                candidate: entry.candidate,
            })
            .collect());
    }
    project_model_runtime_observations_from_legacy_observation_log(store)
}

fn project_model_runtime_observations_from_legacy_observation_log(
    store: &Store,
) -> Result<Vec<ModelRuntimeObservationRecordDto>> {
    let outcome = store.read_observation_entries()?;
    let mut records = Vec::new();
    for entry in outcome.entries {
        if !matches!(
            entry.payload_kind.as_deref(),
            Some(kind) if kind.starts_with("model-runtime:")
        ) {
            continue;
        }
        if let Some(payload) = entry.payload {
            records.push(serde_json::from_value(payload)?);
        }
    }
    Ok(records)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelLoadFailureDto {
    pub model_id: String,
    pub task: ModelTask,
    pub requested_provider: Option<ProviderKind>,
    pub failure_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDowngradeDto {
    pub model_id: String,
    pub task: ModelTask,
    pub requested_provider: ProviderKind,
    pub fallback_provider: ProviderKind,
    pub reason: String,
}

/// The fallback provider is the canonical domain decision represented by the
/// provider-downgrade wire event; model id and reason remain observation data.
impl From<ProviderDowngradeDto> for ProviderKind {
    fn from(value: ProviderDowngradeDto) -> Self {
        value.fallback_provider
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashMismatchDto {
    pub model_id: String,
    pub path: String,
    pub expected_sha256: String,
    pub observed_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DegradedFallbackDto {
    pub model_id: String,
    pub task: ModelTask,
    pub fallback_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalLoadSucceededDto {
    pub model_id: String,
    pub task: ModelTask,
    pub provider: ProviderKind,
    pub loaded_from_local_cache: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalQualityProofDto {
    pub query_id: String,
    pub query: String,
    pub route: String,
    pub recall_at_five: f64,
    pub recall_at_ten: f64,
    pub precision_at_five: f64,
    pub expected_top_k: usize,
    pub returned_top_k: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RerankerLiftProofDto {
    pub query_id: String,
    pub query: String,
    pub pre_rerank_top_k: Vec<String>,
    pub post_rerank_top_k: Vec<String>,
    pub lift_score: f64,
    pub improved: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenReductionProofDto {
    pub query_id: String,
    pub query: String,
    pub naive_tokens: usize,
    pub context_tokens: usize,
}

impl std::fmt::Debug for TokenReductionProofDto {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TokenReductionProofDto")
            .field("query_id", &"[REDACTED]")
            .field("query", &"[REDACTED]")
            .field("naive_tokens", &self.naive_tokens)
            .field("context_tokens", &self.context_tokens)
            .finish()
    }
}

impl TokenReductionProofDto {
    pub fn reduction_ratio(&self) -> f64 {
        if self.context_tokens == 0 {
            0.0
        } else {
            crate::owned_boundary::usize_to_f64(self.naive_tokens)
                / crate::owned_boundary::usize_to_f64(self.context_tokens)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteChoiceImprovementDto {
    pub query_id: String,
    pub query: String,
    pub chosen_route: String,
    pub alternative_route: String,
    pub chosen_route_score: f64,
    pub alternative_route_score: f64,
    pub improvement_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceOrNegativeEvidenceDto {
    pub lesson_id: String,
    pub query_id: Option<String>,
    #[serde(with = "crate::boundary::model_observation::recurrence_kind_wire")]
    pub evidence_kind: RecurrenceNegativeKind,
    pub clean_evidence: bool,
}
