use enforcer_domain::ids::LaneId;
use enforcer_domain::plan_types::{
    LaneStatus, OrchestratorTickCount, PlanCondition, PlanDiagnosticDetail, PlanOwnershipPattern,
};
use enforcer_plan::error::PlanError;
use enforcer_plan::orchestrator::{
    worker_reuse_cap, CoordinationPort, GatekeeperHandoff, InMemoryCoordination, LaneEvent,
    Orchestrator, PlanGraph, RecordingWorktreeSpawner, ScriptedLiveness, TickOutcome, WorkpackNode,
    WorktreeSpawner,
};

fn diagnostic(raw: String) -> PlanDiagnosticDetail {
    let mut candidate = raw;
    loop {
        if let Ok(value) = PlanDiagnosticDetail::try_new(candidate) {
            return value;
        }
        candidate = "invalid test diagnostic".to_owned();
    }
}

#[derive(Default)]
struct GuardRejectingPort {
    events: Vec<LaneEvent>,
}

impl CoordinationPort for GuardRejectingPort {
    fn claim(
        &mut self,
        lane: &LaneId,
        owns: &[PlanOwnershipPattern],
    ) -> Result<PlanCondition, PlanError> {
        self.events.push(LaneEvent::Claimed {
            lane: lane.clone(),
            paths: owns.to_vec(),
        });
        Ok(PlanCondition::Satisfied)
    }

    fn guard(&mut self, lane: &LaneId) -> Result<(), PlanError> {
        Err(PlanError::GraphInvalid {
            reason: diagnostic(format!("ownership guard rejected `{lane}`")),
        })
    }

    fn closeout(&mut self, lane: &LaneId) -> Result<(), PlanError> {
        self.events
            .push(LaneEvent::ClosedOut { lane: lane.clone() });
        Ok(())
    }

    fn events(&self) -> &[LaneEvent] {
        &self.events
    }
}

#[derive(Default)]
struct SpawnRejectingWorktreeSpawner;

impl WorktreeSpawner for SpawnRejectingWorktreeSpawner {
    fn spawn(&mut self, lane: &LaneId) -> Result<(), PlanError> {
        Err(PlanError::GraphInvalid {
            reason: diagnostic(format!("worktree spawn rejected `{lane}`")),
        })
    }
}

fn lane(raw: &str) -> Result<LaneId, PlanError> {
    raw.parse().map_err(|error| PlanError::GraphInvalid {
        reason: diagnostic(format!("invalid test lane `{raw}`: {error}")),
    })
}

fn ownership(raw: &str) -> Result<PlanOwnershipPattern, PlanError> {
    raw.parse().map_err(|error| PlanError::GraphInvalid {
        reason: diagnostic(format!("invalid test ownership `{raw}`: {error}")),
    })
}

fn node(id: &str, deps: &[&str], owns: &[&str]) -> Result<WorkpackNode, PlanError> {
    Ok(WorkpackNode {
        id: lane(id)?,
        deps: deps
            .iter()
            .map(|value| lane(value))
            .collect::<Result<_, _>>()?,
        owns: owns
            .iter()
            .map(|value| ownership(value))
            .collect::<Result<_, _>>()?,
    })
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
    let mut orch = orchestrator(PlanGraph::from_nodes([node("a", &[], &["a.rs"])?]));
    orch.tick()?;
    orch.liveness_mut().set_status(lane("a")?, LaneStatus::Dead);
    orch.tick()?;

    let claim_count = orch
        .coordination_events()
        .iter()
        .filter(|event| matches!(event, LaneEvent::Claimed { lane, .. } if lane.as_str() == "a"))
        .count();
    assert_eq!(claim_count, 2);
    Ok(())
}

#[test]
fn tampered_done_claim_is_rejected_not_integrated() -> Result<(), PlanError> {
    let mut orch = orchestrator(PlanGraph::from_nodes([node("a", &[], &["a.rs"])?]));
    orch.tick()?;
    orch.liveness_mut()
        .set_status(lane("a")?, LaneStatus::ReportedDone);

    let result = orch.tick();
    assert!(
        matches!(result, Err(PlanError::DoneClaimRejected { lane, .. }) if lane.as_str() == "a")
    );
    assert!(!orch.done_workpacks().contains(&lane("a")?));
    Ok(())
}

#[test]
fn verified_done_claim_integrates_and_dispatches_new_frontier() -> Result<(), PlanError> {
    let graph = PlanGraph::from_nodes([node("a", &[], &["a.rs"])?, node("b", &["a"], &["b.rs"])?]);
    let mut orch = orchestrator(graph);
    orch.tick()?;
    assert_eq!(
        orch.dispatch_condition(&lane("a")?),
        PlanCondition::Satisfied
    );
    assert_eq!(
        orch.dispatch_condition(&lane("b")?),
        PlanCondition::Unsatisfied
    );

    orch.liveness_mut()
        .set_status(lane("a")?, LaneStatus::ReportedDone);
    orch.liveness_mut().mark_verifiable(lane("a")?);
    orch.tick()?;
    assert!(orch.done_workpacks().contains(&lane("a")?));
    assert_eq!(
        orch.dispatch_condition(&lane("b")?),
        PlanCondition::Satisfied
    );
    Ok(())
}

