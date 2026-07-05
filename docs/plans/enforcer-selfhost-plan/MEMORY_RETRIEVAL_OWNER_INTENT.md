# MEMORY_RETRIEVAL_OWNER_INTENT — Why x06 Exists

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_OWNER_INTENT`
> Kind: OWNER INTENT — the binding vision statement for the entire x06 memory system.
> Read when: FIRST, before ANY x06 workpack, subpack, architecture doc, or implementation decision. This page outranks every other x06 document. If an implementation satisfies a checklist but moves away from this vision, revise the implementation, not the vision.
> Stop rule: This doc is judged-against, not executed-from. After reading, go to [MEMORY_RETRIEVAL_DESIGN_INPUTS](./MEMORY_RETRIEVAL_DESIGN_INPUTS.md) (the 10 sources), then MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN, then your one subpack.
> Proves: nothing mechanical. It is the criterion the mechanical proofs exist to serve.
> Protection: every line tagged `(owner-set)` is a protected invariant under OWNERSET-1.1 — external rewrites that drop one fail T1.
<!-- /agent-capsule -->

_Author: owner (sujan.mishra), 2026-07-04. Captured verbatim from the live build session; structural headers added, no intent altered._

## What this is — and is not (owner-set)

This is **not** a RAG project.
This is **not** a vector database project.
This is **not** a codebase-memory clone.

This is an attempt to build a **cognitive engineering memory system** for Enforcer that continuously improves as it is used, while remaining **deterministic, provable, local-first, and mechanically testable** (owner-set).

The final system should eventually become one of the flagship demonstrations of modern production RAG engineering.

## Why codebase-memory-mcp

`codebase-memory-mcp` is currently one of the better open-source examples of knowledge-graph-driven code intelligence. It already proves:

- code graph
- symbol graph
- structural traversal
- architecture queries
- impact analysis
- semantic code search
- MCP integration

Those are all excellent capabilities. **We are NOT replacing them. They become our minimum acceptable capability floor** (owner-set).

The expectation:

```text
If codebase-memory can answer it,
Enforcer must answer it.

If Enforcer answers it,
it should answer it with equal or better quality.

Otherwise the implementation is incomplete.
```

**That parity is not optional. It is mechanically proven** (owner-set) — see MEMORY_RETRIEVAL_PARITY_HARNESS.

## Why not stop there?

Because codebase-memory is fundamentally a Knowledge Graph system. Our vision is much larger:

```text
Knowledge Graph
        +
Modern Production RAG
        +
Continuous Learning
        +
Experience Memory
        +
Procedural Memory
        +
Meta Memory
        +
Proof System
        +
Continuous Improvement
```

The KG is only one subsystem.

## Why RAG?

Because KG alone cannot solve: fuzzy reasoning, semantic similarity, natural-language retrieval, previous debugging sessions, "this feels similar", procedural memory, experience recall, contextual recall.

**KG gives deterministic structure. RAG gives semantic intelligence. They complement each other** (owner-set).

## Why continuous learning?

Current AI forgets. Every debugging session starts almost from zero. **Enforcer should become more useful every day it exists** (owner-set).

Every: scan, fix, review, failure, success, lesson, PR, workpack, closeout — **should improve future retrieval. Not by retraining models. By improving memory** (owner-set).

## Why x05 exists

x05 captures lessons. Without x06 they become storage. **Storage is not memory** (owner-set). Memory means:

```text
capture → organize → connect → retrieve → apply → measure → improve → remember forever
```

## Why MIA matters

MIA introduced something important: human memory is not one database — it is many specialized memories. That philosophy survives. **Instead of one giant vector DB we want specialized memories** (owner-set):

- working memory
- episodic memory
- semantic memory
- procedural memory
- experience memory
- summary memory
- meta memory

Each optimized for a different retrieval problem.

## Why TabAgent matters

TabAgent already explored: indexing, knowledge graph, weaver, background enrichment, summarization, model loading, multi-resolution embeddings. Those ideas were incomplete. **Now they become hard production requirements. Nothing should be discarded if it still fits** (owner-set).

## Why the Rag Guide matters

The Rag Guide is our **engineering doctrine**, not a tutorial. It defines: offline pipeline, online pipeline, evaluation, retrieval, reranking, chunking, embedding, ranking, context assembly, production requirements.

**Every architectural decision in x06 should align with that document unless a deliberate reason exists not to** (owner-set).

## Why local models?

Because Enforcer cannot depend on cloud APIs. **Everything critical must work: offline, air-gapped, enterprise, government, private repositories, small laptop. Cloud becomes optional. Local is the default** (owner-set).

## Model philosophy (owner-set)

Fast models exist for searching. Better models exist for precision. Therefore retrieval is layered:

```text
Query
  ↓ fast embedding model
Top 100
  ↓ graph filter
  ↓ permission filter
  ↓ large reranker
Top 10
  ↓ context builder
LLM
```

**Never run expensive models on the entire corpus** (owner-set).

## Continuous indexing philosophy (owner-set)

**Indexes are disposable. Knowledge is not** (owner-set). Every file tracks:

```text
content hash | git hash | commit | timestamp | summary version |
embedding version | chunk version | parser version | graph version
```

If nothing changed: do nothing. If only comments changed: maybe only summary changes. If structure changed: update graph. If semantics changed: update embeddings. **Everything incremental. Never rebuild blindly** (owner-set).

## Weaver philosophy (owner-set)

**The foreground answers queries. The background thinks** (owner-set). The Weaver continuously performs: embeddings, summaries, git-history summaries, module summaries, repo maps, entity linking, associative links, lesson extraction, quality evaluation, benchmark reruns.

**Users should never wait for enrichment** (owner-set).

## Continuous evaluation (owner-set)

The system constantly asks: Did retrieval improve? Did the reranker improve? Did summaries improve? Did token usage decrease? Did users accept results? Did Claude need fewer follow-up MCP calls? Did Codex stop opening unnecessary files? Did recall improve?

**Every answer becomes another learning signal** (owner-set).

## Benchmark philosophy (owner-set)

We do not measure "did the code compile?" — we measure **"did the system think correctly?"** (owner-set). Examples:

- Find every test affected by changing this function.
- Which crates indirectly depend on this type?
- Where is this event eventually consumed?
- What previous bug is most similar?
- Which lesson prevented the most regressions?
- Which retrieval path should have been used?

Over 100 real engineering questions (MEMORY_RETRIEVAL_QA_BENCHMARKS). **Those become permanent regression tests** (owner-set).

## Fake Green Definition (owner-set)

A project is **fake green** if: `cargo test` passes, the graph builds, embeddings exist — **but** retrieval is poor, parity fails, MCP tools cannot answer correctly, benchmark questions fail, learning does not improve, token usage stays high.

**Fake green is failure** (owner-set).

## Final Vision (owner-set)

Eventually Enforcer should behave less like:

```text
grep + vector search
```

and more like:

```text
A senior engineer
who has worked on this repository for years,
remembers every lesson,
knows every architecture decision,
can explain why something exists,
can find anything,
and continuously gets better.
```

**That is the objective. Everything in x06 is judged against that vision. If a proposed implementation satisfies the checklist but moves away from that objective, the implementation is revised rather than accepted** (owner-set).
