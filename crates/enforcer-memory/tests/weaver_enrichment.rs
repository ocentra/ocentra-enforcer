//! X06.5 hard-test suite for the background weaver
//! ([`enforcer_memory::weaver`], [`enforcer_memory::queue`],
//! [`enforcer_memory::enrichment`]).
//!
//! Every test in this file synchronizes deterministically on channels
//! ([`enforcer_domain::memory_types::TaskOutcome`] via
//! `WeaverBuilder::with_outcome_channel`/`WorkerPoolConfig::with_outcome_channel`,
//! or the embedder's own completion signal) -- never on
//! `tokio::time::sleep`-as-synchronization, per the owner-set
//! "foreground never blocks on enrichment" doctrine and the workpack's
//! explicit "deterministic, synchronize with channels/notify, never
//! sleeps" hard-test requirement
//! (`docs/plans/enforcer-selfhost-plan/MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS.md`
//! §6). Backoff *delays themselves* are still real `tokio::time::sleep`
//! calls inside the production retry path (`enrichment::run_one_attempt`)
//! -- that sleep is the feature under test, not a synchronization hack in
//! the test.
//!
//! Fixture files under `tests/fixtures/memory/weaver/` (`widget_v1.rs`,
//! `widget_v2.rs`) provide two real revisions of the same symbol so the
//! "file changed" tests hash actual file content instead of an inline
//! synthetic string, matching `tests/code_graph_indexer.rs`'s
//! real-fixture-over-literal convention.

use enforcer_domain::memory_types::{
    EmbeddingGenerationId, MemoryPriority, MemoryQueueLength, RetryAttemptCount, TaskOutcome,
    WorkerConcurrency,
};
use enforcer_memory::enrichment::{
    Embedder, EnrichmentError, FlakyEmbedder, NullEmbedder, WorkerPoolConfig,
    WorkerPoolOutcomeChannel,
};
use enforcer_memory::queue::{RetryPolicy, WeaverEvent, WeaverQueue};
use enforcer_memory::weaver::Weaver;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

type TestResult = Result<(), Box<dyn Error>>;

const FIXTURE_DIR: &str = "tests/fixtures/memory/weaver";

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURE_DIR)
        .join(name)
}

fn hash_file(name: &str) -> Result<String, Box<dyn Error>> {
    let bytes = fs::read(fixture_path(name))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok(out)
}

/// An [`Embedder`] that signals a one-shot [`tokio::sync::oneshot`]
/// channel the first time it is called, so a test can `.await` "the
/// semantic indexer actually ran" without any sleep.
struct SignalingEmbedder {
    inner: NullEmbedder,
    fired: AtomicBool,
    signal: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

impl SignalingEmbedder {
    fn new() -> (Arc<Self>, tokio::sync::oneshot::Receiver<()>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (
            Arc::new(Self {
                inner: NullEmbedder::new(),
                fired: AtomicBool::new(false),
                signal: tokio::sync::Mutex::new(Some(tx)),
            }),
            rx,
        )
    }
}

impl Embedder for SignalingEmbedder {
    fn embed<'a>(
        &'a self,
        task: &'a enforcer_memory::enrichment::EmbeddingTask,
    ) -> Pin<Box<dyn Future<Output = Result<EmbeddingGenerationId, EnrichmentError>> + Send + 'a>>
    {
        Box::pin(async move {
            let result = self.inner.embed(task).await;
            if !self.fired.swap(true, Ordering::SeqCst) {
                if let Some(tx) = self.signal.lock().await.take() {
                    let _ = tx.send(());
                }
            }
            result
        })
    }
}

