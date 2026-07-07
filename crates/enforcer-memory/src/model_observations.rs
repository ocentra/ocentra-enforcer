//! X06 learning observation contracts for model runtime outcomes.
//!
//! This module defines typed, serializable observation candidates the
//! model-runtime/learning surface can emit before any dedicated
//! persistence seam is built.
//!
//! It is intentionally stand-alone (no graph mutations), so callers can
//! stage these as NDJSON append candidates first and wire them into
//! `Store` follow-up writers later.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::graph::MemoryGraph;
use crate::ingest::{ingest_observation_payload_into_store, Observation};
use crate::model_runtime::{ModelTask, ProviderKind};
use crate::schema::ObservationLogEntry;
use crate::store::Store;

pub const MODEL_RUNTIME_OBSERVATION_SCHEMA_VERSION: u32 = 1;

/// High-level candidate kinds surfaced by model runtime outcome observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelRuntimeObservationKind {
    ModelLoadFailure,
    ProviderDowngrade,
    ArtifactHashMismatch,
    TokenizerHashMismatch,
    DegradedFallback,
    SuccessfulLocalLoad,
    RetrievalQualityProof,
    RerankerLiftProof,
    TokenReductionProof,
    RouteChoiceImprovement,
    RecurrenceOrNegativeEvidence,
}

impl ModelRuntimeObservationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelRuntimeObservationKind::ModelLoadFailure => "model-load-failure",
            ModelRuntimeObservationKind::ProviderDowngrade => "provider-downgrade",
            ModelRuntimeObservationKind::ArtifactHashMismatch => "artifact-hash-mismatch",
            ModelRuntimeObservationKind::TokenizerHashMismatch => "tokenizer-hash-mismatch",
            ModelRuntimeObservationKind::DegradedFallback => "degraded-fallback",
            ModelRuntimeObservationKind::SuccessfulLocalLoad => "successful-local-load",
            ModelRuntimeObservationKind::RetrievalQualityProof => "retrieval-quality-proof",
            ModelRuntimeObservationKind::RerankerLiftProof => "reranker-lift-proof",
            ModelRuntimeObservationKind::TokenReductionProof => "token-reduction-proof",
            ModelRuntimeObservationKind::RouteChoiceImprovement => "route-choice-improvement",
            ModelRuntimeObservationKind::RecurrenceOrNegativeEvidence => {
                "recurrence-or-negative-evidence"
            }
        }
    }
}

/// The store-follow-up envelope: a pure data contract only. This crate
/// does not persist these entries directly yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeObservationRecord {
    pub schema_version: u32,
    pub observed_at: String,
    pub source: String,
    pub run_id: String,
    pub candidate: ModelRuntimeObservationCandidate,
}

impl ModelRuntimeObservationRecord {
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
    ModelLoadFailure(ModelLoadFailure),
    ProviderDowngrade(ProviderDowngrade),
    ArtifactHashMismatch(HashMismatch),
    TokenizerHashMismatch(HashMismatch),
    DegradedFallback(DegradedFallback),
    SuccessfulLocalLoad(LocalLoadSucceeded),
    RetrievalQualityProof(RetrievalQualityProof),
    RerankerLiftProof(RerankerLiftProof),
    TokenReductionProof(TokenReductionProof),
    RouteChoiceImprovement(RouteChoiceImprovement),
    RecurrenceOrNegativeEvidence(RecurrenceOrNegativeEvidence),
}

impl ModelRuntimeObservationCandidate {
    pub fn kind(&self) -> ModelRuntimeObservationKind {
        match self {
            ModelRuntimeObservationCandidate::ModelLoadFailure(_) => {
                ModelRuntimeObservationKind::ModelLoadFailure
            }
            ModelRuntimeObservationCandidate::ProviderDowngrade(_) => {
                ModelRuntimeObservationKind::ProviderDowngrade
            }
            ModelRuntimeObservationCandidate::ArtifactHashMismatch(_) => {
                ModelRuntimeObservationKind::ArtifactHashMismatch
            }
            ModelRuntimeObservationCandidate::TokenizerHashMismatch(_) => {
                ModelRuntimeObservationKind::TokenizerHashMismatch
            }
            ModelRuntimeObservationCandidate::DegradedFallback(_) => {
                ModelRuntimeObservationKind::DegradedFallback
            }
            ModelRuntimeObservationCandidate::SuccessfulLocalLoad(_) => {
                ModelRuntimeObservationKind::SuccessfulLocalLoad
            }
            ModelRuntimeObservationCandidate::RetrievalQualityProof(_) => {
                ModelRuntimeObservationKind::RetrievalQualityProof
            }
            ModelRuntimeObservationCandidate::RerankerLiftProof(_) => {
                ModelRuntimeObservationKind::RerankerLiftProof
            }
            ModelRuntimeObservationCandidate::TokenReductionProof(_) => {
                ModelRuntimeObservationKind::TokenReductionProof
            }
            ModelRuntimeObservationCandidate::RouteChoiceImprovement(_) => {
                ModelRuntimeObservationKind::RouteChoiceImprovement
            }
            ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidence(_) => {
                ModelRuntimeObservationKind::RecurrenceOrNegativeEvidence
            }
        }
    }

