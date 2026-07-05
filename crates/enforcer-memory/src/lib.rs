//! `enforcer-memory` -- x06: the harness memory graph.
//!
//! # Charter
//!
//! This crate is the retrievable/shareable harness-memory graph over
//! the x05 lesson corpus: it ingests append-only NDJSON memory records
//! (`memory/schema/memory-record.schema.json`) and the
//! `orchestration-lessons.md` ledger into an in-process graph
//! ([`graph::MemoryGraph`]), and exposes:
//!
//! - a **local-first, zero-network recall query** ([`recall::recall`]) —
//!   deterministic keyword matching over node text, no embeddings, no
//!   model download, safe to run in every test and every CI job;
//! - a **learning-evidence query** ([`recall::evidence`]) answering
//!   `memory evidence <lessonId>` with the t0 (observed) -> t1 (landed)
//!   -> t2 (recurrence) chain, fail-closed on missing provenance;
//! - the **usage-ingestion seam** ([`ingest::ingest_observation`]) that
//!   scan/check/run/doctor/closeout surfaces call on every run so
//!   enforcement usage automatically feeds the graph — including clean
//!   runs, which count as negative evidence.
//!
//! # Scope of this slice
//!
//! The workpack's full acceptance block
//! (`docs/plans/enforcer-selfhost-plan/workpacks/x06-harness-memory-graph.md`)
//! describes a much larger long-range system: a code-aware KG (files,
//! symbols, call graphs, ADR memory), dense-vector RAG with local
//! embedding/reranker models and HNSW sidecars, background "weaver"
//! enrichment workers, a live parity harness against an installed
//! codebase-memory-mcp baseline, and a 100-row QA benchmark with
//! measured token-reduction and reranker-lift curves. None of that ships
//! in this pass. What ships is the ingestion + graph + deterministic
//! recall + usage-ingestion-seam + evidence-chain core that the rest of
//! that system would sit on top of, kept strictly local-first and
//! zero-network by default (see [`retriever`] for the explicit
//! embedding seam, which is feature-gated and unimplemented here on
//! purpose so no test or default build ever needs a model runtime).
//!
//! # Federation
//!
//! Federation (sharing memory across machines/agents) is not
//! implemented in this slice. Per the crate's local-first mandate, any
//! future federation surface must be an explicit opt-in seam analogous
//! to [`retriever::EmbeddingRetriever`] — never a live dependency of the
//! default ingest/recall path.
//!
//! # X06.1 -- core/store/logs
//!
//! This slice adds the durable, on-disk foundation the rest of x06 sits
//! on: append-only observation/graph-event logs with an independently
//! verifiable SHA-256 hash chain ([`log`]), a SQLite operational
//! graph/read model rebuilt deterministically by replay
//! ([`store::sqlite`]), an analytics read model behind a swappable trait
//! ([`store::analytics`] — see its module docs for the DuckDB decision),
//! a content-addressed artifact manifest and index-manifest staleness
//! check ([`store::manifest`]), and the per-project [`store::Store`]
//! that ties them together with a "no ghost project database" open
//! contract. [`error::MemoryError`] is the single fail-closed error
//! surface all of it returns through; [`ids`] holds the store-local
//! identifier types ([`ids::Seq`], [`ids::ArtifactId`],
//! [`ids::ProjectId`]); [`schema`] holds the wire shapes these logs and
//! manifests persist.
//!
//! # Modules
//!
//! - [`record`] — the `MemoryRecord` wire type mirroring the schema.
//! - [`lesson`] — the orchestration-lessons ledger table parser.
//! - [`graph`] — the in-process node store.
//! - [`ingest`] — NDJSON parsing + the usage-ingestion seam.
//! - [`recall`] — the deterministic recall + evidence queries.
//! - [`retriever`] — the optional, feature-gated embedding seam.
//! - [`error`] — the crate-wide fail-closed error type.
//! - [`ids`] — store-local identifier types (`Seq`, `ArtifactId`, `ProjectId`).
//! - [`schema`] — wire shapes for logs, artifact manifest, index manifest.
//! - [`log`] — the append-only hash-chained log primitive.
//! - [`store`] — the per-project store: SQLite read model, analytics
//!   read model, artifact manifest, index manifests.

pub mod error;
pub mod graph;
pub mod ids;
pub mod ingest;
pub mod lesson;
pub mod log;
pub mod recall;
pub mod record;
pub mod retriever;
pub mod schema;
pub mod store;
