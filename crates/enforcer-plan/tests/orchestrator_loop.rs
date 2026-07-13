use enforcer_plan::error::PlanError;
use enforcer_plan::orchestrator::{
    GatekeeperHandoff, InMemoryCoordination, LaneEvent, LaneStatus, Orchestrator, PlanGraph,
    RecordingWorktreeSpawner, ScriptedLiveness, TickOutcome, WorkpackNode, WORKER_REUSE_CAP,
};

fn node(id: &str, deps: &[&str], owns: &[&str]) -> WorkpackNode {
    WorkpackNode {
        id: id.to_owned(),
        deps: deps.iter().map(|value| (*value).to_owned()).collect(),
        owns: owns.iter().map(|value| (*value).to_owned()).collect(),
    }
}

fn orchestrator(
    graph: PlanGraph,
) -> Orchestrator<InMemoryCoordination, ScriptedLiveness, RecordingWorktreeSpawner> {
    Orchestrator::new(
        graph,
        InMemoryCoordination::new(),
        ScriptedLiveness::new(),
        RecordingWorktreeSpawner::default(),
    )
}

#[test]
fn dead_lane_is_detected_and_respawned() -> Result<(), PlanError> {
    let mut orch = orchestrator(PlanGraph::from_nodes([node("a", &[], &["a.rs"])]));
    orch.tick()?;
    orch.liveness_mut().set_status("a", LaneStatus::Dead);
    orch.tick()?;

    let claim_count = orch
        .coordination_events()
        .iter()
        .filter(|event| matches!(event, LaneEvent::Claimed { lane, .. } if lane == "a"))
        .count();
    assert_eq!(claim_count, 2);
    Ok(())
}

#[test]
fn tampered_done_claim_is_rejected_not_integrated() -> Result<(), PlanError> {
    let mut orch = orchestrator(PlanGraph::from_nodes([node("a", &[], &["a.rs"])]));
    orch.tick()?;
    orch.liveness_mut().set_status("a", LaneStatus::ReportedDone);

    let result = orch.tick();
    assert!(matches!(result, Err(PlanError::DoneClaimRejected { lane, .. }) if lane == "a"));
    assert!(!orch.done_workpacks().contains("a"));
    Ok(())
}

#[test]
fn verified_done_claim_integrates_and_dispatches_new_frontier() -> Result<(), PlanError> {
    let graph = PlanGraph::from_nodes([
        node("a", &[], &["a.rs"]),
        node("b", &["a"], &["b.rs"]),
    ]);
    let mut orch = orchestrator(graph);
    orch.tick()?;
    assert!(orch.is_dispatched("a"));
    assert!(!orch.is_dispatched("b"));

    orch.liveness_mut().set_status("a", LaneStatus::ReportedDone);
    orch.liveness_mut().mark_verifiable("a");
    orch.tick()?;
    assert!(orch.done_workpacks().contains("a"));
    assert!(orch.is_dispatched("b"));
    Ok(())
}

#[test]
fn stalled_frontier_exhausts_bounded_run() {
    let graph = PlanGraph::from_nodes([node("a", &["missing-dep"], &["a.rs"])]);
    let mut orch = orchestrator(graph);
    let result = orch.run_until_done(3);
    assert!(matches!(result, Err(PlanError::GraphInvalid { .. })));
}

#[test]
fn completed_plan_hands_off_to_gatekeeper() -> Result<(), PlanError> {
    let mut orch = orchestrator(PlanGraph::from_nodes([node("a", &[], &["a.rs"])]));
    orch.liveness_mut().mark_verifiable("a");
    orch.tick()?;
    orch.liveness_mut().set_status("a", LaneStatus::ReportedDone);

    assert_eq!(
        orch.tick()?,
        TickOutcome::Done(GatekeeperHandoff {
            done_workpacks: vec!["a".to_owned()],
        })
    );
    Ok(())
}

#[test]
fn worker_reuse_cap_is_bounded() {
    assert_eq!(WORKER_REUSE_CAP, 2);
}

#[test]
fn blocked_ownership_waits_for_retry_without_guarding_or_spawning_early() -> Result<(), PlanError> {
    let graph = PlanGraph::from_nodes([
        node("first", &[], &["shared.rs"]),
        node("second", &[], &["shared.rs"]),
    ]);
    let mut orch = orchestrator(graph);

    // The second claim is rejected by the live ownership port. That is an
    // expected fail-closed wait state, not an attempt to guard an unclaimed
    // lane and not a dispatch to a shared worktree.
    orch.tick()?;
    assert!(orch.is_dispatched("first"));
    assert!(!orch.is_dispatched("second"));

    orch.liveness_mut()
        .set_status("first", LaneStatus::ReportedDone);
    orch.liveness_mut().mark_verifiable("first");
    orch.tick()?;
    assert!(!orch.is_dispatched("second"));

    // A later tick retries only after closeout released the first claim.
    orch.tick()?;
    assert!(orch.is_dispatched("second"));
    Ok(())
}
