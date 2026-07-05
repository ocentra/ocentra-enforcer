//! X06.5: the background weaver -- "the foreground answers queries,
//! the background thinks" (owner-set, `MEMORY_RETRIEVAL_OWNER_INTENT.md`).
//!
//! This module wires [`crate::queue::WeaverQueue`] (event-driven
//! priority queue) to [`crate::enrichment::WorkerPool`] (bounded-
//! concurrency worker pool with retry + dead-letter routing) behind one
//! [`Weaver`] handle, and translates the code indexer's
//! [`crate::code_graph::IndexReport`] into the events the pool
//! consumes ([`Weaver::enqueue_index_report`]) so X06.2's indexer does
//! not need to know the weaver's event vocabulary.
//!
//! # Harvested-from
//!
//! TabAgentServer `Rust/weaver` (tokio MPSC + worker pool, 4 module
//! shape: semantic indexer, entity linker, associative linker,
//! summarizer stub) per `refs/x06-source-scout-digests.md` §2 and
//! `MEMORY_RETRIEVAL_DECISIONS.md` D-09 (LOCKED). Every mechanism below
//! that the digest calls out as absent from the source (dead-letter
//! queue, bounded backoff retry, hot/warm/cold priority, blue/green
//! embedding-version migration) is new work, not adapted from
//! TabAgentServer.
//!
//! # Blue/green embedding migration (Rag-Guide, D-09)
//!
//! "Never mix vector versions" (digest §4). [`EmbeddingGeneration`]
//! models the migration as two ordinally-comparable generations: the
//! active one every new [`crate::enrichment::EmbeddingTask`] is
//! stamped with, and an optional next one being built in the
//! background ("green") while the active ("blue") generation keeps
//! serving queries. [`Weaver::begin_embedding_migration`] /
//! [`Weaver::complete_embedding_migration`] are the cutover hooks; they
//! do not run any embedding model themselves (that is X06.4's
//! concern) -- they only gate which `embedding_version` new tasks are
//! stamped with, which is exactly the seam X06.5 owns per its file
//! claims.

use crate::enrichment::{Embedder, EnrichmentContext, NullEmbedder, WorkerPool, WorkerPoolConfig};
use crate::queue::{Priority, QueueClosed, WeaverEvent, WeaverQueue, WeaverQueueHandle};
use crate::summaries::SummaryStore;
use std::sync::{Arc, Mutex};

/// The state of a blue/green embedding-version migration. `Stable`
/// means every task is stamped with `active`; `Migrating` means a
/// green generation is being built alongside blue -- callers building
/// the green index must use `next`, never mix it with `active`'s
/// vectors (Rag-Guide, digest §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingGeneration {
    Stable { active: u32 },
    Migrating { active: u32, next: u32 },
}

impl EmbeddingGeneration {
    pub fn active(&self) -> u32 {
        match self {
            EmbeddingGeneration::Stable { active }
            | EmbeddingGeneration::Migrating { active, .. } => *active,
        }
    }

    pub fn next(&self) -> Option<u32> {
        match self {
            EmbeddingGeneration::Stable { .. } => None,
            EmbeddingGeneration::Migrating { next, .. } => Some(*next),
        }
    }

    pub fn is_migrating(&self) -> bool {
        matches!(self, EmbeddingGeneration::Migrating { .. })
    }
}

/// Error attempting an invalid migration transition (e.g. completing a
/// migration that was never started, or starting one that is already
/// in flight).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MigrationError {
    #[error("an embedding migration is already in progress (active={active}, next={next})")]
    AlreadyMigrating { active: u32, next: u32 },
    #[error("no embedding migration is in progress to complete")]
    NotMigrating,
}

/// The weaver: owns the queue handle producers enqueue through, the
/// running worker pool, and the current embedding generation. Built via
/// [`WeaverBuilder`] so tests can swap in [`crate::enrichment::FlakyEmbedder`]
/// without threading extra parameters through every constructor.
pub struct Weaver {
    queue_handle: WeaverQueueHandle,
    pool: WorkerPool,
    generation: Arc<Mutex<EmbeddingGeneration>>,
    summaries: Arc<Mutex<SummaryStore>>,
}

/// Builder for [`Weaver`] -- defaults to [`NullEmbedder`] and
/// [`crate::enrichment::WorkerPoolConfig::with_default_concurrency`] so
/// the zero-argument path (`WeaverBuilder::default().build()`) never
/// needs a model runtime, matching the crate-wide "no test needs a
/// model download" contract.
pub struct WeaverBuilder {
    embedder: Arc<dyn Embedder>,
    max_concurrency: Option<usize>,
    retry: crate::queue::RetryPolicy,
    embedding_version: u32,
    on_outcome: Option<tokio::sync::mpsc::UnboundedSender<crate::enrichment::TaskOutcome>>,
}

