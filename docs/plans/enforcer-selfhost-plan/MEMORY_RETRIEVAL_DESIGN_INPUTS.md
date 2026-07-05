# MEMORY_RETRIEVAL_DESIGN_INPUTS — Prior Art, Sources & What Each Contributes

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_DESIGN_INPUTS`
> Kind: OWNER-DICTATED design-input registry for x06. Names every source the system borrows from and exactly what is borrowed.
> Read when: SECOND, right after MEMORY_RETRIEVAL_OWNER_INTENT, before any x06 subpack. When an implementation choice arises, check whether a listed source already solved it — reuse before reinvent (owner-set).
> Stop rule: This doc scopes borrowing only. Implementation follows the x06 workpack owns globs + subpacks.
> Proves: nothing. The parity harness and benchmarks prove.
> Protection: `(owner-set)` lines are OWNERSET-1.1-protected invariants.
<!-- /agent-capsule -->

_Author: owner (sujan.mishra), 2026-07-04, captured verbatim from the live build session; structure added, intent unaltered._

## Primary objective (owner-set)

Build Enforcer Memory (x06) as:

```text
Knowledge Graph
        +
Modern Production RAG
        +
Continuous Learning
        +
Cognitive Memory
        +
Production Proof Harness
```

**The goal is not feature parity. The goal is feature parity + measurable superiority** (owner-set).

## Source 1 — codebase-memory-mcp (the baseline)

Repo: <https://github.com/DeusData/codebase-memory-mcp>
Purpose: **baseline — must reach feature parity** (owner-set).
Borrow: repository indexing, code graph, graph traversal, semantic search, architecture queries, impact analysis, MCP tools, compressed graph, incremental indexing.
**Hard requirement: every tool must exist, every tool must be benchmarked, every tool must have parity proof** (owner-set) — see MEMORY_RETRIEVAL_PARITY_HARNESS.

## Source 2 — Rag Guide (engineering doctrine)

Repo: <https://github.com/sujanmishra-simpro/Rag-Guide> · Site: <https://sujanmishra-simpro.github.io/Rag-Guide/>
Purpose: engineering doctrine — **this is architectural law** (owner-set).
Borrow: the entire production pipeline — parsing, chunking, indexing, embedding, retrieval, reranking, evaluation, context assembly, offline-vs-online split, benchmark philosophy.

## Source 3 — OcentraParent (production AI runtime)

Repo: <https://github.com/ocentra/OcentraParent> · Branch: `codex/tracking-plan-full-continuation-a` · Area: `crates/`
Purpose: **reuse the production AI runtime instead of reinventing it** (owner-set).
Borrow: local model runtime, model lifecycle, capability states, model cache, runtime DTOs, hardware abstraction, CPU/GPU/NPU routing, local inference management.

## Source 4 — TabAgentServer (previous-generation ideas, harvest)

Repo: <https://github.com/ocentra/TabAgentServer> · Areas: `Rust/indexing`, `Rust/knowledge-graph`, `Rust/weaver`
Purpose: previous-generation ideas — many incomplete. **Harvest them; nothing is discarded if it still fits** (owner-set).

- **Indexing crate**: indexing philosophy, metadata-vs-source, rebuildable indexes, hot/warm/cold, multi-resolution embeddings, incremental indexing — especially `fast model → large reranker → better precision`, exactly what we want.
- **Knowledge-graph crate**: node design, edge design, traversal, graph algorithms — modernize everything.
- **Weaver**: probably the biggest inspiration — background enrichment (summaries, embeddings, entity extraction, relationship generation, associative links, continuous processing, queues). **The Weaver becomes x06's background workers** (owner-set).

## Source 5 — MIA Vision (philosophy)

In TabAgentServer: `MIA_VISION.md`
Purpose: the overall philosophy — the human memory model.
Borrow: **not one database — many specialized memories** (owner-set): working, episodic, semantic, procedural, experience, summary, meta. That becomes the Enforcer memory hierarchy.

## Source 6 — MIA Memory Architecture (deep architecture)

In TabAgentServer: `Rust/docs/mia_memory.md`
Purpose: the deep architecture behind the "human brain" model.
Borrow: source-vs-derived, hot-vs-cold, cognitive routing, meta-memory, procedural memory, experience memory.

## Source 7 — Existing x05 (continuous-learning input)

Inside Enforcer.
Purpose: continuous-learning input. **x05 is the source of truth; x06 is the memory engine** (owner-set).

```text
Observations → Lessons → Validation → Memory → Retrieval → Evidence → Improvement
```

## Source 8 — Current Enforcer architecture (integrate, don't invent)

Already existing. x06 must integrate with **arc-21 MCP, arc-22 CLI, arc-23 install, x05, the proof system, and workpacks — without inventing another subsystem** (owner-set).

## Source 9 — The owner's laptop (the hardware contract)

Not a joke. "My laptop can run this" — repeated deliberately.
**Architecture assumes local inference, local embedding, local reranking — never "call OpenAI"** (owner-set). Cloud is optional; a small laptop is the reference deployment.

## Source 10 — Owner vision from live discussions (the unwritten source, now written)

Repeated dozens of times across sessions; binding:

- **Conscious harness** (owner-set): the harness observes itself, learns, benchmarks itself, improves itself.
- **Continuous tuning** (owner-set): never a fixed TopK forever — `observe → measure → retune → compare → keep the better configuration`.
- **Repo Mind** (owner-set): generated summaries, maps, architecture views, crate summaries, file summaries, git summaries — all tied back to code.
- **Token reduction** (owner-set) — one of the strongest requirements. Instead of Claude opening 40 files:

```text
query → MCP → Top 50 → KG filter → Top 20 → reranker → Top 5 → context pack
```

- **Fake Green** (owner-set): not "does cargo test pass" but — does retrieval actually work? did the benchmark improve? did token cost decrease? does Claude need fewer MCP calls? did the reranker improve? can it traverse a million-node graph? did learning improve?

## Longitudinal benchmarks (owner-set, added 2026-07-04)

The benchmark category the plan was missing. Not just "does retrieval work today" but:

- retrieval quality after 1 day / after 100 lessons / after 10,000 lessons
- after 1 million graph nodes / after 100,000 git commits / after repeated repository evolution
- index rebuild time vs incremental update time
- memory growth vs retrieval latency
- token-reduction trend over time

**The system must prove it doesn't just work — it continues to scale and improve** (owner-set). Spec lives in MEMORY_RETRIEVAL_QA_BENCHMARKS §longitudinal; results roll into `proof/memory/x06-longitudinal.json`.
