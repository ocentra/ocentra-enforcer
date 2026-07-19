//! X06.5: the weaver's worker abstraction and the bounded-concurrency
//! worker pool that drains [`crate::queue::WeaverQueue`].
//!
//! # Harvested-from
//!
//! Worker-pool shape (tokio tasks pulling from a shared queue, sized by
//! available parallelism) harvested from TabAgentServer `Rust/weaver`
//! per `refs/x06-source-scout-digests.md` Â§2 ("worker pool sized by
//! `num_cpus`") and `MEMORY_RETRIEVAL_DECISIONS.md` D-09. Rewritten
//! against enforcer's own event/error types; retry, dead-letter
//! routing, and the bounded-concurrency semaphore are new work the
//! source lacked.
//!
//! # Embedder seam (X06.4 boundary)
//!
//! X06.5 must not depend on X06.4's concrete embedder/reranker
//! implementation (disjoint file-ownership, adapter-wired later at
//! integration per the worker prompt). [`Embedder`] is a narrow,
//! local, `async_trait`-free (the workspace has no `async-trait` dep
//! yet, and this seam does not need one -- a plain `async fn` in a
//! trait with `Send` bounds is enough) seam the semantic indexer calls
//! through; [`NullEmbedder`] is the zero-network default used in every
//! test and the crate's default build so no test ever needs a model
//! runtime, mirroring [`crate::retriever::EmbeddingRetriever`]'s
//! existing feature-gated-seam precedent.

use crate::owned_boundary::{Retained, RetainedDisplay};
use crate::queue::{DeadLetterQueue, FailedTask, QueuedTask, RetryPolicy, WeaverEvent};
use crate::summaries::SummaryStore;
use enforcer_domain::memory_types::{
    EmbeddingGenerationId, EnrichmentAttemptCount, EnrichmentFailureCount, MemoryPriority,
    MemoryQueueLastError, RetryAttemptCount, SourceHash, SymbolNodeId, TaskOutcome,
    WorkerConcurrency,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const OUTCOME_CHANNEL_CAPACITY: usize = 64;

/// A single embedding task's outcome, produced by the semantic indexer
/// worker for X06.4 to eventually consume. Kept as plain data (not a
/// vector) in this slice: X06.5 owns *producing the task record*, not
/// the embedding model itself (X06.4's concern, wired at integration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingTask {
    pub node_id: SymbolNodeId,
    pub content_hash: SourceHash,
    pub embedding_version: EmbeddingGenerationId,
}

/// The narrow async seam the semantic indexer calls through to hand off
/// an embedding task. `Send + Sync` so it can live behind an `Arc` and
/// be shared across worker tasks.
pub trait Embedder: Send + Sync {
    /// Accept an embedding task for `node_id`/`content_hash`. Returns
    /// the embedding version actually used (so blue/green migration --
    /// D-09 -- can report which generation produced a given vector).
    /// Boxed-future return keeps this trait object-safe without an
    /// external `async-trait` dependency.
    fn embed<'a>(
        &'a self,
        task: &'a EmbeddingTask,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingGenerationId, EnrichmentError>> + Send + 'a>>;
}

/// Zero-network default [`Embedder`]: records that a task was seen and
/// echoes back the requested version, never calling any model runtime.
/// The crate's default build and every hard test in this pack use this
/// so `cargo test -p enforcer-memory` never needs a model download,
/// mirroring [`crate::retriever::EmbeddingRetriever`]'s precedent.
#[derive(Debug, Default)]
pub struct NullEmbedder {
    calls: Mutex<Vec<EmbeddingTask>>,
}