impl Default for WeaverBuilder {
    fn default() -> Self {
        Self {
            embedder: Arc::new(NullEmbedder::new()),
            max_concurrency: None,
            retry: crate::queue::RetryPolicy::bounded_default(),
            embedding_version: 1,
            on_outcome: None,
        }
    }
}

impl WeaverBuilder {
    pub fn with_embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.embedder = embedder;
        self
    }

    pub fn with_max_concurrency(mut self, max_concurrency: usize) -> Self {
        self.max_concurrency = Some(max_concurrency);
        self
    }

    pub fn with_retry_policy(mut self, retry: crate::queue::RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    pub fn with_embedding_version(mut self, version: u32) -> Self {
        self.embedding_version = version;
        self
    }

    /// Attach a per-attempt [`crate::enrichment::TaskOutcome`] channel,
    /// returning the receiver half. Hard tests use this to `.await` a
    /// specific outcome (e.g. the retry that eventually dead-letters)
    /// instead of polling `tokio::time::sleep` in a loop -- see
    /// `tests/weaver_enrichment.rs`.
    pub fn with_outcome_channel(
        mut self,
    ) -> (
        Self,
        tokio::sync::mpsc::UnboundedReceiver<crate::enrichment::TaskOutcome>,
    ) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.on_outcome = Some(tx);
        (self, rx)
    }

    /// Build and start the weaver: spawns the worker pool immediately
    /// (background task), returning a ready-to-enqueue handle. This
    /// call itself never blocks on any worker running.
    pub fn build(self) -> Weaver {
        let queue = WeaverQueue::new();
        let queue_handle = queue.handle();
        let summaries = Arc::new(Mutex::new(SummaryStore::new()));
        let generation = Arc::new(Mutex::new(EmbeddingGeneration::Stable {
            active: self.embedding_version,
        }));

        let ctx = Arc::new(EnrichmentContext {
            embedder: self.embedder,
            summaries: Arc::clone(&summaries),
            embedding_version: self.embedding_version,
        });

        let mut config = match self.max_concurrency {
            Some(max_concurrency) => WorkerPoolConfig {
                max_concurrency,
                retry: self.retry,
                embedding_version: self.embedding_version,
                on_outcome: None,
            },
            None => WorkerPoolConfig::with_default_concurrency(self.retry, self.embedding_version),
        };
        config.on_outcome = self.on_outcome;

        let pool = WorkerPool::spawn(queue, queue_handle.clone(), ctx, &config);

        Weaver {
            queue_handle,
            pool,
            generation,
            summaries,
        }
    }
}

impl Weaver {
    pub fn builder() -> WeaverBuilder {
        WeaverBuilder::default()
    }

    /// A cloneable producer handle -- enqueue is non-blocking (see
    /// [`crate::queue::WeaverQueueHandle::send`]), so any foreground
    /// caller (MCP tool handler, CLI command, watcher) may hold and use
    /// this without ever waiting on enrichment.
    pub fn handle(&self) -> WeaverQueueHandle {
        self.queue_handle.clone()
    }

    /// Enqueue a single event at its default priority (see
    /// [`crate::enrichment::default_priority`]).
    pub fn enqueue(&self, event: WeaverEvent) -> Result<(), QueueClosed> {
        let priority = crate::enrichment::default_priority(&event);
        self.queue_handle.send(event, priority)
    }

    /// Translate one [`crate::code_graph::IndexReport`] into weaver
    /// events: every changed/added path becomes a `FileChanged` event
    /// (hot -- summary invalidation) plus, when the caller supplies the
    /// symbol/node ids that came out of reindexing that file, a
    /// `NodeChanged` event per node (hot -- semantic indexer +
    /// entity/symbol linker per D-09's worker list). Deleted paths
    /// become `FileDeleted` events (warm). This is the seam X06.2's
    /// indexer (or an integration-time adapter) calls after each
    /// `index_repository` run; it takes plain data, not a live
    /// `CodeGraph` reference, so X06.5 does not need to depend on
    /// X06.2's node-lookup API.
    pub fn enqueue_index_report(
        &self,
        report: &crate::code_graph::IndexReport,
        content_hashes: impl Fn(&str) -> String,
        node_ids_by_path: impl Fn(&str) -> Vec<String>,
    ) -> Result<(), QueueClosed> {
        for rel_path in report.changed.iter().chain(report.added.iter()) {
            let content_hash = content_hashes(rel_path);
            self.queue_handle.send(
                WeaverEvent::FileChanged {
                    rel_path: rel_path.clone(),
                    content_hash: content_hash.clone(),
                },
                Priority::Hot,
            )?;
            for node_id in node_ids_by_path(rel_path) {
                self.queue_handle.send(
                    WeaverEvent::NodeChanged {
                        node_id,
                        rel_path: rel_path.clone(),
                        content_hash: content_hash.clone(),
                    },
                    Priority::Hot,
                )?;
            }
        }
        for rel_path in &report.deleted {
            self.queue_handle.send(
                WeaverEvent::FileDeleted {
                    rel_path: rel_path.clone(),
                },
                Priority::Warm,
            )?;
        }
        Ok(())
    }

