use enforcer_domain::memory_types::{GraphEventKind, ModelTask, ProceduralOutcome, ProviderKind};
use enforcer_memory::boundary::log_schema::{
    ArtifactManifestEntryDto, GraphEventLogEntryDto, IndexManifestDto, ModelObservationLogEntryDto,
    ObservationLogEntryDto, ProceduralLogEntryDto, RouteTraceLogEntryDto, SCHEMA_VERSION,
};
use enforcer_memory::boundary::record::{EvidenceDto, ProvenanceDto};
use enforcer_memory::model_observations::{ModelLoadFailureDto, ModelRuntimeObservationCandidate};
use serde::{de::DeserializeOwned, Serialize};

fn assert_json_round_trip<T>(value: &T) -> Result<(), serde_json::Error>
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let encoded = serde_json::to_vec(value)?;
    let decoded: T = serde_json::from_slice(&encoded)?;
    assert_eq!(&decoded, value);
    Ok(())
}

#[test]
fn observation_entry_round_trips() -> Result<(), serde_json::Error> {
    let entry = ObservationLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 0,
        id: "obs-scan-0000".to_owned(),
        lesson_id: "L1".to_owned(),
        rule_id: None,
        fault_class: None,
        repo_context: "crates/enforcer-memory".to_owned(),
        clean: true,
        source_surface: "scan".to_owned(),
        ts: "2026-07-04T00:00:00Z".to_owned(),
        supersedes_seq: None,
        payload_kind: None,
        payload: None,
    };
    let json = serde_json::to_string(&entry)?;
    let back: ObservationLogEntryDto = serde_json::from_str(&json)?;
    assert_eq!(entry, back);
    Ok(())
}

#[test]
fn graph_event_kind_tags_on_wire() -> Result<(), serde_json::Error> {
    let entry = GraphEventLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 0,
        id: "graph-0000".to_owned(),
        event: GraphEventKind::NodeAdded {
            node_id: "n1".into(),
            node_kind: "file".into(),
        },
        ts: "2026-07-04T00:00:00Z".to_owned(),
        supersedes_seq: None,
    };
    let json = serde_json::to_string(&entry)?;
    let value: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(value["event"]["kind"].as_str(), Some("nodeAdded"));
    Ok(())
}

#[test]
fn index_manifest_round_trips() -> Result<(), serde_json::Error> {
    let manifest = IndexManifestDto {
        schema_version: SCHEMA_VERSION,
        source_log: "observation".to_owned(),
        source_high_watermark: 42,
        built_at: "2026-07-04T00:00:00Z".to_owned(),
    };
    let json = serde_json::to_string(&manifest)?;
    let back: IndexManifestDto = serde_json::from_str(&json)?;
    assert_eq!(manifest, back);
    Ok(())
}

#[test]
fn malformed_graph_event_kind_is_rejected_as_data() {
    let malformed = serde_json::json!({
        "schemaVersion": SCHEMA_VERSION,
        "seq": 1,
        "id": "graph-invalid",
        "event": { "kind": "unknownEvent" },
        "ts": "2026-07-17T00:00:00Z"
    });
    let outcome = serde_json::from_value::<GraphEventLogEntryDto>(malformed);
    assert!(
        outcome.is_err(),
        "unknown graph-event tags must fail closed"
    );
    if let Err(error) = outcome {
        assert_eq!(error.classify(), serde_json::error::Category::Data);
    }
}

#[test]
fn log_schema_dtos_round_trip_without_losing_typed_payloads() -> Result<(), serde_json::Error> {
    let observation = ObservationLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 1,
        id: "obs-1".to_owned(),
        lesson_id: "lesson-1".to_owned(),
        rule_id: Some("RR-1.1".to_owned()),
        fault_class: None,
        repo_context: "crates/enforcer-memory".to_owned(),
        clean: false,
        source_surface: "scan".to_owned(),
        ts: "2026-07-17T00:00:00Z".to_owned(),
        supersedes_seq: None,
        payload_kind: None,
        payload: None,
    };
    let graph_event = GraphEventLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 2,
        id: "graph-2".to_owned(),
        event: GraphEventKind::NodeAdded {
            node_id: "node-1".into(),
            node_kind: "file".into(),
        },
        ts: "2026-07-17T00:00:01Z".to_owned(),
        supersedes_seq: None,
    };
    let procedural = ProceduralLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 3,
        id: "procedure-3".to_owned(),
        lesson_id: "lesson-1".to_owned(),
        outcome: ProceduralOutcome::FixSuccess,
        detail: "applied retained fix".to_owned(),
        ts: "2026-07-17T00:00:02Z".to_owned(),
        supersedes_seq: None,
    };
    let route = RouteTraceLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 4,
        id: "route-4".to_owned(),
        query: "find owner".to_owned(),
        route: "code_graph".to_owned(),
        confidence: 0.75,
        ts: "2026-07-17T00:00:03Z".to_owned(),
        supersedes_seq: None,
    };
    let model = ModelObservationLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 5,
        observed_at: "2026-07-17T00:00:04Z".to_owned(),
        source: "model-runtime".to_owned(),
        run_id: "run-5".to_owned(),
        candidate: ModelRuntimeObservationCandidate::ModelLoadFailureDto(ModelLoadFailureDto {
            model_id: "local-embedding".to_owned(),
            task: ModelTask::Embedding,
            requested_provider: Some(ProviderKind::Cpu),
            failure_reason: "fixture failure".to_owned(),
        }),
        supersedes_seq: None,
    };
    let artifact = ArtifactManifestEntryDto {
        schema_version: SCHEMA_VERSION,
        id: format!("sha256:{}", "ab".repeat(32)),
        rel_path: Some("src/lib.rs".to_owned()),
        byte_len: 42,
        ts: "2026-07-17T00:00:05Z".to_owned(),
    };
    let index = IndexManifestDto {
        schema_version: SCHEMA_VERSION,
        source_log: "observation".to_owned(),
        source_high_watermark: 6,
        built_at: "2026-07-17T00:00:06Z".to_owned(),
    };

    assert_json_round_trip(&observation)?;
    assert_json_round_trip(&graph_event)?;
    assert_json_round_trip(&procedural)?;
    assert_json_round_trip(&route)?;
    assert_json_round_trip(&model)?;
    assert_json_round_trip(&artifact)?;
    assert_json_round_trip(&index)?;
    Ok(())
}

#[test]
fn record_dtos_round_trip_the_external_optional_fields() -> Result<(), serde_json::Error> {
    let evidence = EvidenceDto {
        source: Some("scanner".to_owned().into()),
        r#ref: Some("finding-1".to_owned()),
    };
    let provenance = ProvenanceDto {
        writer: "primary".into(),
        session_id: Some("session-1".to_owned().into()),
        model: Some("codex".to_owned().into()),
        user: Some("operator".to_owned().into()),
    };
    assert_json_round_trip(&evidence)?;
    assert_json_round_trip(&provenance)?;
    Ok(())
}
