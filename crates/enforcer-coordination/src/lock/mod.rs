//! Lock-kind taxonomy + the 6 conflict classes.
//!
//! Ported from `src/coordination/vendor/lock-policy.js`. This is the
//! load-bearing conflict-classification engine: given two enriched claims,
//! classify their relationship into one of `write-lock-conflict`,
//! `branch-write-conflict`, `global-write-conflict`, `branch-lease-conflict`
//! (hard conflicts), `merge-risk` (advisory, cross-branch), or
//! `work-reservation-overlap` (advisory, same-branch `workReservation`).

pub mod singletons;

use std::collections::BTreeSet;

use enforcer_domain::coordination_types::{
    ClaimComparisonKey, ClaimContextPresence, ClaimEventId, ClaimGroup, ClaimLane, ClaimPath,
    ClaimReason, ClaimWriter, ConflictType, CoordinationBranch, CoordinationMatch,
    CoordinationOwnerIdentity, CoordinationProjectId, CoordinationRepository, CoordinationWorktree,
    LockKind, Operation, ProtectedSingletonStatus,
};

use singletons::{normalize_coordination_path, protected_singleton_group};

use crate::error::Result;

/// Caller-supplied identity/environment context attached to a claim.
///
/// L2 finding: every field here MUST come from the CALLER's own
/// worktree/branch/commit resolution, never the coordination server's own
/// `cwd`. The API boundary (arc-16 `api.rs`) requires `project_id`,
/// `worktree_root`, and `branch` as explicit caller-supplied params for this
/// reason â€” this struct has no "resolve from server cwd" fallback baked in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaimContext {
    pub project_id: Option<CoordinationProjectId>,
    pub git_remote: Option<CoordinationRepository>,
    pub repo_root: Option<CoordinationRepository>,
    pub worktree_root: Option<CoordinationWorktree>,
    pub branch: Option<CoordinationBranch>,
    pub codex_thread_id: Option<CoordinationOwnerIdentity>,
    pub codex_session_id: Option<CoordinationOwnerIdentity>,
    /// Present only on a request (not an active claim) to force
    /// "explicit owner" comparisons â€” mirrors `requestHasExplicitOwner`.
    pub explicit_codex_thread_id: Option<CoordinationOwnerIdentity>,
    pub explicit_codex_session_id: Option<CoordinationOwnerIdentity>,
    pub claim_group: Option<ClaimGroup>,
    pub lock_kind: Option<LockKind>,
    pub operation: Option<Operation>,
}

/// A raw claim as recorded on an event, before enrichment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawClaim {
    pub writer: ClaimWriter,
    pub lane: ClaimLane,
    pub paths: Vec<ClaimPath>,
    pub event_id: ClaimEventId,
    pub reason: Option<ClaimReason>,
    pub context: ClaimContext,
}

/// An enriched claim carrying all derived comparison keys. Ported from
/// `lock-policy.js#enrichClaim`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrichedClaim {
    pub writer: ClaimWriter,
    pub lane: ClaimLane,
    pub paths: Vec<ClaimPath>,
    pub event_id: ClaimEventId,
    pub reason: Option<ClaimReason>,
    pub context: ClaimContext,
    pub lock_kind: LockKind,
    pub operation: Operation,
    pub claim_group: Option<ClaimGroup>,
    pub project_key: ClaimComparisonKey,
    pub repo_root_key: Option<ClaimComparisonKey>,
    pub git_remote_key: Option<ClaimComparisonKey>,
    pub worktree_key: ClaimComparisonKey,
    pub branch_key: ClaimComparisonKey,
    pub owner_key: ClaimComparisonKey,
    pub path_keys: Vec<ClaimComparisonKey>,
    pub global_keys: Vec<ClaimComparisonKey>,
    pub physical_keys: Vec<ClaimComparisonKey>,
    pub branch_keys: Vec<ClaimComparisonKey>,
    pub protected_singleton: ProtectedSingletonStatus,
}

fn normalize_key<T: std::fmt::Display + ?Sized>(value: &T) -> Result<ClaimComparisonKey> {
    let path = ClaimPath::from_display(&value)?;
    let normalized = normalize_coordination_path(&path)?;
    ClaimComparisonKey::from_display(&normalized).map_err(Into::into)
}

fn meaningful_owner_part(
    value: Option<&CoordinationOwnerIdentity>,
) -> Option<&CoordinationOwnerIdentity> {
    if value.is_none_or(|identity| identity.as_str().trim() == "unknown") {
        None
    } else {
        value
    }
}

