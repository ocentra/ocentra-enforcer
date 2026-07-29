use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::memory_types::MemoryLessonId;
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::{ingest_observation, Observation};
use enforcer_memory::lesson::LessonRow;
use enforcer_memory::recall::{evidence, recall, EvidenceResult};

type TestResult = Result<(), DecodeError>;

fn lesson_id(value: &str) -> Result<MemoryLessonId, DecodeError> {
    MemoryLessonId::try_from(value.to_owned())
}

fn graph_with_lesson_and_incident() -> MemoryGraph {
    let mut graph = MemoryGraph::new();
    graph.ingest_lesson_row(LessonRow {
        id: "L1".to_string().into(),
        date: "2026-07-04".to_string().into(),
        observed: "init threw raw EEXIST".to_string().into(),
        lesson: "init must be idempotent".to_string().into(),
        landed_at: "arc-16 finding".to_string().into(),
        ships_via: "fixed MCP tool behavior".to_string().into(),
    });
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: ("L1".to_string()).into(),
            rule_id: Some("ARC16-INIT".to_string().into()),
            fault_class: Some("non_idempotent_init".to_string().into()),
            repo_context: ("crates/enforcer-coordination".to_string()).into(),
            clean: (false).into(),
            source_surface: ("check".to_string()).into(),
            ts: ("2026-07-04T00:00:00Z".to_string()).into(),
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
    let ids: Vec<_> = hits.iter().map(|hit| hit.node.id()).collect();
    assert!(
        ids.iter().any(|id| id == "L1"),
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
fn evidence_reports_full_chain_with_provenance() -> TestResult {
    let graph = graph_with_lesson_and_incident();
    let report = evidence(&graph, &lesson_id("L1")?);
    assert!(matches!(&report, EvidenceResult::Chain { .. }));
    if let EvidenceResult::Chain {
        has_t0_provenance,
        steps,
        recurrence_since_landing,
        ..
    } = report
    {
        assert!(has_t0_provenance.has_t0_provenance());
        assert_eq!(steps.len(), 2);
        assert_eq!(recurrence_since_landing, 1);
    }
    Ok(())
}

#[test]
fn evidence_unknown_lesson_is_unknown_not_fabricated() -> TestResult {
    let graph = graph_with_lesson_and_incident();
    let report = evidence(&graph, &lesson_id("L-does-not-exist")?);
    assert!(matches!(&report, EvidenceResult::Unknown { .. }));
    if let EvidenceResult::Unknown { lesson_id } = report {
        assert_eq!(lesson_id.as_str(), "L-does-not-exist");
    }
    Ok(())
}

#[test]
fn evidence_incomplete_when_no_t0_provenance() -> TestResult {
    let mut graph = MemoryGraph::new();
    graph.ingest_lesson_row(LessonRow {
        id: "L2".to_string().into(),
        date: "2026-07-04".to_string().into(),
        observed: "seen once".to_string().into(),
        lesson: "no incidents recorded yet".to_string().into(),
        landed_at: "commit abc123".to_string().into(),
        ships_via: "docs".to_string().into(),
    });
    let report = evidence(&graph, &lesson_id("L2")?);
    assert!(matches!(&report, EvidenceResult::Chain { .. }));
    if let EvidenceResult::Chain {
        has_t0_provenance, ..
    } = report
    {
        assert!(
            !has_t0_provenance.has_t0_provenance(),
            "must report incomplete, not fabricate t0"
        );
    }
    Ok(())
}