#[test]
fn stalled_frontier_exhausts_bounded_run() -> Result<(), PlanError> {
    let graph = PlanGraph::from_nodes([node("a", &["missing-dep"], &["a.rs"])?]);
    let mut orch = orchestrator(graph);
    let result = orch.run_until_done(OrchestratorTickCount::from_count(
        enforcer_domain::plan_types::PlanImportCount::from(3),
    ));
    assert!(matches!(result, Err(PlanError::GraphInvalid { .. })));
    Ok(())
}

#[test]
fn completed_plan_hands_off_to_gatekeeper() -> Result<(), PlanError> {
    let mut orch = orchestrator(PlanGraph::from_nodes([node("a", &[], &["a.rs"])?]));
    orch.liveness_mut().mark_verifiable(lane("a")?);
    orch.tick()?;
    orch.liveness_mut()
        .set_status(lane("a")?, LaneStatus::ReportedDone);

    assert_eq!(
        orch.tick()?,
        TickOutcome::Done(GatekeeperHandoff {
            done_workpacks: vec![lane("a")?],
        })
    );
    Ok(())
}

#[test]
fn worker_reuse_cap_is_bounded() {
    assert_eq!(usize::from(worker_reuse_cap()), 2);
}

#[test]
fn blocked_ownership_waits_for_retry_without_guarding_or_spawning_early() -> Result<(), PlanError> {
    let graph = PlanGraph::from_nodes([
        node("first", &[], &["shared.rs"])?,
        node("second", &[], &["shared.rs"])?,
    ]);
    let mut orch = orchestrator(graph);

    // The second claim is rejected by the live ownership port. That is an
    // expected fail-closed wait state, not an attempt to guard an unclaimed
    // lane and not a dispatch to a shared worktree.
    orch.tick()?;
    assert_eq!(
        orch.dispatch_condition(&lane("first")?),
        PlanCondition::Satisfied
    );
    assert_eq!(
        orch.dispatch_condition(&lane("second")?),
        PlanCondition::Unsatisfied
    );

    orch.liveness_mut()
        .set_status(lane("first")?, LaneStatus::ReportedDone);
    orch.liveness_mut().mark_verifiable(lane("first")?);
    orch.tick()?;
    assert_eq!(
        orch.dispatch_condition(&lane("second")?),
        PlanCondition::Unsatisfied
    );

    // A later tick retries only after closeout released the first claim.
    orch.tick()?;
    assert_eq!(
        orch.dispatch_condition(&lane("second")?),
        PlanCondition::Satisfied
    );
    Ok(())
}

#[test]
fn guard_rejection_never_spawns_a_worktree() -> Result<(), PlanError> {
    let graph = PlanGraph::from_nodes([node("lane-a", &[], &["a.rs"])?]);
    let mut orch = Orchestrator::new(
        graph,
        GuardRejectingPort::default(),
        ScriptedLiveness::new(),
        RecordingWorktreeSpawner::default(),
    );

    assert!(matches!(orch.tick(), Err(PlanError::GraphInvalid { .. })));
    assert!(orch.worktree_spawner().spawned.is_empty());
    assert_eq!(
        orch.dispatch_condition(&lane("lane-a")?),
        PlanCondition::Unsatisfied
    );
    assert!(matches!(
        orch.coordination_events().last(),
        Some(LaneEvent::ClosedOut { lane }) if lane.as_str() == "lane-a"
    ));
    Ok(())
}

#[test]
fn worktree_spawn_rejection_releases_the_claim() -> Result<(), PlanError> {
    let graph = PlanGraph::from_nodes([node("lane-a", &[], &["a.rs"])?]);
    let mut orch = Orchestrator::new(
        graph,
        InMemoryCoordination::new(),
        ScriptedLiveness::new(),
        SpawnRejectingWorktreeSpawner,
    );

    assert!(matches!(orch.tick(), Err(PlanError::GraphInvalid { .. })));
    assert_eq!(
        orch.dispatch_condition(&lane("lane-a")?),
        PlanCondition::Unsatisfied
    );
    assert!(matches!(
        orch.coordination_events(),
        [
            LaneEvent::Claimed { lane, .. },
            LaneEvent::Guarded { lane: guarded },
            LaneEvent::ClosedOut { lane: closed },
        ] if lane.as_str() == "lane-a" && guarded == lane && closed == lane
    ));
    Ok(())
}

#[test]
fn invalid_lane_id_is_rejected_before_claim_or_worktree_spawn() {
    assert!(matches!(
        "invalid!".parse::<LaneId>(),
        Err(error) if error.path == "laneId"
    ));
}
