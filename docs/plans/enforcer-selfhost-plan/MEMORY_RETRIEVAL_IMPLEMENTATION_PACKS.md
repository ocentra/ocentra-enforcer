# MEMORY RETRIEVAL IMPLEMENTATION PACKS — x06 internal decomposition

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS`
> Kind: internal x06 subpack plan.
> Read when: Implementing x06 or assigning internal lanes inside `crates/enforcer-memory/**`.
> Stop rule: This decomposes only x06. Do not open or edit sibling workpacks because of this file.
> Proves: nothing by itself. Each subpack proves through named tests and proof artifacts.
> Does not prove: x06 DONE unless the feature-parity rollup is green.
<!-- /agent-capsule -->

Sources: [MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN](./MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN.md), [MEMORY_RETRIEVAL_PARITY_HARNESS](./MEMORY_RETRIEVAL_PARITY_HARNESS.md), [MEMORY_RETRIEVAL_QA_BENCHMARKS](./MEMORY_RETRIEVAL_QA_BENCHMARKS.md), [TabAgentServer indexing/weaver](https://github.com/ocentra/TabAgentServer), [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp).

---

## 0. Internal ownership rule

x06 remains one external workpack, but it must be implemented as internal subpacks. A subpack is not DONE with stubs, unused traits, or compile-only tests. Each subpack must include runtime behavior, failing fixtures, positive fixtures, and proof output.

---

## 1. Subpack table

| Subpack | Name | Owns inside `crates/enforcer-memory/**` | Proof artifact |
|---|---|---|---|
| X06.1 | core/store/logs | `src/lib.rs`, `src/error.rs`, `src/ids.rs`, `src/store/**`, `src/log.rs`, `src/schema.rs` | `proof/memory/x06-store.json` |
| X06.2 | code KG indexer | `src/code_graph.rs`, `src/parsers/**`, `src/languages/**`, `src/git.rs` | `proof/memory/x06-code-graph.json` |
| X06.3 | graph algorithms | `src/graph.rs`, `src/analysis/**`, `src/architecture.rs`, `src/impact.rs`, `src/adr.rs` | `proof/memory/x06-kg.json` |
| X06.4 | full-text/vector/rerank | `src/search/**`, `src/fulltext.rs`, `src/vector.rs`, `src/embed.rs`, `src/rerank.rs`, `src/ranking.rs` | `proof/memory/x06-rag.json` |
| X06.5 | background weaver | `src/weaver/**`, `src/enrichment.rs`, `src/queue.rs`, `src/summaries.rs` | `proof/memory/x06-weaver.json` |
| X06.6 | continuous learning | `src/learning.rs`, `src/evidence.rs`, `src/observations.rs`, `src/sessionstart.rs` | `proof/memory/x06-learning.json` |
| X06.7 | MCP/CLI/watch/diagnostics | `src/mcp.rs`, `src/cli.rs`, `src/watch.rs`, `src/diagnostics.rs` | `proof/memory/x06-mcp-cli.json` |
| X06.8 | sharing/federation/artifacts | `src/artifacts.rs`, `src/share.rs`, `src/federation.rs`, `src/redaction.rs` | `proof/memory/x06-federation.json` |
| X06.9 | parity/benchmark harness | `tests/feature_parity/**`, `tests/fixtures/memory/**`, `benches/**` | `proof/memory/x06-feature-parity.json` |

---

## 2. X06.1 — core/store/logs

Hard requirements:

- append-only observation and graph-event logs with SHA-256 chain;
- SQLite operational graph/read model;
- DuckDB analytics read model;
- content-addressed artifact manifest;
- index manifests with source high-watermark;
- corruption detection and quarantine;
- no ghost project database creation;
- Windows-safe path normalization.

Hard tests: append, supersede, corrupt row, stale index, rebuild determinism, unknown project, path normalization.

---

## 3. X06.2 — code KG indexer

Hard requirements:

- index current repo head with git metadata;
- skip unchanged files using content hash and previous manifest;
- reindex changed/new/deleted files only;
- retain file history summary: last commit, change count, last authorship metadata where allowed, previous chunk ids;
- parse Rust, TypeScript/JavaScript, Python, config/IaC, and text fallback;
- create nodes for files, modules, packages, functions, types, tests, routes, imports, calls, events, resources;
- unsupported files become `TextOnly` nodes, never silent skip.

Hard tests: unchanged file skip, changed file reindex, deleted file tombstone, symbol extraction, route extraction, import/call edges, fallback node.

---

## 4. X06.3 — graph algorithms

Hard requirements:

- related graph walk;
- call path tracing;
- reverse dependency traversal;
- impact analysis from git diff;
- architecture overview;
- repo mind map;
- centrality/hotspot detection;
- ADR memory linked to graph nodes;
- safe query DSL, no raw database execution through MCP.

Hard tests: connected tests, upstream callers, crate map, graph depth limit, diff impact, architecture sections, ADR roundtrip, unsafe query rejection.

---

## 5. X06.4 — full-text/vector/rerank

Hard requirements:

- code-aware full-text tokenization for camelCase, snake_case, kebab-case, paths, symbols;
- HNSW vector index for code chunks;
- HNSW vector index for lessons/artifacts/summaries;
- Qwen3 embedding and reranker class support;
- local code embedding model support;
- optional candidate local summarizer/model benchmark, including Ornith-family candidates or equivalent small local models;
- rank fusion across graph, full-text, vector, recency, trust, proof, history;
- reranker lift measurement;
- degraded mode labeled but not accepted for x06 feature parity.

Hard tests: exact query, semantic query, reranker lift, vector stale, model manifest, no remote provider, token-reduction estimate.

---

## 6. X06.5 — background weaver

Hard requirements based on TabAgentServer Weaver ideas:

- event-driven enrichment queue;
- semantic indexer worker;
- entity/symbol linker worker;
- associative linker worker;
- summarizer worker;
- dead-letter queue;
- retry with bounded backoff;
- worker resource limits;
- hot/warm/cold queue priority.

Hard tests: node created triggers embedding; file changed triggers summary invalidation; failed task enters dead-letter; retry succeeds; queue does not block foreground query.

---

## 7. X06.6 — continuous learning

Hard requirements:

- every scan/check/run/doctor/closeout writes observation;
- clean scans write negative evidence;
- x05 landed/proof-linked lessons are active;
- unlanded/imported lessons are searchable but inactive;
- recurrence curves update after landing;
- procedural memory records retrieval/fix success and failure;
- meta-memory records route choice and confidence.

Hard tests: observation exists per operation, clean evidence, recurrence, lesson activation, supersede, route choice trace, improvement curve.

---

## 8. X06.7 — MCP/CLI/watch/diagnostics

Hard requirements:

- every MCP tool has CLI mirror;
- live MCP JSON-RPC tests;
- live CLI tests;
- watcher incremental update;
- diagnostics NDJSON;
- no raw prompt/private source text in diagnostics;
- local resource metrics captured.

Hard tests: tool list, schema parity, CLI parity, watcher reindex, diagnostics redaction, stale manifest recovery.

---

## 9. X06.8 — sharing/federation/artifacts

Hard requirements:

- exact artifact/snippet retrieval;
- signed personal/team/community bundles;
- default personal only;
- explicit consent for export;
- zero-trust import;
- community redaction golden;
- team graph compressed bootstrap artifact.

Hard tests: exact retrieval, traversal rejection, signature rejection, inactive import, x05 validation activation, redaction golden, graph artifact import.

---

## 10. X06.9 — parity/benchmark harness

Hard requirements:

- run baseline KG and enforcer KG+RAG on same repo/query set;
- compare every required tool;
- execute 100 QA rows;
- generate improvement curves;
- prove token reduction;
- produce feature parity summary.

Hard tests: baseline missing fails, candidate missing fails, row coverage incomplete fails, no improvement curve fails, proof rollup incomplete fails.
