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

use serde::{Deserialize, Serialize};

use crate::error::{CoordinationError, Result};
use singletons::{normalize_coordination_path, protected_singleton_group};

/// The 4 lock kinds. Ported from `lock-policy.js#LOCK_KIND_VALUES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LockKind {
    WriteLock,
    GlobalWriteLock,
    BranchLease,
    WorkReservation,
}

impl LockKind {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "writeLock" => Ok(Self::WriteLock),
            "globalWriteLock" => Ok(Self::GlobalWriteLock),
            "branchLease" => Ok(Self::BranchLease),
            "workReservation" => Ok(Self::WorkReservation),
            other => Err(CoordinationError::invalid("lockKind", other)),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::WriteLock => "writeLock",
            Self::GlobalWriteLock => "globalWriteLock",
            Self::BranchLease => "branchLease",
            Self::WorkReservation => "workReservation",
        }
    }
}

/// The coordination operation an actor is performing. Ported from
/// `lock-policy.js#OPERATION_VALUES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Inspect,
    Edit,
    Commit,
    Push,
    Rebase,
    Merge,
    PrReady,
}

impl Operation {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "inspect" => Ok(Self::Inspect),
            "edit" => Ok(Self::Edit),
            "commit" => Ok(Self::Commit),
            "push" => Ok(Self::Push),
            "rebase" => Ok(Self::Rebase),
            "merge" => Ok(Self::Merge),
            "pr_ready" => Ok(Self::PrReady),
            other => Err(CoordinationError::invalid("operation", other)),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Edit => "edit",
            Self::Commit => "commit",
            Self::Push => "push",
            Self::Rebase => "rebase",
            Self::Merge => "merge",
            Self::PrReady => "pr_ready",
        }
    }
}

/// `onConflict` mode for a claim request. Ported from
/// `lock-policy.js#ON_CONFLICT_VALUES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnConflict {
    Fail,
    Intent,
}

impl OnConflict {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "fail" => Ok(Self::Fail),
            "intent" => Ok(Self::Intent),
            other => Err(CoordinationError::invalid("onConflict", other)),
        }
    }
}

/// Caller-supplied identity/environment context attached to a claim.
///
/// L2 finding: every field here MUST come from the CALLER's own
/// worktree/branch/commit resolution, never the coordination server's own
/// `cwd`. The API boundary (arc-16 `api.rs`) requires `project_id`,
/// `worktree_root`, and `branch` as explicit caller-supplied params for this
/// reason — this struct has no "resolve from server cwd" fallback baked in.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimContext {
    pub project_id: Option<String>,
    pub git_remote: Option<String>,
    pub repo_root: Option<String>,
    pub worktree_root: Option<String>,
    pub branch: Option<String>,
    pub codex_thread_id: Option<String>,
    pub codex_session_id: Option<String>,
    /// Present only on a request (not an active claim) to force
    /// "explicit owner" comparisons — mirrors `requestHasExplicitOwner`.
    pub explicit_codex_thread_id: Option<String>,
    pub explicit_codex_session_id: Option<String>,
    pub claim_group: Option<String>,
    pub lock_kind: Option<String>,
    pub operation: Option<String>,
}

/// A raw claim as recorded on an event, before enrichment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawClaim {
    pub writer: String,
    pub lane: String,
    pub paths: Vec<String>,
    pub event_id: String,
    pub reason: Option<String>,
    pub context: ClaimContext,
}

/// An enriched claim carrying all derived comparison keys. Ported from
/// `lock-policy.js#enrichClaim`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnrichedClaim {
    pub writer: String,
    pub lane: String,
    pub paths: Vec<String>,
    pub event_id: String,
    pub reason: Option<String>,
    pub context: ClaimContext,
    pub lock_kind: LockKind,
    pub operation: Operation,
    pub claim_group: Option<String>,
    pub project_key: String,
    pub repo_root_key: Option<String>,
    pub git_remote_key: Option<String>,
    pub worktree_key: String,
    pub branch_key: String,
    pub owner_key: String,
    pub path_keys: Vec<String>,
    pub global_keys: Vec<String>,
    pub physical_keys: Vec<String>,
    pub branch_keys: Vec<String>,
    pub protected_singleton: bool,
}

