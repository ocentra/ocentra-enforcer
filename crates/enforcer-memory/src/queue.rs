//! X06.5: the weaver's event queue -- priority tiers, retry/backoff
//! bookkeeping, and the dead-letter queue.
//!
//! # Harvested-from
//!
//! Pattern harvested from TabAgentServer `Rust/weaver` (tokio MPSC +
//! worker-pool event queue), per `refs/x06-source-scout-digests.md` §2
//! and `MEMORY_RETRIEVAL_DECISIONS.md` D-09. TabAgentServer's queue is a
//! single unbounded channel with no priority, no retry, and no
//! dead-letter path; every one of those is new work for enforcer,
//! written from scratch against enforcer's own event/error types (see
//! [`WeaverEvent`], [`FailedTask`]) -- nothing here is copied source.
//!
//! # Foreground-never-blocks (owner-set)
//!
//! [`WeaverQueue::send`] is a non-blocking, unbounded-channel `send`;
//! it never awaits a worker, never awaits I/O, and returns as soon as
//! the event is queued. This is the mechanical half of the owner-set
//! "the foreground answers queries, the background thinks" invariant --
//! see `tests/weaver_enrichment.rs::queue_processing_does_not_block_foreground_query`
//! for the concurrent proof.

use std::time::Duration;

/// Priority tier an event is enqueued at. Hot events (interactive,
/// just-edited files) are drained before warm, warm before cold --
/// see [`WeaverQueue::recv_next`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    /// Background housekeeping: periodic re-summarization sweeps,
    /// low-urgency associative link rebuilds.
    Cold,
    /// Normal enrichment triggered by an indexing pass that was not the
    /// file the user is actively looking at.
    Warm,
    /// Just-edited/just-created content the foreground is likely to ask
    /// about next.
    Hot,
}

/// The unit of work the weaver processes. Deliberately a flat enum
/// (mirrors [`crate::code_graph::CodeNode`]'s "flat, no nested edge
/// list" shape) so a new worker type is a new variant, not a new
/// parallel dispatch mechanism.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeaverEvent {
    /// A node (file, symbol, ...) was created or changed content --
    /// triggers the semantic indexer (embedding-task production) and
    /// the entity/symbol linker.
    NodeChanged {
        node_id: String,
        rel_path: String,
        content_hash: String,
    },
    /// A file changed on disk -- triggers summary invalidation
    /// (distinct from `NodeChanged` because a comment-only edit can
    /// change a file's content hash without changing any symbol node,
    /// yet the file summary is still stale).
    FileChanged {
        rel_path: String,
        content_hash: String,
    },
    /// A file was deleted (tombstoned) -- triggers associative-link
    /// cleanup and summary invalidation for the tombstone.
    FileDeleted { rel_path: String },
    /// Ask the associative linker to (re)compute 2-3 hop links from a
    /// node, per the MIA-framework "associative links" concept
    /// (digest §2).
    RelinkRequested { node_id: String },
}

impl WeaverEvent {
    /// A stable key used for retry/dead-letter bookkeeping and for the
    /// hard-test fixtures to recognize "the same logical task" across
    /// a retry attempt.
    pub fn task_key(&self) -> String {
        match self {
            WeaverEvent::NodeChanged { node_id, .. } => format!("node-changed:{node_id}"),
            WeaverEvent::FileChanged { rel_path, .. } => format!("file-changed:{rel_path}"),
            WeaverEvent::FileDeleted { rel_path } => format!("file-deleted:{rel_path}"),
            WeaverEvent::RelinkRequested { node_id } => format!("relink:{node_id}"),
        }
    }
}

/// One entry actually carried on the channel: the event plus its
/// priority and how many attempts have already been made (0 for a
/// fresh enqueue; incremented by [`crate::enrichment::WorkerPool`] on
/// each retry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedTask {
    pub event: WeaverEvent,
    pub priority: Priority,
    pub attempt: u32,
}

/// A task that exhausted its retry budget. Recorded verbatim (never
/// summarized away) so an operator/diagnostic surface can see exactly
/// what failed and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedTask {
    pub event: WeaverEvent,
    pub attempts: u32,
    pub last_error: String,
}

/// Bounded exponential backoff schedule for task retries. Deterministic
/// (no jitter) so tests can assert exact delays without flaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl RetryPolicy {
    pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;

    /// A conservative default: 3 attempts total (1 initial + 2 retries),
    /// 50ms base doubling up to a 2s cap -- short enough that the hard
    /// tests run in well under a second, long enough to be a real
    /// backoff shape in production.
    pub fn bounded_default() -> Self {
        Self {
            max_attempts: Self::DEFAULT_MAX_ATTEMPTS,
            base_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(2),
        }
    }

    /// Delay before the given (1-based) retry attempt. `attempt = 1`
    /// is the delay before the FIRST retry (i.e. after the initial
    /// attempt, which is attempt 0, has failed).
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let shift = attempt.min(20); // avoid overflow on 1u64 << shift
        let scaled = self.base_delay.saturating_mul(1u32 << shift);
        scaled.min(self.max_delay)
    }

    pub fn is_exhausted(&self, attempts_made: u32) -> bool {
        attempts_made >= self.max_attempts
    }
}

/// In-process priority queue over the three [`Priority`] tiers, backed
/// by one `tokio::sync::mpsc::unbounded_channel` per tier so `send`
/// never blocks the caller (owner-set: foreground never waits on
/// enrichment) regardless of how deep any tier's backlog is.
#[derive(Debug)]
pub struct WeaverQueue {
    hot_tx: tokio::sync::mpsc::UnboundedSender<QueuedTask>,
    hot_rx: tokio::sync::mpsc::UnboundedReceiver<QueuedTask>,
    warm_tx: tokio::sync::mpsc::UnboundedSender<QueuedTask>,
    warm_rx: tokio::sync::mpsc::UnboundedReceiver<QueuedTask>,
    cold_tx: tokio::sync::mpsc::UnboundedSender<QueuedTask>,
    cold_rx: tokio::sync::mpsc::UnboundedReceiver<QueuedTask>,
}

