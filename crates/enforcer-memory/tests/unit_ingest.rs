use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::{ingest_observation, parse_ndjson, IngestError, Observation};

#[test]
fn parses_multiple_lines_and_skips_blanks() -> Result<(), Box<dyn std::error::Error>> {
    let text = "\n{\"schemaVersion\":1,\"id\":\"mem-primary-0001\",\"ts\":\"2026-07-04T00:00:00Z\",\"kind\":\"lesson\",\"domain\":\"harness\",\"statement\":\"a\",\"provenance\":{\"writer\":\"primary\"}}\n\n{\"schemaVersion\":1,\"id\":\"mem-primary-0002\",\"ts\":\"2026-07-04T00:00:01Z\",\"kind\":\"decision\",\"domain\":\"code\",\"statement\":\"b\",\"provenance\":{\"writer\":\"primary\"}}\n";
    let records = parse_ndjson(text)?;
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].id, "mem-primary-0001");
    assert_eq!(records[1].id, "mem-primary-0002");
    Ok(())
}

#[test]
fn rejects_malformed_line() {
    let text = "{not json}\n";
    let result = parse_ndjson(text);
    match result {
        Err(IngestError::InvalidJson { line, .. }) => assert_eq!(line, 1),
        Ok(_) => unreachable!("malformed line must not parse as valid ndjson"),
    }
}

#[test]
fn observation_seam_records_clean_run_as_negative_evidence() {
    let mut graph = MemoryGraph::new();
    let id = ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "L1".to_string(),
            rule_id: None,
            fault_class: None,
            repo_context: "crates/enforcer-memory".to_string(),
            clean: true,
            source_surface: "scan".to_string(),
            ts: "2026-07-04T00:00:00Z".to_string(),
        },
    );
    assert_eq!(graph.len(), 1);
    assert_eq!(graph.incidents_for_lesson("L1").len(), 1);
    assert!(graph.incidents_for_lesson("L1")[0].clean);
    assert!(id.starts_with("obs-scan-"));
}
