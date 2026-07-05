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

use crate::model_runtime::{ModelTask, ProviderKind};

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
