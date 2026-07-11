# g09 Memory KG RAG Explorer

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Memory KG RAG Explorer`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling completion, product status, PR readiness, or broad DONE.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-ui/src/memory_explorer/`, `crates/enforcer-ui/tests/memory_explorer/**`
- deps: `g01`, `g07`, `x06`
- tier: `P3`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN](../MEMORY_RETRIEVAL_KG_RAG_MASTER_PLAN.md), [MEMORY_RETRIEVAL_ARCHITECTURE](../MEMORY_RETRIEVAL_ARCHITECTURE.md), [workpack x06](./x06-harness-memory-graph.md).

## Where We Are
Track G has report UI, actions, settings, hub state, security, and rules/skills browsing, but it does not expose the x06 memory system itself. x06 builds a Store-backed code KG plus RAG, retrieval proof, model-runtime capability state, learning observations, lessons, recurrence curves, and proof artifacts. A human currently has no UI equivalent of a memgraph/codebase-memory graph explorer: no visual graph of files/symbols/calls/traces/lessons, no searchable retrieval/debug panel, no way to inspect why RAG chose a context pack, and no model/learning health panel.

## Where We Want To Be
A memory/KG/RAG explorer module, `crates/enforcer-ui/src/memory_explorer/`, mounted into the g01 view registry and guarded by g07. It is a read-only human surface over x06 derived projections: Store manifest, CodeGraph/MemoryGraph projections, search documents, vector/fulltext retrieval, reranker/RRF decisions, context packs, model runtime capability reports, observations, lessons, recurrence/negative evidence, and proof artifacts. It should be a useful local code-intelligence console, not a JSON dump: graph navigation, path tracing, semantic search, retrieval explainability, learning evidence, and model capability state are first-class.

## Requirement Checklist
- [ ] Mount `crates/enforcer-ui/src/memory_explorer/` into the g01 view registry as a self-contained read-only view (Tauri command + served HTML fallback, no external assets).
- [ ] Render a KG graph browser over x06 projections: files, modules, symbols, functions, types, tests, routes, imports, calls, data-flow/trace edges, ADRs, lessons, observations, proof artifacts, and cross-repo links when present.
- [ ] Provide memgraph-style exploration actions: search symbol/text/semantic query, expand neighbors, trace inbound/outbound calls, inspect snippet/source refs, show architecture clusters, and compare baseline-vs-Rust parity rows where `proof/memory/x06-kg-parity.json` exists.
- [ ] Provide RAG explainability: for a query, show BM25/vector candidates, RRF fusion, reranker scores, selected context pack, token-reduction estimate, expected/actual QA ids, and source refs from `proof/memory/x06-rag-qa.json` / retrieval proof artifacts.
- [ ] Provide learning health: observations, durable lesson candidates, t0/t1/t2 evidence, recurrence curves, negative clean-scan evidence, route-choice/meta-memory, and blockers from `proof/memory/x06-learning-curve.json`.
- [ ] Provide model runtime health: default degraded/provider-unavailable state, cache roots, manifest/hash/tokenizer status, provider fallback, GGUF/ORT capability reports, and explicit local proof artifacts without triggering downloads by default.
- [ ] Read only from x06 Store/projection/proof surfaces through Rust APIs; never parse arbitrary internal JSON directly in TS, never mutate Store, and never start model downloads or model processes from passive render.
- [ ] Keep TypeScript as presentation only: graph expansion, trace lookup, retrieval explanation, parity-label interpretation, model capability classification, and learning-state classification are Rust payloads/commands. TS may hold selected node/query text/open panel only, then sends intent to Rust.
- [ ] Honor f04 silent mode and g07 guards: no UI during inline agent runs; loopback/token/CSRF rules apply to every endpoint.
- [ ] Prove the UI by real interaction: Playwright click-through over the served fallback opens the memory view, runs a seeded graph search/trace intent, opens a RAG explanation, opens learning/model health panels, and verifies Rust-returned payloads render without any TS-side business computation.

## Acceptance And Proof
Tier P3 plus UI click-through proof. Fail-fixture: `memory-explorer-missing-store-degrades` (no Store/proofs/model cache) -> renders a stable degraded/empty state with capability reasons, no panic, no download, no mutation. Pass-fixture: `memory-explorer-seeded-kg-rag` -> seeded x06 Store/proof fixture renders graph nodes/edges, a trace path, semantic/fulltext retrieval candidates, RRF/reranker context-pack explanation, learning evidence, and model capability state. Detection test: `memory-explorer-readonly-contract` (`cargo test -p enforcer-ui memory_explorer::`) asserts every payload is built in Rust from x06 typed APIs/projections, no endpoint mutates Store or starts a model process, baseline parity rows are displayed with equal/better/incomparable/worse labels unchanged, and no external asset is fetched. Browser proof: Playwright opens the served fallback and clicks through graph search, trace, RAG explanation, learning panel, and model health panel against Rust handlers. Artifact: `proof/ui/g09-memory-explorer.json`.

## Parallel Ownership Notes
Owns only `crates/enforcer-ui/src/memory_explorer/` and `crates/enforcer-ui/tests/memory_explorer/**`. Consumes g01 view mounting and g07 security guards; consumes x06 `enforcer-memory` APIs/projections and `proof/memory/**` read-only. It does not own `crates/enforcer-memory/**`, x06 proof generation, model runtime, graph parity, retrieval ranking, or learning persistence. It is disjoint from g02 report, g06 hub, and g08 rules/skills explorer by file and by data source: g09 presents code-memory/RAG/learning/model state; g08 presents rules/skills catalog; g06 presents coordination ledger.