    pub fn model_or_query_context(&self) -> String {
        match self {
            ModelRuntimeObservationCandidate::ModelLoadFailure(event) => event.model_id.clone(),
            ModelRuntimeObservationCandidate::ProviderDowngrade(event) => event.model_id.clone(),
            ModelRuntimeObservationCandidate::ArtifactHashMismatch(event)
            | ModelRuntimeObservationCandidate::TokenizerHashMismatch(event) => {
                event.model_id.clone()
            }
            ModelRuntimeObservationCandidate::DegradedFallback(event) => event.model_id.clone(),
            ModelRuntimeObservationCandidate::SuccessfulLocalLoad(event) => event.model_id.clone(),
            ModelRuntimeObservationCandidate::RetrievalQualityProof(event) => event.query.clone(),
            ModelRuntimeObservationCandidate::RerankerLiftProof(event) => event.query.clone(),
            ModelRuntimeObservationCandidate::TokenReductionProof(event) => event.query.clone(),
            ModelRuntimeObservationCandidate::RouteChoiceImprovement(event) => event.query.clone(),
            ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidence(event) => {
                event.lesson_id.clone()
            }
        }
    }

    pub fn is_clean_evidence(&self) -> bool {
        matches!(
            self,
            ModelRuntimeObservationCandidate::SuccessfulLocalLoad(_)
                | ModelRuntimeObservationCandidate::RetrievalQualityProof(_)
                | ModelRuntimeObservationCandidate::RerankerLiftProof(_)
                | ModelRuntimeObservationCandidate::TokenReductionProof(_)
                | ModelRuntimeObservationCandidate::RouteChoiceImprovement(_)
                | ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidence(
                    RecurrenceOrNegativeEvidence {
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
    record: ModelRuntimeObservationRecord,
) -> Result<String> {
    let kind = record.candidate.kind();
    let observation = Observation {
        lesson_id: match &record.candidate {
            ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidence(event) => {
                event.lesson_id.clone()
            }
            _ => String::new(),
        },
        rule_id: None,
        fault_class: Some(kind.as_str().to_owned()),
        repo_context: record.candidate.model_or_query_context(),
        clean: record.candidate.is_clean_evidence(),
        source_surface: record.source.clone(),
        ts: record.observed_at.clone(),
    };
    ingest_observation_payload_into_store(
        store,
        graph,
        observation,
        Some(format!("model-runtime:{}", kind.as_str())),
        Some(serde_json::to_value(record)?),
    )
}

/// Append a model-runtime observation to the canonical store without
/// mutating the in-memory projection.
pub fn record_model_runtime_observation_in_store(
    store: &mut Store,
    record: &ModelRuntimeObservationRecord,
) -> Result<String> {
    let kind = record.candidate.kind();
    let payload = serde_json::to_value(record)?;
    let payload_kind = format!("model-runtime:{}", kind.as_str());
    let observation = Observation {
        lesson_id: match &record.candidate {
            ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidence(event) => {
                event.lesson_id.clone()
            }
            _ => String::new(),
        },
        rule_id: None,
        fault_class: Some(kind.as_str().to_owned()),
        repo_context: record.candidate.model_or_query_context(),
        clean: record.candidate.is_clean_evidence(),
        source_surface: record.source.clone(),
        ts: record.observed_at.clone(),
    };
    let mut assigned_id = String::new();
    store.append_observation_entry(|seq| {
        let id = format!("obs-{seq:04}");
        assigned_id = id.clone();
        ObservationLogEntry {
            schema_version: crate::schema::SCHEMA_VERSION,
            seq,
            id,
            lesson_id: observation.lesson_id.clone(),
            rule_id: observation.rule_id.clone(),
            fault_class: observation.fault_class.clone(),
            repo_context: observation.repo_context.clone(),
            clean: observation.clean,
            source_surface: observation.source_surface.clone(),
            ts: observation.ts.clone(),
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
) -> Result<Vec<ModelRuntimeObservationRecord>> {
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
pub struct ModelLoadFailure {
    pub model_id: String,
    pub task: ModelTask,
    pub requested_provider: Option<ProviderKind>,
    pub failure_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDowngrade {
    pub model_id: String,
    pub task: ModelTask,
    pub requested_provider: ProviderKind,
    pub fallback_provider: ProviderKind,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashMismatch {
    pub model_id: String,
    pub path: String,
    pub expected_sha256: String,
    pub observed_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DegradedFallback {
    pub model_id: String,
    pub task: ModelTask,
    pub fallback_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalLoadSucceeded {
    pub model_id: String,
    pub task: ModelTask,
    pub provider: ProviderKind,
    pub loaded_from_local_cache: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalQualityProof {
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
pub struct RerankerLiftProof {
    pub query_id: String,
    pub query: String,
    pub pre_rerank_top_k: Vec<String>,
    pub post_rerank_top_k: Vec<String>,
    pub lift_score: f64,
    pub improved: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenReductionProof {
    pub query_id: String,
    pub query: String,
    pub naive_tokens: usize,
    pub context_tokens: usize,
}

impl TokenReductionProof {
    pub fn reduction_ratio(&self) -> f64 {
        if self.context_tokens == 0 {
            0.0
        } else {
            self.naive_tokens as f64 / self.context_tokens as f64
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteChoiceImprovement {
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
pub struct RecurrenceOrNegativeEvidence {
    pub lesson_id: String,
    pub query_id: Option<String>,
    pub evidence_kind: RecurrenceNegativeKind,
    pub clean_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecurrenceNegativeKind {
    /// Recurrence count observed after a lesson has landed.
    RecurrenceCount {
        /// Incident count that is now considered recurrence since the
        /// lesson's first landing evidence.
        recurrence_count: usize,
        /// Optional previous count, if a delta was observed.
        previous_count: Option<usize>,
    },
    /// Explicit clean run that produced `no finding`.
    NegativeEvidence {
        /// Why this was considered negative evidence for the lesson.
        reason: String,
    },
}
