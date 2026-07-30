//! Public command surface: init/message/ack/claim/release/closeout.
//!
//! Ported (narrowed) from `src/coordination/api.mjs`. Three live dogfood
//! findings (`docs/plans/enforcer-selfhost-plan/refs/orchestration-lessons.md`
//! L1/L2/L13) are REQUIREMENTS baked into this module, not incidental
//! behavior:
//!
//! - **L1 (idempotent init):** `init` on an existing identity returns the
//!   existing `HubConfig` rather than propagating a raw
//!   already-exists/`EEXIST`-style IO error. The vendored JS used
//!   `writeFile(..., { flag: "wx" })`, which throws `EEXIST` on a second call
//!   — `init` here checks for an existing identity FIRST and returns it.
//! - **L2 (caller identity required):** `ClaimRequest`/`ReleaseRequest` take
//!   an explicit `CallerContext` (worktree root, branch, commit, project id)
//!   as REQUIRED constructor input. There is no "resolve from the server's
//!   own cwd" fallback anywhere in this module — every event's `context`
//!   reflects what the CALLER passed in, matching the fix direction the
//!   lesson names ("caller identity should be required claim params").
//! - **L13 (glob/dir owns-sets batch transparently):** `normalize_owns_paths`
//!   accepts glob patterns and directory prefixes and expands them against
//!   the repo's tracked files, then `claim_all` transparently splits the
//!   expanded file list into batches of at most `MAX_CLAIM_PATHS` and issues
//!   one claim event per batch — callers never see the cap or do manual
//!   splitting themselves.

use std::path::Path;

use enforcer_domain::ids::{HubName, LaneId};
use enforcer_domain::scan_types::CommitRef;

use crate::domain::{self, HubConfig};
use crate::error::{CoordinationError, Result};
use crate::events::boundary::HubEventResponse;
use crate::ledger::active_claims;
use crate::lock::{blockers_for_request, enrich_claim, ClaimContext, RawClaim};
use crate::sync::{retention, stream::read_all_streams};
use enforcer_domain::coordination_types::{
    ClaimContextPresence, ClaimEventId, ClaimFilterMatch, ClaimGroup, ClaimLane,
    ClaimOutcomeStatus, ClaimPath, ClaimReason, ClaimWriter, CloseoutLaneScope,
    CompactionKeepCount, CoordinationBranch, CoordinationEventKind, CoordinationLedgerRoot,
    CoordinationMessageBody, CoordinationOwnerIdentity, CoordinationProjectId,
    CoordinationRejection, CoordinationRepoRoot, CoordinationRepository, CoordinationWorktree,
    LockKind, NodeId, NodeName, Operation, WriterId,
};

mod boundary;

use boundary::{
    append_event, decode_hub_config, encode_hub_config, now_iso, AppendEventArgs, EventContextRefs,
    EventMetadata,
};

/// Maximum exact file paths per SINGLE claim event, preserved from the
/// vendored JS `claim-policy.js#MAX_CLAIM_PATHS` for wire/event-shape
/// compatibility. Unlike the JS source (which REJECTS a request over 10
/// paths with a raw error — the L13 finding), this crate treats it as an
/// implementation batching unit: `claim_all` transparently issues multiple claim
/// events instead of forcing the caller to split.
pub const MAX_CLAIM_PATHS: usize = 10;

