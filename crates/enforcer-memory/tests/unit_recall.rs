use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::{ingest_observation, Observation};
use enforcer_memory::lesson::LessonRow;
use enforcer_memory::recall::{evidence, recall, EvidenceResult};

fn graph_with_lesson_and_incident() -> MemoryGraph {
    let mut graph = MemoryGraph::new();
    graph.ingest_lesson_row(LessonRow {
        id: "L1".to_string(),
        date: "2026-07-04".to_string(),
        observed: "init threw raw EEXIST".to_string(),
        lesson: "init must be idempotent".to_string(),
        landed_at: "arc-16 finding".to_string(),
        ships_via: "fixed MCP tool behavior".to_string(),
    });
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "L1".to_string(),
            rule_id: Some("ARC16-INIT".to_string()),
            fault_class: Some("non_idempotent_init".to_string()),
            repo_context: "crates/enforcer-coordination".to_string(),
            clean: false,
            source_surface: "check".to_string(),
            ts: "2026-07-04T00:00:00Z".to_string(),
        },
    );
    graph
}

#[test]
fn recall_returns_expected_record_for_query() {
    let graph = graph_with_lesson_and_incident();
    let hits = recall(&graph, "idempotent");
    // Both the lesson row (L1, whose text says "must be idempotent")
    // and the incident recorded against it (fault_class
    // "non_idempotent_init") legitimately mention "idempotent".
    let ids: Vec<&str> = hits.iter().map(|hit| hit.node.id()).collect();
    assert!(
        ids.contains(&"L1"),
        "expected the lesson row in hits, got {ids:?}"
    );
}

#[test]
fn recall_with_no_match_returns_empty_not_everything() {
    let graph = graph_with_lesson_and_incident();
    let hits = recall(&graph, "quantum-flux-capacitor-nonsense");
    assert!(hits.is_empty(), "must not fall back to returning all nodes");
}

#[test]
fn recall_empty_query_returns_empty() {
    let graph = graph_with_lesson_and_incident();
    let hits = recall(&graph, "   ");
    assert!(hits.is_empty());
}

#[test]
fn evidence_reports_full_chain_with_provenance() {
    let graph = graph_with_lesson_and_incident();
    match evidence(&graph, "L1") {
        EvidenceResult::Chain {
            has_t0_provenance,
            steps,
            recurrence_since_landing,
            ..
        } => {
            assert!(has_t0_provenance);
            assert_eq!(steps.len(), 2);
            assert_eq!(recurrence_since_landing, 1);
        }
        EvidenceResult::Unknown { .. } => unreachable!("expected a chain"),
    }
}

#[test]
fn evidence_unknown_lesson_is_unknown_not_fabricated() {
    let graph = graph_with_lesson_and_incident();
    match evidence(&graph, "L-does-not-exist") {
        EvidenceResult::Unknown { lesson_id } => assert_eq!(lesson_id, "L-does-not-exist"),
        EvidenceResult::Chain { .. } => {
            unreachable!("must not fabricate a chain for an unknown lesson")
        }
    }
}

#[test]
fn evidence_incomplete_when_no_t0_provenance() {
    let mut graph = MemoryGraph::new();
    graph.ingest_lesson_row(LessonRow {
        id: "L2".to_string(),
        date: "2026-07-04".to_string(),
        observed: "seen once".to_string(),
        lesson: "no incidents recorded yet".to_string(),
        landed_at: "commit abc123".to_string(),
        ships_via: "docs".to_string(),
    });
    match evidence(&graph, "L2") {
        EvidenceResult::Chain {
            has_t0_provenance, ..
        } => {
            assert!(
                !has_t0_provenance,
                "must report incomplete, not fabricate t0"
            );
        }
        EvidenceResult::Unknown { .. } => unreachable!("lesson exists, must not be Unknown"),
    }
}