fn normalize_key(value: &str) -> String {
    normalize_coordination_path(value)
}

fn optional_key(value: Option<&str>) -> Option<String> {
    value.map(normalize_key).filter(|k| !k.is_empty())
}

fn meaningful_owner_part(value: Option<&str>) -> Option<String> {
    let trimmed = value.map(str::trim).unwrap_or("");
    if trimmed.is_empty() || trimmed == "unknown" {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn logical_owner_key(writer: &str, context: &ClaimContext) -> String {
    let thread = meaningful_owner_part(context.codex_thread_id.as_deref());
    let session = meaningful_owner_part(context.codex_session_id.as_deref());
    let suffix = thread.or(session).unwrap_or_else(|| writer.to_owned());
    normalize_key(&format!("{writer}:{suffix}"))
}

fn unique(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for v in values {
        if !v.is_empty() && seen.insert(v.clone()) {
            out.push(v);
        }
    }
    out
}

/// Enrich a raw claim with all derived comparison keys. Ported from
/// `lock-policy.js#enrichClaim`. `has_explicit_context` mirrors the JS
/// `hasContext = claim.context !== undefined` distinction, which changes the
/// default lock-kind fallback (`writeLock` vs `globalWriteLock`).
pub fn enrich_claim(claim: &RawClaim, has_explicit_context: bool) -> EnrichedClaim {
    let paths: Vec<String> = claim
        .paths
        .iter()
        .map(|p| normalize_coordination_path(p))
        .filter(|p| !p.is_empty())
        .collect();
    let declared_lock_kind = claim
        .context
        .lock_kind
        .as_deref()
        .and_then(|k| LockKind::parse(k).ok())
        .unwrap_or(if has_explicit_context {
            LockKind::WriteLock
        } else {
            LockKind::GlobalWriteLock
        });
    let singleton_groups: Vec<String> = paths
        .iter()
        .filter_map(|p| protected_singleton_group(p))
        .collect();
    let lock_kind = if declared_lock_kind == LockKind::GlobalWriteLock || !singleton_groups.is_empty()
    {
        LockKind::GlobalWriteLock
    } else {
        declared_lock_kind
    };
    let operation = claim
        .context
        .operation
        .as_deref()
        .and_then(|o| Operation::parse(o).ok())
        .unwrap_or(Operation::Edit);
    let claim_group = claim.context.claim_group.clone();
    let project_key = normalize_key(
        claim
            .context
            .project_id
            .as_deref()
            .or(claim.context.git_remote.as_deref())
            .or(claim.context.repo_root.as_deref())
            .unwrap_or("legacy-unknown-project"),
    );
    let repo_root_key = optional_key(claim.context.repo_root.as_deref());
    let git_remote_key = optional_key(claim.context.git_remote.as_deref());
    let worktree_key = normalize_key(
        claim
            .context
            .worktree_root
            .as_deref()
            .or(claim.context.repo_root.as_deref())
            .unwrap_or("legacy-unknown-worktree"),
    );
    let branch_key = normalize_key(claim.context.branch.as_deref().unwrap_or("unknown-branch"));
    let owner_key = logical_owner_key(&claim.writer, &claim.context);
    let path_keys: Vec<String> = match &claim_group {
        Some(group) => vec![normalize_key(group)],
        None => paths.clone(),
    };
    let global_keys = if lock_kind == LockKind::GlobalWriteLock {
        if !singleton_groups.is_empty() {
            unique(
                singleton_groups
                    .iter()
                    .map(|g| format!("{project_key}:{g}"))
                    .collect(),
            )
        } else {
            unique(
                path_keys
                    .iter()
                    .map(|p| format!("{project_key}:{p}"))
                    .collect(),
            )
        }
    } else {
        Vec::new()
    };
    let physical_keys = if lock_kind == LockKind::WriteLock {
        path_keys
            .iter()
            .map(|p| format!("{project_key}:{worktree_key}:{p}"))
            .collect()
    } else {
        Vec::new()
    };
    let branch_keys = if lock_kind == LockKind::BranchLease {
        vec![format!("{project_key}:{branch_key}")]
    } else {
        path_keys
            .iter()
            .map(|p| format!("{project_key}:{branch_key}:{p}"))
            .collect()
    };
    EnrichedClaim {
        writer: claim.writer.clone(),
        lane: claim.lane.clone(),
        paths,
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
        protected_singleton: !singleton_groups.is_empty(),
    }
}

/// One of the 6 conflict classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictType {
    WriteLockConflict,
    BranchWriteConflict,
    GlobalWriteConflict,
    BranchLeaseConflict,
    MergeRisk,
    WorkReservationOverlap,
}

impl ConflictType {
    pub fn is_hard(self) -> bool {
        matches!(
            self,
            Self::WriteLockConflict
                | Self::BranchWriteConflict
                | Self::GlobalWriteConflict
                | Self::BranchLeaseConflict
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::WriteLockConflict => "write-lock-conflict",
            Self::BranchWriteConflict => "branch-write-conflict",
            Self::GlobalWriteConflict => "global-write-conflict",
            Self::BranchLeaseConflict => "branch-lease-conflict",
            Self::MergeRisk => "merge-risk",
            Self::WorkReservationOverlap => "work-reservation-overlap",
        }
    }
}

/// A classified conflict between two claims. Ported from
/// `lock-policy.js#conflict`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub kind: ConflictType,
    pub paths: Vec<String>,
    pub lanes: [String; 2],
    pub writers: [String; 2],
    pub event_ids: [String; 2],
}