/// Caller-supplied identity/environment context (L2). Every field is
/// required or explicitly optional-with-no-server-side-resolution; there is
/// deliberately no method that reads `std::env::current_dir()` or spawns
/// `git` here on the caller's behalf — a thin CLI/MCP wrapper is expected to
/// gather these from the CALLING agent's own worktree, not the coordination
/// server process's cwd.
#[derive(Debug, Clone)]
pub struct CallerContext {
    pub project_id: CoordinationProjectId,
    pub worktree_root: CoordinationWorktree,
    pub branch: CoordinationBranch,
    pub commit: Option<CommitRef>,
    pub codex_thread_id: Option<CoordinationOwnerIdentity>,
    pub codex_session_id: Option<CoordinationOwnerIdentity>,
}
impl CallerContext {
    fn into_claim_context(self, extra: ClaimContextExtras) -> Result<ClaimContext> {
        Ok(ClaimContext {
            project_id: Some(self.project_id),
            git_remote: None,
            // CLONE-JUSTIFICATION: transport context carries both independently owned path fields.
            repo_root: Some(CoordinationRepository::from(&self.worktree_root)),
            worktree_root: Some(self.worktree_root),
            branch: Some(self.branch),
            codex_thread_id: self.codex_thread_id,
            codex_session_id: self.codex_session_id,
            explicit_codex_thread_id: None,
            explicit_codex_session_id: None,
            claim_group: extra.claim_group,
            lock_kind: extra.lock_kind,
            operation: extra.operation,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct ClaimContextExtras {
    claim_group: Option<ClaimGroup>,
    lock_kind: Option<LockKind>,
    operation: Option<Operation>,
}

/// Coordination hub handle: root path + this node's loaded identity.
#[derive(Debug)]
pub struct Hub {
    pub root: CoordinationLedgerRoot,
    pub config: HubConfig,
}

/// Open an existing hub identity without creating or replacing it.
///
/// Desktop and service callers use this when a human explicitly dispatches
/// against a configured ledger. Unlike [`init`], this never creates state.
pub fn open(root: CoordinationLedgerRoot) -> Result<Hub> {
    let config = load_identity(root.as_path())?;
    Ok(Hub { root, config })
}

/// Compact an already-authorized ledger without creating an identity or
/// accepting an untyped retention value at the transport boundary.
///
/// The caller must obtain a [`Hub`] through [`open`] first. This keeps a
/// compaction request from turning a misspelled or absent ledger root into a
/// newly initialized authority as a side effect.
pub fn compact(hub: &Hub, keep_latest: CompactionKeepCount) -> Result<retention::CompactionResult> {
    retention::compact_ledger(&hub.root, keep_latest)
}

/// L1: idempotent init. If an identity already exists at `root`, it is
/// loaded and returned as-is (never a raw filesystem "already exists"
/// error, and never silently re-created with different values). If none
/// exists, a fresh identity is created and persisted.
pub fn init(root: &Path, hub: &HubName, lane: &LaneId) -> Result<HubConfig> {
    let identity_path = domain::boundary::identity_path(root);
    if identity_path.exists() {
        return load_identity(root);
    }
    let node_id = NodeId::random();
    let node_name = hostname_or_fallback();
    let config = HubConfig {
        // CLONE-JUSTIFICATION: configuration owns the hub while the caller retains it for initialization.
        hub: hub.clone(),
        node_id,
        node_name,
        default_lane: lane.clone(),
        created_at: now_iso()?,
    };
    std::fs::create_dir_all(domain::boundary::identity_dir(root))?;
    // Exclusive create: two concurrent FIRST inits still race safely — the
    // loser observes AlreadyExists and falls back to loading what the
    // winner wrote, which is itself the L1 idempotency guarantee extended
    // to the concurrent case.
    let bytes = encode_hub_config(&config)?;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&identity_path)
    {
        Ok(mut handle) => {
            use std::io::Write;
            handle.write_all(&bytes)?;
            handle.write_all(b"\n")?;
            Ok(config)
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => load_identity(root),
        Err(err) => Err(err.into()),
    }
}

/// Load a previously-initialized identity from disk.
pub fn load_identity(root: &Path) -> Result<HubConfig> {
    let raw = std::fs::read_to_string(domain::boundary::identity_path(root))?;
    decode_hub_config(&raw)
}

fn hostname_or_fallback() -> NodeName {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .map_or_else(
            |_| NodeName::sanitize_hostname("unknown-host"),
            |raw| NodeName::sanitize_hostname(&raw),
        )
}

/// L13: normalize an `owns:` entry, which may be an exact file, a glob
/// pattern (`crates/foo/**`), or a bare directory prefix (`crates/foo/`),
/// into a list of exact tracked-file paths. Directories are expanded to
/// every regular file beneath them; globs are expanded via the `glob` crate.
/// Files that don't exist on disk (not-yet-created targets of a fresh
/// workpack) are passed through unexpanded as a literal single-file entry,
/// since a workpack's `owns:` set legitimately includes files a lane is
/// about to CREATE.
pub fn normalize_owns_paths(
    repo_root: &CoordinationRepoRoot,
    entries: &[ClaimPath],
) -> Result<Vec<ClaimPath>> {
    boundary::normalize_owns_paths(repo_root, entries)
}

// Not created yet — keep the literal directory prefix as a
// No matches yet (e.g. a fresh crate not yet created) — keep
// Directory doesn't exist yet — nothing to expand; skip

/// The outcome of one claim ATTEMPT (either a single event, or a blocked
/// decision requiring the caller to escalate/queue).
#[derive(Debug, Clone)]
pub struct ClaimOutcome {
    pub status: ClaimOutcomeStatus,
    pub events: Vec<HubEventResponse>,
    pub blockers: Vec<crate::lock::Conflict>,
}

/// Request parameters for [`claim_all`], bundled to keep the function's
/// argument count within the workspace's `too_many_arguments` lint budget.
/// Every field is itself a reference/`Copy` value, so this struct is `Copy`
/// and passed by value cheaply (no allocation, no clone).
#[derive(Debug, Clone, Copy)]
pub struct ClaimRequestArgs<'a> {
    pub repo_root: &'a CoordinationRepoRoot,
    pub lane: &'a LaneId,
    pub owns: &'a [ClaimPath],
    pub caller: &'a CallerContext,
    pub reason: Option<&'a ClaimReason>,
}

/// L13: claim an arbitrary `owns:`-shaped path list (exact files, dirs, or
/// globs) against a hub, transparently expanding and batching into
/// `MAX_CLAIM_PATHS`-sized claim events. Returns one `HubEvent` per batch on
/// success; on the FIRST conflicting batch, stops and reports blockers
/// (partial claims already appended are the caller's responsibility to
/// release if they choose to abort — mirrors "claim is not transactional
/// across batches" honestly rather than pretending atomicity across many
/// physical events).
pub fn claim_all(hub: &Hub, request: ClaimRequestArgs<'_>) -> Result<ClaimOutcome> {
    let ClaimRequestArgs {
        repo_root,
        lane,
        owns,
        caller,
        reason,
    } = request;
    let expanded = normalize_owns_paths(repo_root, owns)?;
    if expanded.is_empty() {
        return Err(CoordinationError::rejected(
            CoordinationRejection::from_static(
                "claim requires at least one path (exact file, directory, or glob)",
            )?,
        ));
    }
    let writer = WriterId::new(&hub.config.node_id, lane);
    let mut events = Vec::new();
    for batch in expanded.chunks(MAX_CLAIM_PATHS) {
        // CLONE-JUSTIFICATION: each persisted claim batch needs an owned caller context.
        let context = caller.clone().into_claim_context(ClaimContextExtras {
            lock_kind: Some(LockKind::WriteLock),
            operation: Some(Operation::Edit),
            ..Default::default()
        })?;
        let raw = RawClaim {
            writer: ClaimWriter::from(&writer),
            lane: ClaimLane::from(lane),
            paths: batch.to_vec(),
            event_id: ClaimEventId::request_placeholder(),
            reason: reason.cloned(),
            // CLONE-JUSTIFICATION: raw request retains context while later event construction also needs it.
            context: context.clone(),
        };
        let request = enrich_claim(&raw, ClaimContextPresence::Explicit)?;
        let all = read_all_streams(hub.root.as_path())?;
        let active = active_claims(&all.events);
        let enriched_active = active
            .iter()
            .map(|claim| enrich_claim(claim, ClaimContextPresence::Explicit))
            .collect::<Result<Vec<_>>>()?;
        let decision = blockers_for_request(&enriched_active, &request, Operation::Edit);
        if !decision.blockers.is_empty() {
            return Ok(ClaimOutcome {
                status: ClaimOutcomeStatus::Blocked,
                events,
                blockers: decision.blockers,
            });
        }
        let event = append_event(
            hub,
            AppendEventArgs {
                lane,
                kind: CoordinationEventKind::Claim,
                paths: Some(batch.to_vec()),
                reason: reason.cloned(),
                context: Some(EventContextRefs {
                    claim: &context,
                    caller,
                }),
                metadata: EventMetadata::default(),
            },
        )?;
        events.push(event);
    }
    Ok(ClaimOutcome {
        status: ClaimOutcomeStatus::Accepted,
        events,
        blockers: Vec::new(),
    })
}

/// Release exact paths held by `lane`.
pub fn release(
    hub: &Hub,
    lane: &LaneId,
    paths: &[ClaimPath],
    caller: &CallerContext,
    reason: Option<&ClaimReason>,
) -> Result<HubEventResponse> {
    // CLONE-JUSTIFICATION: append_event consumes context while the caller remains borrowed by this API.
    let context = caller
        .clone()
        .into_claim_context(ClaimContextExtras::default())?;
    append_event(
        hub,
        AppendEventArgs {
            lane,
            kind: CoordinationEventKind::Release,
            paths: Some(paths.to_vec()),
            reason: reason.cloned(),
            context: Some(EventContextRefs {
                claim: &context,
                caller,
            }),
            metadata: EventMetadata::default(),
        },
    )
}

/// Append a lane-addressed coordination message to the caller's own stream.
///
/// The recipient is a lane id, not an arbitrary writer string. The caller's
/// context is embedded exactly like claim/release events so desktop dispatch
/// never attributes a message to the Tauri process working directory.
pub fn send_message(
    hub: &Hub,
    lane: &LaneId,
    recipient_lane: LaneId,
    body: CoordinationMessageBody,
    caller: &CallerContext,
) -> Result<HubEventResponse> {
    // CLONE-JUSTIFICATION: append_event consumes context while the caller remains borrowed by this API.
    let context = caller
        .clone()
        .into_claim_context(ClaimContextExtras::default())?;
    append_event(
        hub,
        AppendEventArgs {
            lane,
            kind: CoordinationEventKind::Message,
            paths: None,
            reason: None,
            context: Some(EventContextRefs {
                claim: &context,
                caller,
            }),
            metadata: EventMetadata {
                to: Some(recipient_lane),
                body: Some(body),
                ..Default::default()
            },
        },
    )
}

/// Append an acknowledgement for an existing message or handoff event.
///
/// Rejecting unknown/non-message ids prevents detached acknowledgements that
/// cannot be rendered back to a MailRow.
pub fn acknowledge_message(
    hub: &Hub,
    lane: &LaneId,
    message_id: ClaimEventId,
    caller: &CallerContext,
) -> Result<HubEventResponse> {
    let all = read_all_streams(hub.root.as_path())?;
    let is_message = all.events.iter().any(|event| {
        event.id == message_id.as_str() && (event.kind == "message" || event.kind == "handoff")
    });
    if !is_message {
        return Err(CoordinationError::rejected(
            CoordinationRejection::from_static(
                "coordination acknowledgement requires an existing message id",
            )?,
        ));
    }
    let writer = WriterId::new(&hub.config.node_id, lane);
    let already_acknowledged = all.events.iter().any(|event| {
        event.kind == "ack"
            && event.message_id.as_deref() == Some(message_id.as_str())
            && event.writer == writer.as_str()
    });
    if already_acknowledged {
        return Err(CoordinationError::rejected(
            CoordinationRejection::from_static(
                "coordination message is already acknowledged by this writer",
            )?,
        ));
    }
    // CLONE-JUSTIFICATION: append_event consumes context while the caller remains borrowed by this API.
    let context = caller
        .clone()
        .into_claim_context(ClaimContextExtras::default())?;
    append_event(
        hub,
        AppendEventArgs {
            lane,
            kind: CoordinationEventKind::Acknowledgement,
            paths: None,
            reason: None,
            context: Some(EventContextRefs {
                claim: &context,
                caller,
            }),
            metadata: EventMetadata {
                message_id: Some(message_id),
                ..Default::default()
            },
        },
    )
}

/// Closeout scope filters. Ported from `api.mjs#closeoutFilters` /
/// `matchingCloseoutClaims`.
#[derive(Debug, Clone, Default)]
pub struct CloseoutFilters {
    pub lane: Option<LaneId>,
    pub lane_scope: CloseoutLaneScope,
    pub writer: Option<ClaimWriter>,
    pub node_id_prefix: Option<NodeId>,
    pub codex_thread_id: Option<CoordinationOwnerIdentity>,
    pub codex_session_id: Option<CoordinationOwnerIdentity>,
    pub project_id: Option<CoordinationProjectId>,
    pub worktree_root: Option<CoordinationWorktree>,
}

fn matches_filters(claim: &RawClaim, filters: &CloseoutFilters) -> ClaimFilterMatch {
    // Selected-lane is the safe default. Releasing every owned claim is still
    // constrained by this predicate; only the explicit AllLanes
    // administrative scope may cross sibling lanes.
    if filters.lane_scope == CloseoutLaneScope::SelectedLane {
        if let Some(lane) = &filters.lane {
            if claim.lane.as_str() != lane.as_str() {
                return ClaimFilterMatch::Excluded;
            }
        }
    }
    if let Some(writer) = &filters.writer {
        if &claim.writer != writer {
            return ClaimFilterMatch::Excluded;
        }
    }
    if let Some(prefix) = &filters.node_id_prefix {
        if !claim
            .writer
            .as_str()
            .starts_with(&format!("{}.", prefix.as_str()))
        {
            return ClaimFilterMatch::Excluded;
        }
    }
    if let Some(thread) = &filters.codex_thread_id {
        if claim.context.codex_thread_id.as_ref() != Some(thread) {
            return ClaimFilterMatch::Excluded;
        }
    }
    if let Some(session) = &filters.codex_session_id {
        if claim.context.codex_session_id.as_ref() != Some(session) {
            return ClaimFilterMatch::Excluded;
        }
    }
    if let Some(project) = &filters.project_id {
        if claim.context.project_id.as_ref() != Some(project) {
            return ClaimFilterMatch::Excluded;
        }
    }
    if let Some(worktree) = &filters.worktree_root {
        let claim_worktree = claim
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
            });
        if claim_worktree != Some(worktree.as_str()) {
            return ClaimFilterMatch::Excluded;
        }
    }
    ClaimFilterMatch::Matches
}

/// Closeout: release every claim matching the scope filters. Ported
/// (narrowed) from `api.mjs#coordinationCloseout` — the stale-claim repair
/// pass and the JSON read-index rebuild are deferred (see crate deviation
/// note); the scope-filtered release, which is the load-bearing safety
/// property (closeout scoped to lane A must NOT release lane B's claims),
/// is fully ported and tested.
pub fn closeout(
    hub: &Hub,
    acting_lane: &LaneId,
    filters: &CloseoutFilters,
    caller: &CallerContext,
    reason: Option<&ClaimReason>,
) -> Result<Vec<HubEventResponse>> {
    let all = read_all_streams(hub.root.as_path())?;
    let active = active_claims(&all.events);
    let matching: Vec<RawClaim> = active
        .into_iter()
        .filter(|claim| matches!(matches_filters(claim, filters), ClaimFilterMatch::Matches))
        .collect();
    if matching.is_empty() {
        return Ok(Vec::new());
    }
    let mut by_lane: std::collections::BTreeMap<ClaimLane, Vec<ClaimPath>> =
        std::collections::BTreeMap::new();
    for claim in matching {
        by_lane.entry(claim.lane).or_default().extend(claim.paths);
    }
    // CLONE-JUSTIFICATION: release events own their transport context after validation.
    let context = caller
        .clone()
        .into_claim_context(ClaimContextExtras::default())?;
    let mut events = Vec::new();
    let reason = reason.cloned().map_or_else(
        || ClaimReason::from_static("coordination closeout release"),
        Ok,
    )?;
    for (claim_lane, mut paths) in by_lane {
        paths.sort();
        paths.dedup();
        let lane = claim_lane.to_lane_id()?;
        let event = append_event(
            hub,
            AppendEventArgs {
                lane: &lane,
                kind: CoordinationEventKind::Release,
                paths: Some(paths),
                // CLONE-JUSTIFICATION: each release event owns its optional request reason.
                reason: Some(reason.clone()),
                context: Some(EventContextRefs {
                    claim: &context,
                    caller,
                }),
                metadata: EventMetadata::default(),
            },
        )?;
        events.push(event);
    }
    let _ = acting_lane; // acting lane recorded for future audit-trail use; release events are emitted per-owning-lane, matching api.mjs's claimsByLane grouping.
    Ok(events)
}