/// Hard test 1: a node-created/changed event triggers the semantic
/// indexer's embedding task.
#[tokio::test]
async fn node_created_event_triggers_embedding_task() -> TestResult {
    let content_hash = hash_file("widget_v1.rs")?;
    let (embedder, signal) = SignalingEmbedder::new();
    let weaver = Weaver::builder()
        .with_embedder(Arc::clone(&embedder) as Arc<dyn Embedder>)
        .build();

    weaver.enqueue(WeaverEvent::NodeChanged {
        node_id: "sym:tests/fixtures/memory/weaver/widget_v1.rs:6:render"
            .to_owned()
            .into(),
        rel_path: "tests/fixtures/memory/weaver/widget_v1.rs"
            .to_owned()
            .into(),
        content_hash: content_hash.clone().into(),
    })?;

    // Deterministic: wait for the embedder's own completion signal,
    // never a sleep loop.
    signal.await?;

    let calls = embedder.inner.calls();
    assert_eq!(
        calls.len(),
        1,
        "exactly one embedding task must be produced"
    );
    assert_eq!(calls[0].content_hash.as_str(), content_hash);

    weaver.shutdown().await;
    Ok(())
}

/// Hard test 2: a file-changed event invalidates the cached summary for
/// that file (using the fixture's real "before" hash to seed the cache
/// and the real "after" hash on the invalidating event).
#[tokio::test]
async fn file_changed_event_triggers_summary_invalidation() -> TestResult {
    let rel_path = "tests/fixtures/memory/weaver/widget_v1.rs".to_owned();
    let old_hash = hash_file("widget_v1.rs")?;
    let new_hash = hash_file("widget_v2.rs")?;
    assert_ne!(old_hash, new_hash, "fixture revisions must actually differ");

    let channel = Weaver::builder().with_outcome_channel();
    let weaver = channel.builder.build();
    let mut outcomes = channel.outcomes;
    weaver.with_summaries(|store| {
        store.set_summary(&rel_path, "summary computed against the v1 content");
    });
    let starts_fresh = weaver.with_summaries(|store| !store.is_stale(&rel_path).is_stale());
    assert!(starts_fresh, "summary must start fresh");

    weaver.enqueue(WeaverEvent::FileChanged {
        rel_path: rel_path.clone().into(),
        content_hash: new_hash.into(),
    })?;

    // Deterministic: wait for this task's own `Succeeded` outcome.
    let outcome = outcomes.recv().await;
    assert_eq!(
        outcome,
        Some(TaskOutcome::Succeeded {
            task_key: format!("file-changed:{rel_path}").into()
        })
    );

    let is_stale_after = weaver.with_summaries(|store| store.is_stale(&rel_path).is_stale());
    assert!(
        is_stale_after,
        "the file's cached summary must be marked stale after FileChanged"
    );

    weaver.shutdown().await;
    Ok(())
}