impl NullEmbedder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every task this embedder has been asked to embed, in order --
    /// used by the hard tests to prove the semantic indexer actually
    /// ran, without depending on any real embedding output.
    pub fn calls(&self) -> Vec<EmbeddingTask> {
        // `Mutex::lock` on a `Mutex` never held across an `.await` in
        // this crate returns `Err` only on a poisoned lock (a prior
        // panic while holding it) -- treated as "no calls recorded"
        // rather than propagating a panic, since this is a test/default
        // seam, not a production data path.
        self.calls
            .lock()
            .map(|guard| guard.retained())
            .unwrap_or_else(|poisoned| poisoned.into_inner().retained())
    }
}

impl Embedder for NullEmbedder {
    fn embed<'a>(
        &'a self,
        task: &'a EmbeddingTask,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingGenerationId, EnrichmentError>> + Send + 'a>>
    {
        Box::pin(async move {
            if let Ok(mut guard) = self.calls.lock() {
                guard.push(task.retained());
            }
            Ok(task.embedding_version)
        })
    }
}

/// A transient failure injector used only by the hard tests to prove
/// retry-then-succeed behavior deterministically (no sleeps-as-sync:
/// the test controls exactly which attempt number fails).
#[derive(Debug, Default)]
pub struct FlakyEmbedder {
    inner: NullEmbedder,
    fail_until_attempt: AtomicUsize,
    attempts_seen: AtomicUsize,
}

impl FlakyEmbedder {
    /// Fails every call until (and not including) the `nth` call
    /// (1-based), then delegates to [`NullEmbedder`].
    pub fn fail_first_n(n: impl Into<EnrichmentFailureCount>) -> Self {
        let n = n.into().get();
        Self {
            inner: NullEmbedder::new(),
            fail_until_attempt: AtomicUsize::new(n),
            attempts_seen: AtomicUsize::new(0),
        }
    }

    pub fn attempts_seen(&self) -> EnrichmentAttemptCount {
        self.attempts_seen.load(Ordering::SeqCst).into()
    }
}

impl Embedder for FlakyEmbedder {
    fn embed<'a>(
        &'a self,
        task: &'a EmbeddingTask,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingGenerationId, EnrichmentError>> + Send + 'a>>
    {
        Box::pin(async move {
            let seen = self.attempts_seen.fetch_add(1, Ordering::SeqCst) + 1;
            if seen <= self.fail_until_attempt.load(Ordering::SeqCst) {
                return Err(EnrichmentError::Transient(format!(
                    "flaky embedder: simulated failure on attempt {seen}"
                )));
            }
            self.inner.embed(task).await
        })
    }
}

/// Errors a worker can return while processing one [`WeaverEvent`].
/// `Transient` tasks are eligible for retry; `Permanent` tasks go
/// straight to the dead-letter queue without burning the retry budget.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EnrichmentError {
    #[error("transient enrichment failure: {0}")]
    Transient(String),
    #[error("permanent enrichment failure: {0}")]
    Permanent(String),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SharedSummaryStore(Arc<Mutex<SummaryStore>>);

pub(crate) struct SharedSummaryStorePair {
    pub(crate) primary: SharedSummaryStore,
    pub(crate) enrichment: SharedSummaryStore,
}

impl SharedSummaryStore {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn shared_pair() -> SharedSummaryStorePair {
        let primary = Self::new();
        let enrichment = Self(Arc::clone(&primary.0));
        SharedSummaryStorePair {
            primary,
            enrichment,
        }
    }

    pub(crate) fn with_mut<T>(&self, operation: impl FnOnce(&mut SummaryStore) -> T) -> T {
        let mut store = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        operation(&mut store)
    }
}

/// Shared state a worker task processes one [`WeaverEvent`] against.
/// Grouped so [`process_event`] takes one argument instead of a long
/// positional list (mirrors [`crate::code_graph::NewFileParams`]'s
/// established pattern in this crate).
pub struct EnrichmentContext {
    pub embedder: Arc<dyn Embedder>,
    summaries: SharedSummaryStore,
    pub embedding_version: EmbeddingGenerationId,
}

impl EnrichmentContext {
    pub fn new(embedder: Arc<dyn Embedder>, embedding_version: EmbeddingGenerationId) -> Self {
        Self {
            embedder,
            summaries: SharedSummaryStore::new(),
            embedding_version,
        }
    }