/// A cloneable handle for enqueuing work. Cheap to clone (just three
/// channel senders) and `Send + Sync`, so any number of producers
/// (indexer, watcher, MCP handlers) can hold one.
#[derive(Debug, Clone)]
pub struct WeaverQueueHandle {
    hot_tx: tokio::sync::mpsc::UnboundedSender<QueuedTask>,
    warm_tx: tokio::sync::mpsc::UnboundedSender<QueuedTask>,
    cold_tx: tokio::sync::mpsc::UnboundedSender<QueuedTask>,
}

/// The queue is closed (all handles dropped) -- returned by
/// [`WeaverQueueHandle::send`] when the receiver side is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("weaver queue is closed")]
pub struct QueueClosed;

impl WeaverQueueHandle {
    /// Enqueue `event` at `priority`. Non-blocking: this is a plain
    /// unbounded-channel `send`, never an `await`, so a foreground
    /// query path that also holds a handle can never stall behind
    /// enrichment work (see module docs).
    pub fn send(&self, event: WeaverEvent, priority: Priority) -> Result<(), QueueClosed> {
        let task = QueuedTask {
            event,
            priority,
            attempt: 0,
        };
        let result = match priority {
            Priority::Hot => self.hot_tx.send(task),
            Priority::Warm => self.warm_tx.send(task),
            Priority::Cold => self.cold_tx.send(task),
        };
        // `SendError<QueuedTask>` carries the un-sent task back, which
        // is not itself an "original error to preserve as a source" --
        // the task is already owned by the caller's `Result::Err` path
        // in the form of the original `event`/`priority` arguments they
        // still have, and `QueueClosed` is a plain, sourceless marker
        // (there is exactly one reason a bounded set of senders can
        // fail to send: every receiver is gone).
        result.map_err(|_send_error| QueueClosed)
    }

    /// Re-enqueue a task that failed and is eligible for another
    /// attempt, preserving its original priority.
    pub fn retry(&self, task: QueuedTask) -> Result<(), QueueClosed> {
        let result = match task.priority {
            Priority::Hot => self.hot_tx.send(task),
            Priority::Warm => self.warm_tx.send(task),
            Priority::Cold => self.cold_tx.send(task),
        };
        result.map_err(|_send_error| QueueClosed)
    }
}

impl WeaverQueue {
    pub fn new() -> Self {
        let (hot_tx, hot_rx) = tokio::sync::mpsc::unbounded_channel();
        let (warm_tx, warm_rx) = tokio::sync::mpsc::unbounded_channel();
        let (cold_tx, cold_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            hot_tx,
            hot_rx,
            warm_tx,
            warm_rx,
            cold_tx,
            cold_rx,
        }
    }

    /// A cloneable producer handle sharing this queue's channels.
    pub fn handle(&self) -> WeaverQueueHandle {
        WeaverQueueHandle {
            hot_tx: self.hot_tx.clone(),
            warm_tx: self.warm_tx.clone(),
            cold_tx: self.cold_tx.clone(),
        }
    }

    /// Receive the next task, preferring hot over warm over cold.
    /// `select`s all three so a hot task queued while warm/cold are
    /// being drained still wins as soon as this call runs again;
    /// returns `None` once every sender (including this queue's own,
    /// if dropped) is gone and all three tiers are empty.
    pub async fn recv_next(&mut self) -> Option<QueuedTask> {
        // Drain strictly in priority order: only look at warm/cold if
        // hot has nothing *ready right now*, per D-09's hot/warm/cold
        // priority requirement. `try_recv` is non-blocking so this
        // never favors warm/cold just because hot happened to be empty
        // at a race-prone instant while a hot sender is mid-`send`.
        if let Ok(task) = self.hot_rx.try_recv() {
            return Some(task);
        }
        if let Ok(task) = self.warm_rx.try_recv() {
            return Some(task);
        }
        if let Ok(task) = self.cold_rx.try_recv() {
            return Some(task);
        }
        // Nothing ready in any tier right now -- await whichever
        // channel produces next (or all-closed). Every branch settles
        // this call's result directly (there is nothing left to retry
        // afterward): a task arriving on a lower tier while this await
        // is pending still loses to a hot task per `biased`, and the
        // caller's own loop (`WorkerPool::spawn`'s drain loop) is what
        // calls `recv_next` again for the next task, not this method.
        tokio::select! {
            biased;
            task = self.hot_rx.recv() => task,
            task = self.warm_rx.recv() => task,
            task = self.cold_rx.recv() => task,
        }
    }
}

impl Default for WeaverQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// The dead-letter queue: an in-process, append-only record of tasks
/// that exhausted their retry budget. TabAgentServer's weaver has no
/// equivalent -- this is new work per D-09/the X06.5 pack.
#[derive(Debug, Clone, Default)]
pub struct DeadLetterQueue {
    entries: Vec<FailedTask>,
}

impl DeadLetterQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, failed: FailedTask) {
        self.entries.push(failed);
    }

    pub fn entries(&self) -> &[FailedTask] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Find dead-lettered entries for a given logical task key (see
    /// [`WeaverEvent::task_key`]).
    pub fn find(&self, task_key: &str) -> Vec<&FailedTask> {
        self.entries
            .iter()
            .filter(|f| f.event.task_key() == task_key)
            .collect()
    }
}
