//! X06.5: the weaver's worker abstraction and the bounded-concurrency
//! worker pool that drains [`crate::queue::WeaverQueue`].
//!
//! # Harvested-from
//!
//! Worker-pool shape (tokio tasks pulling from a shared queue, sized by
//! available parallelism) harvested from TabAgentServer `Rust/weaver`
//! per `refs/x06-source-scout-digests.md` §2 ("worker pool sized by
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

use crate::queue::{DeadLetterQueue, FailedTask, Priority, QueuedTask, RetryPolicy, WeaverEvent};
use crate::summaries::SummaryStore;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// A single embedding task's outcome, produced by the semantic indexer
/// worker for X06.4 to eventually consume. Kept as plain data (not a
/// vector) in this slice: X06.5 owns *producing the task record*, not
/// the embedding model itself (X06.4's concern, wired at integration).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingTask {
    pub node_id: String,
    pub content_hash: String,
    pub embedding_version: u32,
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
    ) -> Pin<Box<dyn Future<Output = Result<u32, EnrichmentError>> + Send + 'a>>;
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
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

impl Embedder for NullEmbedder {
    fn embed<'a>(
        &'a self,
        task: &'a EmbeddingTask,
    ) -> Pin<Box<dyn Future<Output = Result<u32, EnrichmentError>> + Send + 'a>> {
        Box::pin(async move {
            if let Ok(mut guard) = self.calls.lock() {
                guard.push(task.clone());
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
    pub fn fail_first_n(n: usize) -> Self {
        Self {
            inner: NullEmbedder::new(),
            fail_until_attempt: AtomicUsize::new(n),
            attempts_seen: AtomicUsize::new(0),
        }
    }

    pub fn attempts_seen(&self) -> usize {
        self.attempts_seen.load(Ordering::SeqCst)
    }
}

impl Embedder for FlakyEmbedder {
    fn embed<'a>(
        &'a self,
        task: &'a EmbeddingTask,
    ) -> Pin<Box<dyn Future<Output = Result<u32, EnrichmentError>> + Send + 'a>> {
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

/// Shared state a worker task processes one [`WeaverEvent`] against.
/// Grouped so [`process_event`] takes one argument instead of a long
/// positional list (mirrors [`crate::code_graph::NewFileParams`]'s
/// established pattern in this crate).
pub struct EnrichmentContext {
    pub embedder: Arc<dyn Embedder>,
    pub summaries: Arc<Mutex<SummaryStore>>,
    pub embedding_version: u32,
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
            let task = EmbeddingTask {
                node_id: node_id.clone(),
                content_hash: content_hash.clone(),
                embedding_version: ctx.embedding_version,
            };
            ctx.embedder.embed(&task).await?;
            if let Ok(mut store) = ctx.summaries.lock() {
                store.link_entity(node_id);
            }
            Ok(())
        }
        WeaverEvent::FileChanged { rel_path, .. } => {
            if let Ok(mut store) = ctx.summaries.lock() {
                store.invalidate(rel_path);
            }
            Ok(())
        }
        WeaverEvent::FileDeleted { rel_path } => {
            if let Ok(mut store) = ctx.summaries.lock() {
                store.remove(rel_path);
                store.unlink_entities_for_path(rel_path);
            }
            Ok(())
        }
        WeaverEvent::RelinkRequested { node_id } => {
            if let Ok(mut store) = ctx.summaries.lock() {
                store.link_entity(node_id);
            }
            Ok(())
        }
    }
}

/// What happened to one dequeued [`QueuedTask`] after exactly one
/// processing attempt. Broadcast on [`WorkerPoolConfig::on_outcome`] so
/// tests can `.await` a specific outcome instead of polling
/// `tokio::time::sleep` in a loop -- per the workpack's hard-test
/// requirement, tests synchronize on channels/notify, never
/// sleeps-as-synchronization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    /// The attempt succeeded; the task is fully done.
    Succeeded { task_key: String },
    /// The attempt failed transiently and was re-enqueued for another
    /// attempt (not yet dead-lettered).
    RetryScheduled { task_key: String, attempt: u32 },
    /// The attempt failed (permanently, or transiently with the retry
    /// budget exhausted) and was routed to the dead-letter queue.
    DeadLettered { task_key: String, attempts: u32 },
}