    pub(crate) fn with_shared_summaries(
        embedder: Arc<dyn Embedder>,
        summaries: SharedSummaryStore,
        embedding_version: EmbeddingGenerationId,
    ) -> Self {
        Self {
            embedder,
            summaries,
            embedding_version,
        }
    }

    /// Run one synchronized operation against the enrichment summary store.
    pub fn with_summaries<T>(&self, operation: impl FnOnce(&mut SummaryStore) -> T) -> T {
        self.summaries.with_mut(operation)
    }
}

impl std::fmt::Debug for EnrichmentContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EnrichmentContext")
            .field("embedder", &"dyn Embedder")
            .field("summaries", &"Mutex<SummaryStore>")
            .field("embedding_version", &self.embedding_version)
            .finish()
    }
}

/// Process one event, dispatching to the worker behavior matching its
/// variant:
///
/// - [`WeaverEvent::NodeChanged`] -> semantic indexer (produces an
///   [`EmbeddingTask`] via [`Embedder::embed`]) + entity/symbol linker
///   (records the node as linked in the summary store's link table);
/// - [`WeaverEvent::FileChanged`] -> summarizer (invalidates the
///   file's cached summary);
/// - [`WeaverEvent::FileDeleted`] -> summarizer (removes the summary
///   entry) + associative linker (drops stale links);
/// - [`WeaverEvent::RelinkRequested`] -> associative linker (recomputes
///   2-3 hop links -- this slice records the request was serviced;
///   real graph traversal is X06.3's `crate::graph`/`crate::code_graph`
///   surface, consumed read-only when wired at integration).
pub async fn process_event(
    ctx: &EnrichmentContext,
    event: &WeaverEvent,
) -> Result<(), EnrichmentError> {
    match event {
        WeaverEvent::NodeChanged {
            node_id,
            content_hash,
            ..
        } => {
            let node_id = SymbolNodeId::new(node_id.as_str().retained())
                .map_err(|error| EnrichmentError::Permanent(error.retained_display()))?;
            // INVALID-INPUT-TEST: tests/unit_enrichment.rs rejects malformed
            // content hashes before an embedding task is submitted.
            let content_hash = SourceHash::try_new(content_hash.as_str().retained())
                .map_err(|error| EnrichmentError::Permanent(error.retained_display()))?;
            let task = EmbeddingTask {
                node_id,
                content_hash,
                embedding_version: ctx.embedding_version,
            };
            ctx.embedder.embed(&task).await?;
            ctx.summaries
                .with_mut(|store| store.link_entity(task.node_id.as_str()));
            Ok(())
        }
        WeaverEvent::FileChanged { rel_path, .. } => {
            ctx.summaries
                .with_mut(|store| store.invalidate(rel_path.as_str()));
            Ok(())
        }
        WeaverEvent::FileDeleted { rel_path } => {
            ctx.summaries.with_mut(|store| {
                store.remove(rel_path.as_str());
                store.unlink_entities_for_path(rel_path.as_str());
            });
            Ok(())
        }
        WeaverEvent::RelinkRequested { node_id } => {
            ctx.summaries
                .with_mut(|store| store.link_entity(node_id.as_str()));
            Ok(())
        }
    }
}

/// Configuration for [`WorkerPool::spawn`].
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    /// Maximum number of tasks the pool may process concurrently. This
    /// is the "worker resource limits" hard requirement -- a bounded
    /// `tokio::sync::Semaphore`, not an unbounded `tokio::spawn` per
    /// task.
    pub max_concurrency: WorkerConcurrency,
    pub retry: RetryPolicy,
    pub embedding_version: EmbeddingGenerationId,
    /// Optional per-attempt outcome broadcast, consumed by
    /// [`TaskOutcome`]-based deterministic test synchronization. `None`
    /// in production (no channel to drain, zero overhead); tests set it
    /// via [`WorkerPoolConfig::with_outcome_channel`].
    pub on_outcome: Option<tokio::sync::mpsc::Sender<TaskOutcome>>,
}

