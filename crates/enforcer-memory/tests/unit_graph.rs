use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::record::{MemoryRecord, Provenance, RecordDomain, RecordKind};

fn sample_record(id: &str) -> MemoryRecord {
    MemoryRecord {
        schema_version: 1,
        id: id.to_string(),
        ts: "2026-07-04T00:00:00Z".to_string(),
        kind: RecordKind::Lesson,
        domain: RecordDomain::Harness,
        statement: "sample statement".to_string(),
        why: None,
        how_to_apply: None,
        applies_to: vec![],
        evidence: None,
        routes: vec![],
        landed_at: vec![],
        supersedes: None,
        provenance: Provenance {
            writer: "primary".to_string(),
            ..Default::default()
        },
    }
}

#[test]
fn ingest_and_lookup_by_id() {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(sample_record("mem-primary-0001"));
    assert_eq!(graph.len(), 1);
    assert_eq!(graph.nodes()[0].id(), "mem-primary-0001");
}

#[test]
fn empty_graph_reports_empty() {
    let graph = MemoryGraph::new();
    assert!(graph.is_empty());
}
