//! X06.7 (D-12, DEFAULT): filesystem watcher with debounce + git-state
//! polling fallback.
//!
//! Improves on the codebase-memory-mcp baseline's pure-polling watcher
//! (`refs/x06-baseline-tool-schemas.md`: background thread polls git per
//! project; adaptive 5s base + 1s/500 files, cap 60s) by using the
//! `notify` crate's native OS filesystem events (inotify/FSEvents/
//! ReadDirectoryChangesW) as the primary signal, with the baseline's
//! adaptive-polling strategy kept as an explicit, always-available
//! fallback mode for filesystems/CI environments where native events are
//! unreliable (D-12's own revisit-trigger: "cross-platform flakiness in
//! CI -> fall back to adaptive polling like baseline").
//!
//! # Debounce
//!
//! A burst of filesystem events for the same path within
//! [`Watcher::debounce_window`] collapses to exactly ONE
//! [`ReindexRequest`] emitted on the channel -- editors and git checkouts
//! routinely fire several events per logical change (write + rename +
//! metadata touch), and the hard test ("watcher detects a file change and
//! emits exactly one debounced reindex request") requires this collapsing
//! to be exact, not approximate.
//!
//! # Incremental path
//!
//! Every [`ReindexRequest`] carries the specific changed paths (not just
//! "something changed") so a consumer (the future weaver/index-refresh
//! wiring) can feed them straight into
//! [`crate::code_graph::CodeGraph::index_repository`]'s existing manifest-
//! diff incremental path (D-02: "indexes are disposable... never rebuild
//! blindly") rather than re-walking the whole repository on every event.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use crate::boundary::watch::ReindexRequest;
use enforcer_domain::memory_types::{
    MemoryWatchDeadline, MemoryWatchDebounceWindow, MemoryWatchEventRelevant, MemoryWatchFileCount,
    MemoryWatchGitHead, MemoryWatchGitHeadChanged, MemoryWatchPollInterval, MemoryWatchRoot,
};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcherTrait};

/// Errors constructing or running a [`Watcher`].
#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("failed to start filesystem watcher: {0}")]
    Notify(#[from] notify::Error),
    #[error("watch channel disconnected")]
    ChannelClosed,
}

/// A running filesystem watcher over one root directory. Owns the
/// underlying `notify` watcher (kept alive for the struct's lifetime --
/// dropping it stops delivery) and the debounce accumulator; emits
/// [`ReindexRequest`]s on an internal channel drained by
/// [`Watcher::next_reindex_request`].
pub struct Watcher {
    _inner: RecommendedWatcher,
    // BRAND-INVARIANT: this receiver carries only paths emitted by the validated notify callback.
    raw_events: Receiver<PathBuf>,
    debounce_window: MemoryWatchDebounceWindow,
}

impl std::fmt::Debug for Watcher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Watcher")
            .field("inner", &"RecommendedWatcher")
            .field("raw_events", &"Receiver<MemoryWatchPath>")
            .field("debounce_window", &self.debounce_window)
            .finish()
    }
}

impl Watcher {
    /// Start watching `root` recursively. `debounce_window` is how long to
    /// wait after the last event in a burst before emitting one
    /// [`ReindexRequest`] for everything collapsed into that burst.
    pub fn start(
        root: impl Into<MemoryWatchRoot>,
        debounce_window: impl Into<MemoryWatchDebounceWindow>,
    ) -> Result<Self, WatchError> {
        let root = root.into();
        let (tx, rx): (Sender<PathBuf>, Receiver<PathBuf>) = mpsc::channel();
        let mut inner = notify::recommended_watcher(move |event: notify::Result<Event>| {
            let Ok(event) = event else {
                return;
            };
            if !bool::from(is_relevant_event(&event.kind)) {
                return;
            }
            for path in event.paths {
                // A closed receiver (watcher outliving its consumer, e.g.
                // during shutdown) is not a panic -- best-effort delivery.
                if tx.send(path).is_err() {
                    break;
                }
            }
        })?;
        inner.watch(root.as_path(), RecursiveMode::Recursive)?;
        Ok(Self {
            _inner: inner,
            raw_events: rx,
            debounce_window: debounce_window.into(),
        })
    }

