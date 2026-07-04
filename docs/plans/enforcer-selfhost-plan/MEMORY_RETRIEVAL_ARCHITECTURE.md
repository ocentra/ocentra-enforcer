# MEMORY RETRIEVAL ARCHITECTURE — x06 local-first graph + RAG

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_ARCHITECTURE`
> Kind: architecture decision note for `x06 Harness Memory Graph` and adjacent retrieval seams.
> Read when: Building or reviewing `x06`, wiring local model runtime, choosing storage/index backends, or deciding whether a memory query should hit queryable data, artifact retrieval, graph traversal, vector retrieval, or reranking.
> Stop rule: This doc is design doctrine and implementation guidance. It does not authorize touching code outside the claimed workpack owns globs. For execution, claim exactly one workpack through `WORKPACK_INDEX.md`.
> Proves: nothing by itself. Proof comes from the named `x06` rows in `TEST_PROOF_EXPECTATIONS.md` and workpack-specific tests.
> Does not prove: model quality, workpack completion, federation safety, or product DONE.
<!-- /agent-capsule -->

Sources: [x06 Harness Memory Graph](./workpacks/x06-harness-memory-graph.md), [x05 Lesson Capture Self-Heal](./workpacks/x05-lesson-capture-selfheal.md), [RUST_ARCHITECTURE](./RUST_ARCHITECTURE.md), [TEST_PROOF_EXPECTATIONS](./TEST_PROOF_EXPECTATIONS.md), [Rag-Guide](https://github.com/sujanmishra-simpro/Rag-Guide), [OcentraParent crates](https://github.com/ocentra/OcentraParent/tree/codex/tracking-plan-full-continuation-a/crates), [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp).

---

## 0. Decision summary

`x06` is not "add a vector database". It is the enforcer's local-first memory substrate:

```text
append-only observations + x05 lessons + proof artifacts
  -> typed graph/read model
  -> artifact retrieval
  -> BM25/sparse index
  -> optional dense vector index
  -> optional local reranker
  -> MCP recall tools + SessionStart injection
  -> evidence curve proving learning over time
