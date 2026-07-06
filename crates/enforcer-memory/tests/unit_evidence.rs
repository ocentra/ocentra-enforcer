use enforcer_memory::evidence::{
    evidence_chain, recurrence_curve, EvidenceReport, NoProofRefs, ProofRefLookup,
};
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::{ingest_observation, Observation};
use enforcer_memory::lesson::LessonRow;
use std::collections::HashMap;

struct FixedLookup(HashMap<String, Vec<String>>);

impl ProofRefLookup for FixedLookup {
    fn lookup(&self, landed_at_ref: &str) -> Vec<String> {
        self.0.get(landed_at_ref).cloned().unwrap_or_default()
    }
}

fn graph_with_landed_lesson_and_incident() -> MemoryGraph {
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
fn evidence_chain_attaches_proof_refs_to_landed_step() {
    let graph = graph_with_landed_lesson_and_incident();
    let mut refs = HashMap::new();
    refs.insert(
        "arc-16 finding".to_string(),
        vec!["proof/journal/arc-16-0007".to_string()],
    );
    let lookup = FixedLookup(refs);

    match evidence_chain(&graph, "L1", &lookup) {
        EvidenceReport::Chain {
            landed,
            observed,
            has_t0_provenance,
            ..
        } => {
            assert!(has_t0_provenance);
            assert_eq!(observed.len(), 1);
            assert_eq!(landed.len(), 1);
            assert_eq!(landed[0].proof_refs, vec!["proof/journal/arc-16-0007"]);
        }
        EvidenceReport::Unknown { .. } => unreachable!("expected a chain"),
    }
}

#[test]
fn evidence_chain_no_lookup_hit_reports_empty_refs_not_error() {
    let graph = graph_with_landed_lesson_and_incident();
    let lookup = NoProofRefs;
    match evidence_chain(&graph, "L1", &lookup) {
        EvidenceReport::Chain { landed, .. } => {
            assert_eq!(landed[0].proof_refs, Vec::<String>::new());
        }
        EvidenceReport::Unknown { .. } => unreachable!("expected a chain"),
    }
}

#[test]
fn evidence_chain_unknown_lesson_is_incomplete_safe() {
    let graph = graph_with_landed_lesson_and_incident();
    let report = evidence_chain(&graph, "L-missing", &NoProofRefs);
    assert!(matches!(report, EvidenceReport::Unknown { .. }));
    assert!(
        !report.is_incomplete(),
        "Unknown is not the same as incomplete-chain"
    );
}

#[test]
fn evidence_chain_missing_t0_is_incomplete() {
    let mut graph = MemoryGraph::new();
    graph.ingest_lesson_row(LessonRow {
        id: "L2".to_string(),
        date: "2026-07-04".to_string(),
        observed: "seen once".to_string(),
        lesson: "no incidents recorded yet".to_string(),
        landed_at: "commit abc123".to_string(),
        ships_via: "docs".to_string(),
    });
    let report = evidence_chain(&graph, "L2", &NoProofRefs);
    assert!(report.is_incomplete(), "missing t0 must report incomplete");
}

#[test]
fn recurrence_curve_counts_only_after_landing_exists() {
    let graph = graph_with_landed_lesson_and_incident();
    let curve = recurrence_curve(&graph, "L1");
    assert_eq!(curve.len(), 1);
    assert!(curve[0].since_landing);
    assert_eq!(curve[0].running_recurrence_count, 1);
}

#[test]
fn recurrence_curve_before_any_landing_is_zero() {
    let mut graph = MemoryGraph::new();
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: "L3".to_string(),
            rule_id: None,
            fault_class: Some("x".to_string()),
            repo_context: "crates/foo".to_string(),
            clean: false,
            source_surface: "scan".to_string(),
            ts: "2026-07-04T00:00:00Z".to_string(),
        },
    );
    let curve = recurrence_curve(&graph, "L3");
    assert_eq!(curve.len(), 1);
    assert!(!curve[0].since_landing, "no landing exists yet for L3");
    assert_eq!(curve[0].running_recurrence_count, 0);
}
