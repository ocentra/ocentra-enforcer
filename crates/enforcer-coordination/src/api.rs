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

use std::path::{Path, PathBuf};

use enforcer_domain::ids::{HubName, LaneId};

use crate::domain::{self, HubConfig, NodeId, NodeName, WriterId};
use crate::error::{CoordinationError, Result};
use crate::events::HubEvent;
use crate::ledger::active_claims;
use crate::lock::{
    blockers_for_request, enrich_claim, ClaimContext, LockKind, Operation, RawClaim,
};
use crate::sync::stream::{append_completed_event, read_all_streams, stream_tip};

/// Maximum exact file paths per SINGLE claim event, preserved from the
/// vendored JS `claim-policy.js#MAX_CLAIM_PATHS` for wire/event-shape
/// compatibility. Unlike the JS source (which REJECTS a request over 10
/// paths with a raw error — the L13 finding), this crate treats it as an
/// internal batching unit: `claim_all` transparently issues multiple claim
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
    pub project_id: String,
    pub worktree_root: String,
    pub branch: String,
    pub commit: Option<String>,
    pub codex_thread_id: Option<String>,
    pub codex_session_id: Option<String>,
}
impl CallerContext {
    fn into_claim_context(self, extra: ClaimContextExtras) -> ClaimContext {
        ClaimContext {
            project_id: Some(self.project_id),
            git_remote: None,
            repo_root: Some(self.worktree_root.clone()),
            worktree_root: Some(self.worktree_root),
            branch: Some(self.branch),
            codex_thread_id: self.codex_thread_id,
            codex_session_id: self.codex_session_id,
            explicit_codex_thread_id: None,
            explicit_codex_session_id: None,
            claim_group: extra.claim_group,
            lock_kind: extra.lock_kind.map(|k| k.as_str().to_owned()),
            operation: extra.operation.map(|o| o.as_str().to_owned()),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ClaimContextExtras {
    claim_group: Option<String>,
    lock_kind: Option<LockKind>,
    operation: Option<Operation>,
}

/// Coordination hub handle: root path + this node's loaded identity.
#[derive(Debug)]
pub struct Hub {
    pub root: PathBuf,
    pub config: HubConfig,
}

/// Open an existing hub identity without creating or replacing it.
///
/// Desktop and service callers use this when a human explicitly dispatches
/// against a configured ledger. Unlike [`init`], this never creates state.
pub fn open(root: &Path) -> Result<Hub> {
    Ok(Hub {
        root: root.to_path_buf(),
        config: load_identity(root)?,
    })
}

/// L1: idempotent init. If an identity already exists at `root`, it is
/// loaded and returned as-is (never a raw filesystem "already exists"
/// error, and never silently re-created with different values). If none
/// exists, a fresh identity is created and persisted.
pub fn init(root: &Path, hub: &HubName, lane: &LaneId) -> Result<HubConfig> {
    let identity_path = domain::identity_path(root);
    if identity_path.exists() {
        return load_identity(root);
    }
    let node_id = NodeId::random();
    let node_name = NodeName::sanitize_hostname(&hostname_or_fallback());
    let config = HubConfig {
        hub: hub.clone(),
        node_id,
        node_name,
        default_lane: lane.clone(),
        created_at: now_iso(),
    };
    std::fs::create_dir_all(domain::identity_dir(root))?;
    // Exclusive create: two concurrent FIRST inits still race safely — the
    // loser observes AlreadyExists and falls back to loading what the
    // winner wrote, which is itself the L1 idempotency guarantee extended
    // to the concurrent case.
    let bytes = serde_json::to_vec_pretty(&config)?;
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
    let raw = std::fs::read_to_string(domain::identity_path(root))?;
    Ok(serde_json::from_str(&raw)?)
}

fn hostname_or_fallback() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".to_owned())
}

fn now_iso() -> String {
    // Minimal RFC3339 stamp without pulling in a chrono dependency; matches
    // the shape (not necessarily the exact monotonic precision) of
    // `identity.js#nowIso`'s `new Date().toISOString()`.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let days = secs / 86400;
    let (year, month, day) = civil_from_days(days as i64);
    let rem = secs % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}

/// Howard Hinnant's civil_from_days algorithm (public domain), used only to
/// avoid a chrono dependency for a display timestamp.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// L13: normalize an `owns:` entry, which may be an exact file, a glob
/// pattern (`crates/foo/**`), or a bare directory prefix (`crates/foo/`),
/// into a list of exact tracked-file paths. Directories are expanded to
/// every regular file beneath them; globs are expanded via the `glob` crate.
/// Files that don't exist on disk (not-yet-created targets of a fresh
/// workpack) are passed through unexpanded as a literal single-file entry,
/// since a workpack's `owns:` set legitimately includes files a lane is
/// about to CREATE.
pub fn normalize_owns_paths(repo_root: &Path, entries: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for raw in entries {
        let trimmed = raw.trim().replace('\\', "/");
        if trimmed.is_empty() {
            continue;
        }
        let is_glob = trimmed.contains('*') || trimmed.contains('?') || trimmed.contains('[');
        let looks_like_dir = trimmed.ends_with('/');
        // A `<dir>/**` owns-set entry (the workpack convention, e.g.
        // `crates/enforcer-coordination/**`) means "every file recursively
        // under this directory". The `glob` crate's `**` only recurses when
        // followed by another path segment (`**/*.rs`), so treat the bare
        // `<dir>/**` shape as directory expansion directly rather than
        // relying on glob's non-recursive-at-the-tail semantics.
        let recursive_dir_suffix = trimmed.strip_suffix("/**");
        if let Some(dir_part) = recursive_dir_suffix
            .filter(|d| !d.contains('*') && !d.contains('?') && !d.contains('['))
        {
            let dir = repo_root.join(dir_part);
            if dir.is_dir() {
                walk_files(&dir, repo_root, &mut out, &mut seen)?;
            } else if !dir_part.is_empty() && seen.insert(dir_part.to_owned()) {
                // Not created yet — keep the literal directory prefix as a
                // single representative entry so a fresh workpack's
                // not-yet-existing crate dir is still represented in the
                // expanded set.
                out.push(dir_part.to_owned());
            }
        } else if is_glob {
            let pattern = repo_root
                .join(&trimmed)
                .to_string_lossy()
                .replace('\\', "/");
            let mut matched_any = false;
            for entry in glob::glob(&pattern)
                .map_err(|e| CoordinationError::rejected(format!("invalid glob {trimmed}: {e}")))?
            {
                let path = entry.map_err(|e| CoordinationError::rejected(e.to_string()))?;
                if path.is_file() {
                    matched_any = true;
                    push_relative(repo_root, &path, &mut out, &mut seen);
                }
            }
            if !matched_any {
                // No matches yet (e.g. a fresh crate not yet created) — keep
                // the pattern's literal directory prefix as a single
                // representative entry rather than silently dropping it.
                let literal = trimmed.trim_end_matches("/**").trim_end_matches('*');
                if !literal.is_empty() && seen.insert(literal.to_owned()) {
                    out.push(literal.to_owned());
                }
            }
        } else if looks_like_dir || repo_root.join(&trimmed).is_dir() {
            let dir = repo_root.join(trimmed.trim_end_matches('/'));
            if dir.is_dir() {
                walk_files(&dir, repo_root, &mut out, &mut seen)?;
            } else {
                // Directory doesn't exist yet — nothing to expand; skip
                // silently (the concrete files inside it will be claimed
                // individually once they exist, or the caller should list
                // them explicitly).
            }
        } else if seen.insert(trimmed.clone()) {
            out.push(trimmed);
        }
    }
    Ok(out)
}

fn push_relative(
    root: &Path,
    path: &Path,
    out: &mut Vec<String>,
    seen: &mut std::collections::BTreeSet<String>,
) {
    if let Ok(rel) = path.strip_prefix(root) {
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if seen.insert(rel_str.clone()) {
            out.push(rel_str);
        }
    }
}

fn walk_files(
    dir: &Path,
    root: &Path,
    out: &mut Vec<String>,
    seen: &mut std::collections::BTreeSet<String>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            // Skip common non-source build/vcs directories to keep dir-owns
            // expansion sane by default.
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if matches!(name.as_ref(), "target" | "node_modules" | ".git") {
                continue;
            }
            walk_files(&path, root, out, seen)?;
        } else if path.is_file() {
            push_relative(root, &path, out, seen);
        }
    }
    Ok(())
}

