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
//! summarizer stub) per `refs/x06-source-scout-digests.md` Â§2 and
//! `MEMORY_RETRIEVAL_DECISIONS.md` D-09 (LOCKED). Every mechanism below
//! that the digest calls out as absent from the source (dead-letter
//! queue, bounded backoff retry, hot/warm/cold priority, blue/green
//! embedding-version migration) is new work, not adapted from
//! TabAgentServer.
//!
//! # Blue/green embedding migration (Rag-Guide, D-09)
//!
//! "Never mix vector versions" (digest Â§4). [`EmbeddingGeneration`]
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

use crate::enrichment::{
    Embedder, EnrichmentContext, NullEmbedder, SharedSummaryStore, WorkerPool, WorkerPoolConfig,
};
use crate::owned_boundary::Retained;
use crate::queue::{QueueSendError, WeaverEvent, WeaverQueue, WeaverQueueHandle};
use crate::summaries::SummaryStore;
use enforcer_domain::memory_types::{
    EmbeddingGeneration, EmbeddingGenerationId, MemoryPriority, TaskOutcome, WeaverContentHash,
    WeaverNodeIds, WeaverRelativePath, WorkerConcurrency,
};
use std::sync::{Arc, Mutex};

/// The state of a blue/green embedding-version migration. `Stable`
/// means every task is stamped with `active`; `Migrating` means a
/// green generation is being built alongside blue -- callers building
/// the green index must use `next`, never mix it with `active`'s
/// vectors (Rag-Guide, digest Â§4).
/// Error attempting an invalid migration transition (e.g. completing a
/// migration that was never started, or starting one that is already
/// in flight).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MigrationError {
    #[error("an embedding migration is already in progress (active={active}, next={next})")]
    AlreadyMigrating {
        active: EmbeddingGenerationId,
        next: EmbeddingGenerationId,
    },
    #[error("no embedding migration is in progress to complete")]
    NotMigrating,
}

/// The weaver: owns the queue handle producers enqueue through, the
/// running worker pool, and the current embedding generation. Built via
/// [`WeaverBuilder`] so tests can swap in [`crate::enrichment::FlakyEmbedder`]
/// without threading extra parameters through every constructor.
#[derive(Debug)]
pub struct Weaver {
    queue_handle: WeaverQueueHandle,
    pool: WorkerPool,
    generation: Arc<Mutex<EmbeddingGeneration>>,
    summaries: SharedSummaryStore,
}

/// Builder for [`Weaver`] -- defaults to [`NullEmbedder`] and
/// [`crate::enrichment::WorkerPoolConfig::with_default_concurrency`] so
/// the zero-argument path (`WeaverBuilder::default().build()`) never
/// needs a model runtime, matching the crate-wide "no test needs a
/// model download" contract.
pub struct WeaverBuilder {
    embedder: Arc<dyn Embedder>,
    max_concurrency: Option<WorkerConcurrency>,
    retry: crate::queue::RetryPolicy,
    embedding_version: EmbeddingGenerationId,
    on_outcome: Option<tokio::sync::mpsc::Sender<TaskOutcome>>,
}

impl std::fmt::Debug for WeaverBuilder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WeaverBuilder")
            .field("embedder", &"dyn Embedder")
            .field("max_concurrency", &self.max_concurrency)
            .field("retry", &self.retry)
            .field("embedding_version", &self.embedding_version)
            .field("on_outcome", &self.on_outcome.is_some())
            .finish()
    }
}

/// Configured Weaver builder plus its deterministic task-outcome receiver.
pub struct WeaverOutcomeChannel {
    pub builder: WeaverBuilder,
    pub outcomes: tokio::sync::mpsc::Receiver<TaskOutcome>,
}

impl std::fmt::Debug for WeaverOutcomeChannel {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WeaverOutcomeChannel")
            .field("builder", &self.builder)
            .field("outcomes", &"Receiver<TaskOutcome>")
            .finish()
    }
}

impl Default for WeaverBuilder {
    fn default() -> Self {
        Self {
            embedder: Arc::new(NullEmbedder::new()),
            max_concurrency: None,
            retry: crate::queue::RetryPolicy::bounded_default(),
            embedding_version: EmbeddingGenerationId::INITIAL,
            on_outcome: None,
        }
    }
}