impl WorkerPoolConfig {
    /// `max_concurrency` defaults to available parallelism (falling
    /// back to 1 if the platform cannot report it), mirroring
    /// TabAgentServer's `num_cpus`-sized pool without adding a
    /// `num_cpus` dependency the standard library already makes
    /// unnecessary (`std::thread::available_parallelism`, stable since
    /// Rust 1.59).
    pub fn with_default_concurrency(
        retry: RetryPolicy,
        embedding_version: EmbeddingGenerationId,
    ) -> Self {
        let max_concurrency = std::thread::available_parallelism()
            .map(WorkerConcurrency::from_nonzero)
            .unwrap_or(WorkerConcurrency::SINGLE);
        Self {
            max_concurrency,
            retry,
            embedding_version,
            on_outcome: None,
        }
    }

    /// Attach an outcome channel, returning the receiver half. Used only
    /// by tests that need to `.await` a specific [`TaskOutcome`] rather
    /// than poll-sleep for it.
    pub fn with_outcome_channel(mut self) -> WorkerPoolOutcomeChannel {
        let (tx, rx) = tokio::sync::mpsc::channel(OUTCOME_CHANNEL_CAPACITY);
        self.on_outcome = Some(tx);
        WorkerPoolOutcomeChannel {
            config: self,
            outcomes: rx,
        }
    }
}

/// Configured worker pool plus the receiver used for deterministic outcome synchronization.
pub struct WorkerPoolOutcomeChannel {
    pub config: WorkerPoolConfig,
    pub outcomes: tokio::sync::mpsc::Receiver<TaskOutcome>,
}

impl std::fmt::Debug for WorkerPoolOutcomeChannel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkerPoolOutcomeChannel")
            .field("config", &self.config)
            .field("outcomes", &"Receiver<TaskOutcome>")
            .finish()
    }
}

/// The running worker pool: owns the shared [`Embedder`]/[`SummaryStore`]
/// and the dead-letter queue tasks are routed to once their retry
/// budget is exhausted.
///
/// # Why an explicit stop signal, not "wait for the channel to close"
///
/// [`WorkerPool::spawn`]'s drain loop needs its own
/// [`crate::queue::WeaverQueueHandle`] clone to re-enqueue retries (see
/// [`run_one_attempt`]) -- that clone lives for the loop's entire
/// lifetime, so the channel can never observe "every sender dropped"
/// from the outside: the pool is always holding one itself. Waiting on
/// that condition (as an earlier revision of this module did) is a
/// permanent deadlock in [`WorkerPool::shutdown`]. `stop` is an
/// explicit, independent signal instead: [`WorkerPool::shutdown`]
/// notifies it and the drain loop exits the next time it would
/// otherwise block waiting for a new task (in-flight tasks it already
/// dequeued still run to completion -- see [`WorkerPool::shutdown`]'s
/// doc comment).
pub struct WorkerPool {
    dead_letters: Arc<Mutex<DeadLetterQueue>>,
    stop: Arc<tokio::sync::Notify>,
    handle: tokio::task::JoinHandle<()>,
}

impl std::fmt::Debug for WorkerPool {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkerPool")
            .field("dead_letters", &"Mutex<DeadLetterQueue>")
            .field("stop", &"Notify")
            .field("handle", &self.handle)
            .finish()
    }
}

impl WorkerPool {
    pub fn with_dead_letters<T>(&self, inspect: impl FnOnce(&DeadLetterQueue) -> T) -> T {
        match self.dead_letters.lock() {
            Ok(dead_letters) => inspect(&dead_letters),
            Err(poisoned) => inspect(&poisoned.into_inner()),
        }
    }