/// The outcome of one claim ATTEMPT (either a single event, or a blocked
/// decision requiring the caller to escalate/queue).
#[derive(Debug, Clone)]
pub struct ClaimOutcome {
    pub ok: bool,
    pub events: Vec<HubEvent>,
    pub blockers: Vec<crate::lock::Conflict>,
}

/// Request parameters for [`claim_all`], bundled to keep the function's
/// argument count within the workspace's `too_many_arguments` lint budget.
/// Every field is itself a reference/`Copy` value, so this struct is `Copy`
/// and passed by value cheaply (no allocation, no clone).
#[derive(Debug, Clone, Copy)]
pub struct ClaimRequestArgs<'a> {
    pub repo_root: &'a Path,
    pub lane: &'a LaneId,
    pub owns: &'a [String],
    pub caller: &'a CallerContext,
    pub reason: Option<&'a str>,
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
            "claim requires at least one path (exact file, directory, or glob)",
        ));
    }
    let writer = WriterId::new(&hub.config.node_id, lane);
    let mut events = Vec::new();
    for batch in expanded.chunks(MAX_CLAIM_PATHS) {
        let context = caller.clone().into_claim_context(ClaimContextExtras {
            lock_kind: Some(LockKind::WriteLock),
            operation: Some(Operation::Edit),
            ..Default::default()
        });
        let raw = RawClaim {
            writer: writer.as_str().to_owned(),
            lane: lane.as_str().to_owned(),
            paths: batch.to_vec(),
            event_id: "__request__".to_owned(),
            reason: reason.map(str::to_owned),
            context: context.clone(),
        };
        let request = enrich_claim(&raw, true);
        let all = read_all_streams(&hub.root)?;
        let active = active_claims(&all.events);
        let enriched_active: Vec<_> = active.iter().map(|c| enrich_claim(c, true)).collect();
        let decision = blockers_for_request(&enriched_active, &request, Operation::Edit);
        if !decision.blockers.is_empty() {
            return Ok(ClaimOutcome {
                ok: false,
                events,
                blockers: decision.blockers,
            });
        }
        let event = append_event(
            hub,
            AppendEventArgs {
                lane,
                kind: "claim",
                paths: Some(batch.to_vec()),
                reason: reason.map(str::to_owned),
                context: Some((&context, caller)),
                metadata: EventMetadata::default(),
            },
        )?;
        events.push(event);
    }
    Ok(ClaimOutcome {
        ok: true,
        events,
        blockers: Vec::new(),
    })
}