```

The vector index is derived. It is not authoritative state. The authoritative record is the append-only, tamper-evident lesson/incident/artifact corpus plus the typed graph edges that explain why a memory item exists and whether it is allowed to influence future work.

Default design:

- embedded local store; no daemon required;
- append-only NDJSON source logs with hash-chain verification;
- materialized graph/read-model tables rebuilt from logs;
- separate artifact retrieval by content-addressed refs;
- BM25/full-text fallback always available;
- dense vectors and reranker behind traits;
- local-only model runtime; model-absent means labeled degradation, never silent fallback;
- explicit-consent sharing with signed bundles and local re-validation before activation.

---

## 1. RAG doctrine for enforcer memory

### 1.1 Do not use RAG to avoid structure

Use queryable data when the answer is a field, status, count, rule id, workpack id, artifact id, timestamp, provenance chain, or aggregate. Use retrieval when the answer depends on ambiguous language, similar incidents, rationale, or task context.

| Question | Primary path |
|---|---|
| `what lessons landed for rule X?` | graph/table query |
| `show artifact for lesson L` | deterministic artifact retrieval |
| `which incidents happened after lesson L?` | graph + time query |
| `what have we learned about stale-base resets?` | hybrid BM25/vector recall + graph enrichment |
| `what should SessionStart inject for this repo/task?` | hybrid recall + strict context budget |
| `is a community lesson active?` | trust/provenance query |

The enforcer is a gate. A gate must not make enforcement decisions from approximate retrieval alone.

### 1.2 Graph first, vector second

A useful memory item carries observed evidence, lesson text, route/landing surface, rule/workpack/harness relation, trust tier, artifact refs, and proof refs. Vectors help find candidates; graph/proof state decides whether a candidate is trusted, active, and injectable.

### 1.3 Permission before retrieval

Filter by consent tier, trust state, repo/project boundary, redaction policy, and local config before candidates enter reranking, context assembly, logs, or MCP responses.

### 1.4 Agent translates; enforcer serves tools

Borrow the `codebase-memory-mcp` posture: the MCP server builds/queryies local indexes and returns structured results. The coding agent translates user intent to tool calls. The enforcer does not need an embedded LLM to be useful.

---

## 2. Borrowed patterns

### 2.1 From Rag-Guide

- Separate offline and online paths.
- Keep indexes derived, not authoritative.
- Use hybrid retrieval before claiming production quality.
- Evaluate with fixture queries, expected ids, and regression gates.
- Log route, filters, candidates, scores, model/index versions, fallback state, and selected context ids.

### 2.2 From OcentraParent

Reuse concepts, not a product dependency unless the owner splits a shared crate:

- local model capability flags: embedding, reranking, BM25, graph query, artifact retrieval;
- resource classes: CPU, GPU, NPU, remote-unavailable;
- model load/degraded/cache/manifest integrity states;
- one local model lane per physical device by default;
- activity-memory read-model style: nodes, edges, trace, custody, degraded reasons, query metadata;
- model outputs are advisory until validated evidence and proof consume them.

### 2.3 From codebase-memory-mcp

Borrow these architecture moves:

- single local binary and MCP tool layer;
- persistent knowledge graph instead of repeated file reading;
- graph + full-text + semantic search, not vector-only;
- compressed shareable graph/memory artifact for teams;
- auto-index/watch only after explicit index correctness exists;
- diagnostics NDJSON that records local resource trajectory without source or prompt text.

Do not copy its problem boundary. It is code structural intelligence. x06 is harness/orchestration/code-learning memory. Optional interop can come later.

---

## 3. Storage architecture

### 3.1 Default layout

```text
.enforce/
  lessons.ndjson
  memory/
    config.json
    logs/
      graph-events.ndjson
      retrieval-trace.ndjson
      diagnostics.ndjson
    db/
      memory.sqlite
      memory.duckdb              # optional analytics materialization
    indexes/
      bm25.sqlite
      vectors/
        lessons.hnsw
        manifest.json
    artifacts/
      manifest.ndjson
      blobs/sha256/<prefix>/<sha256>
    models/
      manifest.json
      qwen3-embedding-0.6b/...
      qwen3-reranker-0.6b/...
    bundles/
      team/*.enforcer-memory-bundle.zst
      community/*.enforcer-memory-bundle.zst
```

### 3.2 Source of truth

| Layer | Role | Authoritative? | Rebuild rule |
|---|---|---:|---|
| x05 `lessons.ndjson` | structured lessons and landed refs | yes | append-only |
| `graph-events.ndjson` | incident/node/edge event log | yes | append-only + verify-on-open |
| artifact manifest | content-addressed artifact refs | yes | append-only |
| SQLite/DuckDB tables | materialized query/read model | no | rebuild from logs |
| BM25 index | sparse search | no | rebuild from text chunks |
| vector index | dense search | no | rebuild from text + model manifest |

If a derived index cannot be deleted and rebuilt, it has become an accidental source of truth and fails the design.

### 3.3 Backend defaults

| Need | Default | Optional later |
|---|---|---|
| append log | NDJSON + hash chain | none |
| graph/metadata | SQLite tables | DuckDB materialized analytics |
| BM25/full-text | SQLite FTS5 or Tantivy | both behind trait |
| vector search | embedded HNSW sidecar behind trait | sqlite-vec, LanceDB, Qdrant, pgvector adapters |
| evidence curves | SQLite aggregate first | DuckDB feature for larger corpora |
| artifact payloads | content-addressed files + manifest | enterprise object store |

Do not require Qdrant, pgvector, LanceDB, or server-mode DuckDB in the default path. The enforcer release thesis is a local per-platform binary.

---

## 4. Data model

Minimum node kinds:

```text
Lesson | Rule | Workpack | HarnessCapability | Incident | Artifact | RepoContext | ModelArtifact | EvalQuery
```

Minimum edge kinds:

```text
DerivedFrom | ShipsVia | Supersedes | ObservedIn | BackedByArtifact | AppliesToRule | AppliesToWorkpack | ActivatedByX05 | ImportedFrom | RedactedInto
```

Activation is a state transition backed by x05 landed artifacts and proof refs. It is not a vector score.

Artifact retrieval is separate from semantic retrieval. `memory artifact get <artifactId>` must retrieve an exact artifact by id/hash, or a redacted exact view. It must never return a merely similar artifact.

---

## 5. Indexing pipeline

### 5.1 Offline path

```text
x05 lessons + observations + proof journals + rule/workpack metadata
  -> parse/normalize
  -> redact at write boundary
  -> append graph events
  -> materialize graph tables
  -> build artifact manifest
  -> chunk lesson/doctrine/artifact text
  -> build BM25 index
  -> build dense vector index when model is present
  -> run eval fixture queries
  -> write index manifest
```

Index manifest must include source log hash/high-watermark, chunker version, sparse analyzer version, embedding model id/digest, vector dimension, enforcer version, and capability state.

### 5.2 Online path

```text
query/repo/task context
  -> route: exact | graph | artifact | hybrid | evidence | share/doctor
  -> pre-filter: consent, trust, repo, rule/workpack, time, redaction, capability
  -> retrieve: graph + BM25 + vector
  -> rank fusion
  -> optional local reranker
  -> graph/proof enrichment
  -> context assembly with provenance
  -> MCP result + retrieval trace
```

Online retrieval must never download a model implicitly. Model download is an explicit install/configure action through the artifact resolver.

---

## 6. Local model runtime

Default model class:

| Task | Default id | Use |
|---|---|---|
| embedding | `Qwen/Qwen3-Embedding-0.6B` | lesson/doctrine/artifact semantic vectors |
| reranking | `Qwen/Qwen3-Reranker-0.6B` | rerank top 20-80 candidates only |
| fallback | BM25/keyword | required when model is absent |

Required traits:

```rust
trait Embedder {
    fn capability(&self) -> CapabilityState;
    fn model_manifest(&self) -> ModelManifest;
    fn embed_documents(&self, texts: &[MemoryText]) -> Result<Vec<EmbeddingVector>>;
    fn embed_query(&self, query: &MemoryQuery) -> Result<EmbeddingVector>;
}

trait Reranker {
    fn capability(&self) -> CapabilityState;
    fn rerank(&self, query: &MemoryQuery, candidates: &[RecallCandidate]) -> Result<Vec<RerankScore>>;
}
```

Backends:

| Backend | Role | Default? |
|---|---|---:|
| BM25-only | guaranteed fallback | yes |
| Rag-Guide localhost model service | dev smoke adapter using `/embed`, `/rerank`, `/health` | dev yes |
| ONNX Runtime | portable production local runtime | target |
| OpenVINO | Intel CPU/iGPU/NPU acceleration | optional feature |
| Candle | pure Rust experiment | optional |
| external model service | enterprise override only | no |

Every response must label capability state: `Present`, `Degraded`, `Unavailable`, or `Invalid`. Silent fallback is a test failure.

---

## 7. MCP and CLI surface

Required tools:

| Tool | Purpose |
|---|---|
| `memory.search` | search lesson/doctrine/artifact text |
| `memory.recall` | top-k lessons for repo/task/session context |
| `memory.related` | graph walk from a node id |
| `memory.evidence` | t0 -> t1 -> t2 learning chain |
| `memory.artifact.get` | deterministic artifact retrieval |
| `memory.ingest` | ingest x05 lessons and observations |
| `memory.rebuild` | rebuild graph/BM25/vector indexes |
| `memory.export` | create signed team/community bundle |
| `memory.import` | zero-trust import |
| `memory.doctor` | integrity/capability/eval check |

CLI mirrors MCP:

```text
enforcer memory search "stale base reset"
enforcer memory recall --repo . --task "arc-16 MCP registration failure"
enforcer memory related lesson:L0007
enforcer memory evidence lesson:L0007
enforcer memory artifact get artifact:...
enforcer memory ingest --from-lessons .enforce/lessons.ndjson
enforcer memory rebuild --all
enforcer memory doctor
enforcer memory export --tier team --out team.enforcer-memory-bundle.zst
enforcer memory import team.enforcer-memory-bundle.zst --trust untrusted
```

SessionStart injection must be small, labeled, and non-authoritative: default top-k 3-8 lessons, with trust state, why-selected, artifact/proof refs, and capability state.

---

## 8. Federation and sharing

| Tier | Default | Leaves machine? | Activation |
|---|---:|---:|---|
| personal | yes | no | active after local x05 landing |
| team | no | only explicit signed export/import | inactive until local x05 validation |
| community | no | only explicit anonymized/redacted export/import | inactive until local x05 validation |

Bundle shape:

```text
bundle.manifest.json
lessons.ndjson
observations.ndjson
artifacts/manifest.ndjson
artifacts/redacted-blobs/...
signatures/...
checksums.txt
```

Imported lessons may be stored and surfaced as untrusted, but cannot alter managed blocks, rules, forest nodes, or active SessionStart recall until they pass local x05 route/doctor validation.

---

## 9. Evaluation and proof

Fixture metrics:

```text
hit@1
hit@5
recall@5
MRR@10
nDCG@10
reranker_lift@10
unauthorized_candidate_count = 0
untrusted_active_count = 0
degradation_labeled = true
```

Required failure cases:

- source log changes but index is not marked stale;
- model missing but result claims dense search ran;
- unauthorized or untrusted candidate enters context assembly;
- artifact lookup returns similar artifact instead of exact artifact;
- signed bundle check accepts altered content;
- imported valid-but-unvalidated lesson becomes active;
- seeded eval query loses expected lesson id without failing;
- evidence query fabricates t0/t1/t2 instead of reporting incomplete provenance.

Retrieval trace must record route, filters, candidate counts, selected ids, score families, capability state, index manifest id, and latency. It must not store raw private source text or prompt text.

---

## 10. Implementation sequence for x06

1. Define DTOs with branded ids: nodes, edges, artifacts, model manifests, capability state, retrieval trace, eval queries.
2. Implement append-only graph event log with verify-on-open and supersede append.
3. Build SQLite graph/read-model tables and exact graph traversal.
4. Implement artifact manifest + exact artifact retrieval.
5. Add BM25/full-text fallback.
6. Add `VectorIndex`, `Embedder`, and `Reranker` traits.
7. Implement Rag-Guide-compatible localhost model adapter for dev smoke.
8. Add ONNX/OpenVINO production local model backend behind features.
9. Implement hybrid route -> filter -> graph/BM25/vector -> rank fusion -> optional rerank -> context assembly.
10. Expose MCP/CLI tools through arc-21/arc-22 seams.
11. Emit c05 SessionStart recall payload through seam, not shared files.
12. Add signed bundle export/import with zero-trust activation.
13. Implement `memory evidence` and aggregate learning curve.
14. Add `memory doctor`, eval metrics, stale-index checks, and retrieval diagnostics.

`x06` is DONE only when the proof rows are green and the system can remember, retrieve, explain, and safely share lessons without turning approximate retrieval into enforcement authority.