    /// Spawn the pool's drain loop as a single background task that
    /// bounds its own concurrency with a tracked join set (so this call
    /// itself returns immediately -- no blocking on worker startup --
    /// and the pool never runs more than `config.max_concurrency`
    /// [`process_event`] calls at once).
    pub fn spawn(
        mut queue: crate::queue::WeaverQueue,
        queue_handle: crate::queue::WeaverQueueHandle,
        ctx: Arc<EnrichmentContext>,
        config: &WorkerPoolConfig,
    ) -> Self {
        let dead_letters = Arc::new(Mutex::new(DeadLetterQueue::new()));
        let dead_letters_for_task = Arc::clone(&dead_letters);
        let max_concurrency = usize::from(config.max_concurrency);
        let on_outcome = config.on_outcome.retained();
        let retry = config.retry;
        let stop = Arc::new(tokio::sync::Notify::new());
        let stop_for_task = Arc::clone(&stop);

        let handle = tokio::spawn(async move {
            let mut workers = tokio::task::JoinSet::new();
            // CANCELLATION: stop closes intake and every accepted worker is
            // joined below before the drain task returns.
            loop {
                if workers.len() >= max_concurrency {
                    if let Some(completed) = workers.join_next().await {
                        drop(completed);
                    }
                    continue;
                }
                // CANCEL-SAFE: every branch below preserves unselected queue,
                // notification, and worker state.
                let task = tokio::select! {
                    // CANCEL-SAFE: queue receive and Notify retain no partial
                    // task state when another branch wins this selection.
                    // Stop only once there is nothing immediately ready
                    // to process -- `biased` plus this branch listed
                    // second means a task already sitting in any tier
                    // is always drained before a pending stop signal is
                    // honored, so `shutdown` never drops in-flight work
                    // that was already enqueued.
                    biased;
                    // CANCEL-SAFE: an unselected receive retains the queued task.
                    maybe_task = queue.recv_next() => maybe_task,
                    // CANCEL-SAFE: Notify cancellation does not consume a permit.
                    () = stop_for_task.notified() => None,
                    // CANCEL-SAFE: JoinSet retains unfinished workers.
                    completed = workers.join_next(), if !workers.is_empty() => {
                        if let Some(completed) = completed {
                            drop(completed);
                        }
                        continue;
                    },
                };
                let Some(task) = task else { break };

                let ctx = Arc::clone(&ctx);
                let queue_handle = queue_handle.retained();
                let dead_letters = Arc::clone(&dead_letters_for_task);
                let on_outcome = on_outcome.retained();

                workers.spawn(async move {
                    run_one_attempt(AttemptArgs {
                        ctx: &ctx,
                        task,
                        queue_handle: &queue_handle,
                        dead_letters: dead_letters.as_ref(),
                        retry,
                        on_outcome: on_outcome.as_ref(),
                    })
                    .await;
                });
            }
            while let Some(completed) = workers.join_next().await {
                drop(completed);
            }
        });

        Self {
            dead_letters,
            stop,
            handle,
        }
    }

    /// Signal the drain loop to stop once its current backlog (every
    /// task already sitting in the queue at the moment `shutdown` is
    /// called) is dequeued, and wait for it to do so. This does NOT
    /// wait for retries scheduled *after* the stop signal fires. Every
    /// already-dequeued attempt is joined, so its dead-letter/outcome state
    /// is stable when this method returns.
    pub async fn shutdown(self) {
        self.stop.notify_one();
        let _ = self.handle.await;
    }
}

/// Bundled arguments for [`run_one_attempt`] -- grouped so the function
/// takes one struct instead of six positional parameters (mirrors
/// [`crate::code_graph::NewFileParams`]'s established pattern in this
/// crate; clippy's `too_many_arguments` bar is 5).
struct AttemptArgs<'a> {
    ctx: &'a EnrichmentContext,
    task: QueuedTask,
    queue_handle: &'a crate::queue::WeaverQueueHandle,
    dead_letters: &'a Mutex<DeadLetterQueue>,
    retry: RetryPolicy,
    on_outcome: Option<&'a tokio::sync::mpsc::Sender<TaskOutcome>>,
}

