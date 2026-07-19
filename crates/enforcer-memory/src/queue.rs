//! X06.5: the weaver's event queue -- priority tiers, retry/backoff
//! bookkeeping, and the dead-letter queue.
//!
//! # Harvested-from
//!
//! Pattern harvested from TabAgentServer `Rust/weaver` (tokio MPSC +
//! worker-pool event queue), per `refs/x06-source-scout-digests.md` Â§2
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

use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{
    MemoryPriority, MemoryQueueEmpty, MemoryQueueExhausted, MemoryQueueLastError,
    MemoryQueueLength, MemoryRetryDelay, MemoryRetryMultiplier, MemoryTaskKey, RetryAttemptCount,
    WeaverContentHash, WeaverNodeId, WeaverRelativePath,
};

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
        node_id: WeaverNodeId,
        rel_path: WeaverRelativePath,
        content_hash: WeaverContentHash,
    },
    /// A file changed on disk -- triggers summary invalidation
    /// (distinct from `NodeChanged` because a comment-only edit can
    /// change a file's content hash without changing any symbol node,
    /// yet the file summary is still stale).
    FileChanged {
        rel_path: WeaverRelativePath,
        content_hash: WeaverContentHash,
    },
    /// A file was deleted (tombstoned) -- triggers associative-link
    /// cleanup and summary invalidation for the tombstone.
    FileDeleted { rel_path: WeaverRelativePath },
    /// Ask the associative linker to (re)compute 2-3 hop links from a
    /// node, per the MIA-framework "associative links" concept
    /// (digest Â§2).
    RelinkRequested { node_id: WeaverNodeId },
}

impl WeaverEvent {
    /// A stable key used for retry/dead-letter bookkeeping and for the
    /// hard-test fixtures to recognize "the same logical task" across
    /// a retry attempt.
    pub fn task_key(&self) -> MemoryTaskKey {
        match self {
            WeaverEvent::NodeChanged { node_id, .. } => format!("node-changed:{node_id}").into(),
            WeaverEvent::FileChanged { rel_path, .. } => format!("file-changed:{rel_path}").into(),
            WeaverEvent::FileDeleted { rel_path } => format!("file-deleted:{rel_path}").into(),
            WeaverEvent::RelinkRequested { node_id } => format!("relink:{node_id}").into(),
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
    pub priority: MemoryPriority,
    pub attempt: RetryAttemptCount,
}

/// A task that exhausted its retry budget. Recorded verbatim (never
/// summarized away) so an operator/diagnostic surface can see exactly
/// what failed and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedTask {
    pub event: WeaverEvent,
    pub attempts: RetryAttemptCount,
    pub last_error: MemoryQueueLastError,
}

/// Bounded exponential backoff schedule for task retries. Deterministic
/// (no jitter) so tests can assert exact delays without flaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: RetryAttemptCount,
    pub base_delay: MemoryRetryDelay,
    pub max_delay: MemoryRetryDelay,
}

impl RetryPolicy {
    pub const DEFAULT_MAX_ATTEMPTS: RetryAttemptCount = RetryAttemptCount::DEFAULT_LIMIT;

    /// A conservative default: 3 attempts total (1 initial + 2 retries),
    /// 50ms base doubling up to a 2s cap -- short enough that the hard
    /// tests run in well under a second, long enough to be a real
    /// backoff shape in production.
    pub fn bounded_default() -> Self {
        Self {
            max_attempts: Self::DEFAULT_MAX_ATTEMPTS,
            base_delay: MemoryRetryDelay::from_millis(50),
            max_delay: MemoryRetryDelay::from_secs(2),
        }
    }

    /// Delay before the given (1-based) retry attempt. `attempt = 1`
    /// is the delay before the FIRST retry (i.e. after the initial
    /// attempt, which is attempt 0, has failed).
    pub fn delay_for(&self, attempt: RetryAttemptCount) -> MemoryRetryDelay {
        let shift = u32::from(attempt).min(20); // avoid overflow on 1u64 << shift
        let scaled = self
            .base_delay
            .saturating_mul(MemoryRetryMultiplier::from(1u32 << shift));
        scaled.min(self.max_delay)
    }

    pub fn is_exhausted(&self, attempts_made: RetryAttemptCount) -> MemoryQueueExhausted {
        (attempts_made >= self.max_attempts).into()
    }
}