    /// Block until either a debounced [`ReindexRequest`] is ready or
    /// `deadline` elapses with no events at all (returns `Ok(None)` in the
    /// latter case). `deadline` exists ONLY as a deadlock guard for tests
    /// and callers that need to poll other work between waits -- it is
    /// never used as a substitute for the debounce window's own timing,
    /// which is computed relative to the last-seen event, not this call's
    /// start time (workpack instruction: "no sleeps as sync -- use notify
    /// events + timeout-as-deadlock-guard only").
    pub fn next_reindex_request(
        &self,
        deadline: impl Into<MemoryWatchDeadline>,
    ) -> Result<Option<ReindexRequest>, WatchError> {
        let deadline_at = WatchDeadlineAt(Instant::now() + deadline.into().get());
        let mut collected: HashSet<PathBuf> = HashSet::new();

        // Wait for the FIRST event in a new burst, honoring the overall
        // deadline guard.
        let first = match self.raw_events.recv_timeout(remaining(deadline_at)) {
            Ok(path) => path,
            Err(RecvTimeoutError::Timeout) => return Ok(None),
            Err(RecvTimeoutError::Disconnected) => return Err(WatchError::ChannelClosed),
        };
        collected.insert(first);

        // Keep draining while events keep arriving within the debounce
        // window of the MOST RECENT event (not the deadline) -- this is
        // what collapses an editor's write+rename+touch burst into one
        // request regardless of how long the burst itself runs, while
        // still being bounded overall by `deadline_at` as a safety cap.
        loop {
            let wait_for = self.debounce_window.get().min(remaining(deadline_at));
            if wait_for.is_zero() {
                break;
            }
            match self.raw_events.recv_timeout(wait_for) {
                Ok(path) => {
                    collected.insert(path);
                }
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return Err(WatchError::ChannelClosed),
            }
        }

        let mut paths: Vec<PathBuf> = collected.into_iter().collect();
        paths.sort();
        Ok(Some(ReindexRequest {
            paths: paths.into_iter().map(Into::into).collect(),
        }))
    }
}

#[derive(Debug, Clone, Copy)]
struct WatchDeadlineAt(Instant);

fn remaining(deadline_at: WatchDeadlineAt) -> Duration {
    deadline_at.0.saturating_duration_since(Instant::now())
}

/// Filter native OS events down to the ones that matter for reindexing:
/// content modifications, creates, removes, and renames. Metadata-only
/// "access" events (read, permission-check) are noise for a code-graph
/// watcher and are dropped here rather than triggering a spurious
/// reindex.
pub fn is_relevant_event(kind: &EventKind) -> MemoryWatchEventRelevant {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
    .into()
}

/// The baseline-style adaptive polling fallback (D-12 revisit-trigger
/// path): computes a poll interval that grows with corpus size, capped,
/// matching the scout-documented baseline formula ("adaptive 5s base +
/// 1s/500 files, cap 60s") so a caller that must fall back to polling
/// (native watcher unavailable, or CI flakiness observed) gets the same
/// interval discipline rather than inventing a new one.
pub fn adaptive_poll_interval(
    file_count: impl Into<MemoryWatchFileCount>,
) -> MemoryWatchPollInterval {
    let file_count = file_count.into().get();
    const BASE: Duration = Duration::from_secs(5);
    const PER_FILES: usize = 500;
    const STEP: Duration = Duration::from_secs(1);
    const CAP: Duration = Duration::from_secs(60);
    let file_steps = u32::try_from(file_count / PER_FILES).unwrap_or(u32::MAX);
    let extra = STEP.saturating_mul(file_steps);
    BASE.saturating_add(extra).min(CAP).into()
}

/// Poll a git repository's current HEAD commit, returning `true` if it
/// differs from `previous_head`. This is the git-state half of D-12's
/// "polling fallback" -- distinct from filesystem events, it catches
/// changes a watcher might miss across a process restart (e.g. `git
/// checkout` while the watcher was not running) using the same read-only
/// [`crate::git::GitMetadata`] this crate's indexer already depends on.
pub fn git_head_changed(
    root: impl Into<MemoryWatchRoot>,
    previous_head: Option<&MemoryWatchGitHead>,
) -> MemoryWatchGitHeadChanged {
    let root = root.into();
    let Ok(Some(metadata)) = crate::git::GitMetadata::open(root.as_path()) else {
        return false.into();
    };
    (metadata.head_commit().as_deref() != previous_head.map(|head| head.as_str())).into()
}
