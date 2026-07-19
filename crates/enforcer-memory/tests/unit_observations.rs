use enforcer_domain::memory_types::{
    ModelTask, ProceduralOutcome, ProviderKind, RecurrenceNegativeKind,
};
use enforcer_domain::paths::RepoRoot;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::model_observations::{
    DegradedFallbackDto, HashMismatchDto, LocalLoadSucceededDto, ModelLoadFailureDto,
    ModelRuntimeObservationCandidate, ModelRuntimeObservationRecordDto, ProviderDowngradeDto,
    RecurrenceOrNegativeEvidenceDto, RerankerLiftProofDto, RetrievalQualityProofDto,
    RouteChoiceImprovementDto, TokenReductionProofDto,
};
use enforcer_memory::observations::{
    procedural_success_rate, record_procedural, record_procedural_in_store, record_route_choice,
    record_route_choice_in_store, replay_procedural_and_routes_from_store, ProceduralStoreInput,
    RouteChoiceStoreInput,
};
use enforcer_memory::store::Store;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

fn assert_json_round_trip<T>(value: &T) -> Result<(), serde_json::Error>
where
    T: Debug + PartialEq + Serialize + DeserializeOwned,
{
    let encoded = serde_json::to_vec(value)?;
    let decoded: T = serde_json::from_slice(&encoded)?;
    assert_eq!(&decoded, value);
    Ok(())
}

#[test]
fn model_observation_dtos_round_trip_without_losing_wire_values() -> Result<(), serde_json::Error> {
    let model_load_failure: ModelLoadFailureDto = ModelLoadFailureDto {
        model_id: "embedding-primary".to_owned(),
        task: ModelTask::Embedding,
        requested_provider: Some(ProviderKind::Cuda),
        failure_reason: "provider unavailable".to_owned(),
    };
    let provider_downgrade: ProviderDowngradeDto = ProviderDowngradeDto {
        model_id: "embedding-primary".to_owned(),
        task: ModelTask::Embedding,
        requested_provider: ProviderKind::Cuda,
        fallback_provider: ProviderKind::Cpu,
        reason: "provider unavailable".to_owned(),
    };
    let hash_mismatch: HashMismatchDto = HashMismatchDto {
        model_id: "embedding-primary".to_owned(),
        path: "models/embedding-primary.bin".to_owned(),
        expected_sha256: "expected".to_owned(),
        observed_sha256: "observed".to_owned(),
    };
    let degraded_fallback: DegradedFallbackDto = DegradedFallbackDto {
        model_id: "embedding-primary".to_owned(),
        task: ModelTask::Embedding,
        fallback_reason: "local artifact unavailable".to_owned(),
    };
    let local_load_succeeded: LocalLoadSucceededDto = LocalLoadSucceededDto {
        model_id: "embedding-primary".to_owned(),
        task: ModelTask::Embedding,
        provider: ProviderKind::Cpu,
        loaded_from_local_cache: true,
    };
    let recurrence_or_negative_evidence: RecurrenceOrNegativeEvidenceDto =
        RecurrenceOrNegativeEvidenceDto {
            lesson_id: "lesson-provider-selection".to_owned(),
            query_id: Some("query-provider-selection".to_owned()),
            evidence_kind: RecurrenceNegativeKind::RecurrenceCount {
                recurrence_count: 2_usize.into(),
                previous_count: Some(1_usize.into()),
            },
            clean_evidence: false,
        };
    let observation_record: ModelRuntimeObservationRecordDto =
        ModelRuntimeObservationRecordDto::new(
            "2026-07-17T00:00:00Z",
            "unit-observations",
            "run-model-observations",
            ModelRuntimeObservationCandidate::ModelLoadFailureDto(model_load_failure.clone()),
        );
    let retrieval_quality = RetrievalQualityProofDto {
        query_id: "query-retrieval".to_owned(),
        query: "where is the local model loaded".to_owned(),
        route: "hybrid".to_owned(),
        recall_at_five: 0.8,
        recall_at_ten: 0.95,
        precision_at_five: 0.6,
        expected_top_k: 10,
        returned_top_k: 10,
    };
    let reranker_lift = RerankerLiftProofDto {
        query_id: "query-reranker".to_owned(),
        query: "rank the local model loader".to_owned(),
        pre_rerank_top_k: vec!["a".to_owned(), "b".to_owned()],
        post_rerank_top_k: vec!["b".to_owned(), "a".to_owned()],
        lift_score: 0.31,
        improved: true,
    };
    let token_reduction = TokenReductionProofDto {
        query_id: "query-token".to_owned(),
        query: "reduce model context".to_owned(),
        naive_tokens: 1_000,
        context_tokens: 200,
    };
    let route_choice = RouteChoiceImprovementDto {
        query_id: "query-route".to_owned(),
        query: "choose the model route".to_owned(),
        chosen_route: "hybrid".to_owned(),
        alternative_route: "semantic-only".to_owned(),
        chosen_route_score: 0.92,
        alternative_route_score: 0.55,
        improvement_delta: 0.37,
    };

    assert_json_round_trip(&model_load_failure)?;
    assert_json_round_trip(&provider_downgrade)?;
    assert_json_round_trip(&hash_mismatch)?;
    assert_json_round_trip(&degraded_fallback)?;
    assert_json_round_trip(&local_load_succeeded)?;
    assert_json_round_trip(&recurrence_or_negative_evidence)?;
    assert_json_round_trip(&observation_record)?;
    assert_json_round_trip::<RetrievalQualityProofDto>(&retrieval_quality)?;
    assert_json_round_trip::<RerankerLiftProofDto>(&reranker_lift)?;
    assert_json_round_trip::<TokenReductionProofDto>(&token_reduction)?;
    assert_json_round_trip::<RouteChoiceImprovementDto>(&route_choice)?;
    Ok(())
}