/// In-process priority queue over the three [`MemoryPriority`] tiers. Each
/// tier has a bounded channel; producers use `try_send`, so foreground work
/// never waits on enrichment and overload is reported explicitly.
#[derive(Debug)]
pub struct WeaverQueue {
    hot_tx: tokio::sync::mpsc::Sender<QueuedTask>,
    hot_rx: tokio::sync::mpsc::Receiver<QueuedTask>,
    warm_tx: tokio::sync::mpsc::Sender<QueuedTask>,
    warm_rx: tokio::sync::mpsc::Receiver<QueuedTask>,
    cold_tx: tokio::sync::mpsc::Sender<QueuedTask>,
    cold_rx: tokio::sync::mpsc::Receiver<QueuedTask>,
}

/// A cloneable handle for enqueuing work. Cheap to clone (just three
/// channel senders) and `Send + Sync`, so any number of producers
/// (indexer, watcher, MCP handlers) can hold one.
#[derive(Debug, Clone)]
pub struct WeaverQueueHandle {
    hot_tx: tokio::sync::mpsc::Sender<QueuedTask>,
    warm_tx: tokio::sync::mpsc::Sender<QueuedTask>,
    cold_tx: tokio::sync::mpsc::Sender<QueuedTask>,
}

const QUEUE_TIER_CAPACITY: usize = 256;

/// A non-blocking enqueue could not accept the task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum QueueSendError {
    #[error("weaver queue is closed")]
    Closed,
    #[error("weaver queue tier is full")]
    Full,
}

/// The queue is unavailable -- returned by
/// [`WeaverQueueHandle::send`] when the receiver side is gone.
impl WeaverQueueHandle {
    /// Enqueue `event` at `priority`. Non-blocking: this is a plain
    /// unbounded-channel `send`, never an `await`, so a foreground
    /// query path that also holds a handle can never stall behind
    /// enrichment work (see module docs).
    pub fn send(&self, event: WeaverEvent, priority: MemoryPriority) -> Result<(), QueueSendError> {
        let task = QueuedTask {
            event,
            priority,
            attempt: RetryAttemptCount::ZERO,
        };
        let result = match priority {
            MemoryPriority::Hot => self.hot_tx.try_send(task),
            MemoryPriority::Warm => self.warm_tx.try_send(task),
            MemoryPriority::Cold => self.cold_tx.try_send(task),
        };
        result.map_err(|error| queue_send_error(&error))
    }

    /// Re-enqueue a task that failed and is eligible for another
    /// attempt, preserving its original priority.
    pub fn retry(&self, task: QueuedTask) -> Result<(), QueueSendError> {
        let result = match task.priority {
            MemoryPriority::Hot => self.hot_tx.try_send(task),
            MemoryPriority::Warm => self.warm_tx.try_send(task),
            MemoryPriority::Cold => self.cold_tx.try_send(task),
        };
        result.map_err(|error| queue_send_error(&error))
    }
}

fn queue_send_error(error: &tokio::sync::mpsc::error::TrySendError<QueuedTask>) -> QueueSendError {
    match error {
        tokio::sync::mpsc::error::TrySendError::Closed(_) => QueueSendError::Closed,
        tokio::sync::mpsc::error::TrySendError::Full(_) => QueueSendError::Full,
    }
}

impl WeaverQueue {
    pub fn new() -> Self {
        let (hot_tx, hot_rx) = tokio::sync::mpsc::channel(QUEUE_TIER_CAPACITY);
        let (warm_tx, warm_rx) = tokio::sync::mpsc::channel(QUEUE_TIER_CAPACITY);
        let (cold_tx, cold_rx) = tokio::sync::mpsc::channel(QUEUE_TIER_CAPACITY);
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
            hot_tx: self.hot_tx.retained(),
            warm_tx: self.warm_tx.retained(),
            cold_tx: self.cold_tx.retained(),
        }
    }

    /// Receive the next task, preferring hot over warm over cold.
    /// `select`s all three so a hot task queued while warm/cold are
    /// being drained still wins as soon as this call runs again;
    /// returns `None` once every sender (including this queue's own,
    /// if dropped) is gone and all three tiers are empty.
    /// Cancellation is executable-proof covered by
    /// `recv_next_cancellation_preserves_queued_work`: dropping a pending
    /// receive leaves every channel ready for subsequent work.
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
        // CANCEL-SAFE: channel receive futures do not remove a task unless
        // their branch wins; dropping an unselected receive preserves it.
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

    pub fn len(&self) -> MemoryQueueLength {
        self.entries.len().into()
    }

    pub fn is_empty(&self) -> MemoryQueueEmpty {
        self.entries.is_empty().into()
    }

    /// Find dead-lettered entries for a given logical task key (see
    /// [`WeaverEvent::task_key`]).
    pub fn find(&self, task_key: &MemoryTaskKey) -> Vec<&FailedTask> {
        self.entries
            .iter()
            .filter(|f| f.event.task_key().as_str() == task_key.as_str())
            .collect()
    }
}
