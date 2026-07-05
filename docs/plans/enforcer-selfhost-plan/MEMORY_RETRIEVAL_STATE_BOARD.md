# MEMORY_RETRIEVAL_STATE_BOARD — x06 living resume state

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `MEMORY_RETRIEVAL_STATE_BOARD`
> Kind: LIVING resume-state ledger. The orchestrator updates it in the SAME push as every integration. Any executor resuming x06 in any harness starts here (after the playbook read order).
> Read when: resuming x06; before spawning; before claiming any subpack.
> Stop rule: state only — no requirements live here. If this board contradicts proof artifacts or git, git/proof win and the board gets corrected.
> Proves: nothing. It records; proof artifacts prove.
<!-- /agent-capsule -->

_Last updated: 2026-07-05 by x06 orchestrator (session 36a5fb74). Update discipline: same commit/push as the integration that changed the state._

## Subpack board

| Subpack | Status | Lane / landed SHA | Proof artifact | Notes |
|---|---|---|---|---|
| First slice (pre-subpack) | LANDED | rust-build b96502e, a0d470b | — | record/lesson/graph/ingest/recall/retriever; 18 tests; `ingest_observation` seam; embeddings feature-gated |
| X06.1 core/store/logs | LANDED | rust-build 42324b0 (integrated by parent session pre-restart) | `proof/memory/x06-store.json` | Zero-trust verified by x06 orchestrator via merged gate 2026-07-05: 74 crate tests green incl. store/log/manifest suites, clippy -D warnings, fmt |
| X06.2 code KG indexer | LANDED | rust-build 9bca1db (from `lane/x06-2-codegraph` 5d07e33) | `proof/memory/x06-code-graph.json` (NOT yet emitted — follow-up with X06.9 harness) | Gatekeeper PASS: independent gate rerun in clean worktree (49 tests), scope clean, tests behavior-real. Known gaps recorded: `.tsx` parsed with TS grammar not LANGUAGE_TSX (JSX symbols silently missed); TS/JS share one grammar. build.rs deviation (advapi32 link fix) accepted |
| X06.3 graph algorithms | IN FLIGHT | `lane/x06-3-graphalgs` | `proof/memory/x06-kg.json` | Sonnet; Cypher-subset DSL per D-05 |
| X06.4 fulltext/vector/rerank | IN FLIGHT | `lane/x06-4-retrieval` | `proof/memory/x06-rag.json` | Sonnet; D-03 (ort default behind trait seam, mock in gates), D-04 hnsw_rs, D-07a proposal due, D-08 RRF |
| X06.5 background weaver | IN FLIGHT | `lane/x06-5-weaver` | `proof/memory/x06-weaver.json` | Sonnet; D-09 pattern harvest + DLQ/retry/tiers |
| X06.6 continuous learning | IN FLIGHT | `lane/x06-6-learning` | `proof/memory/x06-learning.json` | Sonnet; builds on ingest seam + X06.1 store |
| X06.7 MCP/CLI/watch/diagnostics | QUEUED | — | `proof/memory/x06-mcp-cli.json` | 14-tool surface per scout digest §1 |
| X06.8 sharing/federation | QUEUED | — | `proof/memory/x06-federation.json` | zstd artifact per D-11 |
| X06.9 parity/benchmark harness | QUEUED | — | `proof/memory/x06-feature-parity.json` | Closes the workpack; runs QA gate + parity + longitudinal |

## Doc/benchmark lanes

| Lane | Status | Notes |
|---|---|---|
| Scout digests | LANDED (b8a37c6) | `refs/x06-source-scout-digests.md` — 4 sources verified; OcentraParent runtime correction |
| QA-101..QA-250 authoring | LANDED (0b229ff + 898f885) | Gatekeeper PASS after fix pass (6a480c7): 150/150 unique rows recounted independently, zero deletions vs base, no minted owner-set markers (one stream-record mention allowed — provenance, not a marker), all §2 category minimums met, Federation rows re-anchored to X06.8 surfaces |
| Longitudinal benchmark corpora | NOT STARTED | Deterministic synthetic repos + replayed history (QA_BENCHMARKS §3); belongs with X06.9 |

## Open items requiring owner/orchestrator action

1. D-03 is DEFAULT (ort) — flag to owner done 2026-07-05; flips to LOCKED unless owner redirects before X06.4 spawn.
2. D-07a (full-text engine: tantivy vs SQLite FTS5) — X06.4 worker proposes with micro-benchmark, gatekeeper records verdict in DECISIONS.
3. X06.1 liveness — re-check `lane/x06-1-store` each integration cycle; salvage clock starts if silent with no further commits.

## Known constraints inherited from the wider program

- Shared primary worktree with sibling orchestrator sessions: scoped `git add` only, no reset/rebase of the shared tree (L40).
- Per-merge gate scope: workspace build + changed-crate tests + clippy -D warnings; FULL workspace test belongs to CI/z01 (L36).
- New fixture extensions must be eol=lf pinned same commit (L34/L35/L38).
