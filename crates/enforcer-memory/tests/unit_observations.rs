use enforcer_memory::graph::MemoryGraph;
use enforcer_memory::observations::{
    procedural_success_rate, record_procedural, record_route_choice, ProceduralOutcome,
};

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
