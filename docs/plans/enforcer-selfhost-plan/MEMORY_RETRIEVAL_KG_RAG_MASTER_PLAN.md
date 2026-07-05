# MEMORY RETRIEVAL KG + RAG MASTER PLAN

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN`
> Kind: binding master plan for x06 codebase-memory parity plus enforcer KG+RAG learning.
> Read when: AFTER [MEMORY_RETRIEVAL_OWNER_INTENT](./MEMORY_RETRIEVAL_OWNER_INTENT.md) — the owner-intent page is read FIRST and outranks this doc. Then: implementing or reviewing x06; deciding whether the system is only KG parity or KG+RAG+learning; closing proof.
> Stop rule: This doc scopes x06 only. Implementation must still follow the x06 workpack owns globs.
> Proves: nothing by itself. Proof comes from the parity harness, QA benchmark, test matrix, and cargo/MCP/CLI artifacts.
> Does not prove: completion, model quality, or product DONE.
<!-- /agent-capsule -->

Sources: [MEMORY_RETRIEVAL_OWNER_INTENT](./MEMORY_RETRIEVAL_OWNER_INTENT.md) (READ FIRST — binding vision), [x06 Harness Memory Graph](./workpacks/x06-harness-memory-graph.md), [MEMORY_RETRIEVAL_ARCHITECTURE](./MEMORY_RETRIEVAL_ARCHITECTURE.md), [MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS](./MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS.md), [MEMORY_RETRIEVAL_PARITY_HARNESS](./MEMORY_RETRIEVAL_PARITY_HARNESS.md), [MEMORY_RETRIEVAL_QA_BENCHMARKS](./MEMORY_RETRIEVAL_QA_BENCHMARKS.md), [MEMORY_RETRIEVAL_TEST_MATRIX](./MEMORY_RETRIEVAL_TEST_MATRIX.md), [Rag-Guide](https://github.com/sujanmishra-simpro/Rag-Guide), [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp), [TabAgentServer MIA](https://github.com/ocentra/TabAgentServer).

---

## 0. Binding product thesis

`codebase-memory-mcp` is the KG/code-intelligence baseline. `enforcer-memory` is **KG + RAG + learning**.

x06 must implement three layers:

1. **KG parity:** local repository knowledge graph, structural code search, call graph, routes, imports, symbols, snippets, architecture overview, impact analysis, ADR memory, watcher, compressed graph artifact, MCP/CLI tools, diagnostics.
2. **RAG on top:** dense semantic retrieval, code/lesson/artifact embeddings, reranking, query routing, context packing, repo summaries, mind maps, git-history summaries, top-k evidence packs, source refs, token-reduction proof.
3. **Cognitive learning on top:** x05 lessons, continuous observations, procedural memory of what worked, recurrence curves, negative evidence from clean scans, trust gates, consented federation.

A KG-only clone is not x06 DONE. A vector-only search layer is not x06 DONE. A cargo unit green that does not run live MCP/CLI parity is not x06 DONE.

---

## 1. Source model: codebase-memory-mcp parity

The parity floor is a Rust implementation of the same product class:

- one local binary;
- MCP stdio server;
- CLI invocation path;
- project index/cache;
- repository graph;
- file/symbol/function/type/test/route/import/call/event/resource graph nodes;
- graph search;
- code search;
- semantic search;
- snippet retrieval;
- call-path tracing;
- impact analysis from git diff;
- architecture overview;
- ADR memory;
- watcher/incremental indexing;
- compressed graph artifact;
- diagnostics log;
- local-only processing.

The proof harness must run installed `codebase-memory-mcp` and `enforcer memory` on the same fixture repos, same git revision, same query, and same tool class. Equal behavior is the floor. Better behavior must be justified by a machine-readable diff reason.

---

## 2. Source model: MIA cognitive memory

MIA's useful idea for x06 is cognitive memory separation:

| MIA concept | x06 translation |
|---|---|
| Working memory | current repo/task/session recall pack |
| Episodic memory | incidents, operations, closeouts, fixes, clean scans |
| Semantic memory | code graph, rule graph, lesson graph, doctrine graph |
| Embedding memory | dense vectors over code/lessons/artifacts/summaries |
| Tool-result memory | cached MCP query results, baseline comparisons, search traces |
| Procedural/experience memory | what fix/search/retrieval strategy worked or failed |
| Summary memory | repo maps, module summaries, file summaries, git-history summaries |
| Meta-memory | where to look, which route to choose, confidence, fallback state |

Hot/warm/cold access tiers are required:

```text
hot: current worktree, current branch, current workpack, recent observations
warm: active repo graph, active x05 lessons, recent proof/incident history
cold: older git history, archived lessons, imported untrusted memory, old artifacts
```

Every retrieval trace must record which tier was queried and why.

---

## 3. TabAgentServer Rust crate ideas to harvest

| Existing source idea | x06 hard requirement |
|---|---|
| `Rust/indexing` says indexing is metadata/pointers, not source data | x06 indexes are rebuildable; source logs/artifacts own truth |
| `Rust/indexing` multi-resolution embeddings | fast candidate model + stronger rerank/high-precision stage |
| `Rust/knowledge-graph` crate precedent | x06 has explicit KG crate surface, typed nodes/edges, graph algorithms |
| `Rust/weaver` event-driven enrichment | x06 has background enrichment workers for embeddings, links, summaries, evidence curves |
| MIA experience memory | x06 stores retrieval/fix/lesson outcomes as procedural memory |
| MIA meta-memory | x06 records route selection and confidence for every query |

---

## 4. RAG Guide doctrine as hard gate

The pipeline is mandatory:

```text
offline: parse -> normalize -> chunk -> graph -> full-text -> embed -> vector -> eval -> manifest
online: route -> prefilter -> retrieve -> fuse -> rerank -> enrich -> context pack -> trace
```

Every index is derived and rebuildable. Every query has a route. Every result has source refs. Every benchmark has expected ids. Every regression blocks DONE.

Required score floor:

```text
recall_at_5 >= 0.90
mrr_at_10 >= 0.80
ndcg_at_10 >= 0.85
reranker_lift_at_10 >= 0.05 on semantic rows
unauthorized_candidate_count = 0
untrusted_active_count = 0
exact_artifact_mismatch_count = 0
token_reduction_vs_file_reading >= 10x on repo-scale rows
```

---

## 5. Conscious harness

The harness is conscious in the engineering sense: it observes itself, stores outcomes, and refuses fake green.

Each harness run must:

1. run baseline KG tool;
2. run enforcer KG+RAG tool;
3. diff expected ids, routes, source refs, score families, latency, and token cost;
4. classify result as `equal`, `better`, `worse`, or `incomparable`;
5. write the trace into procedural memory;
6. update retrieval and learning curves;
7. fail if any required gate regresses.

---

## 6. Required outputs x06 must generate

For each indexed repo:

- code graph;
- repo mind map;
- crate/package/module map;
- symbol index;
- route/API/event map;
- test connectivity map;
- git-history summary per file/module;
- hot/warm/cold memory manifests;
- repo summary chunks tied back to code nodes;
- retrieval QA benchmark report;
- token-reduction report vs file-by-file exploration.

For each task/session:

- top-k code evidence;
- top-k active lessons;
- exact snippets/artifacts;
- why-selected trace;
- what was excluded and why;
- capability state;
- confidence score;
- suggested next query if recall is low.

---

## 7. Required proof artifacts

```text
proof/memory/x06-kg-parity.json
proof/memory/x06-rag-qa.json
proof/memory/x06-models.json
proof/memory/x06-learning-curve.json
proof/memory/x06-token-reduction.json
proof/memory/x06-feature-parity.json
```

`x06-feature-parity.json` must fail unless:

```text
kgParityComparedAgainstBaseline = true
allRequiredToolsLiveViaMcp = true
allRequiredToolsLiveViaCli = true
qaRowsGreen = 100
retrievalImprovementCurvePresent = true
tokenReductionMedianAtLeast10x = true
rerankerLiftPositive = true
unauthorizedCandidateCount = 0
untrustedActiveCount = 0
exactArtifactMismatchCount = 0
learningEvidenceCurvePresent = true
```