/// Hard test 3: a task that exhausts its retry budget lands in the
/// dead-letter queue with the exact attempt count and is discoverable
/// by its logical task key.
#[tokio::test]
async fn failed_task_enters_dead_letter_after_retries_exhausted() -> TestResult {
    let always_fails = Arc::new(FlakyEmbedder::fail_first_n(usize::MAX));
    let ctx = Arc::new(enforcer_memory::enrichment::EnrichmentContext::new(
        Arc::clone(&always_fails) as Arc<dyn Embedder>,
        EmbeddingGenerationId::INITIAL,
    ));
    let queue = WeaverQueue::new();
    let handle = queue.handle();
    let retry = RetryPolicy {
        max_attempts: RetryAttemptCount::ZERO.next().next(),
        base_delay: std::time::Duration::from_millis(1).into(),
        max_delay: std::time::Duration::from_millis(5).into(),
    };
    let WorkerPoolOutcomeChannel {
        config,
        mut outcomes,
    } = WorkerPoolConfig {
        max_concurrency: WorkerConcurrency::SINGLE,
        retry,
        embedding_version: EmbeddingGenerationId::INITIAL,
        on_outcome: None,
    }
    .with_outcome_channel();
    let pool = enforcer_memory::enrichment::WorkerPool::spawn(queue, handle.clone(), ctx, &config);

    let event = WeaverEvent::NodeChanged {
        node_id: "sym:tests/fixtures/memory/weaver/widget_v1.rs:6:render"
            .to_owned()
            .into(),
        rel_path: "tests/fixtures/memory/weaver/widget_v1.rs"
            .to_owned()
            .into(),
        content_hash: hash_file("widget_v1.rs")?.into(),
    };
    handle.send(event.clone(), MemoryPriority::Hot)?;

    // Deterministic: wait for the `DeadLettered` outcome instead of
    // polling the dead-letter queue on a sleep.
    let mut dead_lettered = None;
    let mut succeeded_unexpectedly = false;
    while dead_lettered.is_none() && !succeeded_unexpectedly {
        match outcomes.recv().await {
            Some(TaskOutcome::DeadLettered { attempts, .. }) => dead_lettered = Some(attempts),
            Some(TaskOutcome::RetryScheduled { .. }) => {}
            Some(TaskOutcome::Succeeded { .. }) | None => succeeded_unexpectedly = true,
        }
    }
    assert!(
        !succeeded_unexpectedly,
        "an always-failing embedder must never succeed"
    );
    assert_eq!(
        dead_lettered,
        Some(retry.max_attempts),
        "must exhaust exactly max_attempts before dead-lettering"
    );

    let dead_letter_state = pool.with_dead_letters(|dead_letters| {
        let found = dead_letters.find(&event.task_key());
        (
            dead_letters.len(),
            found.len(),
            found.first().map(|task| task.attempts),
        )
    });
    drop(handle);
    pool.shutdown().await;

    assert_eq!(
        dead_letter_state,
        (MemoryQueueLength::from(1), 1, Some(retry.max_attempts))
    );
    Ok(())
}

/// Hard test 4: a task that fails transiently but succeeds within its
/// retry budget never reaches the dead-letter queue.
#[tokio::test]
async fn retry_succeeds_on_transient_failure() -> TestResult {
    let flaky = Arc::new(FlakyEmbedder::fail_first_n(2));
    let ctx = Arc::new(enforcer_memory::enrichment::EnrichmentContext::new(
        Arc::clone(&flaky) as Arc<dyn Embedder>,
        EmbeddingGenerationId::INITIAL,
    ));
    let queue = WeaverQueue::new();
    let handle = queue.handle();
    let WorkerPoolOutcomeChannel {
        config,
        mut outcomes,
    } = WorkerPoolConfig {
        max_concurrency: WorkerConcurrency::SINGLE,
        retry: RetryPolicy::bounded_default(),
        embedding_version: EmbeddingGenerationId::INITIAL,
        on_outcome: None,
    }
    .with_outcome_channel();
    let pool = enforcer_memory::enrichment::WorkerPool::spawn(queue, handle.clone(), ctx, &config);

    let event = WeaverEvent::NodeChanged {
        node_id: "sym:tests/fixtures/memory/weaver/widget_v1.rs:6:render"
            .to_owned()
            .into(),
        rel_path: "tests/fixtures/memory/weaver/widget_v1.rs"
            .to_owned()
            .into(),
        content_hash: hash_file("widget_v1.rs")?.into(),
    };
    handle.send(event.clone(), MemoryPriority::Hot)?;

    let mut retries_seen = 0;
    let mut succeeded = false;
    let mut settled = false;
    while !settled {
        match outcomes.recv().await {
            Some(TaskOutcome::RetryScheduled { .. }) => retries_seen += 1,
            Some(TaskOutcome::Succeeded { .. }) => {
                succeeded = true;
                settled = true;
            }
            Some(TaskOutcome::DeadLettered { .. }) | None => settled = true,
        }
    }

    let dead_letters_len = pool.with_dead_letters(|dead_letters| dead_letters.len());
    drop(handle);
    pool.shutdown().await;

    assert!(
        succeeded,
        "task must eventually succeed within its retry budget"
    );
    assert_eq!(
        retries_seen, 2,
        "expected exactly 2 transient failures before success"
    );
    assert_eq!(
        flaky.attempts_seen().get(),
        3,
        "2 failed attempts + 1 successful attempt"
    );
    assert_eq!(
        dead_letters_len, 0,
        "a task that succeeds must never reach the dead-letter queue"
    );
    Ok(())
}

