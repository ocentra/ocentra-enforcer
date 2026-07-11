//! g09 - Memory KG/RAG Explorer.
//!
//! Rust builds the payload for the Memory Explorer. TypeScript displays
//! these fields and sends intent only.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Human,
    Silent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryExplorerPayload {
    pub provenance: MemoryEvidenceProvenance,
    pub project_graph: ProjectGraphEvidence,
    pub retrieval: RetrievalEvidence,
    pub learning: LearningEvidence,
    pub models: ModelEvidence,
    pub parity: ParityEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEvidenceProvenance {
    pub scope: String,
    pub selected_project_root: String,
    pub artifact_root: String,
    pub generated_at_unix_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGraphEvidence {
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
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKind {
    CodeGraph,
    LearningMemory,
    ProofArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphSearchHitPayload {
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
pub struct GraphSearchPayload {
    pub total: usize,
    pub has_more: bool,
    pub query: String,
    pub project_scope: String,
    pub results: Vec<GraphSearchHitPayload>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalEvidence {
    pub available: bool,
    pub status: String,
    pub rows_total: u64,
    pub rows_green: u64,
    pub rows_degraded: u64,
    pub token_reduction_estimate: String,
    pub explanations: Vec<RetrievalExplanation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalExplanation {
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
pub struct LearningEvidence {
    pub available: bool,
    pub status: String,
    pub lessons: Vec<LessonEvidence>,
    pub blockers: Vec<String>,
    pub follow_ups: Vec<String>,
    pub recurrence_signals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonEvidence {
    pub lesson_id: String,
    pub lesson: String,
    pub status: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEvidence {
    pub available: bool,
    pub runtime_mode: String,
    pub capability_state: String,
    pub allow_network: bool,
    pub cache_root: String,
    pub observations: u64,
    pub artifacts: Vec<ModelArtifactEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelArtifactEvidence {
    pub artifact: String,
    pub status: String,
    pub capability: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParityEvidence {
    pub available: bool,
    pub tools_total: u64,
    pub equal: u64,
    pub better: u64,
    pub worse: u64,
    pub incomparable: u64,
    pub unrunnable: u64,
    pub rows: Vec<ParityRowEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParityRowEvidence {
    pub tool: String,
    pub verdict: String,
    pub reason: String,
}

#[must_use]
pub fn render_memory_explorer(
    mode: RunMode,
    selected_project_root: &Path,
    workspace_root: &Path,
) -> MemoryExplorerPayload {
    let proof_root = workspace_root.join("proof").join("memory");
    if mode == RunMode::Silent {
        return empty_payload(selected_project_root, &proof_root);
    }
    let paths = ProofPaths::new(&proof_root);
    let retrieval = read_json(&paths.rag_qa);
    let learning = read_json(&paths.learning_curve);
    let models = read_json(&paths.models);
    let parity = read_json(&paths.kg_parity);
    let token = read_json(&paths.token_reduction);
    let runtime = read_json(&paths.runtime_control_plane);
    MemoryExplorerPayload {
        provenance: MemoryEvidenceProvenance {
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
    }
}

fn empty_payload(selected_project_root: &Path, proof_root: &Path) -> MemoryExplorerPayload {
    MemoryExplorerPayload {
        provenance: MemoryEvidenceProvenance {
            scope: "silent".to_owned(),
            selected_project_root: selected_project_root.display().to_string(),
            artifact_root: proof_root.display().to_string(),
            generated_at_unix_secs: None,
        },
        project_graph: ProjectGraphEvidence {
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
        retrieval: RetrievalEvidence {
            available: false,
            status: "silent".to_owned(),
            rows_total: 0,
            rows_green: 0,
            rows_degraded: 0,
            token_reduction_estimate: String::new(),
            explanations: Vec::new(),
        },
        learning: LearningEvidence {
            available: false,
            status: "silent".to_owned(),
            lessons: Vec::new(),
            blockers: Vec::new(),
            follow_ups: Vec::new(),
            recurrence_signals: Vec::new(),
        },
        models: ModelEvidence {
            available: false,
            runtime_mode: "silent".to_owned(),
            capability_state: "suppressed".to_owned(),
            allow_network: false,
            cache_root: String::new(),
            observations: 0,
            artifacts: Vec::new(),
        },
        parity: ParityEvidence {
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

fn project_graph_evidence(root: &Path) -> ProjectGraphEvidence {
    let store_root = root.join(".enforce").join("memory");
    let sqlite = store_root.join("operational-graph.sqlite");
    if sqlite.exists() {
        ProjectGraphEvidence {
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
        ProjectGraphEvidence {
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
) -> RetrievalEvidence {
    let explanations = value
        .and_then(|value| value.get("rows"))
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |rows| {
            rows.iter()
                .take(8)
                .map(|row| {
                    let source_refs = string_array(row, "sourceRefs");
                    let actual_ids = string_array(row, "actualIds");
                    RetrievalExplanation {
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
    RetrievalEvidence {
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

fn learning_evidence(value: Option<&serde_json::Value>) -> LearningEvidence {
    LearningEvidence {
        available: value.is_some(),
        status: string_field(value, "status"),
        lessons: value
            .and_then(|value| value.get("dogfoodLessons"))
            .and_then(serde_json::Value::as_array)
            .map_or_else(Vec::new, |rows| {
                rows.iter()
                    .take(12)
                    .map(|row| LessonEvidence {
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
    artifacts: Vec<ModelArtifactEvidence>,
) -> ModelEvidence {
    let runtime_mode = string_field(value, "runtimeMode");
    ModelEvidence {
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
) -> Vec<ModelArtifactEvidence> {
    [
        "x06-models-cache-only-missing.json",
        "x06-models-cache-only-preseeded.json",
        "x06-models-qwen3-embedding-ort-cpu.json",
        "x06-models-qwen3-reranker-ort-cpu.json",
        "x06-runtime-control-plane.json",
    ]
    .into_iter()
    .map(|name| ModelArtifactEvidence {
        artifact: name.to_owned(),
        status: if proof_root.join(name).exists() {
            "present"
        } else {
            "missing"
        }
        .to_owned(),
        capability: if name.contains("runtime-control") {
            "control-plane"
        } else if name.contains("ort") {
            "ort-cache-proof"
        } else {
            "cache-policy"
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

fn parity_evidence(value: Option<&serde_json::Value>) -> ParityEvidence {
    let rows = value
        .and_then(|value| value.get("rows"))
        .and_then(serde_json::Value::as_array)
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .filter_map(|row| {
                    Some(ParityRowEvidence {
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
    ParityEvidence {
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
        .map_or(0, |items| items.len() as u64)
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
    use super::{render_memory_explorer, EvidenceKind, RunMode};

    #[test]
    fn missing_store_degrades_without_fake_graph() {
        let root = tempfile::tempdir().expect("temp root");
        let payload = render_memory_explorer(RunMode::Human, root.path(), root.path());
        assert!(!payload.project_graph.available);
        assert_eq!(payload.project_graph.nodes, 0);
        assert_eq!(payload.project_graph.status, "degraded-empty");
        assert!(!payload.retrieval.available);
        assert!(payload.parity.rows.is_empty());
    }

    #[test]
    fn silent_mode_suppresses_payload() {
        let root = tempfile::tempdir().expect("temp root");
        let payload = render_memory_explorer(RunMode::Silent, root.path(), root.path());
        assert_eq!(payload.provenance.scope, "silent");
        assert_eq!(payload.project_graph.status, "silent");
        assert!(payload.learning.lessons.is_empty());
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

        let payload = render_memory_explorer(RunMode::Human, root.path(), root.path());
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
