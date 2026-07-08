# MEMORY_RETRIEVAL_BORROW_POLICY — inspiration, not transplantation

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_BORROW_POLICY`
> Kind: binding policy on HOW x06 uses the DESIGN_INPUTS sources. DESIGN_INPUTS says WHAT to borrow; this doc says HOW.
> Read when: before any worker touches external-source material; before the gatekeeper approves a lane that harvested anything.
> Stop rule: policy only. Borrow scope stays in MEMORY_RETRIEVAL_DESIGN_INPUTS; vision in MEMORY_RETRIEVAL_OWNER_INTENT.
> Proves: nothing. The gatekeeper checklist in MEMORY_RETRIEVAL_EXECUTION_PLAYBOOK enforces it.
<!-- /agent-capsule -->

## 0. The principle

The owner is piecing x06 together from several of their own projects and from public prior art. Every source is **inspiration and raw material, never a transplant**. x06 is one coherent Rust crate with one idiom, one error model, one test discipline — not a collage of copied files.

```text
read the source → understand WHY it works → re-express it in enforcer idiom →
test it against enforcer fixtures → record where the idea came from
```

Direct copy-paste of external code into `crates/enforcer-memory` is a gatekeeper REJECT by default. The narrow exception: small, license-clean, dependency-free algorithmic fragments (a distance function, a tokenizer rule) may be adapted closely — still renamed, re-typed, re-tested, and provenance-noted.

## 1. What "adapt, don't copy" means mechanically

Every harvested design must be:

1. **Re-typed into enforcer's domain** — enforcer branded ids, `thiserror` error enums matching the crate's `error.rs`, the crate's existing graph node/edge model (never a parallel second model).
2. **Re-tested locally** — source-project tests are reference reading; x06 ships its own fixtures and tests per the subpack's hard-test list. A harvested idea with no enforcer test does not exist.
3. **Gate-clean** — same bar as native code: `cargo build --workspace`, `cargo test -p enforcer-memory`, clippy `-D warnings` with zero `#[allow]`, `cargo fmt`.
4. **Provenance-noted** — the commit message (not code comments) names the source: `harvested-from: TabAgentServer Rust/weaver events.rs (pattern: event-driven enrichment queue)`. The lane's memory-stream record carries the same.
5. **Trimmed to need** — take the mechanism the subpack requires, not the surrounding framework. If a source module does five things and x06 needs one, one is what gets written.

## 2. Per-source posture

| Source | Posture | Notes |
|---|---|---|
| codebase-memory-mcp (C) | **BEHAVIORAL parity only — never code** | It is C; nothing is copyable. We match its tool surface, output quality, and behaviors (see scout digest §1) via the parity harness. Its schema/log/artifact formats are reference designs to meet-or-beat, not to replicate byte-for-byte. |
| Rag-Guide | **Doctrine — follow, cite** | Architectural law (owner-set). Its prescriptions (RRF k≈60, pool sizes, versioned manifests, error buckets) are requirements, not code. Deviations need a recorded reason in MEMORY_RETRIEVAL_DECISIONS. |
| TabAgentServer (owner's repo) | **Pattern harvest, rewrite** | Owner's own code, license-safe — but written for a different domain (browser-agent conversations). Weaver/indexing patterns are re-expressed for enforcer events, rules, lessons, proofs. Its Cargo dependency CHOICES may be adopted directly (hnsw_rs, petgraph, dashmap); its qdrant/libmdbx choices are rejected (see DECISIONS). |
| OcentraParent (owner's repo) | **Contracts only** | No runtime code exists there (scout-verified). Adopt its capability-state contract shapes (`LoadState`, `ResourceClass`, `DegradedState`) as enforcer Rust types. |
| MIA docs | **Framework, translated** | The specialized-memories model translated into enforcer terms (see scout digest §2). Concepts, not schemas. |
| x05 / existing enforcer crates | **Native — integrate directly** | These are this repo; normal reuse rules, not borrow rules. x05 is the source of truth for lessons; x06 never forks its record shapes. |

## 3. Licensing and hygiene

- Only owner-controlled repos (TabAgentServer, OcentraParent, Rag-Guide) may influence code closely. Third-party repos (codebase-memory-mcp) contribute behavior specs and test expectations only.
- No vendored source trees, no git submodules to external repos, no `[patch]`/git-dependency on non-owner repos without an explicit DECISIONS entry.
- Model weights: local models are downloaded/cached at runtime by the model-cache layer with manifest + hash; weights are never committed.

## 4. Gatekeeper enforcement

The gatekeeper (orchestrator) checks on every lane that touched harvest material:

- [ ] diff contains no file recognizably lifted verbatim from a source repo (spot-check unusual naming/idioms against the scout digests);
- [ ] commit messages carry `harvested-from:` provenance for adapted patterns;
- [ ] no new dependency outside the adopted list without a DECISIONS entry;
- [ ] harvested code has enforcer-native tests in the same commit;
- [ ] no second graph/error/id model was introduced.
