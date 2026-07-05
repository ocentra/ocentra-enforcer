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
//! # Modules
//!
//! - [`record`] — the `MemoryRecord` wire type mirroring the schema.
//! - [`lesson`] — the orchestration-lessons ledger table parser.
//! - [`graph`] — the in-process node store.
//! - [`ingest`] — NDJSON parsing + the usage-ingestion seam.
//! - [`recall`] — the deterministic recall + evidence queries.
//! - [`retriever`] — the optional, feature-gated embedding seam.

pub mod graph;
pub mod ingest;
pub mod lesson;
pub mod recall;
pub mod record;
pub mod retriever;