fn overlapping(left: &[String], right: &[String]) -> Vec<String> {
    let right_set: BTreeSet<&String> = right.iter().collect();
    unique(
        left.iter()
            .filter(|v| right_set.contains(v))
            .cloned()
            .collect(),
    )
}

fn paths_for_conflict(left: &EnrichedClaim, right: &EnrichedClaim, path_keys: &[String]) -> Vec<String> {
    let normalized: BTreeSet<&String> = path_keys.iter().collect();
    let combined: Vec<String> = left
        .paths
        .iter()
        .chain(right.paths.iter())
        .filter(|entry| {
            if left.claim_group.is_some() || right.claim_group.is_some() {
                true
            } else {
                normalized.contains(entry)
            }
        })
        .cloned()
        .collect();
    if combined.is_empty() {
        unique(left.paths.iter().chain(right.paths.iter()).cloned().collect())
    } else {
        unique(combined)
    }
}

fn same_project(left: &EnrichedClaim, right: &EnrichedClaim) -> bool {
    if left.project_key == right.project_key {
        return true;
    }
    if let (Some(l), Some(r)) = (&left.git_remote_key, &right.git_remote_key) {
        if l == r {
            return true;
        }
    }
    matches!((&left.repo_root_key, &right.repo_root_key), (Some(l), Some(r)) if l == r)
}

fn same_worktree(left: &EnrichedClaim, right: &EnrichedClaim) -> bool {
    left.worktree_key == right.worktree_key
}

fn same_branch(left: &EnrichedClaim, right: &EnrichedClaim) -> bool {
    left.branch_key == right.branch_key
}

