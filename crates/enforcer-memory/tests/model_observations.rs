//! X06 model runtime learning-observation serialization contracts.
//!
//! These tests validate that every required candidate shape stays
//! wire-compatible for a future Store writer (no persistence exists in
//! this pass).

use enforcer_domain::paths::RepoRoot;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::model_observations::{
    ingest_model_runtime_observation, project_model_runtime_observations_from_store,
    record_model_runtime_observation_in_store, DegradedFallback, HashMismatch, LocalLoadSucceeded,
    ModelLoadFailure, ModelRuntimeObservationCandidate, ModelRuntimeObservationRecord,
    ProviderDowngrade, RecurrenceNegativeKind, RecurrenceOrNegativeEvidence, RerankerLiftProof,
    RetrievalQualityProof, RouteChoiceImprovement, TokenReductionProof,
};
use enforcer_memory::model_runtime::{ModelTask, ProviderKind};
use enforcer_memory::schema::{ObservationLogEntry, SCHEMA_VERSION};
use enforcer_memory::store::Store;

#[test]
fn model_load_failure_shape_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let record = ModelRuntimeObservationRecord::new(
        "2026-07-05T12:00:00Z",
        "integration",
        "run-1",
        ModelRuntimeObservationCandidate::ModelLoadFailure(ModelLoadFailure {
            model_id: "Qwen/Qwen3-Embedding-0.6B".to_string(),
            task: ModelTask::Embedding,
            requested_provider: Some(ProviderKind::Cuda),
            failure_reason: "model checksum missing".to_string(),
        }),
    );
    let serialized = serde_json::to_value(&record)?;
    assert_eq!(
        serialized["candidate"]["observationKind"],
        "model-load-failure"
    );
    assert_eq!(
        serialized["candidate"]["modelId"],
        "Qwen/Qwen3-Embedding-0.6B"
    );
    Ok(())
}

#[test]
fn provider_and_hash_candidates_shape_stable() -> Result<(), Box<dyn std::error::Error>> {
    let candidate_provider =
        ModelRuntimeObservationCandidate::ProviderDowngrade(ProviderDowngrade {
            model_id: "Qwen/Qwen3-Reranker-0.6B".to_string(),
            task: ModelTask::Reranking,
            requested_provider: ProviderKind::Cuda,
            fallback_provider: ProviderKind::Cpu,
            reason: "driver unavailable".to_string(),
        });
    let candidate_artifact_hash =
        ModelRuntimeObservationCandidate::ArtifactHashMismatch(HashMismatch {
            model_id: "Qwen/Qwen3-Embedding-0.6B".to_string(),
            path: "artifacts/embedding.onnx".to_string(),
            expected_sha256: "aa".repeat(64),
            observed_sha256: "bb".repeat(64),
        });
    let candidate_tokenizer_hash =
        ModelRuntimeObservationCandidate::TokenizerHashMismatch(HashMismatch {
            model_id: "Qwen/Qwen3-Embedding-0.6B".to_string(),
            path: "artifacts/tokenizer.model".to_string(),
            expected_sha256: "cc".repeat(64),
            observed_sha256: "dd".repeat(64),
        });
    let candidate_degraded = ModelRuntimeObservationCandidate::DegradedFallback(DegradedFallback {
        model_id: "Qwen/Qwen3-Embedding-0.6B".to_string(),
        task: ModelTask::Embedding,
        fallback_reason: "provider unavailable".to_string(),
    });

    let provider_value = serde_json::to_value(candidate_provider)?;
    let artifact_hash_value = serde_json::to_value(candidate_artifact_hash)?;
    let tokenizer_hash_value = serde_json::to_value(candidate_tokenizer_hash)?;
    let degraded_value = serde_json::to_value(candidate_degraded)?;

    assert_eq!(provider_value["observationKind"], "provider-downgrade");
    assert_eq!(provider_value["requestedProvider"], "cuda");
    assert_eq!(
        artifact_hash_value["observationKind"],
        "artifact-hash-mismatch"
    );
    assert_eq!(artifact_hash_value["path"], "artifacts/embedding.onnx");
    assert_eq!(
        tokenizer_hash_value["observationKind"],
        "tokenizer-hash-mismatch"
    );
    assert_eq!(tokenizer_hash_value["path"], "artifacts/tokenizer.model");
    assert_eq!(degraded_value["observationKind"], "degraded-fallback");
    assert_eq!(degraded_value["fallbackReason"], "provider unavailable");
    Ok(())
}