fn logical_owner_key(writer: &ClaimWriter, context: &ClaimContext) -> Result<ClaimComparisonKey> {
    let suffix = meaningful_owner_part(context.codex_thread_id.as_ref())
        .or_else(|| meaningful_owner_part(context.codex_session_id.as_ref()))
        .map_or_else(|| writer.as_str(), CoordinationOwnerIdentity::as_str);
    normalize_key(&format!("{}:{suffix}", writer.as_str()))
}

fn unique<T: Ord>(values: Vec<T>) -> Vec<T> {
    BTreeSet::from_iter(values).into_iter().collect()
}

/// Enrich a raw claim with all derived comparison keys. Ported from
/// `lock-policy.js#enrichClaim`. `has_explicit_context` mirrors the JS
/// `hasContext = claim.context !== undefined` distinction, which changes the
/// default lock-kind fallback (`writeLock` vs `globalWriteLock`).
pub fn enrich_claim(
    claim: &RawClaim,
    context_presence: ClaimContextPresence,
) -> Result<EnrichedClaim> {
    let paths: Vec<ClaimPath> = claim
        .paths
        .iter()
        .map(normalize_coordination_path)
        .collect::<Result<_>>()?;
    let declared_lock_kind = match claim.context.lock_kind {
        Some(kind) => kind,
        None if context_presence == ClaimContextPresence::Explicit => LockKind::WriteLock,
        None => LockKind::GlobalWriteLock,
    };
    let singleton_groups: Vec<ClaimGroup> = paths
        .iter()
        .map(protected_singleton_group)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    let lock_kind =
        if declared_lock_kind == LockKind::GlobalWriteLock || !singleton_groups.is_empty() {
            LockKind::GlobalWriteLock
        } else {
            declared_lock_kind
        };
    let operation = claim.context.operation.unwrap_or(Operation::Edit);
    // CLONE-JUSTIFICATION: enriched claims are durable comparison snapshots;
    // they must not borrow the event projection that produced them.
    let claim_group = claim.context.claim_group.clone();
    let project_source = claim
        .context
        .project_id
        .as_ref()
        .map(CoordinationProjectId::as_str)
        .or_else(|| {
            claim
                .context
                .git_remote
                .as_ref()
                .map(CoordinationRepository::as_str)
        })
        .or_else(|| {
            claim
                .context
                .repo_root
                .as_ref()
                .map(CoordinationRepository::as_str)
        })
        .unwrap_or("legacy-unknown-project");
    let project_key = normalize_key(project_source)?;
    let repo_root_key = claim
        .context
        .repo_root
        .as_ref()
        .map(normalize_key)
        .transpose()?;
    let git_remote_key = claim
        .context
        .git_remote
        .as_ref()
        .map(normalize_key)
        .transpose()?;
    let worktree_source = claim
        .context
        .worktree_root
        .as_ref()
        .map(CoordinationWorktree::as_str)
        .or_else(|| {
            claim
                .context
                .repo_root
                .as_ref()
                .map(CoordinationRepository::as_str)
        })
        .unwrap_or("legacy-unknown-worktree");
    let worktree_key = normalize_key(worktree_source)?;
    let branch_key = normalize_key(
        claim
            .context
            .branch
            .as_ref()
            .map(CoordinationBranch::as_str)
            .unwrap_or("unknown-branch"),
    )?;
    let owner_key = logical_owner_key(&claim.writer, &claim.context)?;
    let path_keys: Vec<ClaimComparisonKey> = match &claim_group {
        Some(group) => vec![normalize_key(group)?],
        None => paths.iter().map(normalize_key).collect::<Result<_>>()?,
    };
    let global_keys = if lock_kind == LockKind::GlobalWriteLock {
        if !singleton_groups.is_empty() {
            unique(
                singleton_groups
                    .iter()
                    .map(|group| normalize_key(&format!("{project_key}:{group}")))
                    .collect::<Result<_>>()?,
            )
        } else {
            unique(
                path_keys
                    .iter()
                    .map(|path| normalize_key(&format!("{project_key}:{path}")))
                    .collect::<Result<_>>()?,
            )
        }
    } else {
        Vec::new()
    };
    let physical_keys = if lock_kind == LockKind::WriteLock {
        path_keys
            .iter()
            .map(|path| normalize_key(&format!("{project_key}:{worktree_key}:{path}")))
            .collect::<Result<_>>()?
    } else {
        Vec::new()
    };
    let branch_keys = if lock_kind == LockKind::BranchLease {
        vec![normalize_key(&format!("{project_key}:{branch_key}"))?]
    } else {
        path_keys
            .iter()
            .map(|path| normalize_key(&format!("{project_key}:{branch_key}:{path}")))
            .collect::<Result<_>>()?
    };
    // CLONE-JUSTIFICATION: this value is a self-contained ledger snapshot;
    // callers retain the raw claim while comparisons retain this projection.
    Ok(EnrichedClaim {
        writer: claim.writer.clone(),
        lane: claim.lane.clone(),
        paths,
        // CLONE-JUSTIFICATION: these event fields complete the independent
        // snapshot used after the raw event projection has been released.
        event_id: claim.event_id.clone(),
        reason: claim.reason.clone(),
        context: claim.context.clone(),
        lock_kind,
        operation,
        claim_group,
        project_key,
        repo_root_key,
        git_remote_key,
        worktree_key,
        branch_key,
        owner_key,
        path_keys,
        global_keys,
        physical_keys,
        branch_keys,
        protected_singleton: if singleton_groups.is_empty() {
            ProtectedSingletonStatus::Ordinary
        } else {
            ProtectedSingletonStatus::Protected
        },
    })
}

