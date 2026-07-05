# x06 Harness Memory Graph

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Harness Memory Graph`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md. THEN read [MEMORY_RETRIEVAL_OWNER_INTENT](../MEMORY_RETRIEVAL_OWNER_INTENT.md) FIRST, before this file's checklist — it outranks every x06 doc (owner-set).
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-memory/** (src/graph.rs, src/index.rs, src/code_graph.rs, src/fulltext.rs, src/vector.rs, src/embed.rs, src/rerank.rs, src/recall.rs, src/weaver/**, src/learning.rs, src/share.rs, src/federation.rs, src/diagnostics.rs, src/mcp.rs, src/cli.rs, Cargo.toml), crates/enforcer-memory/tests/fixtures/memory/**, proof/memory/x06-*.json`
- deps: `x05-lesson-capture-selfheal`, `arc-02-enforcer-domain`, `arc-21-enforcer-mcp`, `arc-22-enforcer-cli`, `arc-23-enforcer-install`
- tier: `P1 unit / P3 live-recall / P4 feature-parity`

Sources: [PLAN_STATE](../PLAN_STATE.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [MEMORY_RETRIEVAL_ARCHITECTURE](../MEMORY_RETRIEVAL_ARCHITECTURE.md), [MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN](../MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN.md), [MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS](../MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS.md), [MEMORY_RETRIEVAL_PARITY_HARNESS](../MEMORY_RETRIEVAL_PARITY_HARNESS.md), [MEMORY_RETRIEVAL_QA_BENCHMARKS](../MEMORY_RETRIEVAL_QA_BENCHMARKS.md), [MEMORY_RETRIEVAL_TEST_MATRIX](../MEMORY_RETRIEVAL_TEST_MATRIX.md), [x05](./x05-lesson-capture-selfheal.md), [c05](./c05-claude-sessionstart-hook.md).

## Where We Are

x05 captures lessons, but the system still needs codebase memory, retrieval quality, continuous indexing, and proof that agent tooling can actually find useful code/lesson/artifact context. The owner requirement is now stronger than the original x06 draft: x06 must be a Rust codebase-memory-class KG engine plus RAG and learning on top.

## Where We Want To Be

A new `crates/enforcer-memory` crate that provides:

```text
codebase-memory-mcp parity baseline
+ enforcer KG over rules/workpacks/proofs/lessons/artifacts
+ RAG retrieval over code, lessons, artifacts, summaries, git history
+ local embedding and reranking
+ background enrichment/weaver workers
+ continuous observations and learning curves
+ live MCP/CLI parity proof
```

KG-only is not enough. Vector-only is not enough. x06 is DONE only when the feature-parity rollup proves live MCP/CLI behavior, 100 QA retrieval rows, token-reduction, local model runtime, reranker lift, and learning evidence curves.

## Requirement Checklist

- [ ] Implement [MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN](../MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN.md) as binding product scope.
- [ ] Implement [MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS](../MEMORY_RETRIEVAL_IMPLEMENTATION_PACKS.md) as internal x06 lane decomposition.
- [ ] Implement [MEMORY_RETRIEVAL_PARITY_HARNESS](../MEMORY_RETRIEVAL_PARITY_HARNESS.md): run installed KG baseline and WIP Rust enforcer-memory over same repos, same git revision, same tools, and emit machine-readable diffs.
- [ ] Implement [MEMORY_RETRIEVAL_QA_BENCHMARKS](../MEMORY_RETRIEVAL_QA_BENCHMARKS.md): 100 practical Q&A retrieval rows, all represented in `proof/memory/x06-rag-qa.json`.
- [ ] Implement [MEMORY_RETRIEVAL_TEST_MATRIX](../MEMORY_RETRIEVAL_TEST_MATRIX.md): every prefix represented in `proof/memory/x06-feature-parity.json`.
- [ ] Code KG: files, crates/packages, modules, symbols, functions, types, tests, routes, imports, calls, events, resources, graph paths, snippets, architecture overview, impact analysis, ADR memory.
- [ ] RAG: code-aware full-text, dense vectors, HNSW sidecars, local embedding, local reranker, rank fusion, source refs, why-selected trace, bounded context packs.
- [ ] Continuous indexing: unchanged files skipped; changed/new/deleted files update; git history and file-history summaries captured; stale summaries/vectors marked and rebuilt.
- [ ] Weaver: background queue for embeddings, summaries, entity/symbol links, associative links, git-history summaries, and retrieval-quality feedback. Foreground query path must not block on background enrichment.
- [ ] Learning: every scan/check/run/doctor/closeout records observation; clean scans count as negative evidence; lessons activate only after x05 landing/proof; recurrence curves prove whether learning improves behavior.
- [ ] **Usage-ingestion seam (owner-set, RESTORED after hardening pass dropped it): every enforcement operation FEEDS the graph automatically** — expose an ingest_observation function contract (finding/fault-class/ruleId/repo-context → Incident node + observedIn edges) that scan/check/run surfaces (arc-15, f01, f05) and coordination closeout (arc-16) CALL on every run; no manual capture step; append-only, redaction-safe; a clean scan still records the clean observation (negative evidence). Usage = learning.
- [ ] **Learning-evidence query (owner-set, RESTORED): memory evidence <lessonId>** returns the t0→t1→t2 chain (observedIn incidents with provenance → landed artifacts with fixtures green → recurrence count since landing), each element carrying enforcer-proof journal refs; aggregate --all emits the per-domain learning curve. Fail-closed: missing t0 provenance reports evidence:incomplete, never fabricates. This is the falsifiable-learning instrument (RUST_ARCHITECTURE learning thesis).
- [ ] Local model runtime: Qwen3 embedding/reranker class models required, code embedding required, small summarizer/model candidate evaluated including Ornith-family or equivalent local models; capability state and model manifest recorded.
- [ ] Policy: filters run before rerank/context/logging; exact artifact/snippet retrieval never returns similar artifacts as exact; imported memory is inactive until local validation.

## Acceptance And Proof

Required commands/artifacts:

```text
cargo test -p enforcer-memory
cargo clippy
cargo fmt --check
enforcer memory index-repository --repo .
enforcer memory parity-harness --baseline codebase-memory-mcp --candidate enforcer-memory
enforcer memory qa-benchmark --queryset MEMORY_RETRIEVAL_QA_BENCHMARKS.md
enforcer memory doctor --strict
proof/memory/x06-feature-parity.json
```

`proof/memory/x06-feature-parity.json` must prove:

```text
kgParityComparedAgainstBaseline = true
allRequiredToolsLiveViaMcp = true
allRequiredToolsLiveViaCli = true
qaRowsGreen = 100
tokenReductionMedianAtLeast10x = true
rerankerLiftPositive = true
retrievalImprovementCurvePresent = true
learningEvidenceCurvePresent = true
unauthorizedCandidateCount = 0
untrustedActiveCount = 0
exactArtifactMismatchCount = 0
```

## Parallel Ownership Notes

x06 owns only `crates/enforcer-memory/**`, its fixtures, and `proof/memory/x06-*.json`. It consumes x05 lessons, emits c05 SessionStart recall payloads through a seam, registers tools through arc-21/arc-22 seams, and resolves model/artifact assets through arc-23. Internal x06 subpacks are file-disjoint under the x06 owns glob.