#[test]
fn records_both_success_and_failure_outcomes() {
    let mut graph = MemoryGraph::new();
    record_procedural(
        &mut graph,
        "L1",
        ProceduralOutcome::FixSuccess,
        "applied idempotent-init fix",
        "2026-07-04T00:00:00Z",
    );
    record_procedural(
        &mut graph,
        "L1",
        ProceduralOutcome::FixFailure,
        "fix regressed on retry",
        "2026-07-04T00:01:00Z",
    );
    assert_eq!(graph.procedural_records().len(), 2);
    assert_eq!(
        procedural_success_rate(&graph, "L1").map(|rate| rate.get()),
        Some(0.5)
    );
}

#[test]
fn success_rate_is_none_when_no_history() {
    let graph = MemoryGraph::new();
    assert_eq!(procedural_success_rate(&graph, "L-never-tried"), None);
}

#[test]
fn records_route_choice_with_confidence() {
    let mut graph = MemoryGraph::new();
    let id = record_route_choice(
        &mut graph,
        "idempotent init",
        "recall",
        0.9,
        "2026-07-04T00:00:00Z",
    );
    assert!(id.starts_with("route-"));
    assert_eq!(graph.route_traces().len(), 1);
    assert_eq!(graph.route_traces()[0].confidence, 0.9);
}

#[test]
fn confidence_is_clamped_not_stored_out_of_range() {
    let mut graph = MemoryGraph::new();
    record_route_choice(&mut graph, "q", "recall", 5.0, "2026-07-04T00:00:00Z");
    record_route_choice(&mut graph, "q2", "recall", -1.0, "2026-07-04T00:00:00Z");
    assert_eq!(graph.route_traces()[0].confidence, 1.0);
    assert_eq!(graph.route_traces()[1].confidence, 0.0);
}