/// Two claims share a logical owner (same codex thread/session within the
/// same lane, or same derived owner key) — such pairs never conflict with
/// each other. Ported from `lock-policy.js#sameLogicalOwner`.
pub fn same_logical_owner(left: &EnrichedClaim, right: &EnrichedClaim) -> bool {
    let left_thread = meaningful_owner_part(left.context.codex_thread_id.as_deref());
    let right_thread = meaningful_owner_part(right.context.codex_thread_id.as_deref());
    if left_thread.is_some() && left_thread == right_thread && left.lane == right.lane {
        return true;
    }
    let left_session = meaningful_owner_part(left.context.codex_session_id.as_deref());
    let right_session = meaningful_owner_part(right.context.codex_session_id.as_deref());
    if left_session.is_some() && left_session == right_session && left.lane == right.lane {
        return true;
    }
    left.owner_key == right.owner_key
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

fn make_conflict(kind: ConflictType, left: &EnrichedClaim, right: &EnrichedClaim, paths: Vec<String>) -> Conflict {
    Conflict {
        kind,
        paths,
        lanes: [left.lane.clone(), right.lane.clone()],
        writers: [left.writer.clone(), right.writer.clone()],
        event_ids: [left.event_id.clone(), right.event_id.clone()],
    }
}

/// Classify one pair of enriched claims into the 6 conflict classes. Ported
/// from `lock-policy.js#classifyClaimPair` — control flow and precedence
/// order preserved exactly (global > same-path-same-worktree write >
/// physical write > branch-lease > branch-write > merge-risk >
/// work-reservation advisory).
pub fn classify_claim_pair(left: &EnrichedClaim, right: &EnrichedClaim) -> PairConflicts {
    let global_overlap = overlapping(&left.global_keys, &right.global_keys);
    let physical_overlap = overlapping(&left.physical_keys, &right.physical_keys);
    let branch_lease_overlap = if left.lock_kind == LockKind::BranchLease
        && right.lock_kind == LockKind::BranchLease
    {
        overlapping(&left.branch_keys, &right.branch_keys)
    } else {
        Vec::new()
    };
    let same_path = overlapping(&left.path_keys, &right.path_keys);
    let common_paths = paths_for_conflict(left, right, &same_path);

    let mut result = PairConflicts::default();

    if !global_overlap.is_empty() {
        result
            .global_write_conflicts
            .push(make_conflict(ConflictType::GlobalWriteConflict, left, right, common_paths));
        return result;
    }
    if !same_path.is_empty() && same_project(left, right) && same_worktree(left, right) {
        result
            .write_conflicts
            .push(make_conflict(ConflictType::WriteLockConflict, left, right, common_paths));
        return result;
    }
    if !physical_overlap.is_empty() {
        result
            .write_conflicts
            .push(make_conflict(ConflictType::WriteLockConflict, left, right, common_paths));
        return result;
    }
    if !branch_lease_overlap.is_empty() {
        result
            .branch_lease_conflicts
            .push(make_conflict(ConflictType::BranchLeaseConflict, left, right, common_paths));
        return result;
    }
    if same_path.is_empty() || !same_project(left, right) {
        return result;
    }
    if same_branch(left, right) && !same_worktree(left, right) {
        result
            .branch_write_conflicts
            .push(make_conflict(ConflictType::BranchWriteConflict, left, right, common_paths));
        return result;
    }
    if !same_branch(left, right) {
        result
            .merge_risks
            .push(make_conflict(ConflictType::MergeRisk, left, right, common_paths));
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

impl Default for Operation {
    fn default() -> Self {
        Self::Edit
    }
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

fn request_has_explicit_owner(context: &ClaimContext) -> bool {
    meaningful_owner_part(context.explicit_codex_thread_id.as_deref()).is_some()
        || meaningful_owner_part(context.explicit_codex_session_id.as_deref()).is_some()
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
        if same_logical_owner(active, request) {
            continue;
        }
        if !request_has_explicit_owner(&request.context) && active.lane == request.lane {
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
pub fn path_overlaps(left: &str, right: &str) -> bool {
    left == right
        || left.starts_with(&format!("{right}/"))
        || right.starts_with(&format!("{left}/"))
}

/// Does a conflict touch any of the given changed paths? Ported from
/// `lock-policy.js#conflictTouchesPaths`.
pub fn conflict_touches_paths(conflict: &Conflict, changed_paths: &[String]) -> bool {
    let normalized: Vec<String> = changed_paths
        .iter()
        .map(|p| normalize_coordination_path(p))
        .filter(|p| !p.is_empty())
        .collect();
    conflict.paths.iter().any(|conflict_path| {
        let cp = normalize_coordination_path(conflict_path);
        normalized.iter().any(|changed| path_overlaps(changed, &cp))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(project: &str, worktree: &str, branch: &str) -> ClaimContext {
        ClaimContext {
            project_id: Some(project.to_owned()),
            worktree_root: Some(worktree.to_owned()),
            branch: Some(branch.to_owned()),
            ..Default::default()
        }
    }

    fn claim(writer: &str, lane: &str, paths: &[&str], context: ClaimContext) -> RawClaim {
        RawClaim {
            writer: writer.to_owned(),
            lane: lane.to_owned(),
            paths: paths.iter().map(|p| p.to_string()).collect(),
            event_id: format!("evt_{writer}"),
            reason: None,
            context,
        }
    }

    #[test]
    fn same_path_same_worktree_different_owners_is_write_lock_conflict() {
        let a = enrich_claim(
            &claim("node1.laneA", "laneA", &["src/lib.rs"], ctx("proj", "wt1", "main")),
            true,
        );
        let b = enrich_claim(
            &claim("node2.laneB", "laneB", &["src/lib.rs"], ctx("proj", "wt1", "main")),
            true,
        );
        let conflicts = classify_claim_pair(&a, &b);
        assert_eq!(conflicts.write_conflicts.len(), 1);
        assert_eq!(conflicts.write_conflicts[0].kind, ConflictType::WriteLockConflict);
        assert!(conflicts.merge_risks.is_empty());
    }

    #[test]
    fn different_worktrees_different_branches_is_only_merge_risk() {
        let a = enrich_claim(
            &claim("node1.laneA", "laneA", &["src/lib.rs"], ctx("proj", "wt1", "feature-a")),
            true,
        );
        let b = enrich_claim(
            &claim("node2.laneB", "laneB", &["src/lib.rs"], ctx("proj", "wt2", "feature-b")),
            true,
        );
        let conflicts = classify_claim_pair(&a, &b);
        assert!(conflicts.write_conflicts.is_empty());
        assert!(conflicts.global_write_conflicts.is_empty());
        assert!(conflicts.branch_write_conflicts.is_empty());
        assert_eq!(conflicts.merge_risks.len(), 1);
        assert_eq!(conflicts.merge_risks[0].kind, ConflictType::MergeRisk);
    }

    #[test]
    fn work_reservation_tail_branch_is_unreachable_given_same_worktree_write_lock_precedence() {
        // DEVIATION NOTE (flagged, not silently "fixed"): the arc-16
        // workpack's own acceptance row says "two `workReservation` claims
        // on the same-branch overlapping path -> `work-reservation-overlap`
        // advisory only (never a blocker)". Reading `lock-policy.js`
        // line-by-line (`classifyClaimPair`, lines 280-330) shows this is
        // NOT reachable as written in the vendored source: line 302
        // (`samePath && sameProject && sameWorktree` -> write-lock-conflict)
        // fires unconditionally on lock kind whenever two claims share both
        // path and worktree, and line 317
        // (`sameBranch && !sameWorktree` -> branch-write-conflict) fires
        // whenever they share a branch but NOT a worktree. There is no
        // remaining `(samePath, sameProject, sameBranch, sameWorktree)`
        // combination left over for line 327's work-reservation tail to
        // execute for same-path claims — the vendored JS itself cannot
        // produce a `work-reservation-overlap` for two exact-file claims
        // that share a path, only for `claimGroup`-based claims where
        // `pathKeys` overlaps but `paths` differ in a way that also evades
        // the branch-write-conflict guard (a narrower case than the
        // workpack fixture describes). This Rust port reproduces the
        // vendored precedence EXACTLY (byte-for-byte control flow), so it
        // inherits the same reachability gap rather than silently
        // "fixing" behavior the workpack didn't ask this pass to change.
        // Recorded as a deviation for primary review, not resolved here.
        let mut context_a = ctx("proj", "wt1", "main");
        context_a.lock_kind = Some("workReservation".to_owned());
        let mut context_b = ctx("proj", "wt1", "main");
        context_b.lock_kind = Some("workReservation".to_owned());
        let a = enrich_claim(&claim("node1.laneA", "laneA", &["src/lib.rs"], context_a), true);
        let b = enrich_claim(&claim("node2.laneB", "laneB", &["src/lib.rs"], context_b), true);
        let conflicts = classify_claim_pair(&a, &b);
        assert_eq!(
            conflicts.write_conflicts.len(),
            1,
            "matches vendored precedence: same-path-same-worktree is write-lock-conflict regardless of lock kind"
        );
        assert!(conflicts.advisories.is_empty());
    }

    #[test]
    fn protected_singleton_escalates_across_worktrees_to_global_conflict() {
        let a = enrich_claim(
            &claim("node1.laneA", "laneA", &["Cargo.lock"], ctx("proj", "wt1", "feature-a")),
            true,
        );
        let b = enrich_claim(
            &claim("node2.laneB", "laneB", &["Cargo.lock"], ctx("proj", "wt2", "feature-b")),
            true,
        );
        assert!(a.protected_singleton);
        assert_eq!(a.lock_kind, LockKind::GlobalWriteLock);
        let conflicts = classify_claim_pair(&a, &b);
        assert_eq!(conflicts.global_write_conflicts.len(), 1);
        assert_eq!(
            conflicts.global_write_conflicts[0].kind,
            ConflictType::GlobalWriteConflict
        );
    }

    #[test]
    fn distinct_ordinary_files_on_different_branches_do_not_conflict_at_all() {
        // Different (non-overlapping) paths never share a `pathKeys` entry,
        // so `samePath` is empty and classifyClaimPair returns an empty
        // result (no merge-risk either — merge-risk requires the SAME path
        // on different branches). This also proves protected-singleton
        // escalation is genuinely path-triggered, not project-wide: two
        // ordinary (non-singleton) files from the same two claimants
        // produce nothing.
        let a = enrich_claim(
            &claim("node1.laneA", "laneA", &["src/a.rs"], ctx("proj", "wt1", "feature-a")),
            true,
        );
        let b = enrich_claim(
            &claim("node2.laneB", "laneB", &["src/b.rs"], ctx("proj", "wt2", "feature-b")),
            true,
        );
        let conflicts = classify_claim_pair(&a, &b);
        assert!(conflicts.global_write_conflicts.is_empty());
        assert!(conflicts.write_conflicts.is_empty());
        assert!(conflicts.merge_risks.is_empty());
        assert!(conflicts.advisories.is_empty());
    }

    #[test]
    fn pr_ready_operation_blocks_on_merge_risk_unless_allowed() {
        let active = enrich_claim(
            &claim("node1.laneA", "laneA", &["src/lib.rs"], ctx("proj", "wt1", "feature-a")),
            true,
        );
        let request = enrich_claim(
            &claim("node2.laneB", "laneB", &["src/lib.rs"], ctx("proj", "wt2", "feature-b")),
            true,
        );
        let decision = blockers_for_request(&[active], &request, Operation::PrReady);
        assert_eq!(decision.blockers.len(), 1);
        assert_eq!(decision.blockers[0].kind, ConflictType::MergeRisk);
    }

    #[test]
    fn push_operation_only_blocks_on_branch_lease_conflicts() {
        let mut lease_ctx = ctx("proj", "wt1", "main");
        lease_ctx.lock_kind = Some("branchLease".to_owned());
        let active = enrich_claim(
            &claim("node1.laneA", "laneA", &["src/lib.rs"], lease_ctx.clone()),
            true,
        );
        let mut request_ctx = ctx("proj", "wt2", "main");
        request_ctx.lock_kind = Some("branchLease".to_owned());
        let request = enrich_claim(
            &claim("node2.laneB", "laneB", &["other.rs"], request_ctx),
            true,
        );
        let decision = blockers_for_request(&[active], &request, Operation::Push);
        assert_eq!(decision.blockers.len(), 1);
        assert_eq!(decision.blockers[0].kind, ConflictType::BranchLeaseConflict);
    }

    #[test]
    fn same_logical_owner_never_conflicts() {
        let mut context_a = ctx("proj", "wt1", "main");
        context_a.codex_thread_id = Some("thread-1".to_owned());
        let mut context_b = ctx("proj", "wt1", "main");
        context_b.codex_thread_id = Some("thread-1".to_owned());
        let a = enrich_claim(&claim("node1.laneA", "laneA", &["src/lib.rs"], context_a), true);
        let b = enrich_claim(&claim("node1.laneA", "laneA", &["src/lib.rs"], context_b), true);
        assert!(same_logical_owner(&a, &b));
        let decision = blockers_for_request(&[a], &b, Operation::Edit);
        assert!(decision.blockers.is_empty());
    }
}