/// A classified conflict between two claims. Ported from
/// `lock-policy.js#conflict`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub kind: ConflictType,
    pub paths: Vec<ClaimPath>,
    pub lanes: [ClaimLane; 2],
    pub writers: [ClaimWriter; 2],
    pub event_ids: [ClaimEventId; 2],
}

fn overlapping(
    left: &[ClaimComparisonKey],
    right: &[ClaimComparisonKey],
) -> Vec<ClaimComparisonKey> {
    let right_set: BTreeSet<&ClaimComparisonKey> = right.iter().collect();
    unique(
        left.iter()
            .filter(|v| right_set.contains(v))
            .cloned()
            .collect(),
    )
}

fn paths_for_conflict(
    left: &EnrichedClaim,
    right: &EnrichedClaim,
    path_keys: &[ClaimComparisonKey],
) -> Vec<ClaimPath> {
    let normalized: BTreeSet<&str> = path_keys.iter().map(ClaimComparisonKey::as_str).collect();
    let combined: Vec<ClaimPath> = left
        .paths
        .iter()
        .chain(right.paths.iter())
        .filter(|entry| {
            if left.claim_group.is_some() || right.claim_group.is_some() {
                true
            } else {
                normalized.contains(entry.as_str())
            }
        })
        .cloned()
        .collect();
    if combined.is_empty() {
        unique(
            left.paths
                .iter()
                .chain(right.paths.iter())
                .cloned()
                .collect(),
        )
    } else {
        unique(combined)
    }
}

fn same_project(left: &EnrichedClaim, right: &EnrichedClaim) -> CoordinationMatch {
    if left.project_key == right.project_key {
        return CoordinationMatch::Matches;
    }
    if let (Some(l), Some(r)) = (&left.git_remote_key, &right.git_remote_key) {
        if l == r {
            return CoordinationMatch::Matches;
        }
    }
    if matches!((&left.repo_root_key, &right.repo_root_key), (Some(l), Some(r)) if l == r) {
        CoordinationMatch::Matches
    } else {
        CoordinationMatch::Differs
    }
}

fn same_worktree(left: &EnrichedClaim, right: &EnrichedClaim) -> CoordinationMatch {
    if left.worktree_key == right.worktree_key {
        CoordinationMatch::Matches
    } else {
        CoordinationMatch::Differs
    }
}

fn same_branch(left: &EnrichedClaim, right: &EnrichedClaim) -> CoordinationMatch {
    if left.branch_key == right.branch_key {
        CoordinationMatch::Matches
    } else {
        CoordinationMatch::Differs
    }
}

