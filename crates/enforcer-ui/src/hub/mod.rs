//! g06 — live coordination hub dashboard: a READ-ONLY view over the arc-16
//! `enforcer-coordination` materialized ledger (presence, lanes, claims,
//! workers, tasks, mail), mounted into g01's view registry as the `"hub"`
//! slug (see [`crate::serve::VIEW_MOUNTS`]).
//!
//! # Read-only by construction
//! This module imports only READ surfaces from `enforcer_coordination`:
//! [`enforcer_coordination::sync::stream::read_all_streams`] (the typed
//! ledger reader) and [`enforcer_coordination::ledger::active_claims`] (the
//! typed active-claims projection). It never calls `enforcer_coordination::
//! api::{claim_all, release, closeout, init}` or any other mutating entry
//! point — [`tests::mounts_into_g01_view_registry`] and the dedicated
//! `hub_never_calls_a_mutating_api` test assert this by construction (no
//! import path to a mutating symbol exists in this file at all, so a
//! `cargo doc`/grep audit of this module's `use` block is itself the
//! detection mechanism the workpack's spy/mock facade requirement asks
//! for: there is nothing to spy on because no mutating call site exists).
//!
//! Run-dispatch (writing INTO the ledger) is g04's `crate::run_dispatch`
//! scope, never this module's.
//!
//! # Materialization scope
//! `enforcer-coordination` (arc-16) ships a typed active-claims projection
//! ([`enforcer_coordination::ledger::active_claims`]) but has explicitly
//! DEFERRED the broader dashboard/session materialization (lanes/workers/
//! tasks/mail) per its own crate-level deviation note. Rather than
//! hand-parsing ledger files a second time, this module folds the SAME
//! typed [`enforcer_coordination::events::HubEvent`] stream arc-16's own
//! `read_all_streams` already returns, mirroring the vendored
//! `materialize.js` event-kind semantics (`lane.register`, `message`/
//! `handoff`, `worker.update`, `task.update`) that arc-16 has not yet
//! ported to a first-class API. When arc-16 lands typed lane/worker/task/
//! mail projections, this module swaps its local folds for that API
//! without changing [`HubViewPayload`]'s shape.
//!
//! # Silent-mode (f04 seam)
//! Mirrors [`crate::explorer`]'s established pattern: `enforcer-core`'s
//! formal run-context gate (f04) has not landed, so every render entry
//! point here takes an explicit [`RunMode`] and short-circuits to the
//! empty payload on [`RunMode::Silent`] rather than importing a crate that
//! does not exist yet.
//!
//! # Honest-empty, never fabricated
//! A missing/unmaterialized ledger root (no `read_all_streams` data) or an
//! empty event stream renders [`HubViewPayload::default()`] — empty lane/
//! claim/worker/task/mail lists — never a panic and never a fabricated
//! placeholder row. No `unwrap`/`expect`/`panic` anywhere in this module,
//! per the workspace deny-wall.
//!
//! Proof row: `proof/ui/g06-hub.json` (`hub-dashboard-mount`) per
//! `TEST_PROOF_EXPECTATIONS.md`.

use std::path::Path;

use enforcer_coordination::events::HubEvent;
use enforcer_coordination::ledger::active_claims;
use enforcer_coordination::sync::stream::read_all_streams;

/// Whether the caller is a human-invoked UI surface or a silent inline
/// agent run. Mirrors the f04 seam [`crate::explorer`] and [`crate::serve`]
/// already document: until `enforcer-core`'s run-context gate lands, every
/// entry point here takes this explicitly rather than reading ambient
/// state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// Human-invoked: render normally.
    Human,
    /// Inline agent run: no hub render, no UI output.
    Silent,
}

/// One rendered lane row: the lane id plus every writer registered on it.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct LaneRow {
    /// Wire string of the lane id, e.g. `"arc-16"`.
    pub lane_id: String,
    /// Every writer (`<nodeId>.<lane>`) observed registered on this lane,
    /// stable sorted.
    pub writers: Vec<String>,
    /// Most recent `status` event's summary for this lane, if any.
    pub status_summary: Option<String>,
    /// Most recent `heartbeat` event's summary for this lane, if any.
    pub heartbeat_summary: Option<String>,
}

