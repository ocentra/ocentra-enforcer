//! Public lock lifecycle contract tests.
//!
//! These assertions exercise conflict handling from outside the implementation
//! module, so callers own the same lifecycle guarantees as the lock engine.

use enforcer_coordination::lock::{
    blockers_for_request, enrich_claim, ClaimContext, ClaimEventId, ClaimLane, ClaimWriter,
    ConflictType, LockKind, OnConflict, Operation, RawClaim,
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
        writer: ClaimWriter::from(writer.to_owned()),
        lane: ClaimLane::from(lane.to_owned()),
        paths: vec!["crates/example/src/lib.rs".to_owned()],
        event_id: ClaimEventId::from(format!("event-{writer}")),
        reason: None,
        context,
    }
}

#[test]
fn typed_claim_identity_preserves_event_values_and_conflict_lifecycle() {
    let writer = ClaimWriter::from("node-a.lane-a".to_owned());
    let lane = ClaimLane::from("lane-a".to_owned());
    let event_id = ClaimEventId::from("evt-a".to_owned());

    assert_eq!(writer.to_string(), "node-a.lane-a");
    assert_eq!(lane.to_string(), "lane-a");
    assert_eq!(event_id.to_string(), "evt-a");

    let active = enrich_claim(
        &claim(
            "node-a.lane-a",
            "lane-a",
            context("worktree-a", "feature-a"),
        ),
        true,
    );
    let request = enrich_claim(
        &claim(
            "node-b.lane-b",
            "lane-b",
            context("worktree-b", "feature-b"),
        ),
        true,
    );
    let decision = blockers_for_request(&[active], &request, Operation::PrReady);
    if let Some(conflict) = decision.blockers.first() {
        assert_eq!(conflict.kind, ConflictType::MergeRisk);
        assert_eq!(conflict.writers[0].to_string(), "node-a.lane-a");
        assert_eq!(conflict.writers[1].to_string(), "node-b.lane-b");
        assert_eq!(conflict.lanes[0].to_string(), "lane-a");
        assert_eq!(conflict.event_ids[1].to_string(), "event-node-b.lane-b");
    } else {
        assert_eq!(
            decision.blockers.len(),
            1,
            "cross-branch overlap must remain reviewable"
        );
    }
}

#[test]
fn lock_lifecycle_distinguishes_active_write_blockers_from_merge_review() {
    let active = enrich_claim(
        &claim("writer-a", "lane-a", context("worktree-a", "feature-a")),
        true,
    );
    let request = enrich_claim(
        &claim("writer-b", "lane-b", context("worktree-b", "feature-b")),
        true,
    );

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
