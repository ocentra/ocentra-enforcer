# MEMORY RETRIEVAL ARCHITECTURE — x06 KG + RAG + learning

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_ARCHITECTURE`
> Kind: binding architecture note for x06 memory, codebase KG parity, RAG retrieval, local model runtime, and learning proof.
> Read when: Building or reviewing x06, implementing MCP/CLI memory tools, selecting index/model backends, or closing retrieval proof.
> Stop rule: This doc defines architecture only. Implementation still follows the x06 workpack owns globs.
> Proves: nothing by itself. Proof comes from the x06 workpack, parity harness, QA benchmarks, and test matrix.
> Does not prove: model quality, workpack completion, federation safety, or product DONE.
<!-- /agent-capsule -->

Sources: [x06 Harness Memory Graph](./workpacks/x06-harness-memory-graph.md), [MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN](./MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN.md), [MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS](./MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS.md), [MEMORY_RETRIEVAL_PARITY_HARNESS](./MEMORY_RETRIEVAL_PARITY_HARNESS.md), [MEMORY_RETRIEVAL_QA_BENCHMARKS](./MEMORY_RETRIEVAL_QA_BENCHMARKS.md), [MEMORY_RETRIEVAL_TEST_MATRIX](./MEMORY_RETRIEVAL_TEST_MATRIX.md), [Rag-Guide](https://github.com/sujanmishra-simpro/Rag-Guide), [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp), [TabAgentServer MIA](https://github.com/ocentra/TabAgentServer).

---

## 0. Binding decision

`codebase-memory-mcp` is the KG/code-intelligence baseline. `enforcer-memory` must be **KG + RAG + learning**.

Required layers:

```text
1. KG parity: repo graph, symbols, imports, calls, routes, snippets, impact, architecture, ADRs, watcher, MCP/CLI.
2. RAG retrieval: full-text + dense vectors + rerank + context packs + source refs + token-reduction proof.
3. Learning memory: x05 lessons + observations + proof refs + recurrence curves + procedural/meta-memory.
```

KG-only is not DONE. Vector-only is not DONE. Cargo-only unit green is not DONE. x06 closes only when live MCP/CLI behavior proves parity with the baseline and the RAG/learning layer proves measurable improvement.

---

## 1. Architecture

```text
repo snapshot + git history + x05 lessons + observations + proof artifacts
  -> append-only source logs
  -> code KG + lesson KG + proof KG + artifact KG
  -> exact artifact/snippet retrieval
  -> code-aware full-text index
  -> dense vector indexes over code, lessons, artifacts, summaries
  -> local reranker
  -> query-route meta-memory
  -> agent context packs
  -> retrieval traces and learning curves
```

Indexes are derived metadata. Source logs, proof refs, artifacts, and x05 lesson records are authoritative.

Required store layout:

```text
.enforce/memory/
  logs/{observations.ndjson,graph-events.ndjson,retrieval-trace.ndjson,diagnostics.ndjson}
  db/{memory.sqlite,memory.duckdb}
  indexes/{fulltext/,vectors/{code.hnsw,lessons.hnsw,artifacts.hnsw,manifest.json}}
  artifacts/{manifest.ndjson,blobs/sha256/<prefix>/<sha256>}
  models/{manifest.json,qwen3-embedding-0.6b/,qwen3-reranker-0.6b/}
  bundles/{personal/,team/,community/,team-graph.zst}
```

---

## 2. Required graph shape

Node families: Project, Repo, Commit, Branch, File, Module, Package, Symbol, Function, Type, Trait, Test, Route, ApiCall, EventChannel, Rule, Workpack, Lesson, Incident, Observation, Artifact, ProofRef, ModelArtifact, EvalQuery, ADR, ShareBundle.

Edge families: Contains, Defines, Imports, Calls, Implements, Inherits, Tests, RoutesTo, HttpCalls, Emits, ListensOn, SimilarTo, SemanticallyRelated, DerivedFrom, ShipsVia, Supersedes, ObservedIn, BackedByArtifact, AppliesToRule, AppliesToWorkpack, ActivatedByX05, ImportedFrom, RedactedInto, Affects, DecidedBy.

---

## 3. Required tool surface

Every MCP tool has a CLI mirror under `enforcer memory ...`: index repository, list projects, search graph, query graph, search code, semantic query, snippet get, trace path, detect changes, impact, architecture, ADR, search, recall, related, evidence, artifact get, export, import, doctor, parity harness, QA benchmark.

---

## 4. Required local model runtime

Required models: code embedding model for source-code semantic search; Qwen3-Embedding-0.6B class model for lessons/artifacts/general text; Qwen3-Reranker-0.6B class reranker; local summarization/model candidate evaluated for repo summaries, mind maps, and git-history summaries, including Ornith-family or equivalent small local models.

Required runtime: local artifact manifest, digest verification, CPU/GPU/NPU resource class, local backend state, model load state, dense vector dimension, latency, recall score, reranker lift, and no silent degradation. Degraded mode can serve a user but cannot close feature parity.

---

## 5. Required proof route

The binding implementation breakdown is [MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS](./MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS.md). Live baseline comparison is [MEMORY_RETRIEVAL_PARITY_HARNESS](./MEMORY_RETRIEVAL_PARITY_HARNESS.md). Practical retrieval quality is [MEMORY_RETRIEVAL_QA_BENCHMARKS](./MEMORY_RETRIEVAL_QA_BENCHMARKS.md). Proof rollup is [MEMORY_RETRIEVAL_TEST_MATRIX](./MEMORY_RETRIEVAL_TEST_MATRIX.md).

x06 DONE requires:

```text
cargo test -p enforcer-memory
cargo clippy
cargo fmt --check
live MCP memory smoke
CLI mirror smoke
kg parity harness green
100 QA rows green
retrieval improvement curve present
feature-parity rollup green
```