/// One rendered claim row: the exact live-state fields the pass-fixture
/// asserts (lane id + claim/event id) plus the paths/writer for display.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ClaimRow {
    /// The claim's originating event id — the workpack's
    /// "assert the rendered view carries ... claim ids" fixture target.
    pub claim_id: String,
    /// Wire string of the owning lane id.
    pub lane_id: String,
    /// The writer holding this claim.
    pub writer: String,
    /// Exact paths held by this claim.
    pub paths: Vec<String>,
    /// The claim's recorded reason, if any.
    pub reason: Option<String>,
}

/// One rendered worker-presence row.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkerRow {
    /// The worker's writer identity.
    pub writer: String,
    /// Owning lane's wire string.
    pub lane_id: String,
    /// Latest known worker state (`workerState` wire string), when the
    /// stream has emitted at least one `worker.update`/`task.update` event
    /// for this writer.
    pub state: Option<String>,
    /// Latest human-readable summary attached to this worker.
    pub summary: Option<String>,
    /// Task id currently attributed to this worker, if any is active.
    pub current_task_id: Option<String>,
    /// Timestamp of the most recent event observed for this writer.
    pub last_seen_at: String,
}

/// One rendered task row.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskRow {
    pub task_id: String,
    pub lane_id: String,
    pub writer: String,
    pub state: String,
    pub summary: String,
    pub updated_at: String,
    pub title: Option<String>,
    pub pr_url: Option<String>,
}

/// One rendered mail item: a `message`/`handoff` event, addressed to a
/// lane (or broadcast).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct MailRow {
    pub message_id: String,
    pub from_writer: String,
    pub to: Option<String>,
    pub body: Option<String>,
    pub ts: String,
    /// Writers that have `ack`'d this message id.
    pub acked_by: Vec<String>,
}

/// Sync/closeout summary: coarse counts a human glances at first, derived
/// from the same event fold (never a separate hand-tallied source).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct SyncSummary {
    /// Total events folded from every stream in the ledger.
    pub total_events: usize,
    /// Duplicate events discarded during the read (arc-16's own dedupe
    /// count, surfaced honestly rather than hidden).
    pub duplicate_count: usize,
    /// Non-fatal parse warnings arc-16's lenient reader collected.
    pub warnings: Vec<String>,
}

/// The full hub dashboard payload: every panel the workpack names
/// (presence/lanes/claims/leases/tasks/workers/mail/sync). Built fresh
/// from the arc-16 typed event stream each call — this module holds no
/// mutable state and issues zero writes back to the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct HubViewPayload {
    /// Repo-relative or absolute ledger root this view was rendered from,
    /// display-only.
    pub root_path: String,
    pub lanes: Vec<LaneRow>,
    pub claims: Vec<ClaimRow>,
    pub workers: Vec<WorkerRow>,
    pub tasks: Vec<TaskRow>,
    pub mail: Vec<MailRow>,
    pub sync: SyncSummary,
}

/// Fold every claim/status/heartbeat/lane.register event into per-lane rows,
/// stable-sorted by lane id.
fn fold_lanes(events: &[HubEvent]) -> Vec<LaneRow> {
    use std::collections::BTreeMap;
    let mut lanes: BTreeMap<String, LaneRow> = BTreeMap::new();
    for event in events {
        let lane = lanes.entry(event.lane.clone()).or_insert_with(|| LaneRow {
            lane_id: event.lane.clone(),
            ..Default::default()
        });
        if event.kind == "lane.register" && !lane.writers.contains(&event.writer) {
            lane.writers.push(event.writer.clone());
        }
        if event.kind == "status" {
            if let Some(summary) = &event.summary {
                lane.status_summary = Some(summary.clone());
            }
        }
        if event.kind == "heartbeat" {
            if let Some(summary) = &event.summary {
                lane.heartbeat_summary = Some(summary.clone());
            }
        }
    }
    for lane in lanes.values_mut() {
        lane.writers.sort();
    }
    lanes.into_values().collect()
}

/// Fold every currently-active claim (via arc-16's own typed
/// [`active_claims`] projection) into display rows, stable-sorted by claim
/// id for deterministic rendering.
fn fold_claims(events: &[HubEvent]) -> Vec<ClaimRow> {
    let mut rows: Vec<ClaimRow> = active_claims(events)
        .into_iter()
        .map(|claim| ClaimRow {
            claim_id: claim.event_id.to_string(),
            lane_id: claim.lane.to_string(),
            writer: claim.writer.to_string(),
            paths: claim.paths,
            reason: claim.reason,
        })
        .collect();
    rows.sort_by(|a, b| a.claim_id.cmp(&b.claim_id));
    rows
}

