use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::{ingest_observation, Observation};
use enforcer_memory::record::{MemoryRecordDto as MemoryRecord, Provenance, RecordDomain, RecordKind};
use enforcer_memory::sessionstart::recall_pack;

fn landed_record(id: &str) -> MemoryRecord {
    MemoryRecord {
        schema_version: 1,
        id: id.to_string(),
        ts: "2026-07-04T00:00:00Z".to_string(),
        kind: RecordKind::Lesson,
        domain: RecordDomain::Harness,
        statement: format!("statement for {id}"),
        why: None,
        how_to_apply: None,
        applies_to: vec![],
        evidence: None,
        routes: vec![],
        landed_at: vec!["commit abc".to_string()],
        supersedes: None,
        provenance: Provenance {
            writer: "primary".to_string(),
            ..Default::default()
        },
    }
}

#[test]
fn recall_pack_is_empty_and_honest_on_a_fresh_graph() {
    let graph = MemoryGraph::new();
    let pack = recall_pack(&graph, 5);
    assert!(pack.active_lessons.is_empty());
    assert_eq!(pack.total_active_lessons, 0);
    assert!(pack.render().contains("no active"));
}

#[test]
fn recall_pack_lists_active_lessons_with_incident_counts() {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(landed_record("mem-a-0001"));
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "mem-a-0001".to_string(),
            rule_id: None,
            fault_class: None,
            repo_context: "crates/foo".to_string(),
            clean: false,
            source_surface: "scan".to_string(),
            ts: "2026-07-04T01:00:00Z".to_string(),
        },
    );
    let pack = recall_pack(&graph, 5);
    assert_eq!(pack.active_lessons.len(), 1);
    assert_eq!(pack.active_lessons[0].lesson_id, "mem-a-0001");
    assert_eq!(pack.active_lessons[0].incident_count, 1);
    assert!(pack.render().contains("mem-a-0001"));
}

#[test]
fn recall_pack_respects_limit_and_reports_overflow() {
    let mut graph = MemoryGraph::new();
    graph.ingest_record(landed_record("mem-a-0001"));
    graph.ingest_record(landed_record("mem-a-0002"));
    graph.ingest_record(landed_record("mem-a-0003"));
    let pack = recall_pack(&graph, 2);
    assert_eq!(pack.active_lessons.len(), 2);
    assert_eq!(pack.total_active_lessons, 3);
    assert!(pack.render().contains("+1 more"));
}

#[test]
fn recall_pack_excludes_unlanded_lessons() {
    let mut graph = MemoryGraph::new();
    let mut unlanded = landed_record("mem-a-0001");
    unlanded.landed_at.clear();
    graph.ingest_record(unlanded);
    let pack = recall_pack(&graph, 5);
    assert!(pack.active_lessons.is_empty());
    assert_eq!(pack.total_active_lessons, 0);
}