    /// Snapshot the summary/link store (read-only; the pool keeps
    /// writing to its own clone of the same `Arc` concurrently, so
    /// callers observe a point-in-time snapshot, never a torn write).
    pub fn summaries(&self) -> Arc<Mutex<SummaryStore>> {
        Arc::clone(&self.summaries)
    }

    pub fn embedding_generation(&self) -> EmbeddingGeneration {
        // A poisoned lock (prior panic while held) is treated as
        // "assume stable at generation 0" rather than propagating a
        // panic from a read-only accessor -- this mirrors the crate's
        // existing poison-tolerant `Mutex` reads in `enrichment.rs`.
        self.generation
            .lock()
            .map(|g| *g)
            .unwrap_or(EmbeddingGeneration::Stable { active: 0 })
    }

    /// Begin a blue/green migration to `next_version`: new
    /// [`crate::enrichment::EmbeddingTask`]s continue being stamped with
    /// the current active version (blue) until
    /// [`Weaver::complete_embedding_migration`] cuts over -- this call
    /// only records that a green generation is being built, matching
    /// Rag-Guide's "build parallel, shadow, compare, cut over" sequence
    /// (digest §4); it does not itself re-embed anything (X06.4's
    /// concern once wired).
    pub fn begin_embedding_migration(&self, next_version: u32) -> Result<(), MigrationError> {
        let mut generation = self.generation.lock().unwrap_or_else(|e| e.into_inner());
        match *generation {
            EmbeddingGeneration::Stable { active } => {
                *generation = EmbeddingGeneration::Migrating {
                    active,
                    next: next_version,
                };
                Ok(())
            }
            EmbeddingGeneration::Migrating { active, next } => {
                Err(MigrationError::AlreadyMigrating { active, next })
            }
        }
    }

    /// Cut over: the green generation becomes active. Never mixes
    /// versions -- from this point every new task is stamped with the
    /// formerly-green version; callers are responsible for having
    /// already re-embedded/compared recall+latency before calling this
    /// (Rag-Guide sequence; the comparison step is X06.9's benchmark
    /// harness, not this module's concern).
    pub fn complete_embedding_migration(&self) -> Result<u32, MigrationError> {
        let mut generation = self.generation.lock().unwrap_or_else(|e| e.into_inner());
        match *generation {
            EmbeddingGeneration::Migrating { next, .. } => {
                *generation = EmbeddingGeneration::Stable { active: next };
                Ok(next)
            }
            EmbeddingGeneration::Stable { .. } => Err(MigrationError::NotMigrating),
        }
    }

    /// Shut the weaver down: drop the last queue handle and wait for
    /// the worker pool's drain loop to finish in-flight work. Callers
    /// must have already dropped every other clone of
    /// [`Weaver::handle`] or this will wait forever (the drain loop
    /// only sees a closed queue once every sender is gone).
    pub async fn shutdown(self) {
        drop(self.queue_handle);
        self.pool.shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::IndexReport;
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    /// See `enrichment::tests::lock_or_msg` -- `PoisonError`'s embedded
    /// guard is not `'static`, so it cannot convert into `Box<dyn
    /// Error>` via a bare `?`.
    fn lock_or_msg<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, Box<dyn Error>> {
        mutex
            .lock()
            .map_err(|_poison_error| "mutex poisoned".into())
    }

    #[tokio::test]
    async fn node_created_event_triggers_embedding_task() -> TestResult {
        let embedder = Arc::new(crate::enrichment::NullEmbedder::new());
        let (builder, mut outcomes) = Weaver::builder()
            .with_embedder(Arc::clone(&embedder) as Arc<dyn Embedder>)
            .with_outcome_channel();
        let weaver = builder.build();

        weaver.enqueue(WeaverEvent::NodeChanged {
            node_id: "sym:src/lib.rs:1:foo".to_owned(),
            rel_path: "src/lib.rs".to_owned(),
            content_hash: "hash-a".to_owned(),
        })?;

        // Deterministic synchronization: wait for the task's own
        // `Succeeded` outcome instead of polling `embedder.calls()` on
        // a sleep.
        let outcome = outcomes.recv().await;
        assert_eq!(
            outcome,
            Some(crate::enrichment::TaskOutcome::Succeeded {
                task_key: "node-changed:sym:src/lib.rs:1:foo".to_owned()
            })
        );

        let calls = embedder.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].node_id, "sym:src/lib.rs:1:foo");

