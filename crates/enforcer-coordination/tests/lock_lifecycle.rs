//! Public lock lifecycle contract tests.
//!
//! These assertions exercise conflict handling from outside the implementation
//! module, so callers own the same lifecycle guarantees as the lock engine.

use enforcer_coordination::lock::{
    blockers_for_request, enrich_claim, ClaimContext, ConflictType, LockKind, OnConflict,
    Operation, RawClaim,
};

#[test]
fn lock_domain_values_cross_the_boundary_through_explicit_wire_conversion() {
    let lock_kind = LockKind::parse("branchLease").expect("known lock kind must decode");
    let operation = Operation::parse("pr_ready").expect("known operation must decode");
    let policy = OnConflict::parse("intent").expect("known conflict policy must decode");

    assert_eq!(lock_kind.as_str(), "branchLease");
    assert_eq!(operation.as_str(), "pr_ready");
    assert_eq!(policy, OnConflict::Intent);
    assert!(LockKind::parse("unknown-lock").is_err());
    assert!(Operation::parse("unknown-operation").is_err());
    assert!(OnConflict::parse("unknown-policy").is_err());
}

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

#[test]
fn malformed_persisted_lock_values_fail_closed_without_weakening_the_claim() {
    let mut malformed = context("worktree-a", "feature-a");
    malformed.lock_kind = Some("unknown-lock".to_owned());
    malformed.operation = Some("unknown-operation".to_owned());

    let enriched = enrich_claim(&claim("writer-a", "lane-a", malformed), true);

    assert_eq!(enriched.lock_kind, LockKind::GlobalWriteLock);
    assert_eq!(enriched.operation, Operation::Edit);
}
