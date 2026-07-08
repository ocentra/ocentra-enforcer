# MEMORY_RETRIEVAL_DECISIONS — x06 architecture decision ledger

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_DECISIONS`
> Kind: append-only ADR ledger for x06. Locked decisions are not re-litigated by workers; a worker that believes a decision is wrong reports back with evidence instead of deviating.
> Read when: before implementing any subpack; when choosing a dependency/backend/format; when a scout/worker finding contradicts a design assumption.
> Stop rule: decisions only. New entries are appended by the orchestrator (or owner); status changes are edits to the Status field only.
> Proves: nothing. Proof artifacts prove; this prevents wondering and re-derivation.
<!-- /agent-capsule -->

Format per entry: context → decision → consequences. Status: LOCKED (execute as stated) | DEFAULT (execute unless the named revisit-trigger fires) | OPEN (orchestrator/owner must close before the named subpack spawns).

## D-01 — Operational store: SQLite via `rusqlite` (bundled) — LOCKED
Owner-set in the X06.1 dispatch. Context: local-first, zero-install, Windows-safe. Decision: SQLite is the operational graph/read model; bundled feature so no system dependency. Analytics is a DuckDB-or-fallback seam (X06.1), not a second source of truth. Consequence: no libmdbx, no sled, no external DB server, ever, for the operational store.

## D-02 — Truth lives in append-only logs; every index is derived and rebuildable — LOCKED
From Rag-Guide doctrine + TabAgent indexing philosophy + owner-set continuous-indexing philosophy. Hash-chained NDJSON logs (with independent sidecar per L37) are the source of truth; SQLite/vector/full-text indexes are disposable projections with manifests + high-watermarks. Consequence: any index can be deleted and rebuilt deterministically; rebuild determinism is a hard test.

## D-03 — Local inference backend: backend-neutral local runtime; llama.cpp/GGUF first-class, ONNX Runtime optional — LOCKED
Context: live OcentraParent truth uses a llama.cpp/local llama CLI runtime with CPU/Vulkan/CUDA distribution selection and local-only provider status; treating `ort` as the default runtime was a drifted assumption. Laptop contract remains Windows-first with CPU/GPU/NPU routing. Decision: X06 implements backend-neutral local runtime contracts: `llama-cpp`/GGUF is first-class for Parent-compatible local runtime semantics; `onnx-ort` remains an optional embedding/reranker backend behind the explicit `ort-models` seam; deterministic fallback remains the zero-network default. Capability-state contract is adopted from OcentraParent shapes (`LoadState: unavailable|loading|loaded|degraded|failed`, `ResourceClass: cpu|gpu|npu`, typed degraded reasons); provider/runtime ordering prefers explicit local config, then llama.cpp cache-ready candidates, then ORT only when feature/cache allow it, then deterministic degraded fallback. Consequence: `proof/memory/x06-models.json` records backend, model ids, artifact format, acceleration/provider, and capability state per run; degraded mode is labeled and is NOT accepted for feature parity.

## D-04 — Vector index: `hnsw_rs` (pure Rust), memory-mapped cold tier — DEFAULT
From TabAgent indexing harvest. Pure Rust, concurrent, no C++ toolchain. Consequence: no qdrant-client or any external vector service (violates laptop-local, owner-set). Revisit-trigger: scale tiers in longitudinal benchmarks (1M nodes) failing latency targets.

## D-05 — Graph query surface: read-only Cypher-subset DSL — LOCKED
Parity requires `query_graph`-class behavior (baseline supports MATCH/WHERE/RETURN/ORDER BY/LIMIT/aggregates); doctrine requires a safe DSL with no raw database execution through MCP. Decision: X06.3 implements a read-only-by-construction Cypher subset evaluator over the enforcer graph (write verbs recognized and rejected). Row ceiling like baseline (100k). Consequence: unsafe-query rejection is a hard test; MCP never receives SQL.

## D-06 — Parser layer: tree-sitter for Rust, TS/JS, Python, config (toml/json/yaml); everything else is `TextOnly` — LOCKED
The pack-mandated floor. 158-language breadth and LSP-grade type-aware resolution are explicitly out of the parity fixtures (fixtures use supported languages); call resolution uses a symbol-registry fallback approach (baseline's own fallback mode). Consequence: never silent-skip — unsupported files become TextOnly nodes with full-text coverage; per-file skip/fallback records in diagnostics.

## D-07 — Full-text: code-aware BM25 matching baseline behaviors — LOCKED
Must match scout-documented baseline behaviors: camelCase/snake_case/kebab/path/symbol tokenization, structural label boosts (Functions > Routes > Classes), pagination. Implementation is enforcer-native (tantivy or custom over SQLite FTS — X06.4 worker proposes, orchestrator approves before merge; record choice here as D-07a when made).

### D-07a — Full-text engine: SQLite FTS5 over the crate's existing `rusqlite` (bundled) dependency, NOT tantivy — CONFIRMED (proposed by X06.4 worker; confirmed by orchestrator at integration, 2026-07-05)
Context: `enforcer-memory` already depends on `rusqlite` with the `bundled` feature for the X06.1 operational store; that same bundled SQLite build has `SQLITE_ENABLE_FTS5` on by default (verified locally — `CREATE VIRTUAL TABLE t USING fts5(body)` succeeds with zero extra Cargo features). Decision: tokenization (camelCase/snake_case/kebab/path splitting) happens in Rust before insert into a `tokenize='unicode61'` FTS5 table populated with pre-tokenized text; ranking uses FTS5's built-in `bm25()` function, negated and multiplied by the D-07 structural label boost, with results re-sorted by the boosted score (not the raw bm25 order) before truncation to the caller's limit. Micro-benchmark (40 synthetic documents, debug build, single-threaded, median of 20 runs): SQLite FTS5 index build ~0.9ms / query ~0.06ms vs tantivy's published-bench-projected ~2-4ms build / ~0.05-0.1ms query at this corpus scale — within noise of each other; the decision is dependency weight (zero new heavy deps, one on-disk index format instead of two, one "is my index stale" story instead of two), not raw speed, matching the borrow-policy bias toward fewest heavy deps meeting behavior. Consequence: no tantivy dependency added to `enforcer-memory`. Revisit-trigger: unchanged from D-07 — a later longitudinal benchmark (1M+ documents) showing FTS5 query latency growing unacceptably.

## D-08 — Fusion and rerank pipeline: hybrid dense+BM25, RRF k≈60, retrieve 100–200 → rerank 20–40 → context 5–10 — LOCKED
Rag-Guide doctrine (architectural law) + owner-set model philosophy (never run expensive models on the entire corpus). Hard filters (permission/trust) EXCLUDE before rerank; soft signals (recency/authority/proof-linkage) only boost. Recall@100 measured pre-rerank. Reranker-lift measured as the QA gate requires.

## D-09 — Weaver: event-driven queue harvested from TabAgentServer pattern + DLQ/retry/hot-warm-cold added — LOCKED
Foreground never blocks on enrichment (owner-set). tokio MPSC + worker pool pattern re-expressed for enforcer events; dead-letter queue, bounded backoff retry, and hot/warm/cold priorities are x06 additions the source lacks. Blue/green index migration for embedding-version changes; never mix vector versions (Rag-Guide).

## D-10 — Memory hierarchy: MIA specialized-memories model translated to enforcer — LOCKED
Working/episodic/semantic/procedural/experience/summary/meta memories over hot/warm/cold tiers (owner-set); every retrieval trace records tier + route + confidence. Source-vs-derived split follows D-02.

## D-11 — Compressed graph artifact: zstd bundle with manifest + git hash, parity-comparable to baseline `.zst` artifact — DEFAULT
For team bootstrap (X06.8). Enforcer-native schema with schema-version; must satisfy the parity harness "compressed graph artifact" row.

## D-12 — Watcher: `notify` crate (filesystem events) with git-state polling fallback — DEFAULT
Improves on baseline's pure polling. Debounced; incremental reindex path per D-02 manifests. Revisit-trigger: cross-platform flakiness in CI → fall back to adaptive polling like baseline.

## D-13 — Owner-set markers are owner-only — LOCKED
Workers and orchestrators never mint `(owner-set)` tags (L41 incident). Gatekeeper diff-greps every lane for ADDED `(owner-set` lines; any non-owner addition is auto-reject. Candidate T1 mechanization filed with L41.

## D-14 — QA row anchors must exist — LOCKED
Every QA row (001–250) anchors to a real repo symbol/file/rule/workpack or a deterministic synthetic-corpus anchor. Gatekeeper verifies counts mechanically (L41). Rows without recorded per-row results in `proof/memory/x06-rag-qa.json` are FAILING, not pending (owner-set, QA_PROOF_GATE).

## D-15 — Tokenizer/model-cache dependency policy — LOCKED
No tokenizer, Hugging Face fetch, llama.cpp binary fetch, or ONNX Runtime dependency enters the default build. Runtime dependencies land only behind explicit backend seams (`llama-cpp` cache/binary integration, `ort-models` ONNX integration) and must preserve a zero-network default: local cache validation first, explicit-network fetch only when a caller opts in, manifest/hash/tokenizer validation before load, and degraded/provider-unavailable reporting on every missing or invalid artifact. X06 model-runtime harvest may define contracts, proof shapes, and fixture validation before adding those dependencies; compile green without cache/hash/tokenizer/provider proof is not accepted as parity.

## D-16 — Backend-neutral local runtime: llama-cpp/GGUF first-class, ONNX/ORT optional — DEFAULT
Context: the x06 local-runtime slice now has an explicit backend-neutral selection contract in `crates/enforcer-memory/src/local_runtime.rs`, and that contract is clearer than the older ONNX-first wording. Decision: the first-class local path is `llama-cpp` loading `GGUF` artifacts; `onnx-ort` stays optional behind the `ort-models` seam; deterministic fallback remains the safety net only. Consequence: proof artifacts must state the policy separately from runtime execution evidence, and `proof/memory/x06-models.json` / `proof/memory/x06-feature-parity.json` must stay honest about contract-only status until a real backend run exists.

Remaining blockers for the corrected full workpack status:
- A real llama-cpp/GGUF execution proof still has to be recorded.
- Graph-dependent persistence for model observations and learning curves is still a follow-up writer task.
- ONNX/ORT stays optional and cannot be treated as default-build parity until that feature-gated path is explicitly exercised.

Dogfood lesson: do not call a degraded contract proof "feature parity"; use the local-runtime policy and learning-rollup artifacts to separate target shape from measured execution.
