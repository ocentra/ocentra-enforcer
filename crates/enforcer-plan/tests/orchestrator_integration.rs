use std::collections::HashSet;

use enforcer_domain::ids::LaneId;
use enforcer_domain::plan_types::{PlanCondition, PlanOwnershipPattern};
use enforcer_plan::error::PlanError;
use enforcer_plan::orchestrator::{
    pack_lanes, CoordinationPort, InMemoryCoordination, IntentQueue, LaneEvent, PlanGraph,
    WorkpackNode,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn lane(raw: &str) -> TestResult<LaneId> {
    Ok(raw.parse()?)
}

fn ownership(raw: &str) -> TestResult<PlanOwnershipPattern> {
    Ok(raw.parse()?)
}

fn workpack(id: &str, deps: &[&str], owns: &[&str]) -> TestResult<WorkpackNode> {
    Ok(WorkpackNode {
        id: lane(id)?,
        deps: deps
            .iter()
            .map(|value| lane(value))
            .collect::<TestResult<Vec<_>>>()?,
        owns: owns
            .iter()
            .map(|value| ownership(value))
            .collect::<TestResult<Vec<_>>>()?,
    })
}

#[test]
fn frontier_advances_only_after_all_dependencies_complete() -> TestResult {
    let graph = PlanGraph::from_nodes([
        workpack("prepare", &[], &["prepare.rs"])?,
        workpack("validate", &["prepare"], &["validate.rs"])?,
    ]);
    let initial = graph.frontier(&HashSet::new());
    assert_eq!(initial, vec![lane("prepare")?]);

    let complete = HashSet::from([lane("prepare")?]);
    assert_eq!(graph.frontier(&complete), vec![lane("validate")?]);
    Ok(())
}

#[test]
fn cyclic_dependencies_are_detected_before_dispatch() -> TestResult {
    let graph = PlanGraph::from_nodes([
        workpack("a", &["b"], &["a.rs"])?,
        workpack("b", &["a"], &["b.rs"])?,
    ]);
    assert_eq!(
        graph.find_cycle(),
        Some(vec![lane("a")?, lane("b")?, lane("a")?])
    );
    Ok(())
}

#[test]
fn overlapping_ownership_is_split_into_separate_batches() -> Result<(), Box<dyn std::error::Error>>
{
    let graph = PlanGraph::from_nodes([
        workpack("first", &[], &["crates/enforcer-plan/src/orchestrator.rs"])?,
        workpack("second", &[], &["crates/enforcer-plan/src/orchestrator.rs"])?,
    ]);
    let batches = pack_lanes(&graph, &[lane("first")?, lane("second")?])?;
    assert_eq!(batches.len(), 2);
    assert!(batches.iter().all(|batch| usize::from(batch.len()) == 1));
    Ok(())
}

#[test]
fn disjoint_ownership_stays_in_one_batch() -> Result<(), Box<dyn std::error::Error>> {
    let graph = PlanGraph::from_nodes([
        workpack("first", &[], &["first.rs"])?,
        workpack("second", &[], &["second.rs"])?,
    ]);
    let batches = pack_lanes(&graph, &[lane("first")?, lane("second")?])?;
    assert_eq!(
        batches[0].iter().cloned().collect::<Vec<_>>(),
        vec![lane("first")?, lane("second")?]
    );
    Ok(())
}

#[test]
fn coordination_records_claim_guard_closeout_and_rejects_unclaimed_guard(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut coordination = InMemoryCoordination::new();
    let lane_a = lane("lane-a")?;
    let paths = [ownership("orchestrator.rs")?];
    assert_eq!(
        coordination.claim(&lane_a, &paths)?,
        PlanCondition::Satisfied
    );
    coordination.guard(&lane_a)?;
    coordination.closeout(&lane_a)?;
    assert_eq!(
        coordination.events(),
        [
            LaneEvent::Claimed {
                lane: lane_a.clone(),
                paths: paths.to_vec(),
            },
            LaneEvent::Guarded {
                lane: lane_a.clone(),
            },
            LaneEvent::ClosedOut { lane: lane_a },
        ]
    );
    match coordination.guard(&lane("unknown")?) {
        Err(PlanError::GraphInvalid { reason }) => {
            assert_eq!(
                reason.as_str(),
                "guard called for lane `unknown` with no held claim"
            );
        }
        Err(other) => return Err(format!("unexpected guard error: {other}").into()),
        Ok(()) => return Err("unclaimed guard unexpectedly succeeded".into()),
    }
    Ok(())
}

#[test]
fn ownership_blocks_overlap_then_retries_after_closeout() -> Result<(), Box<dyn std::error::Error>>
{
    let mut coordination = InMemoryCoordination::new();
    let mut queue = IntentQueue::new();
    let lane_a = lane("lane-a")?;
    let lane_b = lane("lane-b")?;
    let paths = [ownership("shared.rs")?];

    assert_eq!(
        queue.try_claim_or_queue(&mut coordination, &lane_a, &paths)?,
        PlanCondition::Satisfied
    );
    assert_eq!(
        queue.try_claim_or_queue(&mut coordination, &lane_b, &paths)?,
        PlanCondition::Unsatisfied
    );
    assert_eq!(queue.pending().len(), 1);

    coordination.closeout(&lane_a)?;
    assert_eq!(queue.drain_retry(&mut coordination)?, vec![lane_b.clone()]);
    assert_eq!(
        coordination.events().last(),
        Some(&LaneEvent::Claimed {
            lane: lane_b,
            paths: paths.to_vec(),
        })
    );
    Ok(())
}
