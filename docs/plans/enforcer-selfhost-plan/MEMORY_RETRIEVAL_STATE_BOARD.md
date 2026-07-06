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
| X06.3 graph algorithms | LANDED | rust-build 4d1d0d5+3592cb5 (lane deleted) | `proof/memory/x06-kg.json` (NOT yet emitted — X06.9) | Gatekeeper PASS: 116 tests independent rerun; Cypher-subset DSL (D-05) with write-verb rejection; salvage-adopted after limit-death; adopter fixed real diff-impact under-reporting bug |
| X06.4 fulltext/vector/rerank | LANDED | rust-build 8310eed+6a1821f (lane deleted) | `proof/memory/x06-rag.json` (P1-unit tier, honest) | Gatekeeper PASS: 120 tests independent rerun; D-07a=SQLite FTS5 CONFIRMED; known follow-ups: soft-signal boosting seam-only in fuse_rrf, ort-models feature empty per D-03, real Qwen3 path deferred |
| X06.5 background weaver | LANDED | rust-build 6666ff1+a0a7a03 (lane deleted) | `proof/memory/x06-weaver.json` (NOT yet emitted — X06.9) | Gatekeeper PASS: 95 tests independent rerun in clean worktree; worker fixed a real WorkerPool::shutdown deadlock in the salvage; owner-set hits were rustdoc citations only |

## LIVE PARITY MILESTONE (2026-07-06, commits 235c27b + 73f4f60)

Live tool-by-tool diff vs the installed C binary (proof/memory/x06-kg-parity.json):
**11 equal · 1 better (snippet sha256) · 0 worse · 0 unrunnable · 1 incomparable (manage_adr storage-model, behavior-level roundtrip passes both sides)** — candidate 10-1000x faster on every row. 14/14 MCP tools + CLI mirror wired and live-tested; all trace modes; search_graph 3-mode parity; federation slice; ~510 crate tests green; clippy -D warnings + fmt clean. Known follow-ups: ADR harness row flip post-W3d compat; verdict rows for index_repository/delete_project/ingest_traces; RR branded-type conformance (workspace-wide, awaiting owner decision — even enforcer-domain fails the full profile today).

## Parity push (baseline 14-tool floor) — wave 3, spawned 2026-07-05

| Lane | Status | Scope |
|---|---|---|
| `lane/x06-author-schemas` | IN FLIGHT (sonnet) | refs/x06-baseline-tool-schemas.md — exact wire contracts of all 14 baseline tools from the C source; the implementation contract for the MCP surface + X06.9 diff logic |
| `lane/x06-p1-readtools` | RESPAWNED after BOM abort (L43) | snippet.rs (byte-exact+hash), graph_schema.rs, code_search.rs (graph-augmented grep), projects.rs (list/delete/index_status over Store) |
| `lane/x06-p2-tracetools` | IN FLIGHT (sonnet) | analysis/trace.rs (data_flow + cross_service modes), traces.rs (ingest_traces, runtime-provenance edges), impact.rs risk classification |
| X06.7 MCP/CLI surface | QUEUED — spawns when schemas doc lands | mcp.rs, cli.rs, watch.rs, diagnostics.rs wrapping all 14 tools per the schemas doc |
| X06.6 continuous learning | LANDED | rust-build f8110ce..5994496 (lane deleted) | `proof/memory/x06-learning.json` (NOT yet emitted — X06.9) | Gatekeeper PASS: 101 tests independent rerun; both salvage hunks (graph.rs procedural/route-trace, record.rs Hash) call-site-justified; flagged follow-up: procedural/meta records not yet persisted via Store |
| X06.7 MCP/CLI/watch/diagnostics | QUEUED | — | `proof/memory/x06-mcp-cli.json` | 14-tool surface per scout digest §1 |
| X06.8 sharing/federation | QUEUED | — | `proof/memory/x06-federation.json` | zstd artifact per D-11 |
| X06.9 parity/benchmark harness | QUEUED | — | `proof/memory/x06-feature-parity.json` | Closes the workpack; runs QA gate + parity + longitudinal |

## Doc/benchmark lanes

| Lane | Status | Notes |
|---|---|---|
| Scout digests | LANDED (b8a37c6) | `refs/x06-source-scout-digests.md` — 4 sources verified; OcentraParent runtime correction |
| QA-101..QA-250 authoring | LANDED (0b229ff + 898f885) | Gatekeeper PASS after fix pass (6a480c7): 150/150 unique rows recounted independently, zero deletions vs base, no minted owner-set markers (one stream-record mention allowed — provenance, not a marker), all §2 category minimums met, Federation rows re-anchored to X06.8 surfaces |
| Longitudinal benchmark corpora | NOT STARTED | Deterministic synthetic repos + replayed history (QA_BENCHMARKS §3); belongs with X06.9 |

## HANDOFF — external Codex session (owner-directed, 2026-07-05)

The owner is running a Codex session on a machine with BOTH this repo and OcentraParent/TabAgentServer local, to do the literal copy/adapt harvest that remote scouting could not settle. Contract for that session:

1. **Verify the branch first**: the authoritative OcentraParent branch is `codex/tracking-plan-full-continuation-a` — the remote scout's "no runtime code" finding (DECISIONS D-03 context) MUST be re-verified against that exact local checkout; if runtime crates DO exist there, correct D-03's context in MEMORY_RETRIEVAL_DECISIONS.md (status change only per its stop rule) and prefer DEPEND/VENDOR over rebuild.
2. **Where the harvest plugs in**: `crates/enforcer-memory/src/embed.rs` + `src/rerank.rs` define the `Embedder`/`Reranker` traits and capability states (D-03); the empty `ort-models` feature in `crates/enforcer-memory/Cargo.toml` is the slot for the real local-model backend; TabAgentServer `execution-providers` (CPU/GPU/NPU detection) and `model-cache` (manifest/quant tracking) are the named harvest targets (digest §2/§3).
3. **Branch discipline**: push work to a single lane branch `lane/x06-models-harvest` off `origin/rust-build` — do NOT commit to rust-build directly; the x06 orchestrator gatekeeps and integrates on return (playbook §5). Do not touch `lane/x06-5-weaver` (pending integration) or any other x06 surface.
4. Follow MEMORY_RETRIEVAL_BORROW_POLICY for anything copied (re-type, re-test, provenance trailers) — owner-authorized literal lifting is fine as a STARTING point, but it must end gate-clean (build/test/clippy -D warnings/fmt).

## Open items requiring owner/orchestrator action

1. D-03 is DEFAULT (ort) — flag to owner done 2026-07-05; flips to LOCKED unless owner redirects before X06.4 spawn.
2. D-07a (full-text engine: tantivy vs SQLite FTS5) — X06.4 worker proposes with micro-benchmark, gatekeeper records verdict in DECISIONS.
3. X06.1 liveness — re-check `lane/x06-1-store` each integration cycle; salvage clock starts if silent with no further commits.

## Known constraints inherited from the wider program

- Shared primary worktree with sibling orchestrator sessions: scoped `git add` only, no reset/rebase of the shared tree (L40).
- Per-merge gate scope: workspace build + changed-crate tests + clippy -D warnings; FULL workspace test belongs to CI/z01 (L36).
- New fixture extensions must be eol=lf pinned same commit (L34/L35/L38).