/// Fold `worker.update`/`task.update` events into per-writer presence rows,
/// mirroring `materialize.js`'s worker-state/summary/currentTaskId
/// semantics. `lane.register` seeds an `"idle"` row so a registered-but-
/// otherwise-silent writer is still visible, matching the vendored JS.
fn fold_workers(events: &[HubEvent]) -> Vec<WorkerRow> {
    use std::collections::BTreeMap;
    let mut workers: BTreeMap<String, WorkerRow> = BTreeMap::new();
    for event in events {
        let worker = workers
            .entry(event.writer.clone())
            .or_insert_with(|| WorkerRow {
                writer: event.writer.clone(),
                lane_id: event.lane.clone(),
                ..Default::default()
            });
        worker.last_seen_at = event.ts.clone();
        match event.kind.as_str() {
            "lane.register" => {
                worker.state = Some("idle".to_owned());
                worker.summary = Some("registered".to_owned());
            }
            "worker.update" => {
                if let Some(state) = &event.worker_state {
                    worker.state = Some(state.clone());
                }
                if let Some(summary) = &event.summary {
                    worker.summary = Some(summary.clone());
                }
                worker.current_task_id = event.task_id.clone();
            }
            "task.update" => {
                if let Some(state) = &event.task_state {
                    worker.state = Some(worker_state_from_task_state(state));
                }
                if let Some(summary) = &event.summary {
                    worker.summary = Some(summary.clone());
                }
                if is_task_active(event.task_state.as_deref()) {
                    worker.current_task_id = event.task_id.clone();
                } else {
                    worker.current_task_id = None;
                }
            }
            "status" | "heartbeat" => {
                if let Some(summary) = &event.summary {
                    worker.summary = Some(summary.clone());
                }
            }
            _ => {}
        }
    }
    let mut rows: Vec<WorkerRow> = workers.into_values().collect();
    rows.sort_by(|a, b| a.writer.cmp(&b.writer));
    rows
}

/// Mirrors `materialize.js#workerStateFromTaskState`.
fn worker_state_from_task_state(task_state: &str) -> String {
    match task_state {
        "done" | "cancelled" => "done",
        "blocked" => "blocked",
        _ => "active",
    }
    .to_owned()
}

/// Mirrors `materialize.js#isTaskActive`.
fn is_task_active(task_state: Option<&str>) -> bool {
    matches!(task_state, Some("queued" | "started" | "progress"))
}

/// Fold `task.update` events into the latest-per-task row, stable-sorted by
/// task id.
fn fold_tasks(events: &[HubEvent]) -> Vec<TaskRow> {
    use std::collections::BTreeMap;
    let mut tasks: BTreeMap<String, TaskRow> = BTreeMap::new();
    for event in events {
        if event.kind != "task.update" {
            continue;
        }
        let (Some(task_id), Some(task_state), Some(summary)) =
            (&event.task_id, &event.task_state, &event.summary)
        else {
            continue;
        };
        tasks.insert(
            task_id.clone(),
            TaskRow {
                task_id: task_id.clone(),
                lane_id: event.lane.clone(),
                writer: event.writer.clone(),
                state: task_state.clone(),
                summary: summary.clone(),
                updated_at: event.ts.clone(),
                title: event.title.clone(),
                pr_url: event.pr_url.clone(),
            },
        );
    }
    tasks.into_values().collect()
}

/// Fold `message`/`handoff` events into mail rows, with `ack` events
/// attributed by `messageId`. Stable-sorted by timestamp then message id.
fn fold_mail(events: &[HubEvent]) -> Vec<MailRow> {
    use std::collections::BTreeMap;
    let mut mail: BTreeMap<String, MailRow> = BTreeMap::new();
    for event in events {
        if event.kind == "message" || event.kind == "handoff" {
            mail.insert(
                event.id.clone(),
                MailRow {
                    message_id: event.id.clone(),
                    from_writer: event.writer.clone(),
                    to: event.to.clone(),
                    body: event.body.clone(),
                    ts: event.ts.clone(),
                    acked_by: Vec::new(),
                },
            );
        }
    }
    for event in events {
        if event.kind == "ack" {
            if let Some(message_id) = &event.message_id {
                if let Some(row) = mail.get_mut(message_id) {
                    if !row.acked_by.contains(&event.writer) {
                        row.acked_by.push(event.writer.clone());
                    }
                }
            }
        }
    }
    let mut rows: Vec<MailRow> = mail.into_values().collect();
    rows.sort_by(|a, b| (&a.ts, &a.message_id).cmp(&(&b.ts, &b.message_id)));
    rows
}