#[test]
fn retrieval_and_reranker_evidence_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let retrieval =
        ModelRuntimeObservationCandidate::RetrievalQualityProof(RetrievalQualityProof {
            query_id: "q-123".to_string(),
            query: "load local model".to_string(),
            route: "hybrid".to_string(),
            recall_at_five: 0.80,
            recall_at_ten: 0.95,
            precision_at_five: 0.50,
            expected_top_k: 10,
            returned_top_k: 10,
        });
    let rerank = ModelRuntimeObservationCandidate::RerankerLiftProof(RerankerLiftProof {
        query_id: "q-123".to_string(),
        query: "load local model".to_string(),
        pre_rerank_top_k: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        post_rerank_top_k: vec!["b".to_string(), "a".to_string(), "d".to_string()],
        lift_score: 0.31,
        improved: true,
    });

    let serialized = serde_json::to_string(&retrieval)?;
    let serialized_json: serde_json::Value = serde_json::from_str(&serialized)?;
    let deserialized = serde_json::from_str::<ModelRuntimeObservationCandidate>(&serialized)?;
    assert!(matches!(
        deserialized,
        ModelRuntimeObservationCandidate::RetrievalQualityProof(_)
    ));
    assert_eq!(
        serialized_json["observationKind"].as_str(),
        Some("retrieval-quality-proof")
    );

    let rerank_round = serde_json::to_string(&rerank)?;
    let rerank_deserialized =
        serde_json::from_str::<ModelRuntimeObservationCandidate>(&rerank_round)?;
    assert!(matches!(
        rerank_deserialized,
        ModelRuntimeObservationCandidate::RerankerLiftProof(_)
    ));
    Ok(())
}

#[test]
fn token_reduction_and_route_choice_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let token_reduction = TokenReductionProof {
        query_id: "q-456".to_string(),
        query: "route route-choice".to_string(),
        naive_tokens: 1000,
        context_tokens: 200,
    };
    let route_choice =
        ModelRuntimeObservationCandidate::RouteChoiceImprovement(RouteChoiceImprovement {
            query_id: "q-789".to_string(),
            query: "where models are loaded".to_string(),
            chosen_route: "hybrid-search".to_string(),
            alternative_route: "semantic-only".to_string(),
            chosen_route_score: 0.92,
            alternative_route_score: 0.55,
            improvement_delta: 0.37,
        });

    let token_value = serde_json::to_value(&token_reduction)?;
    assert_eq!(token_value["naiveTokens"], 1000);
    assert_eq!(token_value["contextTokens"], 200);
    assert_eq!(token_reduction.reduction_ratio(), 5.0);

    let route_value = serde_json::to_value(route_choice)?;
    assert_eq!(route_value["observationKind"], "route-choice-improvement");
    assert_eq!(route_value["chosenRoute"], "hybrid-search");
    Ok(())
}

#[test]
fn recurrence_and_negative_evidence_shape() -> Result<(), Box<dyn std::error::Error>> {
    let recurrence = ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidence(
        RecurrenceOrNegativeEvidence {
            lesson_id: "L123".to_string(),
            query_id: Some("q-789".to_string()),
            evidence_kind: RecurrenceNegativeKind::RecurrenceCount {
                recurrence_count: 2,
                previous_count: Some(1),
            },
            clean_evidence: false,
        },
    );
    let negative = ModelRuntimeObservationCandidate::RecurrenceOrNegativeEvidence(
        RecurrenceOrNegativeEvidence {
            lesson_id: "L124".to_string(),
            query_id: None,
            evidence_kind: RecurrenceNegativeKind::NegativeEvidence {
                reason: "clean run, no finding".to_string(),
            },
            clean_evidence: true,
        },
    );
    let local_load = ModelRuntimeObservationCandidate::SuccessfulLocalLoad(LocalLoadSucceeded {
        model_id: "Qwen/Qwen3-Embedding-0.6B".to_string(),
        task: ModelTask::Embedding,
        provider: ProviderKind::Cpu,
        loaded_from_local_cache: true,
    });

    assert_eq!(
        serde_json::to_value(recurrence)?["observationKind"],
        "recurrence-or-negative-evidence"
    );
    let negative_value = serde_json::to_value(negative)?;
    assert_eq!(
        negative_value["observationKind"],
        "recurrence-or-negative-evidence"
    );
    assert_eq!(negative_value["cleanEvidence"], true);
    let local_value = serde_json::to_value(local_load)?;
    assert_eq!(local_value["observationKind"], "successful-local-load");
    Ok(())
}

