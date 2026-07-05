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
//! - [`code_graph`] — the X06.2 code KG indexer: files, symbols,
//!   imports, calls, routes, git metadata, incremental reindexing.
//! - [`parsers`] / [`languages`] — the tree-sitter-backed language
//!   extraction [`code_graph`] builds nodes/edges from.
//! - [`git`] — read-only git metadata ([`git2`]) for the indexer.
//! - [`analysis`] — X06.3: graph algorithms (related walk, call-path
//!   tracing, reverse dependency traversal, centrality/hotspots) over
//!   [`code_graph::CodeGraph`], plus [`analysis::query`], the read-only
//!   Cypher-subset query DSL (D-05).
//! - [`architecture`] — X06.3: architecture overview / repo mind map.
//! - [`impact`] — X06.3: impact analysis from a git diff.
//! - [`adr`] — X06.3: ADR memory linked to graph nodes.
//! - [`learning`] — lesson activation, supersede, per-domain learning curves.
//! - [`evidence`] — t0->t1->t2 chain with proof-ref seam + recurrence curve.
//! - [`observations`] — procedural memory + meta-memory (route/confidence).
//! - [`sessionstart`] — the SessionStart recall-pack seam.
//!
//! # X06.6 -- continuous learning
//!
//! [`learning`] adds lesson activation rules (landed = active,
//! unlanded/imported = inactive, superseded = never active) and
//! per-domain aggregate learning-curve emission; [`evidence`] extends
//! [`recall::evidence`] with enforcer-proof journal refs per chain
//! element (via a caller-supplied [`evidence::ProofRefLookup`] seam,
//! never a hard dependency on `enforcer-proof`) and an ordered
//! recurrence curve; [`observations`] adds procedural memory (fix/
//! retrieval success AND failure) and meta-memory (route choice +
//! confidence) as their own append-only record kinds on
//! [`graph::MemoryGraph`]; [`sessionstart`] computes the bounded,
//! deterministic recall-pack payload a Claude SessionStart hook (c05,
//! `crates/enforcer-install/**`, out of this crate) would inject at the
//! start of a new session.
//!
//! # X06.4 -- full-text/vector/rerank retrieval stack
//!
//! This slice adds the hybrid retrieval pipeline the workpack's "modern
//! production RAG" half of the vision (OWNER_INTENT) requires: a
//! code-aware full-text index ([`fulltext`]), an HNSW dense vector index
//! ([`vector`]), the embedding/reranking capability seams
//! ([`embed`]/[`rerank`], deterministic zero-network defaults per D-03),
//! RRF rank fusion with hard-filter exclusion ([`ranking`]), and the
//! [`search::HybridSearcher`] that wires all of it together while
//! extending (not forking) the existing [`retriever::EmbeddingRetriever`]
//! seam.

pub mod adr;
pub mod analysis;
pub mod architecture;
pub mod code_graph;
pub mod embed;
pub mod error;
pub mod evidence;
pub mod fulltext;
pub mod git;
pub mod graph;
pub mod ids;
pub mod impact;
pub mod ingest;
pub mod languages;
pub mod learning;
pub mod lesson;
pub mod log;
pub mod observations;
pub mod parsers;
pub mod ranking;
pub mod recall;
pub mod record;
pub mod rerank;
pub mod retriever;
pub mod schema;
pub mod search;
pub mod sessionstart;
pub mod store;
pub mod vector;