#[test]
fn procedural_and_route_records_replay_from_store() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root: RepoRoot = "C:/Projects/x06-procedural-store".parse()?;
    let mut store = Store::init(dir.path(), &root, "2026-07-04T00:00:00Z")?;
    let mut graph = MemoryGraph::new();

    record_procedural_in_store(
        &mut store,
        &mut graph,
        &ProceduralStoreInput::new(
            "L1",
            ProceduralOutcome::FixSuccess,
            "applied idempotent-init fix",
            "2026-07-04T00:00:00Z",
        ),
    )?;
    record_route_choice_in_store(
        &mut store,
        &mut graph,
        &RouteChoiceStoreInput::new(
            "idempotent init",
            "hybrid-search",
            0.91,
            "2026-07-04T00:00:01Z",
        ),
    )?;

    assert_eq!(graph.procedural_records().len(), 1);
    assert_eq!(graph.route_traces().len(), 1);
    let entries = store.read_observation_entries()?;
    assert_eq!(entries.entries.len(), 2);
    let procedural_entries = store.read_procedural_entries()?;
    assert_eq!(procedural_entries.entries.len(), 1);
    assert_eq!(procedural_entries.entries[0].lesson_id, "L1");
    let missing_payload_seqs: Vec<_> = entries
        .entries
        .iter()
        .filter(|entry| entry.payload.is_none())
        .map(|entry| entry.seq)
        .collect();
    assert_eq!(missing_payload_seqs, Vec::new());
    let route_entries = store.read_route_trace_entries()?;
    assert_eq!(route_entries.entries.len(), 1);
    assert_eq!(route_entries.entries[0].route, "hybrid-search");

    let mut replayed = MemoryGraph::new();
    let replay_count = replay_procedural_and_routes_from_store(&store, &mut replayed)?;
    assert_eq!(replay_count, 2);
    assert_eq!(replayed.procedural_records().len(), 1);
    assert_eq!(replayed.route_traces().len(), 1);
    Ok(())
}

#[test]
fn procedural_replay_falls_back_to_legacy_observation_payload_when_native_log_is_empty(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root: RepoRoot = "C:/Projects/x06-procedural-legacy-fallback".parse()?;
    let mut store = Store::init(dir.path(), &root, "2026-07-04T00:00:00Z")?;
    let mut graph = MemoryGraph::new();

    store.append_observation_entry(|seq| {
        enforcer_memory::boundary::log_schema::ObservationLogEntryDto {
            schema_version: enforcer_memory::boundary::log_schema::SCHEMA_VERSION,
            seq: seq.into(),
            id: format!("proc-{seq:04}").into(),
            lesson_id: "L1".into(),
            rule_id: None,
            fault_class: Some("fix-success".into()),
            repo_context: "applied idempotent-init fix".into(),
            clean: true.into(),
            source_surface: "procedural-memory".into(),
            ts: "2026-07-04T00:00:00Z".into(),
            supersedes_seq: None,
            payload_kind: Some("procedural-memory".into()),
            payload: Some(serde_json::json!({
                "id": format!("proc-{seq:04}"),
                "lesson_id": "L1",
                "outcome": "fix-success",
                "detail": "applied idempotent-init fix",
                "ts": "2026-07-04T00:00:00Z"
            }).into()),
        }
    })?;

    assert!(store.read_procedural_entries()?.entries.is_empty());

    let replay_count = replay_procedural_and_routes_from_store(&store, &mut graph)?;
    assert_eq!(replay_count, 1);
    assert_eq!(graph.procedural_records().len(), 1);
    assert_eq!(graph.procedural_records()[0].lesson_id, "L1");
    Ok(())
}

#[test]
fn route_trace_replay_falls_back_to_legacy_observation_payload_when_native_log_is_empty(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root: RepoRoot = "C:/Projects/x06-route-trace-legacy-fallback".parse()?;
    let mut store = Store::init(dir.path(), &root, "2026-07-04T00:00:00Z")?;
    let mut graph = MemoryGraph::new();

    store.append_observation_entry(|seq| {
        enforcer_memory::boundary::log_schema::ObservationLogEntryDto {
            schema_version: enforcer_memory::boundary::log_schema::SCHEMA_VERSION,
            seq: seq.into(),
            id: format!("route-{seq:04}").into(),
            lesson_id: String::new().into(),
            rule_id: None,
            fault_class: Some("route-choice".into()),
            repo_context: "idempotent init".into(),
            clean: true.into(),
            source_surface: "route-choice".into(),
            ts: "2026-07-04T00:00:01Z".into(),
            supersedes_seq: None,
            payload_kind: Some("route-choice".into()),
            payload: Some(serde_json::json!({
                "id": format!("route-{seq:04}"),
                "query": "idempotent init",
                "route": "hybrid-search",
                "confidence": 0.91,
                "ts": "2026-07-04T00:00:01Z"
            }).into()),
        }
    })?;

    assert!(store.read_route_trace_entries()?.entries.is_empty());

    let replay_count = replay_procedural_and_routes_from_store(&store, &mut graph)?;
    assert_eq!(replay_count, 1);
    assert_eq!(graph.route_traces().len(), 1);
    assert_eq!(graph.route_traces()[0].route, "hybrid-search");
    Ok(())
}