        weaver.shutdown().await;
        Ok(())
    }

    #[tokio::test]
    async fn file_changed_event_invalidates_cached_summary() -> TestResult {
        let (builder, mut outcomes) = Weaver::builder().with_outcome_channel();
        let weaver = builder.build();
        {
            let summaries = weaver.summaries();
            let mut store = lock_or_msg(&summaries)?;
            store.set_summary("src/lib.rs", "an old summary");
        }

        weaver.enqueue(WeaverEvent::FileChanged {
            rel_path: "src/lib.rs".to_owned(),
            content_hash: "hash-b".to_owned(),
        })?;

        // Deterministic synchronization: wait for this task's
        // `Succeeded` outcome instead of polling `is_stale` on a sleep.
        let outcome = outcomes.recv().await;
        assert_eq!(
            outcome,
            Some(crate::enrichment::TaskOutcome::Succeeded {
                task_key: "file-changed:src/lib.rs".to_owned()
            })
        );

        let summaries = weaver.summaries();
        assert!(lock_or_msg(&summaries)?.is_stale("src/lib.rs"));
        weaver.shutdown().await;
        Ok(())
    }

    #[tokio::test]
    async fn index_report_translates_into_file_and_node_events() -> TestResult {
        let embedder = Arc::new(crate::enrichment::NullEmbedder::new());
        let (builder, mut outcomes) = Weaver::builder()
            .with_embedder(Arc::clone(&embedder) as Arc<dyn Embedder>)
            .with_outcome_channel();
        let weaver = builder.build();

        let report = IndexReport {
            unchanged: vec!["src/untouched.rs".to_owned()],
            changed: vec!["src/lib.rs".to_owned()],
            added: vec!["src/new.rs".to_owned()],
            deleted: vec!["src/old.rs".to_owned()],
        };

        weaver.enqueue_index_report(
            &report,
            |rel_path| format!("hash-of-{rel_path}"),
            |rel_path| vec![format!("sym:{rel_path}:1:main")],
        )?;

        // The report yields 5 tasks: 2 `FileChanged` (changed + added)
        // + 2 `NodeChanged` (one per changed+added file) + 1
        // `FileDeleted`. Deterministic synchronization: wait for all 5
        // `Succeeded` outcomes instead of polling on a sleep.
        let mut succeeded = 0;
        let mut unexpected = None;
        while succeeded < 5 {
            match outcomes.recv().await {
                Some(crate::enrichment::TaskOutcome::Succeeded { .. }) => succeeded += 1,
                other => {
                    unexpected = Some(other);
                    break;
                }
            }
        }
        assert_eq!(unexpected, None, "expected only Succeeded outcomes");

        let calls = embedder.calls();
        assert_eq!(calls.len(), 2, "one NodeChanged per changed+added file");
        let summaries = weaver.summaries();
        assert!(lock_or_msg(&summaries)?.get("src/old.rs").is_none());

        weaver.shutdown().await;
        Ok(())
    }

    #[tokio::test]
    async fn migration_cutover_never_mixes_generations() -> TestResult {
        let weaver = Weaver::builder().with_embedding_version(1).build();
        assert_eq!(
            weaver.embedding_generation(),
            EmbeddingGeneration::Stable { active: 1 }
        );

        weaver.begin_embedding_migration(2)?;
        let generation = weaver.embedding_generation();
        assert!(generation.is_migrating());
        assert_eq!(
            generation.active(),
            1,
            "active generation stays 1 during migration"
        );
        assert_eq!(generation.next(), Some(2));

        // Cannot start a second migration while one is in flight.
        assert!(weaver.begin_embedding_migration(3).is_err());

        let cutover = weaver.complete_embedding_migration()?;
        assert_eq!(cutover, 2);
        assert_eq!(
            weaver.embedding_generation(),
            EmbeddingGeneration::Stable { active: 2 }
        );

        // Cannot complete a migration that is not in flight.
        assert!(weaver.complete_embedding_migration().is_err());

        weaver.shutdown().await;
        Ok(())
    }
}
