use enforcer_domain::paths::RepoRoot;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::observations::{
    procedural_success_rate, record_procedural, record_procedural_in_store, record_route_choice,
    record_route_choice_in_store, replay_procedural_and_routes_from_store, ProceduralOutcome,
    ProceduralStoreInput, RouteChoiceStoreInput,
};
use enforcer_memory::store::Store;

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
    assert_eq!(procedural_success_rate(&graph, "L1"), Some(0.5));
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
    let missing_payload_seqs: Vec<u64> = entries
        .entries
        .iter()
        .filter(|entry| entry.payload.is_none())
        .map(|entry| entry.seq)
        .collect();
    assert_eq!(missing_payload_seqs, Vec::<u64>::new());
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

    store.append_observation_entry(|seq| enforcer_memory::schema::ObservationLogEntry {
        schema_version: enforcer_memory::schema::SCHEMA_VERSION,
        seq,
        id: format!("proc-{seq:04}"),
        lesson_id: "L1".to_owned(),
        rule_id: None,
        fault_class: Some("fix-success".to_owned()),
        repo_context: "applied idempotent-init fix".to_owned(),
        clean: true,
        source_surface: "procedural-memory".to_owned(),
        ts: "2026-07-04T00:00:00Z".to_owned(),
        supersedes_seq: None,
        payload_kind: Some("procedural-memory".to_owned()),
        payload: Some(serde_json::json!({
            "id": format!("proc-{seq:04}"),
            "lesson_id": "L1",
            "outcome": "fix-success",
            "detail": "applied idempotent-init fix",
            "ts": "2026-07-04T00:00:00Z"
        })),
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

    store.append_observation_entry(|seq| enforcer_memory::schema::ObservationLogEntry {
        schema_version: enforcer_memory::schema::SCHEMA_VERSION,
        seq,
        id: format!("route-{seq:04}"),
        lesson_id: String::new(),
        rule_id: None,
        fault_class: Some("route-choice".to_owned()),
        repo_context: "idempotent init".to_owned(),
        clean: true,
        source_surface: "route-choice".to_owned(),
        ts: "2026-07-04T00:00:01Z".to_owned(),
        supersedes_seq: None,
        payload_kind: Some("route-choice".to_owned()),
        payload: Some(serde_json::json!({
            "id": format!("route-{seq:04}"),
            "query": "idempotent init",
            "route": "hybrid-search",
            "confidence": 0.91,
            "ts": "2026-07-04T00:00:01Z"
        })),
    })?;

    assert!(store.read_route_trace_entries()?.entries.is_empty());

    let replay_count = replay_procedural_and_routes_from_store(&store, &mut graph)?;
    assert_eq!(replay_count, 1);
    assert_eq!(graph.route_traces().len(), 1);
    assert_eq!(graph.route_traces()[0].route, "hybrid-search");
    Ok(())
}
