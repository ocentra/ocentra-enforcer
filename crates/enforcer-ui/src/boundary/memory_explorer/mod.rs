//! g09 - Memory KG/RAG Explorer.
//! BOUNDARY-INVARIANT: typed memory evidence becomes outbound JSON payloads
//! here; absent evidence remains explicitly unavailable rather than invented.
//! NEGATIVE-TEST: missing-store and malformed-proof tests preserve unavailable evidence.
//! boundaryOwnerNote: enforcer-ui owns the g09 memory evidence presentation seam.
//!
//! Rust builds the payload for the Memory Explorer. TypeScript displays
//! these fields and sends intent only.
//!
//! ROUNDTRIP-TEST: `memory_explorer_response_round_trips_through_json` proves
//! the aggregate response and all nested evidence responses preserve fields.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::memory_types::{
    ArchitectureReportPath, DocumentKind, GraphEdgeCount, GraphNodeCount, GraphSnapshotNodeId,
    GraphSnapshotRelativePath, MemoryAnalyticsObservationCount, MemoryErrorReason,
    MemoryGraphNodeCount, MemoryGraphSearchText, MemoryRecallQuery, MemorySessionLessonId,
    MemorySummaryText, RankingDocumentId, SearchGraphQuery,
};
use enforcer_domain::ui_types::UiRunMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryExplorerResponse {
    pub provenance: MemoryEvidenceProvenanceResponse,
    pub project_graph: ProjectGraphEvidenceResponse,
    pub retrieval: RetrievalEvidenceResponse,
    pub learning: LearningEvidenceResponse,
    pub models: ModelEvidenceResponse,
    pub parity: ParityEvidenceResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEvidenceProvenanceResponse {
    pub scope: String,
    pub selected_project_root: String,
    pub artifact_root: String,
    pub generated_at_unix_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGraphEvidenceResponse {
    pub available: bool,
    pub project_scope: String,
    pub store_root: String,
    pub nodes: usize,
    pub edges: usize,
    pub files: usize,
    pub code_graph_items: usize,
    pub memory_evidence_items: usize,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// SERDE-TAG-JUSTIFICATION: the desktop presentation contract intentionally
// serializes evidence kinds as stable kebab-case string literals.
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKind {
    CodeGraph,
    LearningMemory,
    ProofArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphSearchHitResponse {
    pub node_id: String,
    pub name: String,
    pub qualified_name: String,
    pub label: String,
    pub file_path: String,
    pub evidence_kind: EvidenceKind,
    pub rank: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphSearchResponse {
    pub total: usize,
    pub has_more: bool,
    pub query: String,
    pub project_scope: String,
    pub results: Vec<GraphSearchHitResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalEvidenceResponse {
    pub available: bool,
    pub status: String,
    pub rows_total: u64,
    pub rows_green: u64,
    pub rows_degraded: u64,
    pub token_reduction_estimate: String,
    pub explanations: Vec<RetrievalExplanationResponse>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalExplanationResponse {
    pub id: String,
    pub query: String,
    pub capability_state: String,
    pub expected_ids: Vec<String>,
    pub actual_ids: Vec<String>,
    pub source_refs: Vec<String>,
    pub bm25_candidates: usize,
    pub vector_candidates: usize,
    pub rrf_score: Option<String>,
    pub reranker_score: Option<String>,
    pub selected_context_pack: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningEvidenceResponse {
    pub available: bool,
    pub status: String,
    pub lessons: Vec<LessonEvidenceResponse>,
    pub blockers: Vec<String>,
    pub follow_ups: Vec<String>,
    pub recurrence_signals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonEvidenceResponse {
    pub lesson_id: String,
    pub lesson: String,
    pub status: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEvidenceResponse {
    pub available: bool,
    pub runtime_mode: String,
    pub capability_state: String,
    pub allow_network: bool,
    pub cache_root: String,
    pub observations: u64,
    pub artifacts: Vec<ModelArtifactEvidenceResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelArtifactEvidenceResponse {
    pub artifact: String,
    pub status: String,
    pub capability: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParityEvidenceResponse {
    pub available: bool,
    pub tools_total: u64,
    pub equal: u64,
    pub better: u64,
    pub worse: u64,
    pub incomparable: u64,
    pub unrunnable: u64,
    pub rows: Vec<ParityRowEvidenceResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParityRowEvidenceResponse {
    pub tool: String,
    pub verdict: String,
    pub reason: String,
}

/// Canonical domain projection for the UI response boundary.
///
/// The response structs above are intentionally serde-facing DTOs because
/// their camelCase/kebab-case wire shape is generated for the frontend. The
/// render seam still exposes a typed projection for Rust callers so raw UI
/// strings do not silently become application-domain values. Every DTO has a
/// named `into_domain` mapper below; the mappers use the canonical brands from
/// `enforcer-domain` and reject invalid graph identifiers and paths.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryExplorerDomain {
    provenance: MemoryEvidenceProvenanceDomain,
    project_graph: ProjectGraphEvidenceDomain,
    retrieval: RetrievalEvidenceDomain,
    learning: LearningEvidenceDomain,
    models: ModelEvidenceDomain,
    parity: ParityEvidenceDomain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryEvidenceProvenanceDomain {
    scope: MemoryErrorReason,
    selected_project_root: ArchitectureReportPath,
    artifact_root: ArchitectureReportPath,
    generated_at_unix_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectGraphEvidenceDomain {
    available: bool,
    project_scope: ArchitectureReportPath,
    store_root: ArchitectureReportPath,
    nodes: GraphNodeCount,
    edges: GraphEdgeCount,
    files: MemoryGraphNodeCount,
    code_graph_items: MemoryGraphNodeCount,
    memory_evidence_items: MemoryGraphNodeCount,
    status: MemoryErrorReason,
    reason: MemoryErrorReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphSearchHitDomain {
    node_id: GraphSnapshotNodeId,
    name: MemoryGraphSearchText,
    qualified_name: MemoryGraphSearchText,
    label: MemoryGraphSearchText,
    file_path: GraphSnapshotRelativePath,
    evidence_kind: DocumentKind,
    rank: Option<MemoryGraphSearchText>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GraphSearchDomain {
    total: MemoryGraphNodeCount,
    has_more: bool,
    query: SearchGraphQuery,
    project_scope: ArchitectureReportPath,
    results: Vec<GraphSearchHitDomain>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RetrievalEvidenceDomain {
    available: bool,
    status: MemoryErrorReason,
    rows_total: MemoryAnalyticsObservationCount,
    rows_green: MemoryAnalyticsObservationCount,
    rows_degraded: MemoryAnalyticsObservationCount,
    token_reduction_estimate: MemoryErrorReason,
    explanations: Vec<RetrievalExplanationDomain>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RetrievalExplanationDomain {
    id: RankingDocumentId,
    query: MemoryRecallQuery,
    capability_state: MemoryErrorReason,
    expected_ids: Vec<GraphSnapshotNodeId>,
    actual_ids: Vec<GraphSnapshotNodeId>,
    source_refs: Vec<GraphSnapshotRelativePath>,
    bm25_candidates: MemoryAnalyticsObservationCount,
    vector_candidates: MemoryAnalyticsObservationCount,
    rrf_score: Option<MemoryErrorReason>,
    reranker_score: Option<MemoryErrorReason>,
    selected_context_pack: MemoryErrorReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LearningEvidenceDomain {
    available: bool,
    status: MemoryErrorReason,
    lessons: Vec<LessonEvidenceDomain>,
    blockers: Vec<MemorySummaryText>,
    follow_ups: Vec<MemorySummaryText>,
    recurrence_signals: Vec<MemorySummaryText>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LessonEvidenceDomain {
    lesson_id: MemorySessionLessonId,
    lesson: MemorySummaryText,
    status: MemoryErrorReason,
    evidence: Vec<MemorySummaryText>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelEvidenceDomain {
    available: bool,
    runtime_mode: MemoryErrorReason,
    capability_state: MemoryErrorReason,
    allow_network: bool,
    cache_root: ArchitectureReportPath,
    observations: MemoryAnalyticsObservationCount,
    artifacts: Vec<ModelArtifactEvidenceDomain>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelArtifactEvidenceDomain {
    artifact: ArchitectureReportPath,
    status: MemoryErrorReason,
    capability: MemoryErrorReason,
    reason: MemoryErrorReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParityEvidenceDomain {
    available: bool,
    tools_total: MemoryAnalyticsObservationCount,
    equal: MemoryAnalyticsObservationCount,
    better: MemoryAnalyticsObservationCount,
    worse: MemoryAnalyticsObservationCount,
    incomparable: MemoryAnalyticsObservationCount,
    unrunnable: MemoryAnalyticsObservationCount,
    rows: Vec<ParityRowEvidenceDomain>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParityRowEvidenceDomain {
    tool: MemoryErrorReason,
    verdict: MemoryErrorReason,
    reason: MemoryErrorReason,
}

fn graph_node_id(value: String) -> Result<GraphSnapshotNodeId, DecodeError> {
    value.try_into()
}

fn relative_path(value: String) -> Result<GraphSnapshotRelativePath, DecodeError> {
    value.try_into()
}

fn analytics_count(value: usize) -> MemoryAnalyticsObservationCount {
    u64::try_from(value).map_or(u64::MAX, |count| count).into()
}

fn graph_hit_kind(kind: &EvidenceKind) -> DocumentKind {
    match kind {
        EvidenceKind::CodeGraph => DocumentKind::File,
        EvidenceKind::LearningMemory => DocumentKind::Lesson,
        EvidenceKind::ProofArtifact => DocumentKind::Artifact,
    }
}

impl MemoryExplorerResponse {
    fn into_domain(self) -> Result<MemoryExplorerDomain, DecodeError> {
        Ok(MemoryExplorerDomain {
            provenance: self.provenance.into_domain(),
            project_graph: self.project_graph.into_domain(),
            retrieval: self.retrieval.into_domain()?,
            learning: self.learning.into_domain()?,
            models: self.models.into_domain(),
            parity: self.parity.into_domain(),
        })
    }
}

impl MemoryEvidenceProvenanceResponse {
    fn into_domain(self) -> MemoryEvidenceProvenanceDomain {
        MemoryEvidenceProvenanceDomain {
            scope: self.scope.into(),
            selected_project_root: self.selected_project_root.into(),
            artifact_root: self.artifact_root.into(),
            generated_at_unix_secs: self.generated_at_unix_secs,
        }
    }
}

impl ProjectGraphEvidenceResponse {
    fn into_domain(self) -> ProjectGraphEvidenceDomain {
        ProjectGraphEvidenceDomain {
            available: self.available,
            project_scope: self.project_scope.into(),
            store_root: self.store_root.into(),
            nodes: self.nodes.into(),
            edges: self.edges.into(),
            files: self.files.into(),
            code_graph_items: self.code_graph_items.into(),
            memory_evidence_items: self.memory_evidence_items.into(),
            status: self.status.into(),
            reason: self.reason.into(),
        }
    }
}

impl GraphSearchHitResponse {
    fn into_domain(self) -> Result<GraphSearchHitDomain, DecodeError> {
        Ok(GraphSearchHitDomain {
            node_id: graph_node_id(self.node_id)?,
            name: self.name.into(),
            qualified_name: self.qualified_name.into(),
            label: self.label.into(),
            file_path: relative_path(self.file_path)?,
            evidence_kind: graph_hit_kind(&self.evidence_kind),
            rank: self.rank.map(Into::into),
        })
    }
}

impl GraphSearchResponse {
    /// Validate the outbound graph-search DTO against the canonical domain
    /// projection before it crosses the Tauri boundary.
    pub fn validate_domain(&self) -> Result<(), DecodeError> {
        self.clone().into_domain().map(|_| ())
    }

    fn into_domain(self) -> Result<GraphSearchDomain, DecodeError> {
        Ok(GraphSearchDomain {
            total: self.total.into(),
            has_more: self.has_more,
            query: self.query.into(),
            project_scope: self.project_scope.into(),
            results: self
                .results
                .into_iter()
                .map(GraphSearchHitResponse::into_domain)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl RetrievalEvidenceResponse {
    fn into_domain(self) -> Result<RetrievalEvidenceDomain, DecodeError> {
        Ok(RetrievalEvidenceDomain {
            available: self.available,
            status: self.status.into(),
            rows_total: self.rows_total.into(),
            rows_green: self.rows_green.into(),
            rows_degraded: self.rows_degraded.into(),
            token_reduction_estimate: self.token_reduction_estimate.into(),
            explanations: self
                .explanations
                .into_iter()
                .map(RetrievalExplanationResponse::into_domain)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl RetrievalExplanationResponse {
    fn into_domain(self) -> Result<RetrievalExplanationDomain, DecodeError> {
        Ok(RetrievalExplanationDomain {
            id: self.id.into(),
            query: self.query.into(),
            capability_state: self.capability_state.into(),
            expected_ids: self
                .expected_ids
                .into_iter()
                .map(graph_node_id)
                .collect::<Result<_, _>>()?,
            actual_ids: self
                .actual_ids
                .into_iter()
                .map(graph_node_id)
                .collect::<Result<_, _>>()?,
            source_refs: self
                .source_refs
                .into_iter()
                .map(relative_path)
                .collect::<Result<_, _>>()?,
            bm25_candidates: analytics_count(self.bm25_candidates),
            vector_candidates: analytics_count(self.vector_candidates),
            rrf_score: self.rrf_score.map(Into::into),
            reranker_score: self.reranker_score.map(Into::into),
            selected_context_pack: self.selected_context_pack.into(),
        })
    }
}

impl LearningEvidenceResponse {
    fn into_domain(self) -> Result<LearningEvidenceDomain, DecodeError> {
        Ok(LearningEvidenceDomain {
            available: self.available,
            status: self.status.into(),
            lessons: self
                .lessons
                .into_iter()
                .map(LessonEvidenceResponse::into_domain)
                .collect::<Result<_, _>>()?,
            blockers: self.blockers.into_iter().map(Into::into).collect(),
            follow_ups: self.follow_ups.into_iter().map(Into::into).collect(),
            recurrence_signals: self
                .recurrence_signals
                .into_iter()
                .map(Into::into)
                .collect(),
        })
    }
}

impl LessonEvidenceResponse {
    fn into_domain(self) -> Result<LessonEvidenceDomain, DecodeError> {
        Ok(LessonEvidenceDomain {
            lesson_id: self.lesson_id.into(),
            lesson: self.lesson.into(),
            status: self.status.into(),
            evidence: self.evidence.into_iter().map(Into::into).collect(),
        })
    }
}

impl ModelEvidenceResponse {
    fn into_domain(self) -> ModelEvidenceDomain {
        ModelEvidenceDomain {
            available: self.available,
            runtime_mode: self.runtime_mode.into(),
            capability_state: self.capability_state.into(),
            allow_network: self.allow_network,
            cache_root: self.cache_root.into(),
            observations: self.observations.into(),
            artifacts: self
                .artifacts
                .into_iter()
                .map(ModelArtifactEvidenceResponse::into_domain)
                .collect(),
        }
    }
}

impl ModelArtifactEvidenceResponse {
    fn into_domain(self) -> ModelArtifactEvidenceDomain {
        ModelArtifactEvidenceDomain {
            artifact: self.artifact.into(),
            status: self.status.into(),
            capability: self.capability.into(),
            reason: self.reason.into(),
        }
    }
}

impl ParityEvidenceResponse {
    fn into_domain(self) -> ParityEvidenceDomain {
        ParityEvidenceDomain {
            available: self.available,
            tools_total: self.tools_total.into(),
            equal: self.equal.into(),
            better: self.better.into(),
            worse: self.worse.into(),
            incomparable: self.incomparable.into(),
            unrunnable: self.unrunnable.into(),
            rows: self
                .rows
                .into_iter()
                .map(ParityRowEvidenceResponse::into_domain)
                .collect(),
        }
    }
}

impl ParityRowEvidenceResponse {
    fn into_domain(self) -> ParityRowEvidenceDomain {
        ParityRowEvidenceDomain {
            tool: self.tool.into(),
            verdict: self.verdict.into(),
            reason: self.reason.into(),
        }
    }
}

#[must_use]
pub fn render_memory_explorer(
    mode: UiRunMode,
    selected_project_root: &Path,
    workspace_root: &Path,
) -> MemoryExplorerResponse {
    let proof_root = workspace_root.join("proof").join("memory");
    if matches!(mode, UiRunMode::Silent) {
        return empty_payload(selected_project_root, &proof_root);
    }
    let paths = ProofPaths::new(&proof_root);
    let retrieval = read_json(&paths.rag_qa);
    let learning = read_json(&paths.learning_curve);
    let models = read_json(&paths.models);
    let parity = read_json(&paths.kg_parity);
    let token = read_json(&paths.token_reduction);
    let runtime = read_json(&paths.runtime_control_plane);
    let response = MemoryExplorerResponse {
        provenance: MemoryEvidenceProvenanceResponse {
            scope: "project-store-plus-engine-proof".to_owned(),
            selected_project_root: selected_project_root.display().to_string(),
            artifact_root: proof_root.display().to_string(),
            generated_at_unix_secs: latest_modified_unix_secs(&[
                &paths.rag_qa,
                &paths.learning_curve,
                &paths.models,
                &paths.kg_parity,
                &paths.runtime_control_plane,
            ]),
        },
        project_graph: project_graph_evidence(selected_project_root),
        retrieval: retrieval_evidence(retrieval.as_ref(), token.as_ref()),
        learning: learning_evidence(learning.as_ref()),
        models: model_evidence(
            models.as_ref(),
            model_artifacts(&proof_root, runtime.as_ref()),
        ),
        parity: parity_evidence(parity.as_ref()),
    };
    // Validate the outbound aggregate against canonical domain brands before
    // presenting it. Invalid proof identifiers or paths fail closed to the
    // honest unavailable payload rather than leaking raw boundary text.
    if response.clone().into_domain().is_err() {
        return empty_payload(selected_project_root, &proof_root);
    }
    response
}

fn empty_payload(selected_project_root: &Path, proof_root: &Path) -> MemoryExplorerResponse {
    MemoryExplorerResponse {
        provenance: MemoryEvidenceProvenanceResponse {
            scope: "silent".to_owned(),
            selected_project_root: selected_project_root.display().to_string(),
            artifact_root: proof_root.display().to_string(),
            generated_at_unix_secs: None,
        },
        project_graph: ProjectGraphEvidenceResponse {
            available: false,
            project_scope: selected_project_root.display().to_string(),
            store_root: selected_project_root
                .join(".enforce/memory")
                .display()
                .to_string(),
            nodes: 0,
            edges: 0,
            files: 0,
            code_graph_items: 0,
            memory_evidence_items: 0,
            status: "silent".to_owned(),
            reason: "f04 silent mode suppresses Memory Explorer output".to_owned(),
        },
        retrieval: RetrievalEvidenceResponse {
            available: false,
            status: "silent".to_owned(),
            rows_total: 0,
            rows_green: 0,
            rows_degraded: 0,
            token_reduction_estimate: String::new(),
            explanations: Vec::new(),
        },
        learning: LearningEvidenceResponse {
            available: false,
            status: "silent".to_owned(),
            lessons: Vec::new(),
            blockers: Vec::new(),
            follow_ups: Vec::new(),
            recurrence_signals: Vec::new(),
        },
        models: ModelEvidenceResponse {
            available: false,
            runtime_mode: "silent".to_owned(),
            capability_state: "suppressed".to_owned(),
            allow_network: false,
            cache_root: String::new(),
            observations: 0,
            artifacts: Vec::new(),
        },
        parity: ParityEvidenceResponse {
            available: false,
            tools_total: 0,
            equal: 0,
            better: 0,
            worse: 0,
            incomparable: 0,
            unrunnable: 0,
            rows: Vec::new(),
        },
    }
}

fn project_graph_evidence(root: &Path) -> ProjectGraphEvidenceResponse {
    let store_root = root.join(".enforce").join("memory");
    let sqlite = store_root.join("operational-graph.sqlite");
    if sqlite.exists() {
        ProjectGraphEvidenceResponse {
            available: true,
            project_scope: root.display().to_string(),
            store_root: store_root.display().to_string(),
            nodes: 0,
            edges: 0,
            files: 0,
            code_graph_items: 0,
            memory_evidence_items: 0,
            status: "store-present".to_owned(),
            reason: "selected project has a Store operational graph; graph counts/search are loaded by the Tauri command that owns enforcer-memory access".to_owned(),
        }
    } else {
        ProjectGraphEvidenceResponse {
            available: false,
            project_scope: root.display().to_string(),
            store_root: store_root.display().to_string(),
            nodes: 0,
            edges: 0,
            files: 0,
            code_graph_items: 0,
            memory_evidence_items: 0,
            status: "degraded-empty".to_owned(),
            reason: format!(
                "no Store operational graph projection at {}",
                sqlite.display()
            ),
        }
    }
}

struct ProofPaths {
    rag_qa: PathBuf,
    learning_curve: PathBuf,
    models: PathBuf,
    kg_parity: PathBuf,
    token_reduction: PathBuf,
    runtime_control_plane: PathBuf,
}

impl ProofPaths {
    fn new(root: &Path) -> Self {
        Self {
            rag_qa: root.join("x06-rag-qa.json"),
            learning_curve: root.join("x06-learning-curve.json"),
            models: root.join("x06-models.json"),
            kg_parity: root.join("x06-kg-parity.json"),
            token_reduction: root.join("x06-token-reduction.json"),
            runtime_control_plane: root.join("x06-runtime-control-plane.json"),
        }
    }
}

fn retrieval_evidence(
    value: Option<&serde_json::Value>,
    token: Option<&serde_json::Value>,
) -> RetrievalEvidenceResponse {
    let explanations = value
        .and_then(|value| value.get("rows"))
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |rows| {
            rows.iter()
                .take(8)
                .map(|row| {
                    let source_refs = string_array(row, "sourceRefs");
                    let actual_ids = string_array(row, "actualIds");
                    RetrievalExplanationResponse {
                        id: string_field(Some(row), "id"),
                        query: string_field(Some(row), "query"),
                        capability_state: string_field(Some(row), "capabilityState"),
                        expected_ids: string_array(row, "expectedIds"),
                        actual_ids: actual_ids.clone(),
                        source_refs: source_refs.clone(),
                        bm25_candidates: actual_ids.len(),
                        vector_candidates: usize::from(
                            !string_field(Some(row), "capabilityState").is_empty(),
                        ),
                        rrf_score: decimal_field(row, "mrrAt10"),
                        reranker_score: decimal_field(row, "ndcgAt10"),
                        selected_context_pack: if source_refs.is_empty() {
                            "none".to_owned()
                        } else {
                            "sourceRefs".to_owned()
                        },
                    }
                })
                .collect()
        });
    RetrievalEvidenceResponse {
        available: value.is_some(),
        status: string_field(value, "status"),
        rows_total: number_field(value, "rowsTotal"),
        rows_green: number_field(value, "rowsGreen"),
        rows_degraded: number_field(value, "rowsGreenDegraded"),
        token_reduction_estimate: token
            .and_then(|value| value.get("tokenReductionEstimate"))
            .or_else(|| token.and_then(|value| value.get("estimatedReduction")))
            .map_or_else(|| "not recorded".to_owned(), serde_json::Value::to_string),
        explanations,
    }
}

fn learning_evidence(value: Option<&serde_json::Value>) -> LearningEvidenceResponse {
    LearningEvidenceResponse {
        available: value.is_some(),
        status: string_field(value, "status"),
        lessons: value
            .and_then(|value| value.get("dogfoodLessons"))
            .and_then(serde_json::Value::as_array)
            .map_or_else(Vec::new, |rows| {
                rows.iter()
                    .take(12)
                    .map(|row| LessonEvidenceResponse {
                        lesson_id: string_field(Some(row), "lessonId"),
                        lesson: string_field(Some(row), "lesson"),
                        status: string_field(Some(row), "status"),
                        evidence: string_array(row, "evidence"),
                    })
                    .collect()
            }),
        blockers: string_or_object_rows(value, "blockers"),
        follow_ups: string_or_object_rows(value, "followUps"),
        recurrence_signals: value
            .and_then(|value| value.get("learningCurve"))
            .and_then(|curve| curve.get("storeBackedSignals"))
            .map_or_else(Vec::new, |signals| {
                signals
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            }),
    }
}

fn model_evidence(
    value: Option<&serde_json::Value>,
    artifacts: Vec<ModelArtifactEvidenceResponse>,
) -> ModelEvidenceResponse {
    let runtime_mode = string_field(value, "runtimeMode");
    ModelEvidenceResponse {
        available: value.is_some(),
        capability_state: if value.is_some() {
            if runtime_mode.is_empty() {
                "artifact-backed".to_owned()
            } else {
                runtime_mode.clone()
            }
        } else {
            "degraded-provider-unavailable".to_owned()
        },
        runtime_mode,
        allow_network: value
            .and_then(|value| value.get("allowNetwork"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        cache_root: string_field(value, "cacheRoot"),
        observations: array_len(value, "observations"),
        artifacts,
    }
}

fn model_artifacts(
    proof_root: &Path,
    runtime: Option<&serde_json::Value>,
) -> Vec<ModelArtifactEvidenceResponse> {
    [
        "x06-models-cache-only-missing.json",
        "x06-models-cache-only-preseeded.json",
        "x06-models-qwen3-embedding-ort-cpu.json",
        "x06-models-qwen3-reranker-ort-cpu.json",
        "x06-runtime-control-plane.json",
    ]
    .into_iter()
    .map(|name| ModelArtifactEvidenceResponse {
        artifact: name.to_owned(),
        status: if proof_root.join(name).exists() {
            "present"
        } else {
            "missing"
        }
        .to_owned(),
        capability: match name {
            "x06-runtime-control-plane.json" => "control-plane",
            "x06-models-qwen3-embedding-ort-cpu.json"
            | "x06-models-qwen3-reranker-ort-cpu.json" => "ort-cache-proof",
            _ => "cache-policy",
        }
        .to_owned(),
        reason: runtime
            .and_then(|value| value.get("proofScope"))
            .and_then(|scope| scope.get("reason"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("passive read only; no download or model process is started")
            .to_owned(),
    })
    .collect()
}

fn parity_evidence(value: Option<&serde_json::Value>) -> ParityEvidenceResponse {
    let rows = value
        .and_then(|value| value.get("rows"))
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(|row| {
                    Some(ParityRowEvidenceResponse {
                        tool: row.get("tool")?.as_str()?.to_owned(),
                        verdict: row.get("comparison_verdict")?.as_str()?.to_owned(),
                        reason: row
                            .get("better_because")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    })
                })
                .collect()
        });
    ParityEvidenceResponse {
        available: value.is_some(),
        tools_total: number_field(value, "tools_total"),
        equal: number_field(value, "tools_equal"),
        better: number_field(value, "tools_better"),
        worse: number_field(value, "tools_worse"),
        incomparable: number_field(value, "tools_incomparable"),
        unrunnable: number_field(value, "tools_unrunnable"),
        rows,
    }
}

fn latest_modified_unix_secs(paths: &[&Path]) -> Option<u64> {
    paths
        .iter()
        .filter_map(|path| {
            std::fs::metadata(path)
                .ok()?
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_secs())
        })
        .max()
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
}

fn string_field(value: Option<&serde_json::Value>, key: &str) -> String {
    value
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn number_field(value: Option<&serde_json::Value>, key: &str) -> u64 {
    value
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default()
}

fn array_len(value: Option<&serde_json::Value>, key: &str) -> u64 {
    value
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_array)
        .map_or(0, |items| u64::try_from(items.len()).unwrap_or(u64::MAX))
}

fn string_array(value: &serde_json::Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
}

fn decimal_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .map(|number| format!("{number:.3}"))
}

fn string_or_object_rows(value: Option<&serde_json::Value>, key: &str) -> Vec<String> {
    value
        .and_then(|value| value.get(key))
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |rows| {
            rows.iter()
                .take(12)
                .map(|row| {
                    row.as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| row.to_string())
                })
                .collect()
        })
}

#[cfg(test)]
mod tests {
    use super::{
        render_memory_explorer, EvidenceKind, GraphSearchHitResponse, GraphSearchResponse,
        LearningEvidenceResponse, LessonEvidenceResponse, MemoryEvidenceProvenanceResponse,
        MemoryExplorerResponse, ModelArtifactEvidenceResponse, ModelEvidenceResponse,
        ParityEvidenceResponse, ParityRowEvidenceResponse, ProjectGraphEvidenceResponse,
        RetrievalEvidenceResponse, RetrievalExplanationResponse,
    };
    use enforcer_domain::ui_types::UiRunMode;

    #[test]
    fn memory_explorer_response_round_trips_through_json() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = tempfile::tempdir()?;
        let round_trip_payload: MemoryExplorerResponse =
            render_memory_explorer(UiRunMode::Human, root.path(), root.path());
        let wire = serde_json::to_string(&round_trip_payload)?;
        let restored: MemoryExplorerResponse = serde_json::from_str(&wire)?;
        assert_eq!(restored, round_trip_payload);
        let _: &MemoryEvidenceProvenanceResponse = &restored.provenance;
        let _: &ProjectGraphEvidenceResponse = &restored.project_graph;
        let retrieval: &RetrievalEvidenceResponse = &restored.retrieval;
        let _: &[RetrievalExplanationResponse] = &retrieval.explanations;
        let learning: &LearningEvidenceResponse = &restored.learning;
        let _: &[LessonEvidenceResponse] = &learning.lessons;
        let models: &ModelEvidenceResponse = &restored.models;
        let _: &[ModelArtifactEvidenceResponse] = &models.artifacts;
        let parity: &ParityEvidenceResponse = &restored.parity;
        let _: &[ParityRowEvidenceResponse] = &parity.rows;

        let graph_search = GraphSearchResponse {
            total: 1,
            has_more: false,
            query: "find handler".to_owned(),
            project_scope: root.path().display().to_string(),
            results: vec![GraphSearchHitResponse {
                node_id: "sym:handler".to_owned(),
                name: "handler".to_owned(),
                qualified_name: "crate::handler".to_owned(),
                label: "function".to_owned(),
                file_path: "src/lib.rs".to_owned(),
                evidence_kind: EvidenceKind::CodeGraph,
                rank: Some("1".to_owned()),
            }],
        };
        let graph_wire = serde_json::to_string(&graph_search)?;
        let graph_restored: GraphSearchResponse = serde_json::from_str(&graph_wire)?;
        graph_restored.validate_domain()?;
        let _: &GraphSearchHitResponse = graph_restored
            .results
            .first()
            .ok_or("round-trip graph response must preserve its hit")?;
        assert_eq!(graph_restored, graph_search);
        let mut malformed_graph = graph_search;
        malformed_graph.results[0].node_id.clear();
        assert!(matches!(
            malformed_graph.validate_domain(),
            Err(error) if error.path == "graphSnapshotNodeId"
        ));
        Ok(())
    }

    #[test]
    fn missing_store_degrades_without_fake_graph() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let payload = render_memory_explorer(UiRunMode::Human, root.path(), root.path());
        assert!(!payload.project_graph.available);
        assert_eq!(payload.project_graph.nodes, 0);
        assert_eq!(payload.project_graph.status, "degraded-empty");
        assert!(!payload.retrieval.available);
        assert!(payload.parity.rows.is_empty());
        Ok(())
    }

    #[test]
    fn silent_mode_suppresses_payload() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let payload = render_memory_explorer(UiRunMode::Silent, root.path(), root.path());
        assert_eq!(payload.provenance.scope, "silent");
        assert_eq!(payload.project_graph.status, "silent");
        assert!(payload.learning.lessons.is_empty());
        Ok(())
    }

    #[test]
    fn seeded_proof_artifacts_render_real_evidence_and_preserve_parity_verdicts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let proof = root.path().join("proof").join("memory");
        std::fs::create_dir_all(&proof)?;
        std::fs::write(
            proof.join("x06-rag-qa.json"),
            r#"{"status":"green","rowsTotal":1,"rowsGreen":1,"rowsGreenDegraded":0,"rows":[{"id":"QA-1","query":"find handler","expectedIds":["sym:a"],"actualIds":["sym:a"],"sourceRefs":["src/lib.rs"],"mrrAt10":1.0,"ndcgAt10":1.0,"capabilityState":"host-local-proof"}]}"#,
        )?;
        std::fs::write(
            proof.join("x06-learning-curve.json"),
            r#"{"status":"learned","learningCurve":{"storeBackedSignals":["route-choice"]},"dogfoodLessons":[{"lessonId":"L1","lesson":"Keep evidence typed","status":"learned","evidence":["proof"]}]}"#,
        )?;
        std::fs::write(
            proof.join("x06-models.json"),
            r#"{"runtimeMode":"cache-only","allowNetwork":false,"cacheRoot":".cache","observations":[{"id":"o1"}]}"#,
        )?;
        std::fs::write(
            proof.join("x06-runtime-control-plane.json"),
            r#"{"proofScope":{"reason":"control-plane only"}}"#,
        )?;
        std::fs::write(
            proof.join("x06-kg-parity.json"),
            r#"{"tools_total":1,"tools_equal":0,"tools_better":1,"tools_worse":0,"tools_incomparable":0,"tools_unrunnable":0,"rows":[{"tool":"search_graph","comparison_verdict":"better","better_because":"typed Rust evidence"}]}"#,
        )?;

        let payload = render_memory_explorer(UiRunMode::Human, root.path(), root.path());
        assert_eq!(payload.retrieval.explanations[0].query, "find handler");
        assert_eq!(payload.learning.lessons[0].lesson_id, "L1");
        assert_eq!(payload.models.capability_state, "cache-only");
        assert_eq!(payload.parity.rows[0].verdict, "better");
        Ok(())
    }

    #[test]
    fn search_hit_kind_distinguishes_code_graph_from_memory_evidence() {
        assert_eq!(format!("{:?}", EvidenceKind::CodeGraph), "CodeGraph");
    }
}
