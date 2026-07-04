# EXECUTION_MODEL — how we build the Rust enforcer (bootstrap-safe + orchestrated)

Governs HOW the finalized plan is executed. Set by the owner (2026-07-04). Companion to
[RUST_ARCHITECTURE.md](./RUST_ARCHITECTURE.md) (WHAT we build) and [WORKPACK_INDEX.md](./WORKPACK_INDEX.md).

## 1. Bootstrap safety — never break the running enforcer
- The current `.mjs` enforcer AND its live harness MCP registration STAY INTACT throughout the rebuild. We are
  wired to the working enforcer MCP right now and deliberately USE it (coordination hub, scan, proof) to build
  its own replacement — a parallax bootstrap.
- The Rust build happens in a SEPARATE git WORKTREE + BRANCH (e.g. `git worktree add ../enforcer-rust
  rust-engine`). `main` and the installed `.mjs` MCP are untouched while the crates are built.
- SWAP the MCP only after the Rust engine is proven GREEN — the `z01` dogfood gate: `cargo build` +
  `cargo test`/`clippy`/`fmt --check`/`deny`/`audit` clean, the built `enforcer` binary runs its own Rust rules
  on its own crates (non-hollow, honest ran-count), and MCP stdio smoke passes. Rollback = re-point the harness
  MCP to the `.mjs` server.
- Sequence: **finalize plan → push → create worktree+branch → orchestrate build → prove green → swap MCP here → retire `.mjs`.**

### Execution-session kickoff (the fresh Fable-5 session — it has NO prior chat context; follow this exactly)
This chat (Opus 4.8) FINISHES + pushes the plan. Then the owner switches THIS chat's model to Fable 5 and
RESTARTS it. The restarted session has no chat history — it relies on this doc + the memory files +
WORKPACK_INDEX.md. It is the ORCHESTRATOR and does, in order:
1. **Cut the shared build worktree:** `git worktree add ../enforcer-rust -b rust-engine` (off the pushed final
   plan). ALL crate work happens in `../enforcer-rust`; `main` + the live `.mjs` MCP (served from the main
   working tree) stay pinned. This is the bootstrap-safety boundary.
2. **Re-enable coordination writes:** the MCP staleness guard blocks `claim`/`guard`/`intent` writes until the
   running server matches disk — restart/reload the enforcer MCP (or use the `ocentra_enforcer_run` CLI
   fallback it returns).
3. **Fan out the frontier:** read WORKPACK_INDEX, take the dependency-free disjoint-`owns:` frontier, and spawn
   WORKER sub-agents (Sonnet / Haiku / Opus, reasoning matched to difficulty) — one per workpack — each
   operating on ABSOLUTE paths inside `../enforcer-rust`, each taking a hub lane + `claim`→`guard`→`closeout`.
   Do NOT use per-agent ephemeral worktrees (`isolation:'worktree'`) — ALL workers share the ONE
   `enforcer-rust` worktree, coordinated by the hub.
4. **Advance wave-by-wave** up the dep graph; run the `z01` dogfood gate; when green, SWAP the harness MCP
   registration to the new Rust binary and retire `.mjs`.
- The orchestrator ORCHESTRATES — it does not hand-write crate code beyond trivial glue.

## 2. Vendoring — take working code, don't re-implement
- `enforcer-events` = VENDOR `ocentra-eventing` from OcentraParent **as-is** (expected 0/minimal deps). Rename
  the crate → `enforcer-events`; swap any ocentra-specific deps for `enforcer-domain`/`enforcer-core`; keep the
  full working implementation. Do NOT spend tokens re-deriving a "lean subset" — trim unused modules later only
  if wanted.
- Logging substrate = VENDOR `logging-core` primitives **as-is** into `enforcer-core`/`enforcer-domain`/
  `enforcer-proof` (records-as-serde-structs, two-layer redaction, NDJSON writer, hash-chain), re-typing bare
  ids to `enforcer-domain` branded newtypes.
- Optionally vendor SVP/TS UI components from the repo as-is for `enforcer-ui`.
- Everything else: we already have the `.mjs` source to convert crate-by-crate.
- Attribution: record the OcentraParent origin (as we did for the vendored cybersecurity-skills).

## 3. Orchestration — orchestrator + worker swarm
- A high-capability ORCHESTRATOR model (Fable 5) drives; it does NOT write code itself unless absolutely
  necessary. It reads WORKPACK_INDEX, picks the dependency-free frontier of disjoint-`owns:` workpacks, and
  spawns WORKER sub-agents.
- WORKERS are Sonnet / Haiku / Opus, reasoning level matched to task difficulty (Haiku for mechanical, Sonnet
  for standard crate builds, Opus for the hard validator/parity/security work).
- Granularity: one worker per disjoint workpack; within a workpack, further disjoint sub-units may run in
  parallel too.
- Coordination via the enforcer coordination hub: each worker takes a lane (`<agent>-<wp>`), `claim`s its
  `owns:` globs, `guard`s before write, posts `intent` on any unavoidable overlap (intent-queue serializes via
  mail), `closeout` on done. Disjoint-`owns:` + no dep edge ⇒ safe parallel by construction (the plan's
  verified disjoint-owns design is what makes 100s of parallel workers safe).

## 4. Coordination substrate status (Claude / this harness) — SMOKE-TESTED 2026-07-04
- **Reads work:** `coordination_health` on an isolated hub returns healthy (`canLockPaths:true`,
  `canWriteClaimedPaths:true`, empty ledger, zero conflicts).
- **Writes are fail-closed when the MCP is stale:** `coordination_claim` was REFUSED with
  `writeCompatible:false, reloadRequired:true` because the long-running MCP process predates a git pull that
  updated `src/coordination/{api,runner}.mjs` on disk this session. The server refuses to write coordination
  events when its loaded code ≠ disk — a GOOD fail-closed guard (no mixed-version ledger writes). Git working
  tree is CLEAN (matches HEAD) — no rogue edits; bootstrap safety intact. The same guard will PROTECT the
  eventual `.mjs`→Rust MCP swap (it won't let a half-swapped server write).
- **Fix:** restart/reload the enforcer MCP (or use the `ocentra_enforcer_run` CLI fallback it returns) to
  re-enable writes.
- **VERDICT — the claim/intent parallel model WORKS with Claude.** Workflow/Agent sub-agents reach the
  coordination MCP via ToolSearch and can call claim/guard/release/intent. Execution uses BOTH layers:
  (1) PRIMARY safety = the orchestrator spawns only the pre-proven disjoint-`owns:` frontier per wave (the
  plan's disjointness is mechanically verified, so no live lock is strictly required for correctness);
  (2) SECONDARY = live claims/guard/intent for within-workpack sub-splits, drift detection, and the
  intent-queue on any unexpected overlap. Restart the MCP before an orchestration run so writes are enabled.
