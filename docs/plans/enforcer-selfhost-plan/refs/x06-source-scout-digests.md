# x06 Source Scout Digests — grounded intelligence on the 4 external sources

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `x06-source-scout-digests`
> Kind: orchestrator-integrated scout reports (2026-07-04, haiku scouts, verified shape by x06 orchestrator). Grounds every x06 worker prompt in what the DESIGN_INPUTS sources ACTUALLY contain.
> Read when: implementing any X06.2–X06.9 subpack, before the single allowed research pass — this doc may replace that pass entirely.
> Stop rule: digests only. Borrow authority comes from MEMORY_RETRIEVAL_DESIGN_INPUTS; binding vision from MEMORY_RETRIEVAL_OWNER_INTENT.
> Proves: nothing. Scouts are advisory; verify at point of copy.
<!-- /agent-capsule -->

## 1. codebase-memory-mcp (parity baseline) — DeusData/codebase-memory-mcp

Pure C, ~67.5K LOC, zero deps beyond vendored tree-sitter + SQLite. Single static binary; MCP stdio JSON-RPC server + `cli <tool> <json>` mode + optional HTTP graph UI (localhost:9749).

**The 14-tool parity floor** (every one must exist in enforcer-memory MCP+CLI and be benchmarked):

| # | Tool | Notes for parity |
|---|---|---|
| 1 | `index_repository` | modes full/moderate/fast; cross-repo intelligence; `persistence` exports `.codebase-memory/graph.db.zst` |
| 2 | `search_graph` | 3 modes: BM25 (camelCase split, label boosts +10 Function/+8 Route/+5 Class), regex `name_pattern`, `semantic_query` keyword-array cosine. Pagination, label/file/degree filters |
| 3 | `query_graph` | READ-ONLY Cypher subset: MATCH/WHERE/RETURN/ORDER BY/LIMIT/DISTINCT/aggregates/CASE; node complexity props (cyclomatic, cognitive, loop depth, recursion flags); 100k row ceiling |
| 4 | `trace_path` | modes calls/data_flow/cross_service; direction in/out/both; depth default 3 |
| 5 | `get_code_snippet` | by qualified_name, optional neighbors |
| 6 | `get_graph_schema` | node labels + edge types |
| 7 | `get_architecture` | aspects incl. Leiden/Louvain clustering, hotspots, layers, file_tree |
| 8 | `search_code` | graph-augmented grep: text match → containing function → rank by structural importance; modes compact/full/files |
| 9 | `list_projects` | — |
| 10 | `delete_project` | — |
| 11 | `index_status` | — |
| 12 | `detect_changes` | git diff → affected symbols + risk classification; base_branch/since |
| 13 | `manage_adr` | get/update/sections |
| 14 | `ingest_traces` | runtime traces {caller, callee, count} enrich CALLS edges |

**Indexing**: 7-stage pipeline (structure → definitions → imports → calls → usages → semantic → post). tree-sitter for 158 languages; LSP-grade type-aware call resolution for 9 languages with registry fallback. SQLite store (in-memory during pipeline, then persisted). zstd artifact (`graph.db.zst` + `artifact.json` with schema version + git hash) for team bootstrap. Incremental via file-hash table (rel_path, sha256, mtime_ns, size) — but re-index on change is still full, not truly incremental (**we can beat this**).

**Graph model**: labels Function/Method/Class/Interface/Module/Package/File/Route/Resource/Variable/Enum/Struct/Lambda…; edges CALLS/IMPORTS/HTTP_CALLS/ASYNC_CALLS/INHERITS/IMPLEMENTS/DECORATES/DEFINES/TYPE_REF/DATA_FLOW/SIMILAR_TO/SEMANTICALLY_RELATED/CROSS_*.

**"Semantic" search has NO neural model** — 11 in-process lexical/structural signals: TF-IDF, Random Indexing (768D sparse, co-occurrence window 5), MinHash, API-signature vectors, type-signature vectors, module proximity, decorator patterns, AST structural profile, data-flow approximation, graph diffusion, Halstead-lite. Frequent-token cap 512. Our dense+reranker stack should clear its nDCG; our BM25 must match its camelCase-split + label-boost behavior.

**Watcher**: background thread polls git per project; adaptive 5s base + 1s/500 files, cap 60s; re-index in supervised subprocess for memory isolation. Rust `notify` crate can beat polling.

**Diagnostics**: structured KV logs to stderr ONLY (stdout reserved for JSON-RPC), levels via `CBM_LOG_LEVEL`, text/json formats, per-pass timing lines, per-file skip records (path, reason, phase) — never silent skip, never fail the run.

**Parity difficulty map**: hardest = LSP type-aware call resolution (9 langs) and 158-grammar breadth; our scoped floor (Rust/TS-JS/Py/config + TextOnly fallback, registry-fallback resolution) is the pack-mandated floor — parity fixtures use our supported languages, recorded honestly. Cypher-subset evaluator is real work (X06.3) but bounded: read-only by construction satisfies our safe-DSL doctrine.

## 2. TabAgentServer (harvest) — ocentra/TabAgentServer (public, clones cleanly)

