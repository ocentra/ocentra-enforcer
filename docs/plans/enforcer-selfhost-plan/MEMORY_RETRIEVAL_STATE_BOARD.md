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
| X06.1 core/store/logs | IN FLIGHT | `lane/x06-1-store` @ aa20844 (checkpoint) | `proof/memory/x06-store.json` (on lane) | Parent-session worker; scope verified correct (store/**, log.rs, schema.rs, ids.rs, error.rs + tamper test). Integrate on completion or salvage per playbook §6 |
| X06.2 code KG indexer | IN FLIGHT | `lane/x06-2-codegraph` (spawned, no checkpoint yet) | `proof/memory/x06-code-graph.json` | Sonnet worker; storage via trait seam pending X06.1 |
| X06.3 graph algorithms | QUEUED | — | `proof/memory/x06-kg.json` | Needs X06.2. Cypher-subset DSL per D-05 |
| X06.4 fulltext/vector/rerank | QUEUED (spec ready) | — | `proof/memory/x06-rag.json` | Backend per D-03 (ort default, trait seam); D-07a full-text engine choice to be recorded at spawn |
| X06.5 background weaver | QUEUED | — | `proof/memory/x06-weaver.json` | Pattern harvest per D-09 |
| X06.6 continuous learning | QUEUED | — | `proof/memory/x06-learning.json` | Builds on first-slice ingest seam |
| X06.7 MCP/CLI/watch/diagnostics | QUEUED | — | `proof/memory/x06-mcp-cli.json` | 14-tool surface per scout digest §1 |
| X06.8 sharing/federation | QUEUED | — | `proof/memory/x06-federation.json` | zstd artifact per D-11 |
| X06.9 parity/benchmark harness | QUEUED | — | `proof/memory/x06-feature-parity.json` | Closes the workpack; runs QA gate + parity + longitudinal |

## Doc/benchmark lanes

| Lane | Status | Notes |
|---|---|---|
| Scout digests | LANDED (b8a37c6) | `refs/x06-source-scout-digests.md` — 4 sources verified; OcentraParent runtime correction |
| QA-101..QA-250 authoring | FIX PASS in flight | `lane/x06-author-qa250` @ 6d26bce REJECTED by gatekeeper: 140/150 rows (missing 119,129,143,161,176,193,201,221,230,241), worker-minted owner-set marker, invalid federation shortfall. Fix pass dispatched with exact defect list |
| Longitudinal benchmark corpora | NOT STARTED | Deterministic synthetic repos + replayed history (QA_BENCHMARKS §3); belongs with X06.9 |

## Open items requiring owner/orchestrator action

1. D-03 is DEFAULT (ort) — flag to owner done 2026-07-05; flips to LOCKED unless owner redirects before X06.4 spawn.
2. D-07a (full-text engine: tantivy vs SQLite FTS5) — X06.4 worker proposes with micro-benchmark, gatekeeper records verdict in DECISIONS.
3. X06.1 liveness — re-check `lane/x06-1-store` each integration cycle; salvage clock starts if silent with no further commits.

## Known constraints inherited from the wider program

- Shared primary worktree with sibling orchestrator sessions: scoped `git add` only, no reset/rebase of the shared tree (L40).
- Per-merge gate scope: workspace build + changed-crate tests + clippy -D warnings; FULL workspace test belongs to CI/z01 (L36).
- New fixture extensions must be eol=lf pinned same commit (L34/L35/L38).
