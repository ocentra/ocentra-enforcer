use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_memory::evidence::{
    evidence_chain, recurrence_curve, EvidenceReport, NoProofRefs, ProofRefLookup,
};
use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::ingest::{ingest_observation, Observation};
use enforcer_memory::lesson::LessonRow;
use std::collections::HashMap;

struct FixedLookup(HashMap<MemoryEvidenceLandedAt, Vec<MemoryEvidenceProofRef>>);

impl ProofRefLookup for FixedLookup {
    fn lookup(&self, landed_at_ref: &MemoryEvidenceLandedAt) -> Vec<MemoryEvidenceProofRef> {
        self.0.get(landed_at_ref).cloned().unwrap_or_default()
    }
}

type TestResult = Result<(), DecodeError>;

fn lesson_id(value: &str) -> Result<MemoryLessonId, DecodeError> {
    MemoryLessonId::try_from(value.to_owned())
}

fn graph_with_landed_lesson_and_incident() -> MemoryGraph {
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
fn evidence_chain_attaches_proof_refs_to_landed_step() -> TestResult {
    let graph = graph_with_landed_lesson_and_incident();
    let mut refs = HashMap::new();
    refs.insert(
        "arc-16 finding".into(),
        vec!["proof/journal/arc-16-0007".into()],
    );
    let lookup = FixedLookup(refs);

    let report = evidence_chain(&graph, &lesson_id("L1")?, &lookup);
    assert!(matches!(&report, EvidenceReport::Chain { .. }));
    if let EvidenceReport::Chain {
        landed,
        observed,
        has_t0_provenance,
        ..
    } = report
    {
        assert!(has_t0_provenance.has_t0_provenance());
        assert_eq!(observed.len(), 1);
        assert_eq!(landed.len(), 1);
        assert_eq!(
            landed[0]
                .proof_refs
                .iter()
                .map(|proof_ref| proof_ref.as_str())
                .collect::<Vec<_>>(),
            vec!["proof/journal/arc-16-0007"]
        );
    }
    Ok(())
}

#[test]
fn evidence_chain_no_lookup_hit_reports_empty_refs_not_error() -> TestResult {
    let graph = graph_with_landed_lesson_and_incident();
    let lookup = NoProofRefs;
    let report = evidence_chain(&graph, &lesson_id("L1")?, &lookup);
    assert!(matches!(&report, EvidenceReport::Chain { .. }));
    if let EvidenceReport::Chain { landed, .. } = report {
        assert!(landed[0].proof_refs.is_empty());
    }
    Ok(())
}

#[test]
fn evidence_chain_unknown_lesson_is_incomplete_safe() -> TestResult {
    let graph = graph_with_landed_lesson_and_incident();
    let report = evidence_chain(&graph, &lesson_id("L-missing")?, &NoProofRefs);
    assert!(matches!(report, EvidenceReport::Unknown { .. }));
    assert!(
        !report.is_incomplete().is_incomplete(),
        "Unknown is not the same as incomplete-chain"
    );
    Ok(())
}

#[test]
fn evidence_chain_missing_t0_is_incomplete() -> TestResult {
    let mut graph = MemoryGraph::new();
    graph.ingest_lesson_row(LessonRow {
        id: "L2".to_string().into(),
        date: "2026-07-04".to_string().into(),
        observed: "seen once".to_string().into(),
        lesson: "no incidents recorded yet".to_string().into(),
        landed_at: "commit abc123".to_string().into(),
        ships_via: "docs".to_string().into(),
    });
    let report = evidence_chain(&graph, &lesson_id("L2")?, &NoProofRefs);
    assert!(
        report.is_incomplete().is_incomplete(),
        "missing t0 must report incomplete"
    );
    Ok(())
}

#[test]
fn recurrence_curve_counts_only_after_landing_exists() -> TestResult {
    let graph = graph_with_landed_lesson_and_incident();
    let curve = recurrence_curve(&graph, &lesson_id("L1")?);
    assert_eq!(curve.len(), 1);
    assert!(curve[0].since_landing.is_since_landing());
    assert_eq!(curve[0].running_recurrence_count, 1);
    Ok(())
}

#[test]
fn recurrence_curve_before_any_landing_is_zero() -> TestResult {
    let mut graph = MemoryGraph::new();
    ingest_observation(
        &mut graph,
        Observation {
            lesson_id: ("L3".to_string()).into(),
            rule_id: None,
            fault_class: Some("x".to_string().into()),
            repo_context: ("crates/foo".to_string()).into(),
            clean: (false).into(),
            source_surface: ("scan".to_string()).into(),
            ts: ("2026-07-04T00:00:00Z".to_string()).into(),
        },
    );
    let curve = recurrence_curve(&graph, &lesson_id("L3")?);
    assert_eq!(curve.len(), 1);
    assert!(
        !curve[0].since_landing.is_since_landing(),
        "no landing exists yet for L3"
    );
    assert_eq!(curve[0].running_recurrence_count, 0);
    Ok(())
}
use enforcer_domain::memory_types::{
    MemoryEvidenceLandedAt, MemoryEvidenceProofRef, MemoryLessonId,
};
