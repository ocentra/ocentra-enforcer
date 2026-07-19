//! Public lock lifecycle contract tests.
//!
//! These assertions exercise conflict handling from outside the implementation
//! module, so callers own the same lifecycle guarantees as the lock engine.

use enforcer_coordination::lock::{blockers_for_request, enrich_claim, ClaimContext, RawClaim};
use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::coordination_types::{
    ClaimContextPresence, ClaimEventId, ClaimLane, ClaimPath, ClaimWriter, ConflictType,
    CoordinationBranch, CoordinationProjectId, CoordinationWorktree, LockKind, OnConflict,
    Operation,
};

#[test]
fn lock_domain_values_cross_the_boundary_through_explicit_wire_conversion(
) -> Result<(), Box<dyn std::error::Error>> {
    let lock_kind = LockKind::parse("branchLease")?;
    let operation = Operation::parse("pr_ready")?;
    let policy = OnConflict::parse("intent")?;

    assert_eq!(lock_kind.as_str(), "branchLease");
    assert_eq!(operation.as_str(), "pr_ready");
    assert_eq!(policy, OnConflict::Intent);
    assert_eq!(
        LockKind::parse("unknown-lock"),
        Err(DecodeError::new(
            "lockKind",
            "expected writeLock, globalWriteLock, branchLease, or workReservation",
        ))
    );
    assert_eq!(
        Operation::parse("unknown-operation"),
        Err(DecodeError::new(
            "operation",
            "expected inspect, edit, commit, push, rebase, merge, or pr_ready",
        ))
    );
    assert_eq!(
        OnConflict::parse("unknown-policy"),
        Err(DecodeError::new("onConflict", "expected fail or intent"))
    );
    Ok(())
}

fn context(worktree: &str, branch: &str) -> Result<ClaimContext, Box<dyn std::error::Error>> {
    Ok(ClaimContext {
        project_id: Some(CoordinationProjectId::parse("test-project")?),
        worktree_root: Some(CoordinationWorktree::parse(worktree)?),
        branch: Some(CoordinationBranch::parse(branch)?),
        ..Default::default()
    })
}

fn claim(
    writer: &str,
    lane: &str,
    context: ClaimContext,
) -> Result<RawClaim, Box<dyn std::error::Error>> {
    Ok(RawClaim {
        writer: ClaimWriter::try_from(writer.to_owned())?,
        lane: ClaimLane::try_from(lane.to_owned())?,
        paths: vec![ClaimPath::parse("crates/example/src/lib.rs")?],
        event_id: ClaimEventId::try_from(format!("event-{writer}"))?,
        reason: None,
        context,
    })
}

#[test]
fn typed_claim_identity_preserves_event_values_and_conflict_lifecycle(
) -> Result<(), Box<dyn std::error::Error>> {
    let writer = ClaimWriter::try_from("node-a.lane-a".to_owned())?;
    let lane = ClaimLane::try_from("lane-a".to_owned())?;
    let event_id = ClaimEventId::try_from("evt-a".to_owned())?;

    assert_eq!(writer.to_string(), "node-a.lane-a");
    assert_eq!(lane.to_string(), "lane-a");
    assert_eq!(event_id.to_string(), "evt-a");

    let active = enrich_claim(
        &claim(
            "node-a.lane-a",
            "lane-a",
            context("worktree-a", "feature-a")?,
        )?,
        ClaimContextPresence::Explicit,
    )?;
    let request = enrich_claim(
        &claim(
            "node-b.lane-b",
            "lane-b",
            context("worktree-b", "feature-b")?,
        )?,
        ClaimContextPresence::Explicit,
    )?;
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
    Ok(())
}

#[test]
fn lock_lifecycle_distinguishes_active_write_blockers_from_merge_review(
) -> Result<(), Box<dyn std::error::Error>> {
    let active = enrich_claim(
        &claim("writer-a", "lane-a", context("worktree-a", "feature-a")?)?,
        ClaimContextPresence::Explicit,
    )?;
    let request = enrich_claim(
        &claim("writer-b", "lane-b", context("worktree-b", "feature-b")?)?,
        ClaimContextPresence::Explicit,
    )?;

    let edit = blockers_for_request(std::slice::from_ref(&active), &request, Operation::Edit);
    let pr_ready = blockers_for_request(&[active], &request, Operation::PrReady);

    assert!(edit.blockers.is_empty());
    assert_eq!(edit.merge_risks.len(), 1);
    assert_eq!(pr_ready.blockers.len(), 1);
    assert_eq!(pr_ready.blockers[0].kind, ConflictType::MergeRisk);
    Ok(())
}

#[test]
fn malformed_persisted_lock_values_are_rejected_at_the_typed_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        LockKind::parse("unknown-lock"),
        Err(DecodeError::new(
            "lockKind",
            "expected writeLock, globalWriteLock, branchLease, or workReservation",
        ))
    );
    assert_eq!(
        Operation::parse("unknown-operation"),
        Err(DecodeError::new(
            "operation",
            "expected inspect, edit, commit, push, rebase, merge, or pr_ready",
        ))
    );
    Ok(())
}