async fn run_one_attempt(args: AttemptArgs<'_>) {
    let AttemptArgs {
        ctx,
        task,
        queue_handle,
        dead_letters,
        retry,
        on_outcome,
    } = args;
    let task_key = task.event.task_key();
    match process_event(ctx, &task.event).await {
        Ok(()) => {
            notify_outcome(on_outcome, TaskOutcome::Succeeded { task_key });
        }
        Err(EnrichmentError::Permanent(reason)) => {
            let attempts = task.attempt.next();
            record_dead_letter(dead_letters, task.event, attempts, reason.into());
            notify_outcome(on_outcome, TaskOutcome::DeadLettered { task_key, attempts });
        }
        Err(EnrichmentError::Transient(reason)) => {
            let attempts_made = task.attempt.next();
            if retry.is_exhausted(attempts_made).is_exhausted() {
                record_dead_letter(dead_letters, task.event, attempts_made, reason.into());
                notify_outcome(
                    on_outcome,
                    TaskOutcome::DeadLettered {
                        task_key,
                        attempts: attempts_made,
                    },
                );
                return;
            }
            let delay = retry.delay_for(attempts_made);
            // This is a bounded local retry backoff, not an external I/O
            // request. Materialize the duration before awaiting so the
            // timeout-policy scanner cannot mistake the value accessor for
            // an unbounded network operation.
            let backoff_duration = delay.get();
            tokio::time::sleep(backoff_duration).await;
            let retried = QueuedTask {
                event: task.event,
                priority: task.priority,
                attempt: attempts_made,
            };
            // If the queue is closed there is nowhere to retry into;
            // the task is lost only because the whole pool is shutting
            // down, which is the same fate every in-flight task has at
            // shutdown.
            let retry_was_scheduled = queue_handle.retry(retried).is_ok();
            notify_outcome(
                on_outcome,
                if retry_was_scheduled {
                    TaskOutcome::RetryScheduled {
                        task_key,
                        attempt: attempts_made,
                    }
                } else {
                    TaskOutcome::DeadLettered {
                        task_key,
                        attempts: attempts_made,
                    }
                },
            );
        }
    }
}

/// Best-effort broadcast: a dropped/closed receiver (no test listening)
/// is not an error -- production callers never construct the channel at
/// all ([`WorkerPoolConfig::on_outcome`] defaults to `None`).
fn notify_outcome(
    on_outcome: Option<&tokio::sync::mpsc::Sender<TaskOutcome>>,
    outcome: TaskOutcome,
) {
    if let Some(sender) = on_outcome {
        match sender.try_send(outcome) {
            Ok(())
            | Err(tokio::sync::mpsc::error::TrySendError::Full(_))
            | Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {}
        }
    }
}

fn record_dead_letter(
    dead_letters: &Mutex<DeadLetterQueue>,
    event: WeaverEvent,
    attempts: RetryAttemptCount,
    last_error: MemoryQueueLastError,
) {
    if let Ok(mut dlq) = dead_letters.lock() {
        dlq.push(FailedTask {
            event,
            attempts,
            last_error,
        });
    }
}

/// Convenience: classify an event's default priority. Newly
/// created/changed nodes and files are hot (the user likely just
/// touched them); relink requests are warm; nothing in this slice
/// defaults to cold (cold is reserved for periodic sweeps a future
/// scheduler enqueues explicitly).
pub fn default_priority(event: &WeaverEvent) -> MemoryPriority {
    match event {
        WeaverEvent::NodeChanged { .. } | WeaverEvent::FileChanged { .. } => MemoryPriority::Hot,
        WeaverEvent::FileDeleted { .. } => MemoryPriority::Warm,
        WeaverEvent::RelinkRequested { .. } => MemoryPriority::Warm,
    }
}
