use enforcer_domain::memory_types::{GraphEventKind, ModelTask, ProceduralOutcome, ProviderKind};
use enforcer_memory::boundary::log_schema::{
    ArtifactManifestEntryDto, GraphEventLogEntryDto, IndexManifestDto, MemoryObservationPayloadDto,
    ModelObservationLogEntryDto, ObservationLogEntryDto, ProceduralLogEntryDto,
    RouteTraceLogEntryDto, SCHEMA_VERSION,
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
        seq: 0.into(),
        id: "obs-scan-0000".into(),
        lesson_id: "L1".into(),
        rule_id: None,
        fault_class: None,
        repo_context: "crates/enforcer-memory".into(),
        clean: true.into(),
        source_surface: "scan".into(),
        ts: "2026-07-04T00:00:00Z".into(),
        supersedes_seq: None,
        payload_kind: None,
        payload: Some(
            serde_json::json!({
                "observationKind": "runtime-trace",
                "source": "unit-schema"
            })
            .into(),
        ),
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
        seq: 0.into(),
        id: "graph-0000".into(),
        event: GraphEventKind::NodeAdded {
            node_id: "n1".into(),
            node_kind: "file".into(),
        },
        ts: "2026-07-04T00:00:00Z".into(),
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
        source_log: "observation".into(),
        source_high_watermark: 42.into(),
        built_at: "2026-07-04T00:00:00Z".into(),
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
    let payload: MemoryObservationPayloadDto = serde_json::json!({
        "observationKind": "runtime-trace",
        "source": "unit-schema"
    })
    .into();
    let observation = ObservationLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 1.into(),
        id: "obs-1".into(),
        lesson_id: "lesson-1".into(),
        rule_id: Some("RR-1.1".into()),
        fault_class: None,
        repo_context: "crates/enforcer-memory".into(),
        clean: false.into(),
        source_surface: "scan".into(),
        ts: "2026-07-17T00:00:00Z".into(),
        supersedes_seq: None,
        payload_kind: None,
        payload: Some(payload.clone()),
    };
    let graph_event = GraphEventLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 2.into(),
        id: "graph-2".into(),
        event: GraphEventKind::NodeAdded {
            node_id: "node-1".into(),
            node_kind: "file".into(),
        },
        ts: "2026-07-17T00:00:01Z".into(),
        supersedes_seq: None,
    };
    let procedural = ProceduralLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 3.into(),
        id: "procedure-3".into(),
        lesson_id: "lesson-1".into(),
        outcome: ProceduralOutcome::FixSuccess,
        detail: "applied retained fix".into(),
        ts: "2026-07-17T00:00:02Z".into(),
        supersedes_seq: None,
    };
    let route = RouteTraceLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 4.into(),
        id: "route-4".into(),
        query: "find owner".into(),
        route: "code_graph".into(),
        confidence: 0.75.into(),
        ts: "2026-07-17T00:00:03Z".into(),
        supersedes_seq: None,
    };
    let model = ModelObservationLogEntryDto {
        schema_version: SCHEMA_VERSION,
        seq: 5.into(),
        observed_at: "2026-07-17T00:00:04Z".into(),
        source: "model-runtime".into(),
        run_id: "run-5".into(),
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
        id: enforcer_domain::memory_types::ArtifactId::from_content(b"fixture"),
        rel_path: Some("src/lib.rs".into()),
        byte_len: 42.into(),
        ts: "2026-07-17T00:00:05Z".into(),
    };
    let index = IndexManifestDto {
        schema_version: SCHEMA_VERSION,
        source_log: "observation".into(),
        source_high_watermark: 6.into(),
        built_at: "2026-07-17T00:00:06Z".into(),
    };

    assert_json_round_trip(&payload)?;
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