| Area | Verdict | What |
|---|---|---|
| `Rust/weaver` | **COPY-AND-REWIRE (highest value)** | Production-ready event-driven enrichment: tokio MPSC unbounded channel, worker pool sized by `num_cpus`, 4 modules (SemanticIndexer, EntityLinker, AssociativeLinker 2–3-hop, Summarizer stub), `MlBridge` async_trait abstraction + mock for tests, 10 passing tests. Copy `lib.rs`, `events.rs`, `ml_bridge.rs`, modules; add enforcer events + dead-letter queue + bounded retry + hot/warm/cold queue priority (TabAgent lacks DLQ — new work per X06.5 pack) |
| `Rust/indexing` | **COPY-AND-REWIRE** | HNSW via `hnsw_rs` 0.3 (pure Rust, concurrent) with hot/warm/cold caching; B-tree structural property indexes; adjacency-list graph indexes O(1) neighbor lookup; 20+ petgraph algorithms (dijkstra, articulation points, community detection, dominators); batch ops; `memmap2` zero-copy for cold tier. Metadata payload format is domain-specific — reuse machinery, swap schema |
| `Rust/knowledge-graph` | HARVEST-IDEAS-ONLY | Thin facade over indexing; value is the layering pattern (KG ≠ storage ≠ indexing), which enforcer-memory already follows |
| `MIA_VISION.md` + `Rust/docs/mia_memory.md` | COPY FRAMEWORK | 7 specialized DBs (source-of-truth / embeddings / knowledge / tool-results / experience / summaries / meta); source-vs-derived; hot/warm/cold with lazy promotion/demotion; temperature-scoped query API (`Query { semantic, time_scope, context, use_knowledge_graph, search_depth, temperature, limit }`); confidence-scored results with traversal reasoning ("found via 2-hop: A → implies → B") |
| Deps to adopt | — | `tokio` full, `hnsw_rs` 0.3, `petgraph` 0.8, `async-trait`, `thiserror` 2, `dashmap` 6, `ordered-float`, `memmap2`, `rkyv` (eval carefully) |
| Deps to REJECT | — | `qdrant-client` (external service — violates laptop-local owner contract), `libmdbx` (X06.1 doctrine is rusqlite-bundled SQLite) |
| `execution-providers` (~236 lines) | **DEPEND/EXTRACT for X06.4** | CPU/GPU/NPU detection + fallback ordering — the hardware-routing piece x06 needs |
| `model-cache` (~27 lines) | VENDOR-AND-TRIM | chunked model download + manifest/quant tracking patterns |

## 3. OcentraParent (runtime) — **CORRECTION to DESIGN_INPUTS Source 3**

**There is NO production local-model runtime in OcentraParent yet.** Branch `codex/tracking-plan-full-continuation-a` is at contract-definition (V0.6) / design (V0.7): `child-ai-core`, `screen-ai-core`, `parent-runtime-core` own boundaries and orchestration, zero inference code; execution disabled by default (`executionAllowed=false`). Its own reuse index points back to TabAgentServer for runtime pieces.

**What we DO adopt (contracts, not code):**

- `LoadState: unavailable | loading | loaded | degraded | failed`
- `ResourceClass: cpu | gpu | npu`
- typed `DegradedState` reasons: provider-unavailable, overloaded, invalid-output, low-confidence
- status surface decoupled from execution (`LocalModelRuntimeStatus` → loadState, capabilityFlags, resourceClass, degradedState, lastCheckedTime)

**Consequence for X06.4**: enforcer-memory implements its own inference layer (embed + rerank) behind a trait seam, harvesting TabAgentServer `execution-providers`/`model-cache` patterns and adopting the OcentraParent capability-state contract. Backend selection (ort/onnxruntime vs candle vs llama.cpp-class) is an orchestrator/owner decision recorded in the X06.4 worker prompt and `proof/memory/x06-models.json`.

## 4. Rag-Guide (doctrine) — concrete prescriptions per subpack

Full extraction preserved at the scout's `rag-guide-x06-prescriptions.txt`; binding numbers:

**X06.2 (chunking/manifests)**: AST-aware code chunking (function/class units, imports preserved); parent-child chunking (index small children, return large parents); recursive section→paragraph→sentence fallback for text; sizes: FAQ 100–300 tok, API 300–800, policy 500–1200, legal 800–2000. Every derived vector/chunk carries: embedding_model, model_version, dimension, dtype, similarity_metric, normalization, input_formatter_version, chunker_version, parser_version, source_document_version — changing ANY invalidates. Chunking quality is the retrieval ceiling.

**X06.4 (retrieval/fusion/rerank)**: hybrid dense+BM25 is the production default; **RRF fusion, k≈60**, combine ranks not scores; retrieve 50–200 candidates → dedupe → rerank 20–40 → context 5–10; cross-encoder reranker default; hard filters (ACL/trust) EXCLUDE before rerank, soft signals (freshness/authority) only BOOST; measure Recall@100 pre-rerank (reranker cannot fix a missing candidate); window long chunks or rerank children + return parents — never silently truncate.

**X06.5 (context assembly/weaver)**: greedy token budgeting; citations from metadata (S1, S2…), never model memory; contradictions cited both-sides with version/date/authority; direct evidence early (lost-in-the-middle); extractive compression over abstractive for high-stakes; blue/green index migration (build parallel, shadow, compare recall/latency, cut over — NEVER mix vector versions); tombstone deletion across ALL indexes/caches/logs.

**X06.9 (eval)**: rows as `{query, relevant_chunk_ids[], relevance_grades, category, filters}`; 100–300 rows to start, 1000+ mature (matches QA-250 plan); benchmark 4 baselines on same data: BM25-only, dense-only, hybrid, hybrid+reranker (this is how reranker lift is proven); error-analysis buckets: parser → chunking → embedding → lexical → ACL → reranker → context → generation (matches parity-harness §5 failure classification); frozen regression set + bootstrap resampling + per-category reporting for the longitudinal ratchet.

**Observability (all subpacks)**: trace every answer end-to-end (raw query, rewrite, filters, retriever candidates, reranker scores, context, citations, latency, cost, feedback); per-chunk status tracking; content-hash + model-version dedup on batch embedding.
