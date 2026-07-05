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

/// Configuration for [`WorkerPool::spawn`].
#[derive(Debug, Clone, Copy)]
pub struct WorkerPoolConfig {
    /// Maximum number of tasks the pool may process concurrently. This
    /// is the "worker resource limits" hard requirement -- a bounded
    /// `tokio::sync::Semaphore`, not an unbounded `tokio::spawn` per
    /// task.
    pub max_concurrency: usize,
    pub retry: RetryPolicy,
    pub embedding_version: u32,
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
        }
    }
}

/// The running worker pool: owns the shared [`Embedder`]/[`SummaryStore`]
/// and the dead-letter queue tasks are routed to once their retry
/// budget is exhausted.
pub struct WorkerPool {
    pub dead_letters: Arc<Mutex<DeadLetterQueue>>,
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
        config: WorkerPoolConfig,
    ) -> Self {
        let dead_letters = Arc::new(Mutex::new(DeadLetterQueue::new()));
        let dead_letters_for_task = Arc::clone(&dead_letters);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrency.max(1)));

        let handle = tokio::spawn(async move {
            while let Some(task) = queue.recv_next().await {
                let ctx = Arc::clone(&ctx);
                let queue_handle = queue_handle.clone();
                let dead_letters = Arc::clone(&dead_letters_for_task);
                let semaphore = Arc::clone(&semaphore);
                let retry = config.retry;

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
                    run_one_attempt(&ctx, task, &queue_handle, &dead_letters, retry).await;
                });
            }
        });

        Self {
            dead_letters,
            handle,
        }
    }

    /// Stop accepting new work and wait for the drain loop to notice
    /// the queue is closed. Callers that hold a
    /// [`crate::queue::WeaverQueueHandle`] must drop every clone of it
    /// before this resolves (the drain loop's `recv_next` only returns
    /// `None` once every sender is gone).
    pub async fn shutdown(self) {
        let _ = self.handle.await;
    }
}

async fn run_one_attempt(
    ctx: &EnrichmentContext,
    task: QueuedTask,
    queue_handle: &crate::queue::WeaverQueueHandle,
    dead_letters: &Arc<Mutex<DeadLetterQueue>>,
    retry: RetryPolicy,
) {
    match process_event(ctx, &task.event).await {
        Ok(()) => {}
        Err(EnrichmentError::Permanent(reason)) => {
            record_dead_letter(dead_letters, task.event, task.attempt + 1, reason);
        }
        Err(EnrichmentError::Transient(reason)) => {
            let attempts_made = task.attempt + 1;
            if retry.is_exhausted(attempts_made) {
                record_dead_letter(dead_letters, task.event, attempts_made, reason);
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
        }
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

    fn test_ctx(embedder: Arc<dyn Embedder>) -> Arc<EnrichmentContext> {
        Arc::new(EnrichmentContext {
            embedder,
            summaries: Arc::new(Mutex::new(SummaryStore::new())),
            embedding_version: 1,
        })
    }

    #[tokio::test]
    async fn node_changed_event_produces_an_embedding_task() {
        let embedder = Arc::new(NullEmbedder::new());
        let ctx = test_ctx(Arc::clone(&embedder) as Arc<dyn Embedder>);
        let event = WeaverEvent::NodeChanged {
            node_id: "sym:src/lib.rs:1:foo".to_owned(),
            rel_path: "src/lib.rs".to_owned(),
            content_hash: "hash-1".to_owned(),
        };

        process_event(&ctx, &event).await.expect("process ok");

        let calls = embedder.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].node_id, "sym:src/lib.rs:1:foo");
        assert_eq!(calls[0].content_hash, "hash-1");
    }

    #[tokio::test]
    async fn file_changed_event_invalidates_summary() {
        let embedder = Arc::new(NullEmbedder::new()) as Arc<dyn Embedder>;
        let ctx = test_ctx(embedder);
        {
            let mut store = ctx.summaries.lock().expect("lock");
            store.set_summary("src/lib.rs", "old summary");
        }
        let event = WeaverEvent::FileChanged {
            rel_path: "src/lib.rs".to_owned(),
            content_hash: "hash-2".to_owned(),
        };

        process_event(&ctx, &event).await.expect("process ok");

        let store = ctx.summaries.lock().expect("lock");
        assert!(store.is_stale("src/lib.rs"));
    }

    #[tokio::test]
    async fn retry_succeeds_after_transient_failure() {
        let flaky = Arc::new(FlakyEmbedder::fail_first_n(2));
        let ctx = test_ctx(Arc::clone(&flaky) as Arc<dyn Embedder>);
        let mut queue = WeaverQueue::new();
        let handle = queue.handle();
        let pool = WorkerPool::spawn(
            queue,
            handle.clone(),
            ctx,
            WorkerPoolConfig {
                max_concurrency: 2,
                retry: RetryPolicy::bounded_default(),
                embedding_version: 1,
            },
        );

        handle
            .send(
                WeaverEvent::NodeChanged {
                    node_id: "n1".to_owned(),
                    rel_path: "src/lib.rs".to_owned(),
                    content_hash: "hash-3".to_owned(),
                },
                Priority::Hot,
            )
            .expect("send");

        // Give the pool a bounded, deterministic amount of wall-clock
        // room to retry twice (backoff delays are tens of ms) --
        // polling completion via the flaky embedder's own counter
        // rather than an arbitrary fixed sleep-as-synchronization.
        for _ in 0..200 {
            if flaky.attempts_seen() >= 3 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        let dead_letters_len = pool.dead_letters.lock().map(|d| d.len()).unwrap_or(0);
        drop(handle);
        pool.shutdown().await;

        assert_eq!(flaky.attempts_seen(), 3, "expected 2 failures + 1 success");
        assert_eq!(
            dead_letters_len, 0,
            "a task that eventually succeeds must never reach the dead-letter queue"
        );
    }

    #[tokio::test]
    async fn task_failing_every_retry_lands_in_dead_letter_queue() {
        let flaky = Arc::new(FlakyEmbedder::fail_first_n(1_000));
        let ctx = test_ctx(Arc::clone(&flaky) as Arc<dyn Embedder>);
        let mut queue = WeaverQueue::new();
        let handle = queue.handle();
        let retry = RetryPolicy {
            max_attempts: 2,
            base_delay: std::time::Duration::from_millis(5),
            max_delay: std::time::Duration::from_millis(20),
        };
        let pool = WorkerPool::spawn(
            queue,
            handle.clone(),
            ctx,
            WorkerPoolConfig {
                max_concurrency: 2,
                retry,
                embedding_version: 1,
            },
        );

        let event = WeaverEvent::NodeChanged {
            node_id: "n-dlq".to_owned(),
            rel_path: "src/dlq.rs".to_owned(),
            content_hash: "hash-dlq".to_owned(),
        };
        handle.send(event.clone(), Priority::Hot).expect("send");

        let dead_letters = Arc::clone(&pool.dead_letters);
        for _ in 0..200 {
            let len = dead_letters.lock().map(|d| d.len()).unwrap_or(0);
            if len >= 1 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        drop(handle);
        pool.shutdown().await;

        let dlq = dead_letters.lock().expect("lock");
        assert_eq!(dlq.len(), 1);
        let found = dlq.find(&event.task_key());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].attempts, 2);
    }
}