#[test]
fn model_runtime_observation_persists_to_store_and_graph() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let root: RepoRoot = "C:/Projects/x06-model-observation-store".parse()?;
    let mut store = Store::init(dir.path(), &root, "2026-07-05T12:00:00Z")?;
    let mut graph = MemoryGraph::new();
    let record = ModelRuntimeObservationRecord::new(
        "2026-07-05T12:00:00Z",
        "x06-model-runtime-proof",
        "run-1",
        ModelRuntimeObservationCandidate::DegradedFallback(DegradedFallback {
            model_id: "Qwen/Qwen3-Embedding-0.6B".to_string(),
            task: ModelTask::Embedding,
            fallback_reason: "provider unavailable".to_string(),
        }),
    );

    let id = ingest_model_runtime_observation(&mut store, &mut graph, record.clone())?;
    assert!(id.starts_with("obs-x06-model-runtime-proof-"));
    assert_eq!(graph.len(), 1);

    let entries = store.read_observation_entries()?;
    assert_eq!(entries.entries.len(), 1);
    assert_eq!(
        entries.entries[0].payload_kind.as_deref(),
        Some("model-runtime:degraded-fallback")
    );
    assert_eq!(
        entries.entries[0].payload.as_ref().and_then(|payload| {
            payload
                .pointer("/candidate/observationKind")
                .and_then(serde_json::Value::as_str)
        }),
        Some("degraded-fallback")
    );
    let native_entries = store.read_model_observation_entries()?;
    assert_eq!(native_entries.entries.len(), 1);
    assert_eq!(native_entries.entries[0].candidate, record.candidate);
    Ok(())
}

#[test]
fn store_backed_model_runtime_observation_replays_without_duplicate_writes(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root: RepoRoot = "C:/Projects/x06-model-observation-store-replay".parse()?;
    let mut store = Store::init(dir.path(), &root, "2026-07-05T12:00:00Z")?;
    let record = ModelRuntimeObservationRecord::new(
        "2026-07-05T12:00:00Z",
        "x06-model-runtime-proof",
        "run-2",
        ModelRuntimeObservationCandidate::SuccessfulLocalLoad(LocalLoadSucceeded {
            model_id: "Qwen/Qwen3-Embedding-0.6B".to_string(),
            task: ModelTask::Embedding,
            provider: ProviderKind::Cpu,
            loaded_from_local_cache: true,
        }),
    );

    let id = record_model_runtime_observation_in_store(&mut store, &record)?;
    assert!(id.starts_with("obs-"));

    let entries = store.read_observation_entries()?;
    assert_eq!(entries.entries.len(), 1);
    assert_eq!(
        entries.entries[0].payload_kind.as_deref(),
        Some("model-runtime:successful-local-load")
    );
    let native_entries = store.read_model_observation_entries()?;
    assert_eq!(native_entries.entries.len(), 1);
    assert_eq!(native_entries.entries[0].run_id, "run-2");

    let replayed = project_model_runtime_observations_from_store(&store)?;
    assert_eq!(replayed, vec![record.clone()]);
    let replayed_again = project_model_runtime_observations_from_store(&store)?;
    assert_eq!(replayed_again, replayed);
    Ok(())
}

#[test]
fn projection_falls_back_to_legacy_observation_payloads_when_native_log_is_empty(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root: RepoRoot = "C:/Projects/x06-model-observation-legacy-fallback".parse()?;
    let mut store = Store::init(dir.path(), &root, "2026-07-05T12:00:00Z")?;
    let record = ModelRuntimeObservationRecord::new(
        "2026-07-05T12:00:00Z",
        "x06-model-runtime-proof",
        "run-legacy",
        ModelRuntimeObservationCandidate::DegradedFallback(DegradedFallback {
            model_id: "Qwen/Qwen3-Embedding-0.6B".to_string(),
            task: ModelTask::Embedding,
            fallback_reason: "provider unavailable".to_string(),
        }),
    );
    let payload = serde_json::to_value(&record)?;
    store.append_observation_entry(|seq| ObservationLogEntry {
        schema_version: SCHEMA_VERSION,
        seq,
        id: format!("obs-{seq:04}"),
        lesson_id: String::new(),
        rule_id: None,
        fault_class: Some("degraded-fallback".to_string()),
        repo_context: "Qwen/Qwen3-Embedding-0.6B".to_string(),
        clean: false,
        source_surface: "x06-model-runtime-proof".to_string(),
        ts: "2026-07-05T12:00:00Z".to_string(),
        supersedes_seq: None,
        payload_kind: Some("model-runtime:degraded-fallback".to_string()),
        payload: Some(payload),
    })?;

    let native_entries = store.read_model_observation_entries()?;
    assert!(native_entries.entries.is_empty());

    let replayed = project_model_runtime_observations_from_store(&store)?;
    assert_eq!(replayed, vec![record]);
    Ok(())
}
