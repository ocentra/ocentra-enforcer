use enforcer_domain::paths::RepoRoot;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::{
    ingest_observation, ingest_observation_into_store, parse_ndjson,
    replay_incident_observations_from_store, IngestError, Observation,
};
use enforcer_memory::store::Store;

#[test]
fn parses_multiple_lines_and_skips_blanks() -> Result<(), Box<dyn std::error::Error>> {
    let text = "\n{\"schemaVersion\":1,\"id\":\"mem-primary-0001\",\"ts\":\"2026-07-04T00:00:00Z\",\"kind\":\"lesson\",\"domain\":\"harness\",\"statement\":\"a\",\"provenance\":{\"writer\":\"primary\"}}\n\n{\"schemaVersion\":1,\"id\":\"mem-primary-0002\",\"ts\":\"2026-07-04T00:00:01Z\",\"kind\":\"decision\",\"domain\":\"code\",\"statement\":\"b\",\"provenance\":{\"writer\":\"primary\"}}\n";
    let records = parse_ndjson(text)?;
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].id(), "mem-primary-0001");
    assert_eq!(records[1].id(), "mem-primary-0002");
    Ok(())
}

#[test]
fn rejects_malformed_line() -> Result<(), Box<dyn std::error::Error>> {
    let text = "{not json}\n";
    let result = parse_ndjson(text);
    match result {
        Err(IngestError::InvalidJson { line, .. }) => {
            assert_eq!(line, 1);
            Ok(())
        }
        Ok(_) => Err("malformed line parsed as valid ndjson".into()),
    }
}

#[test]
fn observation_seam_records_clean_run_as_negative_evidence() {
    let mut graph = MemoryGraph::new();
    let id = ingest_observation(
        &mut graph,
        Observation {
            lesson_id: ("L1".to_string()).into(),
            rule_id: None,
            fault_class: None,
            repo_context: ("crates/enforcer-memory".to_string()).into(),
            clean: (true).into(),
            source_surface: ("scan".to_string()).into(),
            ts: ("2026-07-04T00:00:00Z".to_string()).into(),
        },
    );
    assert_eq!(graph.len(), 1);
    assert_eq!(graph.incidents_for_lesson(&"L1".into()).len(), 1);
    assert!(graph.incidents_for_lesson(&"L1".into())[0].clean.is_clean());
    assert!(id.starts_with("obs-scan-"));
}

#[test]
fn store_backed_observation_appends_then_replays_projection(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root: RepoRoot = "C:/Projects/x06-observation-store".parse()?;
    let mut store = Store::init(dir.path(), &root, "2026-07-04T00:00:00Z")?;
    let mut graph = MemoryGraph::new();

    let id = ingest_observation_into_store(
        &mut store,
        &mut graph,
        Observation {
            lesson_id: ("L-store".to_string()).into(),
            rule_id: Some("RULE-1".to_string().into()),
            fault_class: Some("model-load-failure".to_string().into()),
            repo_context: ("crates/enforcer-memory".to_string()).into(),
            clean: (false).into(),
            source_surface: ("scan".to_string()).into(),
            ts: ("2026-07-04T00:00:00Z".to_string()).into(),
        },
    )?;

    assert_eq!(graph.incidents_for_lesson(&"L-store".into()).len(), 1);
    let entries = store.read_observation_entries()?;
    assert_eq!(entries.entries.len(), 1);
    assert_eq!(entries.entries[0].id, id.as_str());

    let mut replayed = MemoryGraph::new();
    let replay_count = replay_incident_observations_from_store(&store, &mut replayed)?;
    assert_eq!(replay_count, 1);
    assert_eq!(replayed.incidents_for_lesson(&"L-store".into()).len(), 1);
    Ok(())
}