/// Configuration for [`WorkerPool::spawn`].
#[derive(Debug, Clone)]
pub struct WorkerPoolConfig {
    /// Maximum number of tasks the pool may process concurrently. This
    /// is the "worker resource limits" hard requirement -- a bounded
    /// `tokio::sync::Semaphore`, not an unbounded `tokio::spawn` per
    /// task.
    pub max_concurrency: usize,
    pub retry: RetryPolicy,
    pub embedding_version: u32,
    /// Optional per-attempt outcome broadcast, consumed by
    /// [`TaskOutcome`]-based deterministic test synchronization. `None`
    /// in production (no channel to drain, zero overhead); tests set it
    /// via [`WorkerPoolConfig::with_outcome_channel`].
    pub on_outcome: Option<tokio::sync::mpsc::UnboundedSender<TaskOutcome>>,
}

impl WorkerPoolConfig {
    /// `max_concurrency` defaults to available parallelism (falling
    /// back to 1 if the platform cannot report it), mirroring
    /// TabAgentServer's `num_cpus`-sized pool without adding a
    /// `num_cpus` dependency the standard library already makes
    /// unnecessary (`std::thread::available_parallelism`, stable since
    /// Rust 1.59).
    pub fn with_default_concurrency(retry: RetryPolicy, embedding_version: u32) -> Self {
        let max_concurrency = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1);
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
    pub fn with_outcome_channel(
        mut self,
    ) -> (Self, tokio::sync::mpsc::UnboundedReceiver<TaskOutcome>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.on_outcome = Some(tx);
        (self, rx)
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
    pub dead_letters: Arc<Mutex<DeadLetterQueue>>,
    stop: Arc<tokio::sync::Notify>,
    handle: tokio::task::JoinHandle<()>,
}

impl WorkerPool {
    /// Spawn the pool's drain loop as a single background task that
    /// bounds its own concurrency with a semaphore (so this call
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
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrency.max(1)));
        let on_outcome = config.on_outcome.clone();
        let retry = config.retry;
        let stop = Arc::new(tokio::sync::Notify::new());
        let stop_for_task = Arc::clone(&stop);

        let handle = tokio::spawn(async move {
            loop {
                let task = tokio::select! {
                    // Stop only once there is nothing immediately ready
                    // to process -- `biased` plus this branch listed
                    // second means a task already sitting in any tier
                    // is always drained before a pending stop signal is
                    // honored, so `shutdown` never drops in-flight work
                    // that was already enqueued.
                    biased;
                    maybe_task = queue.recv_next() => maybe_task,
                    () = stop_for_task.notified() => None,
                };
                let Some(task) = task else { break };

                let ctx = Arc::clone(&ctx);
                let queue_handle = queue_handle.clone();
                let dead_letters = Arc::clone(&dead_letters_for_task);
                let semaphore = Arc::clone(&semaphore);
                let on_outcome = on_outcome.clone();

                tokio::spawn(async move {
                    // Bound concurrency: acquire before doing any work,
                    // release (via drop) once this task's single
                    // attempt finishes. A poisoned/closed semaphore
                    // (would require every permit + the semaphore
                    // itself to be dropped, which cannot happen while
                    // this async block holds `semaphore`) is treated as
                    // "run without the bound" rather than losing the
                    // task.
                    let _permit = semaphore.acquire().await;
                    run_one_attempt(AttemptArgs {
                        ctx: &ctx,
                        task,
                        queue_handle: &queue_handle,
                        dead_letters: &dead_letters,
                        retry,
                        on_outcome: on_outcome.as_ref(),
                    })
                    .await;
                });
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
    /// wait for retries scheduled *after* the stop signal fires, nor for
    /// already-dequeued tasks' spawned [`run_one_attempt`] futures to
    /// finish -- callers that need every in-flight attempt to fully
    /// resolve (e.g. so dead-letter/outcome state is stable to assert
    /// on) should drain their own completion signal
    /// ([`WorkerPoolConfig::on_outcome`]) first, then call this.
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
    dead_letters: &'a Arc<Mutex<DeadLetterQueue>>,
    retry: RetryPolicy,
    on_outcome: Option<&'a tokio::sync::mpsc::UnboundedSender<TaskOutcome>>,
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
            let attempts = task.attempt + 1;
            record_dead_letter(dead_letters, task.event, attempts, reason);
            notify_outcome(on_outcome, TaskOutcome::DeadLettered { task_key, attempts });
        }
        Err(EnrichmentError::Transient(reason)) => {
            let attempts_made = task.attempt + 1;
            if retry.is_exhausted(attempts_made) {
                record_dead_letter(dead_letters, task.event, attempts_made, reason);
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
            tokio::time::sleep(delay).await;
            let retried = QueuedTask {
                event: task.event,
                priority: task.priority,
                attempt: attempts_made,
            };
            // If the queue is closed there is nowhere to retry into;
            // the task is lost only because the whole pool is shutting
            // down, which is the same fate every in-flight task has at
            // shutdown.
            let _ = queue_handle.retry(retried);
            notify_outcome(
                on_outcome,
                TaskOutcome::RetryScheduled {
                    task_key,
                    attempt: attempts_made,
                },
            );
        }
    }
}

/// Best-effort broadcast: a dropped/closed receiver (no test listening)
/// is not an error -- production callers never construct the channel at
/// all ([`WorkerPoolConfig::on_outcome`] defaults to `None`).
fn notify_outcome(
    on_outcome: Option<&tokio::sync::mpsc::UnboundedSender<TaskOutcome>>,
    outcome: TaskOutcome,
) {
    if let Some(sender) = on_outcome {
        let _ = sender.send(outcome);
    }
}

fn record_dead_letter(
    dead_letters: &Arc<Mutex<DeadLetterQueue>>,
    event: WeaverEvent,
    attempts: u32,
    last_error: String,
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
pub fn default_priority(event: &WeaverEvent) -> Priority {
    match event {
        WeaverEvent::NodeChanged { .. } | WeaverEvent::FileChanged { .. } => Priority::Hot,
        WeaverEvent::FileDeleted { .. } => Priority::Warm,
        WeaverEvent::RelinkRequested { .. } => Priority::Warm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::WeaverQueue;
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    /// `std::sync::Mutex::lock`'s `PoisonError<MutexGuard<'_, T>>` is not
    /// `'static` (it embeds the guard), so it cannot convert into
    /// `Box<dyn Error>` via a bare `?` -- this maps it to an owned,
    /// `'static` message first.
    fn lock_or_msg<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, Box<dyn Error>> {
        mutex
            .lock()
            .map_err(|_poison_error| "mutex poisoned".into())
    }

    fn test_ctx(embedder: Arc<dyn Embedder>) -> Arc<EnrichmentContext> {
        Arc::new(EnrichmentContext {
            embedder,
            summaries: Arc::new(Mutex::new(SummaryStore::new())),
            embedding_version: 1,
        })
    }

    #[tokio::test]
    async fn node_changed_event_produces_an_embedding_task() -> TestResult {
        let embedder = Arc::new(NullEmbedder::new());
        let ctx = test_ctx(Arc::clone(&embedder) as Arc<dyn Embedder>);
        let event = WeaverEvent::NodeChanged {
            node_id: "sym:src/lib.rs:1:foo".to_owned(),
            rel_path: "src/lib.rs".to_owned(),
            content_hash: "hash-1".to_owned(),
        };

        process_event(&ctx, &event).await?;

        let calls = embedder.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].node_id, "sym:src/lib.rs:1:foo");
        assert_eq!(calls[0].content_hash, "hash-1");
        Ok(())
    }

    #[tokio::test]
    async fn file_changed_event_invalidates_summary() -> TestResult {
        let embedder = Arc::new(NullEmbedder::new()) as Arc<dyn Embedder>;
        let ctx = test_ctx(embedder);
        {
            let mut store = lock_or_msg(&ctx.summaries)?;
            store.set_summary("src/lib.rs", "old summary");
        }
        let event = WeaverEvent::FileChanged {
            rel_path: "src/lib.rs".to_owned(),
            content_hash: "hash-2".to_owned(),
        };

        process_event(&ctx, &event).await?;

        let is_stale = {
            let store = lock_or_msg(&ctx.summaries)?;
            store.is_stale("src/lib.rs")
        };
        assert!(is_stale);
        Ok(())
    }

    #[tokio::test]
    async fn retry_succeeds_after_transient_failure() -> TestResult {
        let flaky = Arc::new(FlakyEmbedder::fail_first_n(2));
        let ctx = test_ctx(Arc::clone(&flaky) as Arc<dyn Embedder>);
        let queue = WeaverQueue::new();
        let handle = queue.handle();
        let (config, mut outcomes) = WorkerPoolConfig {
            max_concurrency: 2,
            retry: RetryPolicy::bounded_default(),
            embedding_version: 1,
            on_outcome: None,
        }
        .with_outcome_channel();
        let pool = WorkerPool::spawn(queue, handle.clone(), ctx, &config);

        handle.send(
            WeaverEvent::NodeChanged {
                node_id: "n1".to_owned(),
                rel_path: "src/lib.rs".to_owned(),
                content_hash: "hash-3".to_owned(),
            },
            Priority::Hot,
        )?;

        // Deterministic synchronization: wait on the outcome channel
        // for exactly two `RetryScheduled` outcomes followed by one
        // `Succeeded` -- no sleep-as-synchronization.
        let mut retries_seen = 0;
        let mut succeeded = false;
        let mut dead_lettered = false;
        while let Some(outcome) = outcomes.recv().await {
            match outcome {
                TaskOutcome::RetryScheduled { .. } => retries_seen += 1,
                TaskOutcome::Succeeded { .. } => {
                    succeeded = true;
                    break;
                }
                TaskOutcome::DeadLettered { .. } => {
                    dead_lettered = true;
                    break;
                }
            }
        }

        let dead_letters_len = pool.dead_letters.lock().map(|d| d.len()).unwrap_or(0);
        drop(handle);
        pool.shutdown().await;

        assert!(
            !dead_lettered,
            "task must not dead-letter: it succeeds on the 3rd attempt"
        );
        assert_eq!(
            retries_seen, 2,
            "expected exactly 2 retry-scheduled outcomes"
        );
        assert!(succeeded, "expected the 3rd attempt to succeed");
        assert_eq!(flaky.attempts_seen(), 3, "expected 2 failures + 1 success");
        assert_eq!(
            dead_letters_len, 0,
            "a task that eventually succeeds must never reach the dead-letter queue"
        );
        Ok(())
    }

    #[tokio::test]
    async fn task_failing_every_retry_lands_in_dead_letter_queue() -> TestResult {
        let flaky = Arc::new(FlakyEmbedder::fail_first_n(1_000));
        let ctx = test_ctx(Arc::clone(&flaky) as Arc<dyn Embedder>);
        let queue = WeaverQueue::new();
        let handle = queue.handle();
        let retry = RetryPolicy {
            max_attempts: 2,
            base_delay: std::time::Duration::from_millis(5),
            max_delay: std::time::Duration::from_millis(20),
        };
        let (config, mut outcomes) = WorkerPoolConfig {
            max_concurrency: 2,
            retry,
            embedding_version: 1,
            on_outcome: None,
        }
        .with_outcome_channel();
        let pool = WorkerPool::spawn(queue, handle.clone(), ctx, &config);

        let event = WeaverEvent::NodeChanged {
            node_id: "n-dlq".to_owned(),
            rel_path: "src/dlq.rs".to_owned(),
            content_hash: "hash-dlq".to_owned(),
        };
        handle.send(event.clone(), Priority::Hot)?;

        // Deterministic synchronization: wait for the `DeadLettered`
        // outcome instead of polling the dead-letter queue on a sleep.
        let mut dead_lettered = false;
        while let Some(outcome) = outcomes.recv().await {
            if let TaskOutcome::DeadLettered { .. } = outcome {
                dead_lettered = true;
                break;
            }
        }
        assert!(
            dead_lettered,
            "expected the task to reach the dead-letter queue"
        );

        let dead_letters = Arc::clone(&pool.dead_letters);
        drop(handle);
        pool.shutdown().await;

        let dlq_guard = lock_or_msg(&dead_letters)?;
        assert_eq!(dlq_guard.len(), 1);
        let found = dlq_guard.find(&event.task_key());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].attempts, 2);
        Ok(())
    }
}