impl WeaverBuilder {
    pub fn with_embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.embedder = embedder;
        self
    }

    pub fn with_max_concurrency(mut self, max_concurrency: WorkerConcurrency) -> Self {
        self.max_concurrency = Some(max_concurrency);
        self
    }

    pub fn with_retry_policy(mut self, retry: crate::queue::RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    pub fn with_embedding_version(mut self, version: EmbeddingGenerationId) -> Self {
        self.embedding_version = version;
        self
    }

    /// Attach a per-attempt [`TaskOutcome`] channel,
    /// returning the receiver half. Hard tests use this to `.await` a
    /// specific outcome (e.g. the retry that eventually dead-letters)
    /// instead of polling `tokio::time::sleep` in a loop -- see
    /// `tests/weaver_enrichment.rs`.
    pub fn with_outcome_channel(mut self) -> WeaverOutcomeChannel {
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        self.on_outcome = Some(tx);
        WeaverOutcomeChannel {
            builder: self,
            outcomes: rx,
        }
    }

    /// Build and start the weaver: spawns the worker pool immediately
    /// (background task), returning a ready-to-enqueue handle. This
    /// call itself never blocks on any worker running.
    pub fn build(self) -> Weaver {
        let queue = WeaverQueue::new();
        let queue_handle = queue.handle();
        let summary_stores = SharedSummaryStore::shared_pair();
        let summaries = summary_stores.primary;
        let enrichment_summaries = summary_stores.enrichment;
        let generation = Arc::new(Mutex::new(EmbeddingGeneration::Stable {
            active: self.embedding_version,
        }));
        let ctx = Arc::new(EnrichmentContext::with_shared_summaries(
            self.embedder,
            enrichment_summaries,
            self.embedding_version,
        ));

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

        let pool = WorkerPool::spawn(queue, queue_handle.retained(), ctx, &config);

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
        self.queue_handle.retained()
    }

    /// Enqueue a single event at its default priority (see
    /// [`crate::enrichment::default_priority`]).
    pub fn enqueue(&self, event: WeaverEvent) -> Result<(), QueueSendError> {
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
        content_hashes: impl Fn(&WeaverRelativePath) -> WeaverContentHash,
        node_ids_by_path: impl Fn(&WeaverRelativePath) -> WeaverNodeIds,
    ) -> Result<(), QueueSendError> {
        for rel_path in report.changed.iter().chain(report.added.iter()) {
            let rel_path = WeaverRelativePath::from(rel_path);
            let content_hash = content_hashes(&rel_path);
            self.queue_handle.send(
                WeaverEvent::FileChanged {
                    rel_path: rel_path.retained(),
                    content_hash: content_hash.retained(),
                },
                MemoryPriority::Hot,
            )?;
            for node_id in node_ids_by_path(&rel_path) {
                self.queue_handle.send(
                    WeaverEvent::NodeChanged {
                        node_id,
                        rel_path: rel_path.retained(),
                        content_hash: content_hash.retained(),
                    },
                    MemoryPriority::Hot,
                )?;
            }
        }
        for rel_path in &report.deleted {
            self.queue_handle.send(
                WeaverEvent::FileDeleted {
                    rel_path: rel_path.into(),
                },
                MemoryPriority::Warm,
            )?;
        }
        Ok(())
    }

    /// Run one synchronized operation against the summary/link store.
    /// The raw lock never crosses this boundary, so callers cannot retain a
    /// guard or couple their API to the store's synchronization primitive.
    pub fn with_summaries<T>(&self, operation: impl FnOnce(&mut SummaryStore) -> T) -> T {
        self.summaries.with_mut(operation)
    }

    pub fn embedding_generation(&self) -> EmbeddingGeneration {
        // A poisoned lock (prior panic while held) is treated as
        // "assume stable at generation 0" rather than propagating a
        // panic from a read-only accessor -- this mirrors the crate's
        // existing poison-tolerant `Mutex` reads in `enrichment.rs`.
        self.generation
            .lock()
            .map(|g| *g)
            .unwrap_or(EmbeddingGeneration::Stable {
                active: EmbeddingGenerationId::RECOVERY,
            })
    }

    /// Begin a blue/green migration to `next_version`: new
    /// [`crate::enrichment::EmbeddingTask`]s continue being stamped with
    /// the current active version (blue) until
    /// [`Weaver::complete_embedding_migration`] cuts over -- this call
    /// only records that a green generation is being built, matching
    /// Rag-Guide's "build parallel, shadow, compare, cut over" sequence
    /// (digest Â§4); it does not itself re-embed anything (X06.4's
    /// concern once wired).
    pub fn begin_embedding_migration(
        &self,
        next_version: EmbeddingGenerationId,
    ) -> Result<(), MigrationError> {
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
    pub fn complete_embedding_migration(&self) -> Result<EmbeddingGenerationId, MigrationError> {
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
