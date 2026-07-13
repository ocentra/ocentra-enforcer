//! Public lock lifecycle contract tests.
//!
//! These assertions exercise conflict handling from outside the implementation
//! module, so callers own the same lifecycle guarantees as the lock engine.

use enforcer_coordination::lock::{
    blockers_for_request, enrich_claim, ClaimContext, ConflictType, Operation, RawClaim,
};

fn context(worktree: &str, branch: &str) -> ClaimContext {
    ClaimContext {
        project_id: Some("test-project".to_owned()),
        worktree_root: Some(worktree.to_owned()),
        branch: Some(branch.to_owned()),
        ..Default::default()
    }
}

fn claim(writer: &str, lane: &str, context: ClaimContext) -> RawClaim {
    RawClaim {
        writer: writer.to_owned(),
        lane: lane.to_owned(),
        paths: vec!["crates/example/src/lib.rs".to_owned()],
        event_id: format!("event-{writer}"),
        reason: None,
        context,
    }
}

#[test]
fn lock_lifecycle_distinguishes_active_write_blockers_from_merge_review() {
    let active = enrich_claim(&claim("writer-a", "lane-a", context("worktree-a", "feature-a")), true);
    let request = enrich_claim(&claim("writer-b", "lane-b", context("worktree-b", "feature-b")), true);

    let edit = blockers_for_request(&[active.clone()], &request, Operation::Edit);
    let pr_ready = blockers_for_request(&[active], &request, Operation::PrReady);

    assert!(edit.blockers.is_empty());
    assert_eq!(edit.merge_risks.len(), 1);
    assert_eq!(pr_ready.blockers.len(), 1);
    assert_eq!(pr_ready.blockers[0].kind, ConflictType::MergeRisk);
}