/// Two claims share a logical owner (same codex thread/session within the
/// same lane, or same derived owner key) â€” such pairs never conflict with
/// each other. Ported from `lock-policy.js#sameLogicalOwner`.
pub fn same_logical_owner(left: &EnrichedClaim, right: &EnrichedClaim) -> CoordinationMatch {
    let left_thread = meaningful_owner_part(left.context.codex_thread_id.as_ref());
    let right_thread = meaningful_owner_part(right.context.codex_thread_id.as_ref());
    if left_thread.is_some() && left_thread == right_thread && left.lane == right.lane {
        return CoordinationMatch::Matches;
    }
    let left_session = meaningful_owner_part(left.context.codex_session_id.as_ref());
    let right_session = meaningful_owner_part(right.context.codex_session_id.as_ref());
    if left_session.is_some() && left_session == right_session && left.lane == right.lane {
        return CoordinationMatch::Matches;
    }
    if left.owner_key == right.owner_key {
        CoordinationMatch::Matches
    } else {
        CoordinationMatch::Differs
    }
}

/// The classification result for a pair of claims. Ported from
/// `lock-policy.js#classifyClaimPair`'s per-pair accumulator shape.
#[derive(Debug, Clone, Default)]
pub struct PairConflicts {
    pub write_conflicts: Vec<Conflict>,
    pub branch_write_conflicts: Vec<Conflict>,
    pub global_write_conflicts: Vec<Conflict>,
    pub branch_lease_conflicts: Vec<Conflict>,
    pub merge_risks: Vec<Conflict>,
    pub advisories: Vec<Conflict>,
}

fn make_conflict(
    kind: ConflictType,
    left: &EnrichedClaim,
    right: &EnrichedClaim,
    paths: Vec<ClaimPath>,
) -> Conflict {
    // CLONE-JUSTIFICATION: conflicts outlive the borrowed pair and expose
    // both actor identities in the durable coordination decision.
    Conflict {
        kind,
        paths,
        // CLONE-JUSTIFICATION: the result stores both sides of a conflict
        // after the borrowed claim pair has gone out of scope.
        lanes: [left.lane.clone(), right.lane.clone()],
        writers: [left.writer.clone(), right.writer.clone()],
        event_ids: [left.event_id.clone(), right.event_id.clone()],
    }
}

/// Classify one pair of enriched claims into the 6 conflict classes. Ported
/// from `lock-policy.js#classifyClaimPair` â€” control flow and precedence
/// order preserved exactly (global > same-path-same-worktree write >
/// physical write > branch-lease > branch-write > merge-risk >
/// work-reservation advisory).
pub fn classify_claim_pair(left: &EnrichedClaim, right: &EnrichedClaim) -> PairConflicts {
    let global_overlap = overlapping(&left.global_keys, &right.global_keys);
    let physical_overlap = overlapping(&left.physical_keys, &right.physical_keys);
    let branch_lease_overlap =
        if left.lock_kind == LockKind::BranchLease && right.lock_kind == LockKind::BranchLease {
            overlapping(&left.branch_keys, &right.branch_keys)
        } else {
            Vec::new()
        };
    let same_path = overlapping(&left.path_keys, &right.path_keys);
    let common_paths = paths_for_conflict(left, right, &same_path);

    let mut result = PairConflicts::default();

    if !global_overlap.is_empty() {
        result.global_write_conflicts.push(make_conflict(
            ConflictType::GlobalWriteConflict,
            left,
            right,
            common_paths,
        ));
        return result;
    }
    if !same_path.is_empty()
        && matches!(same_project(left, right), CoordinationMatch::Matches)
        && matches!(same_worktree(left, right), CoordinationMatch::Matches)
    {
        result.write_conflicts.push(make_conflict(
            ConflictType::WriteLockConflict,
            left,
            right,
            common_paths,
        ));
        return result;
    }
    if !physical_overlap.is_empty() {
        result.write_conflicts.push(make_conflict(
            ConflictType::WriteLockConflict,
            left,
            right,
            common_paths,
        ));
        return result;
    }
    if !branch_lease_overlap.is_empty() {
        result.branch_lease_conflicts.push(make_conflict(
            ConflictType::BranchLeaseConflict,
            left,
            right,
            common_paths,
        ));
        return result;
    }
    if same_path.is_empty() || !matches!(same_project(left, right), CoordinationMatch::Matches) {
        return result;
    }
    if matches!(same_branch(left, right), CoordinationMatch::Matches)
        && !matches!(same_worktree(left, right), CoordinationMatch::Matches)
    {
        result.branch_write_conflicts.push(make_conflict(
            ConflictType::BranchWriteConflict,
            left,
            right,
            common_paths,
        ));
        return result;
    }
    if !matches!(same_branch(left, right), CoordinationMatch::Matches) {
        result.merge_risks.push(make_conflict(
            ConflictType::MergeRisk,
            left,
            right,
            common_paths,
        ));
        return result;
    }
    if left.lock_kind == LockKind::WorkReservation || right.lock_kind == LockKind::WorkReservation {
        result.advisories.push(make_conflict(
            ConflictType::WorkReservationOverlap,
            left,
            right,
            common_paths,
        ));
    }
    result
}