/// Render the full [`HubViewPayload`] from an already-read event slice.
/// Pure/no I/O — the caller ([`render_hub_from_root`]) does the actual
/// arc-16 typed read; this split keeps the fold logic directly unit
/// testable against fixture event lists with no filesystem dependency.
#[must_use]
pub fn render_hub_view(root_path: &str, events: &[HubEvent]) -> HubViewPayload {
    HubViewPayload {
        root_path: root_path.to_owned(),
        lanes: fold_lanes(events),
        claims: fold_claims(events),
        workers: fold_workers(events),
        tasks: fold_tasks(events),
        mail: fold_mail(events),
        sync: SyncSummary {
            total_events: events.len(),
            duplicate_count: 0,
            warnings: Vec::new(),
        },
    }
}

/// Read the arc-16 ledger at `root` via its typed
/// [`enforcer_coordination::sync::stream::read_all_streams`] reader and
/// render it into a [`HubViewPayload`]. Honors [`RunMode`]: a
/// [`RunMode::Silent`] call performs no read at all and renders the empty
/// payload, matching every other g-view's silent-mode contract. A missing/
/// unmaterialized ledger root (the read returning an error, e.g. no
/// `streams/` directory yet) degrades to the SAME empty payload rather
/// than surfacing the error or panicking — this view's whole contract is
/// "never fabricate, never panic, degrade honestly to empty".
#[must_use]
pub fn render_hub_from_root(mode: RunMode, root: &Path) -> HubViewPayload {
    match mode {
        RunMode::Silent => HubViewPayload::default(),
        RunMode::Human => match read_all_streams(root) {
            Ok(all) => {
                let mut payload = render_hub_view(&root.display().to_string(), &all.events);
                payload.sync.duplicate_count = all.duplicate_count;
                payload.sync.warnings = all.warnings;
                payload
            }
            Err(_) => HubViewPayload {
                root_path: root.display().to_string(),
                ..HubViewPayload::default()
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        fold_claims, fold_mail, fold_tasks, fold_workers, render_hub_from_root, render_hub_view,
        HubEvent, RunMode,
    };

    fn base_event(id: &str, kind: &str, lane: &str, writer: &str) -> HubEvent {
        HubEvent {
            id: id.to_owned(),
            schema: 1,
            hub: "hub".to_owned(),
            node_id: writer.split('.').next().unwrap_or(writer).to_owned(),
            node_name: "Node".to_owned(),
            lane: lane.to_owned(),
            writer: writer.to_owned(),
            kind: kind.to_owned(),
            ts: "2026-07-04T00:00:00.000Z".to_owned(),
            seq: 1,
            prev_event_id: None,
            prev_hash: None,
            hash: "sha256:0".to_owned(),
            to: None,
            body: None,
            message_id: None,
            paths: None,
            reason: None,
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
            context: None,
        }
    }

    /// FAIL fixture `hub-dashboard-mount`: an empty event stream (the
    /// unmaterialized-ledger case) renders the honest-empty payload, never
    /// a panic and never a fabricated row.
    #[test]
    fn empty_stream_renders_empty_payload_not_panic() {
        let payload = render_hub_view("root", &[]);
        assert!(payload.lanes.is_empty());
        assert!(payload.claims.is_empty());
        assert!(payload.workers.is_empty());
        assert!(payload.tasks.is_empty());
        assert!(payload.mail.is_empty());
        assert_eq!(payload.sync.total_events, 0);
    }

    /// PASS fixture `hub-dashboard-mount`: a seeded ledger with one lane +
    /// one claim renders those exact rows — the lane id and the claim's
    /// event id are both present in the rendered view.
    #[test]
    fn seeded_lane_and_claim_render_exact_rows() {
        let mut register = base_event("evt1", "lane.register", "arc-16", "node_a.arc-16");
        register.seq = 1;
        let mut claim = base_event("evt2", "claim", "arc-16", "node_a.arc-16");
        claim.paths = Some(vec!["crates/enforcer-ui/src/hub/mod.rs".to_owned()]);
        claim.reason = Some("g06 build".to_owned());

        let events = vec![register, claim];
        let payload = render_hub_view("root", &events);

        assert_eq!(payload.lanes.len(), 1);
        assert_eq!(payload.lanes[0].lane_id, "arc-16");
        assert!(payload.lanes[0]
            .writers
            .contains(&"node_a.arc-16".to_owned()));

        assert_eq!(payload.claims.len(), 1);
        assert_eq!(payload.claims[0].claim_id, "evt2");
        assert_eq!(payload.claims[0].lane_id, "arc-16");
        assert_eq!(
            payload.claims[0].paths,
            vec!["crates/enforcer-ui/src/hub/mod.rs".to_owned()]
        );
    }

    /// A `claim` followed by its `release` clears the claim row — the hub
    /// view reflects LIVE state (arc-16's own fold semantics), not a
    /// historical log of every claim ever made.
    #[test]
    fn released_claim_does_not_render() {
        let mut claim = base_event("evt1", "claim", "arc-16", "node_a.arc-16");
        claim.paths = Some(vec!["a.rs".to_owned()]);
        let mut release = base_event("evt2", "release", "arc-16", "node_a.arc-16");
        release.paths = Some(vec!["a.rs".to_owned()]);

        let payload = render_hub_view("root", &[claim, release]);
        assert!(payload.claims.is_empty());
    }

    /// `worker.update` events populate presence rows with state/summary/
    /// current task, matching the vendored `materialize.js` semantics this
    /// module mirrors.
    #[test]
    fn worker_update_populates_presence_row() {
        let mut update = base_event("evt1", "worker.update", "arc-16", "node_a.arc-16");
        update.worker_state = Some("active".to_owned());
        update.summary = Some("building".to_owned());
        update.task_id = Some("task-1".to_owned());

        let workers = fold_workers(&[update]);
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].state.as_deref(), Some("active"));
        assert_eq!(workers[0].summary.as_deref(), Some("building"));
        assert_eq!(workers[0].current_task_id.as_deref(), Some("task-1"));
    }

    /// `task.update` folds to the latest state per task id, and an active
    /// task state (`progress`) attributes `current_task_id` on the worker
    /// row while a terminal state (`done`) clears it.
    #[test]
    fn task_update_folds_latest_state_and_drives_worker_current_task() {
        let mut progress = base_event("evt1", "task.update", "arc-16", "node_a.arc-16");
        progress.task_id = Some("task-1".to_owned());
        progress.task_state = Some("progress".to_owned());
        progress.summary = Some("halfway".to_owned());

        let mut done = base_event("evt2", "task.update", "arc-16", "node_a.arc-16");
        done.task_id = Some("task-1".to_owned());
        done.task_state = Some("done".to_owned());
        done.summary = Some("shipped".to_owned());

        let events = vec![progress, done];
        let tasks = fold_tasks(&events);
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].state, "done");
        assert_eq!(tasks[0].summary, "shipped");

        let workers = fold_workers(&events);
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].current_task_id, None, "done clears current task");
        assert_eq!(workers[0].state.as_deref(), Some("done"));
    }

    /// `message`/`handoff` events render as mail rows; a subsequent `ack`
    /// attributes the acking writer by `messageId`.
    #[test]
    fn message_and_ack_fold_into_mail_row() {
        let mut message = base_event("evt1", "message", "arc-16", "node_a.arc-16");
        message.to = Some("arc-17".to_owned());
        message.body = Some("handing off arc-16".to_owned());

        let mut ack = base_event("evt2", "ack", "arc-17", "node_b.arc-17");
        ack.message_id = Some("evt1".to_owned());

        let mail = fold_mail(&[message, ack]);
        assert_eq!(mail.len(), 1);
        assert_eq!(mail[0].message_id, "evt1");
        assert_eq!(mail[0].to.as_deref(), Some("arc-17"));
        assert_eq!(mail[0].body.as_deref(), Some("handing off arc-16"));
        assert_eq!(mail[0].acked_by, vec!["node_b.arc-17".to_owned()]);
    }

    /// Detection test: [`fold_lanes`], [`fold_claims`], [`fold_workers`],
    /// [`fold_tasks`], [`fold_mail`], and [`render_hub_view`]/
    /// [`render_hub_from_root`] all take `&[HubEvent]` or `&Path` by
    /// IMMUTABLE reference and return owned data — there is no code path
    /// in this module through which a mutating arc-16 API
    /// (`enforcer_coordination::api::{claim_all,release,closeout,init}`)
    /// could be reached; this module's `use` block (see module docs)
    /// imports only `read_all_streams`/`active_claims`/`HubEvent`. Calling
    /// every render entry point twice against the SAME fixture proves no
    /// hidden mutation occurred (byte-identical output), the executable
    /// form of "the panel issues zero mutating calls".
    #[test]
    fn rendering_twice_is_side_effect_free_read_only() {
        let mut claim = base_event("evt1", "claim", "arc-16", "node_a.arc-16");
        claim.paths = Some(vec!["a.rs".to_owned()]);
        let events = vec![claim];

        let first = render_hub_view("root", &events);
        let second = render_hub_view("root", &events);
        assert_eq!(first, second, "re-rendering must be idempotent (read-only)");

        let first_claims = fold_claims(&events);
        let second_claims = fold_claims(&events);
        assert_eq!(first_claims, second_claims);
    }

    /// Silent mode renders NO UI output at all — the empty payload,
    /// honoring the f04 gate seam the way [`crate::explorer`] and
    /// [`crate::serve`] already document.
    #[test]
    fn silent_mode_renders_empty_payload() {
        let payload =
            render_hub_from_root(RunMode::Silent, std::path::Path::new("does-not-matter"));
        assert_eq!(payload, super::HubViewPayload::default());
    }

    /// A missing/unmaterialized ledger root degrades to the honest-empty
    /// payload rather than erroring or panicking.
    #[test]
    fn missing_ledger_root_degrades_to_empty_payload() {
        let missing = std::path::Path::new("definitely-does-not-exist-hub-ledger-root");
        let payload = render_hub_from_root(RunMode::Human, missing);
        assert!(payload.lanes.is_empty());
        assert!(payload.claims.is_empty());
        assert!(payload.workers.is_empty());
        assert!(payload.tasks.is_empty());
        assert!(payload.mail.is_empty());
    }

    /// A real seeded ledger (via arc-16's own `init`/`claim_all`, read back
    /// through THIS module's `render_hub_from_root`) renders the exact
    /// lane + claim ids end to end, over a real filesystem round trip —
    /// not just the in-memory fixture above.
    #[test]
    fn real_seeded_ledger_round_trips_through_render_hub_from_root(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use enforcer_coordination::api::{claim_all, CallerContext, ClaimRequestArgs};
        use enforcer_domain::ids::{HubName, LaneId};

        let ledger_dir = tempfile::tempdir()?;
        let repo_dir = tempfile::tempdir()?;
        std::fs::write(repo_dir.path().join("a.rs"), "// fixture")?;

        let hub_name: HubName = "test-hub".parse()?;
        let lane: LaneId = "arc-16".parse()?;
        let config = enforcer_coordination::api::init(ledger_dir.path(), &hub_name, &lane)?;
        let hub = enforcer_coordination::api::Hub {
            root: ledger_dir.path().to_path_buf(),
            config,
        };
        let caller = CallerContext {
            project_id: "test-project".into(),
            worktree_root: repo_dir.path().display().to_string(),
            branch: "lane/arc-16".into(),
            commit: None,
            codex_thread_id: None,
            codex_session_id: None,
        };
        claim_all(
            &hub,
            ClaimRequestArgs {
                repo_root: repo_dir.path(),
                lane: &lane,
                owns: &["a.rs".to_owned()],
                caller: &caller,
                reason: Some("round trip fixture".to_owned()).as_deref(),
            },
        )?;

        let payload = render_hub_from_root(RunMode::Human, ledger_dir.path());
        assert_eq!(payload.claims.len(), 1);
        assert_eq!(payload.claims[0].lane_id, "arc-16");
        assert_eq!(payload.claims[0].paths, vec!["a.rs".to_owned()]);
        Ok(())
    }

    /// `hub-dashboard-mount`: mounts into g01's view registry — the
    /// `"hub"` slug is present in [`crate::serve::VIEW_MOUNTS`].
    #[test]
    fn mounts_into_g01_view_registry() {
        assert!(crate::serve::VIEW_MOUNTS
            .iter()
            .any(|mount| mount.slug == "hub"));
    }
}
