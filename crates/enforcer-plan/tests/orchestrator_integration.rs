use std::collections::HashSet;

use enforcer_plan::error::PlanError;
use enforcer_plan::orchestrator::{
    pack_lanes, CoordinationPort, InMemoryCoordination, IntentQueue, LaneEvent, PlanGraph,
    WorkpackNode,
};

fn workpack(id: &str, deps: &[&str], owns: &[&str]) -> WorkpackNode {
    WorkpackNode {
        id: id.to_owned(),
        deps: deps.iter().map(|value| (*value).to_owned()).collect(),
        owns: owns.iter().map(|value| (*value).to_owned()).collect(),
    }
}

#[test]
fn frontier_advances_only_after_all_dependencies_complete() {
    let graph = PlanGraph::from_nodes([
        workpack("prepare", &[], &["prepare.rs"]),
        workpack("validate", &["prepare"], &["validate.rs"]),
    ]);
    let initial = graph.frontier(&HashSet::new());
    assert_eq!(initial, vec!["prepare".to_owned()]);

    let complete = HashSet::from(["prepare".to_owned()]);
    assert_eq!(graph.frontier(&complete), vec!["validate".to_owned()]);
}

#[test]
fn cyclic_dependencies_are_detected_before_dispatch() {
    let graph = PlanGraph::from_nodes([
        workpack("a", &["b"], &["a.rs"]),
        workpack("b", &["a"], &["b.rs"]),
    ]);
    assert_eq!(
        graph.find_cycle(),
        Some(vec!["a".to_owned(), "b".to_owned(), "a".to_owned()])
    );
}

#[test]
fn overlapping_ownership_is_split_into_separate_batches() -> Result<(), Box<dyn std::error::Error>>
{
    let graph = PlanGraph::from_nodes([
        workpack("first", &[], &["crates/enforcer-plan/src/orchestrator.rs"]),
        workpack("second", &[], &["crates/enforcer-plan/src/orchestrator.rs"]),
    ]);
    let batches = pack_lanes(&graph, &["first".to_owned(), "second".to_owned()])?;
    assert_eq!(batches.len(), 2);
    assert!(batches.iter().all(|batch| batch.len() == 1));
    Ok(())
}

#[test]
fn disjoint_ownership_stays_in_one_batch() -> Result<(), Box<dyn std::error::Error>> {
    let graph = PlanGraph::from_nodes([
        workpack("first", &[], &["first.rs"]),
        workpack("second", &[], &["second.rs"]),
    ]);
    let batches = pack_lanes(&graph, &["first".to_owned(), "second".to_owned()])?;
    assert_eq!(batches, vec![vec!["first".to_owned(), "second".to_owned()]]);
    Ok(())
}

#[test]
fn coordination_records_claim_guard_closeout_and_rejects_unclaimed_guard(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut coordination = InMemoryCoordination::new();
    let paths = ["orchestrator.rs".to_owned()];
    assert!(coordination.claim("lane-a", &paths)?);
    coordination.guard("lane-a")?;
    coordination.closeout("lane-a")?;
    assert_eq!(
        coordination.events(),
        [
            LaneEvent::Claimed {
                lane: "lane-a".to_owned(),
                paths: paths.to_vec(),
            },
            LaneEvent::Guarded {
                lane: "lane-a".to_owned(),
            },
            LaneEvent::ClosedOut {
                lane: "lane-a".to_owned(),
            },
        ]
    );
    match coordination.guard("unknown") {
        Err(PlanError::GraphInvalid { reason }) => {
            assert_eq!(reason, "guard called for lane `unknown` with no held claim");
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
    let paths = ["shared.rs".to_owned()];

    assert!(queue.try_claim_or_queue(&mut coordination, "lane-a", &paths)?);
    assert!(!queue.try_claim_or_queue(&mut coordination, "lane-b", &paths)?);
    assert_eq!(queue.pending().len(), 1);

    coordination.closeout("lane-a")?;
    assert_eq!(
        queue.drain_retry(&mut coordination)?,
        vec!["lane-b".to_owned()]
    );
    assert_eq!(
        coordination.events().last(),
        Some(&LaneEvent::Claimed {
            lane: "lane-b".to_owned(),
            paths: paths.to_vec(),
        })
    );
    Ok(())
}