/// The decision for a claim request against currently-active claims. Ported
/// from `lock-policy.js#blockersForRequest`.
#[derive(Debug, Clone, Default)]
pub struct RequestDecision {
    pub operation: Operation,
    pub hard_conflicts: Vec<Conflict>,
    pub merge_risks: Vec<Conflict>,
    pub advisories: Vec<Conflict>,
    pub blockers: Vec<Conflict>,
}

fn request_has_explicit_owner(context: &ClaimContext) -> CoordinationMatch {
    if meaningful_owner_part(context.explicit_codex_thread_id.as_ref()).is_some()
        || meaningful_owner_part(context.explicit_codex_session_id.as_ref()).is_some()
    {
        CoordinationMatch::Matches
    } else {
        CoordinationMatch::Differs
    }
}

/// Compute the blockers for a claim request against the set of currently
/// active (already-enriched) claims. Ported from
/// `lock-policy.js#blockersForRequest`.
pub fn blockers_for_request(
    active_claims: &[EnrichedClaim],
    request: &EnrichedClaim,
    operation: Operation,
) -> RequestDecision {
    let mut hard_conflicts = Vec::new();
    let mut merge_risks = Vec::new();
    let mut advisories = Vec::new();

    for active in active_claims {
        if matches!(
            same_logical_owner(active, request),
            CoordinationMatch::Matches
        ) {
            continue;
        }
        if !matches!(
            request_has_explicit_owner(&request.context),
            CoordinationMatch::Matches
        ) && active.lane == request.lane
        {
            continue;
        }
        let conflicts = classify_claim_pair(active, request);
        hard_conflicts.extend(conflicts.write_conflicts);
        hard_conflicts.extend(conflicts.branch_write_conflicts);
        hard_conflicts.extend(conflicts.global_write_conflicts);
        hard_conflicts.extend(conflicts.branch_lease_conflicts);
        merge_risks.extend(conflicts.merge_risks);
        advisories.extend(conflicts.advisories);
    }

    // CLONE-JUSTIFICATION: a request decision retains the complete hard set
    // and, for most operations, also owns that same set as its blockers.
    let blockers = match operation {
        Operation::Inspect => Vec::new(),
        Operation::Push => hard_conflicts
            .iter()
            .filter(|c| c.kind == ConflictType::BranchLeaseConflict)
            .cloned()
            .collect(),
        Operation::PrReady => hard_conflicts
            .iter()
            .cloned()
            .chain(merge_risks.iter().cloned())
            .collect(),
        // CLONE-JUSTIFICATION: both decision fields intentionally retain the
        // complete hard-conflict set for caller diagnostics.
        _ => hard_conflicts.clone(),
    };

    RequestDecision {
        operation,
        hard_conflicts,
        merge_risks,
        advisories,
        blockers,
    }
}

/// Two normalized paths "overlap" if equal or one is a directory-prefix of
/// the other. Ported from `lock-policy.js#pathOverlaps`.
pub fn path_overlaps(left: &ClaimPath, right: &ClaimPath) -> CoordinationMatch {
    if left == right
        || left.as_str().starts_with(&format!("{right}/"))
        || right.as_str().starts_with(&format!("{left}/"))
    {
        CoordinationMatch::Matches
    } else {
        CoordinationMatch::Differs
    }
}

/// Does a conflict touch any of the given changed paths? Ported from
/// `lock-policy.js#conflictTouchesPaths`.
pub fn conflict_touches_paths(
    conflict: &Conflict,
    changed_paths: &[ClaimPath],
) -> Result<CoordinationMatch> {
    let normalized: Vec<ClaimPath> = changed_paths
        .iter()
        .map(normalize_coordination_path)
        .collect::<Result<_>>()?;
    let touches = conflict.paths.iter().any(|conflict_path| {
        normalized.iter().any(|changed| {
            matches!(
                path_overlaps(changed, conflict_path),
                CoordinationMatch::Matches
            )
        })
    });
    Ok(if touches {
        CoordinationMatch::Matches
    } else {
        CoordinationMatch::Differs
    })
}