/// Release exact paths held by `lane`.
pub fn release(
    hub: &Hub,
    lane: &LaneId,
    paths: &[String],
    caller: &CallerContext,
    reason: Option<&str>,
) -> Result<HubEvent> {
    let context = caller
        .clone()
        .into_claim_context(ClaimContextExtras::default());
    append_event(
        hub,
        AppendEventArgs {
            lane,
            kind: "release",
            paths: Some(paths.to_vec()),
            reason: reason.map(str::to_owned),
            context: Some((&context, caller)),
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
    recipient_lane: &LaneId,
    body: &str,
    caller: &CallerContext,
) -> Result<HubEvent> {
    let body = body.trim();
    if body.is_empty() {
        return Err(CoordinationError::rejected(
            "coordination message body is required",
        ));
    }
    let context = caller
        .clone()
        .into_claim_context(ClaimContextExtras::default());
    append_event(
        hub,
        AppendEventArgs {
            lane,
            kind: "message",
            paths: None,
            reason: None,
            context: Some((&context, caller)),
            metadata: EventMetadata {
                to: Some(recipient_lane.as_str().to_owned()),
                body: Some(body.to_owned()),
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
    message_id: &str,
    caller: &CallerContext,
) -> Result<HubEvent> {
    let message_id = message_id.trim();
    let all = read_all_streams(&hub.root)?;
    let is_message = all.events.iter().any(|event| {
        event.id == message_id && (event.kind == "message" || event.kind == "handoff")
    });
    if !is_message {
        return Err(CoordinationError::rejected(
            "coordination acknowledgement requires an existing message id",
        ));
    }
    let writer = WriterId::new(&hub.config.node_id, lane);
    let already_acknowledged = all.events.iter().any(|event| {
        event.kind == "ack"
            && event.message_id.as_deref() == Some(message_id)
            && event.writer == writer.as_str()
    });
    if already_acknowledged {
        return Err(CoordinationError::rejected(
            "coordination message is already acknowledged by this writer",
        ));
    }
    let context = caller
        .clone()
        .into_claim_context(ClaimContextExtras::default());
    append_event(
        hub,
        AppendEventArgs {
            lane,
            kind: "ack",
            paths: None,
            reason: None,
            context: Some((&context, caller)),
            metadata: EventMetadata {
                message_id: Some(message_id.to_owned()),
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
    pub include_all_lanes: bool,
    pub writer: Option<String>,
    pub node_id_prefix: Option<String>,
    pub codex_thread_id: Option<String>,
    pub codex_session_id: Option<String>,
    pub project_id: Option<String>,
    pub worktree_root: Option<String>,
}

fn matches_filters(claim: &RawClaim, filters: &CloseoutFilters) -> bool {
    if !filters.include_all_lanes {
        if let Some(lane) = &filters.lane {
            if claim.lane != lane.as_str() {
                return false;
            }
        }
    }
    if let Some(writer) = &filters.writer {
        if &claim.writer != writer {
            return false;
        }
    }
    if let Some(prefix) = &filters.node_id_prefix {
        if !claim.writer.starts_with(&format!("{prefix}.")) {
            return false;
        }
    }
    if let Some(thread) = &filters.codex_thread_id {
        if claim.context.codex_thread_id.as_deref() != Some(thread.as_str()) {
            return false;
        }
    }
    if let Some(session) = &filters.codex_session_id {
        if claim.context.codex_session_id.as_deref() != Some(session.as_str()) {
            return false;
        }
    }
    if let Some(project) = &filters.project_id {
        if claim.context.project_id.as_deref() != Some(project.as_str()) {
            return false;
        }
    }
    if let Some(worktree) = &filters.worktree_root {
        let claim_worktree = claim
            .context
            .worktree_root
            .as_deref()
            .or(claim.context.repo_root.as_deref());
        if claim_worktree != Some(worktree.as_str()) {
            return false;
        }
    }
    true
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
    reason: Option<&str>,
) -> Result<Vec<HubEvent>> {
    let all = read_all_streams(&hub.root)?;
    let active = active_claims(&all.events);
    let matching: Vec<&RawClaim> = active
        .iter()
        .filter(|c| matches_filters(c, filters))
        .collect();
    if matching.is_empty() {
        return Ok(Vec::new());
    }
    let mut by_lane: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for claim in matching {
        by_lane
            .entry(claim.lane.clone())
            .or_default()
            .extend(claim.paths.clone());
    }
    let context = caller
        .clone()
        .into_claim_context(ClaimContextExtras::default());
    let mut events = Vec::new();
    let reason = reason
        .map(str::to_owned)
        .unwrap_or_else(|| "coordination closeout release".to_owned());
    for (lane_str, mut paths) in by_lane {
        paths.sort();
        paths.dedup();
        let lane: LaneId = lane_str
            .parse()
            .map_err(|e: enforcer_core::error::DecodeError| CoordinationError::from(e))?;
        let event = append_event(
            hub,
            AppendEventArgs {
                lane: &lane,
                kind: "release",
                paths: Some(paths),
                reason: Some(reason.clone()),
                context: Some((&context, caller)),
                metadata: EventMetadata::default(),
            },
        )?;
        events.push(event);
    }
    let _ = acting_lane; // acting lane recorded for future audit-trail use; release events are emitted per-owning-lane, matching api.mjs's claimsByLane grouping.
    Ok(events)
}

fn claim_context_to_json(context: &ClaimContext, caller: &CallerContext) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    macro_rules! set {
        ($key:literal, $field:expr) => {
            if let Some(v) = &$field {
                map.insert($key.to_owned(), serde_json::Value::String(v.clone()));
            }
        };
    }
    set!("projectId", context.project_id);
    set!("gitRemote", context.git_remote);
    set!("repoRoot", context.repo_root);
    set!("worktreeRoot", context.worktree_root);
    set!("branch", context.branch);
    set!("codexThreadId", context.codex_thread_id);
    set!("codexSessionId", context.codex_session_id);
    set!("claimGroup", context.claim_group);
    set!("lockKind", context.lock_kind);
    set!("operation", context.operation);
    set!("commit", caller.commit);
    serde_json::Value::Object(map)
}

/// Arguments for [`append_event`], bundled to keep the function's argument
/// count within the workspace's `too_many_arguments` lint budget.
struct AppendEventArgs<'a> {
    lane: &'a LaneId,
    kind: &'a str,
    paths: Option<Vec<String>>,
    reason: Option<String>,
    context: Option<(&'a ClaimContext, &'a CallerContext)>,
    metadata: EventMetadata,
}

#[derive(Default)]
struct EventMetadata {
    to: Option<String>,
    body: Option<String>,
    message_id: Option<String>,
}

/// Build + hash-chain + append one event to the caller's own writer stream.
fn append_event(hub: &Hub, args: AppendEventArgs<'_>) -> Result<HubEvent> {
    let AppendEventArgs {
        lane,
        kind,
        paths,
        reason,
        context,
        metadata,
    } = args;
    let tip = stream_tip(&hub.root, &hub.config.node_id, lane)?;
    let writer = WriterId::new(&hub.config.node_id, lane);
    let seq = tip.as_ref().map_or(1, |t| t.seq + 1);
    let prev_event_id = tip.as_ref().map(|t| t.id.clone());
    let prev_hash = tip.as_ref().map(|t| t.hash.clone());
    let mut event = HubEvent {
        id: random_event_id(),
        schema: 1,
        hub: hub.config.hub.as_str().to_owned(),
        node_id: hub.config.node_id.as_str().to_owned(),
        node_name: hub.config.node_name.as_str().to_owned(),
        lane: lane.as_str().to_owned(),
        writer: writer.as_str().to_owned(),
        kind: kind.to_owned(),
        ts: now_iso(),
        seq,
        prev_event_id,
        prev_hash,
        hash: String::new(),
        to: metadata.to,
        body: metadata.body,
        message_id: metadata.message_id,
        paths,
        reason,
        owner: None,
        owners: None,
        state: None,
        worker_state: None,
        task_id: None,
        task_state: None,
        title: None,
        pr_url: None,
        summary: None,
        ttl_seconds: None,
        session_id: None,
        context: context.map(|(context, caller)| claim_context_to_json(context, caller)),
    };
    event.hash = crate::events::hash_for_event(&event)?;
    append_completed_event(&hub.root, &hub.config.node_id, lane, &event)?;
    Ok(event)
}

fn random_event_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|before_epoch| before_epoch.duration());
    format!(
        "evt_{:032x}",
        now.as_nanos() ^ (u128::from(std::process::id()) << 32)
    )
}
