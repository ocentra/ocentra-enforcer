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
//! # Federation (X06.8)
//!
//! Federation (sharing memory across machines/agents) is an explicit
//! opt-in surface, never a live dependency of the default ingest/recall
//! path (the crate's local-first mandate): [`share`] exports signed,
//! zstd-compressed bundles (personal DEFAULT / team / community scopes,
//! explicit per-call consent required beyond personal); [`federation`]
//! is the zero-trust import gate (signature vs a local trust list,
//! checksum, schema version — typed rejection reasons, imported lessons
//! forced inactive until local x05 validation supersedes them);
//! [`redaction`] is the community-scope redaction pipeline (paths,
//! identities, secret-shaped strings, bounded snippets — golden
//! byte-exact fixture-tested); [`artifacts`] is exact fail-closed
//! content-addressed retrieval plus the D-11 `.codebase-memory/
//! graph.db.zst` + `artifact.json` code-graph bootstrap artifact.
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
//! - [`queue`] — the X06.5 weaver's event-driven priority queue
//!   (hot/warm/cold) plus its dead-letter queue and retry/backoff
//!   policy.
//! - [`enrichment`] — the X06.5 weaver's worker abstraction: the
//!   semantic indexer / entity linker / associative linker /
//!   summarizer dispatch, the bounded-concurrency worker pool, and the
//!   [`enrichment::Embedder`] seam X06.4's embedder is adapter-wired
//!   into at integration.
//! - [`summaries`] — the X06.5 weaver's summary cache and
//!   entity-link table.
//! - [`weaver`] — the X06.5 background weaver: wires [`queue`] to
//!   [`enrichment`], translates [`code_graph::IndexReport`] into
//!   weaver events, and owns the blue/green embedding-version
//!   migration cutover.
//!
//! # X06.P1 -- parity read tools
//!
//! Library functions underneath 5 of the codebase-memory-mcp parity
//! baseline's 14 tools (scout digest §1); the MCP/CLI wrapper surface
//! (X06.7) wires these in at integration, not part of this slice.
//!
//! - [`snippet`] — `get_code_snippet`: byte-exact source retrieval by
//!   qualified symbol name, with SHA-256 verification and an optional
//!   same-file neighbor listing. Fails closed on an unknown symbol --
//!   never a similar-name substitute.
//! - [`graph_schema`] — `get_graph_schema`: node labels and edge types
//!   present in a [`code_graph::CodeGraph`], with counts, in
//!   deterministic order.
//! - [`code_search`] — `search_code`: graph-augmented grep -- a
//!   regex/text scan over indexed file contents, each hit enriched with
//!   its containing symbol and ranked by structural importance (inbound
//!   call-degree). Unreadable files are reported, never silently
//!   skipped.
//! - [`projects`] — the project registry (`list_projects`/
//!   `delete_project`/`index_status`) over the X06.1
//!   [`store::Store`] layout: one store per project under a root.
//!   Delete removes only the derived store and refuses any path outside
//!   the store root.
//!
//! # X06.P2 -- trace_path / ingest_traces / detect_changes parity
//!
//! - [`analysis::trace`] — `trace_path`'s three modes (calls/data_flow/
//!   cross_service) plus its baseline-verified `risk_labels` hop-distance
//!   scheme, layered over [`analysis::CodeAdjacency`].
//! - [`traces`] — `ingest_traces`: an additive runtime-call-trace overlay
//!   over [`code_graph::CodeGraph`] (the baseline's own `ingest_traces`
//!   is an unimplemented stub, so this module's merge/idempotency/
//!   provenance semantics are this crate's own documented design).
//! - [`impact::detect_changes_view`] — the baseline-parity `detect_changes`
//!   response shape (file-level `impacted_symbols`, no risk field);
//!   [`impact::analyze_diff_impact_scoped`] is a separate, richer,
//!   non-parity risk-classification extension layered alongside it.
//!
//! # X06.7 -- MCP/CLI wrapper, filesystem watcher, diagnostics
//!
//! - [`mcp`] — the MCP stdio JSON-RPC server exposing the
//!   codebase-memory-mcp 14-tool parity floor (`tools/list`/`tools/call`,
//!   dual framing via [`enforcer_mcp::transport`], honest `not_wired`
//!   results for genuinely unlanded tools/modes).
//! - [`cli`] — the CLI mirror of [`mcp`]'s tool surface: same registry,
//!   same dispatch, same envelope, so CLI and MCP are call-for-call
//!   identical.
//! - [`watch`] — the D-12 filesystem watcher: native OS events (`notify`)
//!   with debounce, plus an adaptive-polling + git-HEAD-diff fallback.
//! - [`diagnostics`] — stderr-only structured KV/JSON logging for the
//!   MCP/CLI/watch surface, with redaction so no raw source text ever
//!   reaches a log line.

pub mod adr;
pub mod analysis;
pub mod architecture;
pub mod artifacts;
pub mod cli;
pub mod code_graph;
pub mod code_search;
pub mod diagnostics;
pub mod embed;
pub mod enrichment;
pub mod error;
pub mod evidence;
pub mod federation;
pub mod fulltext;
pub mod git;
pub mod graph;
pub mod graph_schema;
pub mod ids;
pub mod impact;
pub mod ingest;
pub mod languages;
pub mod learning;
pub mod lesson;
pub mod log;
pub mod mcp;
pub mod observations;
pub mod parsers;
pub mod projects;
pub mod queue;
pub mod ranking;
pub mod recall;
pub mod record;
pub mod redaction;
pub mod rerank;
pub mod retriever;
pub mod schema;
pub mod search;
pub mod sessionstart;
pub mod share;
pub mod snippet;
pub mod store;
pub mod summaries;
pub mod traces;
pub mod vector;
pub mod watch;
pub mod weaver;