/// Hard test 5 (concurrent): the weaver's queue never blocks a
/// foreground query -- enqueuing enrichment work and performing a
/// "foreground query" (reading the summary store) interleave freely
/// even while the background pool is busy processing a slow task.
#[tokio::test]
async fn queue_does_not_block_foreground_query() -> TestResult {
    /// An embedder that blocks until explicitly released, standing in
    /// for a slow/real embedding call the foreground must never wait
    /// on.
    struct BlockingEmbedder {
        inner: NullEmbedder,
        release: tokio::sync::Notify,
    }

    impl Embedder for BlockingEmbedder {
        fn embed<'a>(
            &'a self,
            task: &'a enforcer_memory::enrichment::EmbeddingTask,
        ) -> Pin<Box<dyn Future<Output = Result<EmbeddingGenerationId, EnrichmentError>> + Send + 'a>>
        {
            Box::pin(async move {
                self.release.notified().await;
                self.inner.embed(task).await
            })
        }
    }

    let embedder = Arc::new(BlockingEmbedder {
        inner: NullEmbedder::new(),
        release: tokio::sync::Notify::new(),
    });
    let channel = Weaver::builder()
        .with_embedder(Arc::clone(&embedder) as Arc<dyn Embedder>)
        .with_max_concurrency(WorkerConcurrency::SINGLE)
        .with_outcome_channel();
    let weaver = channel.builder.build();
    let mut outcomes = channel.outcomes;

    // Enqueue a slow task that will not resolve until `release` is
    // notified -- this occupies the pool's single concurrency slot.
    weaver.enqueue(WeaverEvent::NodeChanged {
        node_id: "sym:slow.rs:1:slow".to_owned().into(),
        rel_path: "slow.rs".to_owned().into(),
        content_hash: "5555555555555555555555555555555555555555555555555555555555555555"
            .to_owned()
            .into(),
    })?;

    // While the slow task is stuck (no `release.notify_*` called yet),
    // the foreground must still be able to: (a) enqueue more work
    // without blocking, and (b) read the summary store without
    // blocking. Both operations are plain non-blocking calls (`send`
    // is a channel send, `summaries()` is an `Arc::clone` + `lock`), so
    // proving they return at all -- inside a bounded `tokio::time::timeout`
    // used only as a deadlock guard, not as a sleep-based poll -- proves
    // the foreground path never awaits the background worker.
    let foreground_result: Result<Result<bool, Box<dyn Error>>, tokio::time::error::Elapsed> =
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            weaver.enqueue(WeaverEvent::FileChanged {
                rel_path: "other.rs".to_owned().into(),
                content_hash: "hash-other".to_owned().into(),
            })?;
            let is_stale =
                weaver.with_summaries(|store| store.is_stale("never-seen.rs").is_stale());
            Ok(is_stale)
        })
        .await;

    let foreground_is_stale: bool = match foreground_result {
        Ok(inner) => inner?,
        Err(elapsed) => {
            let boxed: Box<dyn Error> = Box::new(elapsed);
            return Err(boxed);
        }
    };
    assert!(foreground_is_stale, "an unseen path is considered stale");

    // Release the slow task so the pool (and the test) can shut down
    // cleanly. Deterministic: wait for both tasks' own `Succeeded`
    // outcomes on the channel rather than polling.
    embedder.release.notify_one();
    let mut succeeded = 0;
    let mut unexpected = None;
    while succeeded < 2 && unexpected.is_none() {
        match outcomes.recv().await {
            Some(TaskOutcome::Succeeded { .. }) => succeeded += 1,
            other => unexpected = Some(other),
        }
    }
    assert_eq!(unexpected, None, "expected only Succeeded outcomes");
    let other_is_stale = weaver.with_summaries(|store| store.is_stale("other.rs").is_stale());
    assert!(other_is_stale);

    weaver.shutdown().await;
    Ok(())
}
