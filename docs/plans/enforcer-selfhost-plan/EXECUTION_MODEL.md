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
1. **Cut the first build worktree:** `git worktree add ../enforcer-rust -b rust-engine` (off the pushed final
   plan). `main` (the ORIGINAL working tree, `C:\Projects\ocentra-enforcer`) + the live `.mjs` MCP process stay
   pinned and untouched — that is the bootstrap-safety boundary, not a work location.
2. **Re-enable coordination writes:** the MCP staleness guard blocks `claim`/`guard`/`intent` writes until the
   running server matches disk — restart/reload the enforcer MCP (or use the `ocentra_enforcer_run` CLI
   fallback it returns, which is a proven working path — see §4).
3. **Fan out the frontier:** read WORKPACK_INDEX, take the dependency-free disjoint-`owns:` frontier, and spawn
   WORKER sub-agents (Sonnet / Haiku / Opus, reasoning matched to difficulty) — one per workpack — each taking
   a hub LANE + `claim`→`guard`→`closeout`. **Lane ≠ worktree.** The MAIN lane is special: permanently tied to
   the main tree/branch (the coordination schema's `allowPrimaryWithoutClaims` flag — the primary context acts
   without claiming). Every OTHER lane (A, B, C, D, EA, EB, EC, ED, ...) is INDEPENDENTLY bindable to any
   worktree + any branch — proven live 2026-07-04 (a lane's presence row carries its own `worktreeRoot` +
   `branch` + `commit`, not a shared global). DEFAULT TO CUTTING A NEW WORKTREE PER LANE (or per closely-related
   lane group) rather than reusing `enforcer-rust` for everything — see the TOTAL ISOLATION principle below.
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

## 2b. TOTAL ISOLATION between lanes/worktrees — a core design principle (owner-set, 2026-07-04)
**We do NOT share `Cargo.lock`/`target/`/`node_modules`/any build cache across lanes. Each lane's worktree is
fully self-contained; disk space is NOT a constraint we optimize for.**
- Every lane that gets its own worktree is responsible for installing/building its OWN toolchain state from
  scratch in that worktree — `cargo build` populates its own `target/`, `npm install` (for the UI/frontend
  lane) populates its own `node_modules/`. This is the WORKER'S job, not something pre-provisioned or shared.
- Do NOT point `CARGO_TARGET_DIR` (or any other cache) at a shared location across worktrees. No shared
  incremental-build state, no shared lockfile resolution — total isolation, on purpose. This trades disk +
  redundant compile time for zero cross-lane contamination risk (a broken/half-built crate in one lane can
  never poison another lane's build).
- Lanes can be PARKED: leave a worktree's `target/`/`node_modules/`/build artifacts on disk indefinitely
  between sessions — cheap, no cleanup needed.
- Lanes can be DELETED and REBUILT: `rm -rf target/ node_modules/` (or the whole worktree) and re-run the
  worker from scratch is a completely normal, expected operation — never treat it as a failure or something to
  avoid. When in doubt, prefer a clean rebuild over debugging shared/stale state that shouldn't exist anyway.
- Corollary for worker prompts: every WORKER's task prompt must assume NOTHING is pre-built in its worktree —
  it verifies/installs its own toolchain and builds from a clean slate every time it's spawned into a lane.
- **Scope of isolation, precisely:** total isolation applies to BUILD/RUN artifacts only — `target/`,
  `node_modules/`, and each lane's own `.enforce/` (harness run logs/diagnostics/NDJSON — these are written
  relative to whichever root a run targets, so they naturally live inside that lane's own worktree). It does
  NOT apply to the coordination ledger — see §2c, that layer is deliberately SHARED/SYNCED, not isolated.

## 2c. The coordination ledger is a SHARED, SYNCED plane — not isolated (verified against vendored source)
Confirmed by reading `src/coordination/vendor/{presence,peers,sync/http,sync/local}.js` directly (2026-07-04) —
this already exists and is more capable than a single-PC assumption:
- **Presence/heartbeat is already multi-machine, multi-project, multi-worktree aware.** `presence.js` groups
  every lane's row `byPc` (machine name), `byProject` (projectId), `byWorktree` (worktreeRoot), `byLane`, and
  `byThread`; each row carries `heartbeatExpiresAt` + a `stale`/offline flag. This is NOT scoped to one repo or
  one machine — `projectId` and `machine` are independent dimensions, so ONE ledger can track lanes across
  DIFFERENT PROJECTS on DIFFERENT MACHINES simultaneously.
- **Peer sync is real LAN/WAN replication, already implemented.** `peers.js` maintains a named peer registry
  (`peers.json`: name, URL, `pull`/`push`/`both` mode, optional `tokenEnv` for auth). `sync/http.js` pulls a
  peer's event-stream manifest over HTTP, appends only new lines (append-only, never rewrites), and detects
  divergence by comparing the last-known-common line — on conflict it writes a `<stream>.conflict.<ts>` file
  rather than silently merging or overwriting. `sync/local.js` does the analogous thing between two local
  filesystem roots (no HTTP needed — e.g. two worktrees/projects on the same machine).
- **What this enables:** a genuinely distributed swarm — lanes on THIS PC (multiple isolated worktrees) AND
  lanes on ANOTHER PC (each with their own isolated worktrees) all coordinate through the SAME hub via peer
  sync, for THIS project or ANY other project the hub is pointed at. Mail/inbox, claims, presence, and the task
  queue all propagate between machines; build/run artifacts (§2b) never do and never should.
- **Practical implication — `.enforce` does NOT sync.** Peer sync replicates the coordination LEDGER's event
  streams (claims/presence/mail/tasks), not a lane's raw `.enforce/` run diagnostics. If the orchestrator (or a
  human on another machine) needs visibility into what a remote lane actually built/ran, that lane must POST A
  SUMMARY into the ledger (a coordination report or mail message) — reading its `.enforce/` directly only works
  if you have filesystem access to that specific worktree on that specific machine.
- Rust port note: arc-16's subsystem-coverage checklist already lists `peers`, `notify`, `sync + stream` — this
  section is the DOCTRINE explaining WHY those exist and what they must preserve (peer registry + pull/push/
  both modes + append-only conflict-safe merge + multi-machine/multi-project presence grouping), not new scope.

## 2d. Lock scope, mail, and the PR gate — the actual concurrency protocol (owner-set, 2026-07-04, verified against vendored source)
**Locking is for same-file race prevention only, and only matters within one lane's own worktree (potentially
many sub-agents/threads inside that one lane touching the same files). Across DIFFERENT lanes/worktrees/
branches there is nothing to lock — those are physically different files — so cross-lane coordination is MAIL,
not locks.** This is the classic shared-mutable-state-vs-message-passing split from concurrent programming,
applied to the swarm.
- **Claim/guard/lock = intra-lane.** Use it when multiple actors could touch the identical file inside the SAME
  worktree (e.g. 100 sub-agents spawned within one lane's scope). This is what `claim`→`guard`→`closeout`
  already does and what we smoke-tested successfully (§4).
- **Mail = inter-lane, for everything else** (status, questions, hand-offs, and — the important one — PR
  gating). Different lane/worktree/branch ⇒ no lock needed, by construction, ⇒ mail is the coordination
  primitive.
- **Worker mail lifecycle (owner-set, 2026-07-04): started → progress → done/blocked, never final-only.**
  A worker lane mails `primary` at THREE points, not one: (1) **`<lane> started`** immediately after its claim
  succeeds and BEFORE it builds anything — so the orchestrator's presence picture always says who is actively
  working what, not just who claimed; (2) **`<lane> progress`** whenever it has something SUBSTANTIAL to report —
  a proof gone green, a checkpoint pushed, an unexpected finding, a decision taken inside its own scope — not a
  timer tick and not silence-until-done; (3) **`<lane> done`** (or **`<lane> blocked`** with the exact blocker)
  with branch + commit sha + proof summary. A worker that reports only at the end has violated the protocol even
  if its work is perfect: the orchestrator cannot re-plan around invisible in-flight state.
- **VERIFIED GAP (read the code, don't just trust the model):** `detectConflicts()` in
  `src/coordination/vendor/materialize.js:374` compares claims by BARE NORMALIZED RELATIVE PATH only
  (`normalizeCoordinationPath` = slash/case normalization; no `worktreeRoot` folded into the key — confirmed by
  reading `lock-policy.js:53`). So "different worktree needs no lock" is automatically true only when either
  (a) each worktree/lane uses its OWN hub (what we did for the `enforcer-rust-build` smoke test), or (b) the
  plan's `owns:` disjointness already guarantees no two concurrently-claimed workpacks target the same relative
  path regardless of worktree (true for us by construction). If a future design shares ONE hub across MANY
  worktrees where two lanes legitimately claim an identical relative path (e.g. both touching their own
  worktree's `Cargo.toml`), today's conflict detector would false-positive. **arc-16 must decide and preserve
  ONE of: (i) keep the per-worktree/per-lane-hub convention (simplest — what we use today), or (ii) key claim
  conflicts by `(worktreeRoot, normalizedRelPath)` instead of bare relPath so a genuinely shared hub is safe.**
  Default to (i) unless a workpack explicitly needs (ii).
- **PR creation is CENTRALIZED and THREE-ROLE — worker → orchestrator → gatekeeper (owner-set, 2026-07-04).**
  `WorkerState`/`TaskState` include `pr_ready` as a first-class state (`domain.js:22-23`, NOT an ad-hoc
  message); `guard.js:122` gates the `pr_ready` operation on `allowMergeRisks`; `guard.js:153` recognizes a
  literally-named `"primary"` lane with `allowPrimaryWithoutClaims`. The roles are DISTINCT — the lane that
  coordinates work must not be the lane that certifies it:
  1. **Worker lane** does its OWN local due diligence — runs its tests, collects its proof, genuinely believes
     it is done — mails `<lane> done` to the orchestrator. A worker NEVER declares `pr_ready` and NEVER opens
     a PR.
  2. **Orchestrator (primary lane)** does NOT trust the done-claim on faith (same zero-trust doctrine as rule
     authoring — "even our own validators are untrusted until fixtures pass" — now applied to orchestration).
     Since proof is never uploaded anywhere central (§2b/§2c — `.enforce/` stays local), the orchestrator goes
     to that lane's ACTUAL pushed branch/worktree, inspects the diff against the pack's `owns:` set, checks the
     claimed proof, and integrates the checkpoint into the working branch. When a coherent milestone is
     assembled (not per-pack — per genuinely shippable slice), THE ORCHESTRATOR is the one who declares
     `pr_ready`/CI-ready.
  3. **Gatekeeper (a separate verifier lane — NOT the orchestrator)** owns the final gate. On `pr_ready` it
     verifies the assembled milestone AGAINST THE PLAN: every claimed workpack's proof rows in
     TEST_PROOF_EXPECTATIONS.md are green and evidenced, the `owns:` boundaries were respected, checkpoints
     exist on the remote, and the CI gates pass when IT re-runs them. Only a gatekeeper-green milestone
     produces an actual PR to the protected branch. The orchestrator assembling the work and the gatekeeper
     certifying it being the same mind is the exact self-review failure the whole enforcer exists to prevent.
  4. **CI gates can be heavy — potentially hours.** This is exactly why `pr_ready` is a deliberate, rare
     escalation the orchestrator only declares after genuine integration due diligence — not something
     re-triggered casually, and not something worker lanes attempt speculatively "to see if they're ready."
- **Heartbeat / mail-check cadence.** AI sessions are not continuously live, so the primary lane needs a
  recurring scheduled check ("you are on this lane, check your mail") at a cadence matched to urgency (e.g.
  1/5/15-minute). Codex apparently does this natively (direct cross-session task-passing with little friction).
  For Claude, the equivalent primitives are `ScheduleWakeup` (self-reschedule a check) and `CronCreate` (a
  standing scheduled task) to simulate the same "wake up, check inbox, act if needed" loop per lane; `SendMessage`
  resumes a specific prior agent/session by id for direct two-way follow-up when needed.
- **Universal session registration — an "org chart" of every live participant.** ANY new session touching this
  project — wherever it's created (Claude Code, Cowork, a plain claude.ai chat, another harness) — should
  register itself into the SAME hub: project id, worktree, and a session identifier if one is available (a
  Codex thread/session id, or else a stable chat name as fallback). This is not limited to sessions the
  orchestrator deliberately spawns; it is the identity/presence mechanism (already visible in our own smoke
  test's presence row: `nodeId`/`nodeName`/`machine`/`projectId`/`codexSessionId`/`codexThreadId`) applied
  universally, so the hub always has a complete picture of who is doing what, anywhere.
- **This is a shipped PRODUCT capability, not just how we build the enforcer.** `enforcer-coordination`
  (arc-16) ships `enforcer coordination lane new/park/rm` (CLI + MCP tool) so any harness driving parallel
  agents against this hub gets one-worktree-per-lane, zero-shared-build-state for free instead of hand-rolling
  `git worktree` calls. `enforcer-plan`'s orchestrator binding (b04) defaults to it when assigning lanes. Every
  install adapter (c01/c03/c06/c08/c09) ships this doctrine to its harness (Claude Code, Cursor, Codex,
  Windsurf, ...) as part of the installed guidance. Our manual worktree wiring today (§1, the `enforcer-rust`
  worktree + `enforcer-rust-build` hub) is the hand-run PROTOTYPE of exactly this — once arc-16/b04/c01 ship,
  it's one command instead of `git worktree add` + `coordination init` + explicit `--repo-root` plumbing.

## 2e. Checkpoint discipline — commit + push after every step (owner-set, 2026-07-04)
**There is no local undo.** Claude / this harness has NO built-in step-checkpoint or working-tree revert (unlike
Codex / Cursor); the ONLY safety net is the git remote. So every executor treats commit+push as PART OF finishing
a step, not an afterthought:
- After each workpack close-out — and after any self-contained intermediate step worth not losing (a passing
  proof, a completed file, a green build) — `git add` the touched scope, commit with a scoped message, and
  `git push` the lane's branch to `origin`. **A step is not "done" until its bytes are on the remote.**
- **One checkpoint = one revertable point.** Keep commits small and scoped to the lane's `owns:` set so a bad
  step reverts in isolation without dragging siblings. This is per-LANE/worktree branch (each lane pushes its own
  branch).
- Checkpoints are CHEAP and FREQUENT; they PRECEDE and are distinct from the §2d `pr_ready` PR gate (rare,
  heavy, due-diligence-gated). Push freely to your lane branch; open a PR only on genuine done.
- **Never use the working tree as memory.** If it isn't committed+pushed, assume it can vanish — a clobbering
  tool, a crashed session, or a discarded/rebuilt worktree (§2b) is normal, not a failure. Commit+push is the
  save button the harness doesn't give you.

## 2f. Multi-orchestrator isolation — §2b's principle extends to orchestrator-level sessions (owner-set, 2026-07-05)
**§2b's TOTAL ISOLATION principle was written assuming exactly ONE orchestrator holds the primary worktree
(`C:\Projects\enforcer-rust`, checked out to `rust-build`) and spawns many isolated WORKER lanes from it. It
never anticipated a SECOND long-lived orchestrator-level session (e.g. a dedicated x06 memory-system
orchestrator, or any future dedicated sub-plan orchestrator) also being handed that SAME physical directory.**
This gap was hit live on 2026-07-05: the primary orchestrator's `git commit` against the shared worktree
repeatedly failed on `.git/worktrees/enforcer-rust/index.lock` because a sibling x06 orchestrator session was
concurrently running its own `git add`/`commit`/`push` in the exact same local directory. Both sessions' actual
SUBSTANTIVE work already happens on properly isolated lane branches (`lane/x06*`) — the collision was narrower:
ad hoc orchestrator-level HOUSEKEEPING commits (state-board docs, memory-stream appends) landing directly
against the shared checkout from two processes at once.

**The fix is not a new lock protocol — reusing a live coordination-hub claim was considered and rejected,
because the hub MCP surface has been unreliable/disconnected for long stretches this entire session (see §4);
building safety on top of an already-flaky dependency just relocates the failure, it doesn't remove it.**
Instead, extend §2b's own principle one level up:

- **Any session that becomes a second (or Nth) long-lived ORCHESTRATOR-level participant — not a short-lived
  worker lane — MUST get its OWN dedicated local worktree directory tracking the SAME integration branch
  (e.g. `git worktree add ../enforcer-rust-x06 rust-build`), never the primary orchestrator's physical
  checkout.** This is the exact same "total isolation" reasoning §2b already applies to worker lanes, just
  scoped up: two processes sharing one `.git/worktrees/<name>/index` file is the root cause, and separate
  worktree directories eliminate that root cause by construction — no locking mechanism needs to work
  correctly for this fix to hold, because there's nothing left to lock.
- **Both orchestrators still push to the same remote branch (`origin/rust-build`) from their separate local
  worktrees.** This can still race at the REMOTE ref-update level (git's own atomic compare-and-swap on the
  remote), which is a well-understood, standard failure mode — a rejected non-fast-forward push — NOT the
  local index corruption/contention this section fixes. The standard remedy applies: `git fetch origin`,
  rebase/merge onto the current `origin/rust-build`, retry the push. Every orchestrator session (primary and
  any secondary) does this defensively before every push to the shared integration branch, exactly as the
  primary orchestrator has already been doing all session via its cherry-pick-then-push pattern.
- **Practical remediation for an already-running secondary orchestrator session:** it does not need to be
  killed or restarted — it needs to stop doing ad hoc git operations directly in the shared checkout going
  forward and instead either (a) get a fresh dedicated worktree for its own housekeeping commits, or (b) route
  ALL its output (including doc/state-board updates) through its own lane branches and let the primary
  orchestrator integrate them via the normal §2d pattern (zero-trust verify, cherry-pick, push) — the same
  treatment every worker lane already gets. Option (b) requires no new infrastructure and is the default
  unless a secondary orchestrator's cadence genuinely needs direct-push autonomy.
- **Follow-up (T2, not blocking):** once `enforcer-coordination`'s `lane new`/`park`/`rm` CLI (arc-16, §2d) is
  shipped as a product capability, extend it with an `--orchestrator` mode that scaffolds a secondary
  orchestrator's isolated worktree in one command instead of a manual `git worktree add`, and (once the hub MCP
  surface is reliably reachable) layer an OPTIONAL courtesy claim on top as defense-in-depth — never as the
  sole safety mechanism, per the reasoning above.

## 3. Orchestration — orchestrator + worker swarm
- A high-capability ORCHESTRATOR model (Fable 5) drives; it does NOT write code itself unless absolutely
  necessary. It reads WORKPACK_INDEX, picks the dependency-free frontier of disjoint-`owns:` workpacks, and
  spawns WORKER sub-agents.
- WORKERS are Sonnet / Haiku / Opus, reasoning level matched to task difficulty (Haiku for mechanical, Sonnet
  for standard crate builds, Opus for the hard validator/parity/security work).
- Granularity: one worker per disjoint workpack; within a workpack, further disjoint sub-units may run in
  parallel too.
- Coordination via the enforcer coordination hub: each worker takes a lane (`<agent>-<wp>`), tied to its OWN
  worktree by default (per §1.3 / §2b — total isolation, not a shared tree), `claim`s its `owns:` globs (via
  `--repo-root`/`--worktree-root` scoped to that lane's worktree), `guard`s before write, posts `intent` on any
  unavoidable overlap (intent-queue serializes via mail), `closeout` on done. Disjoint-`owns:` + no dep edge ⇒
  safe parallel by construction (the plan's verified disjoint-owns design is what makes 100s of parallel
  workers safe) — per-lane worktree isolation is a SECOND layer on top, and per §2b it is the DEFAULT, not an
  optional extra.

## 3a. Agent / sub-agent / swarm taxonomy — the target agentic model
This is the vocabulary the orchestration doctrine (§3) and the hub (arc-16) build on. It defines the enforcer's
TARGET agentic system — the shape §3b then maps onto each harness's real primitives.
- **Orchestrator (primary lane).** The single high-capability driver session (Fable 5). It is permanently tied
  to the MAIN tree/branch, acts without claiming (`allowPrimaryWithoutClaims`), and is the ONLY lane that opens
  PRs (§2d). It reads WORKPACK_INDEX, picks the disjoint-`owns:` dependency-free frontier, and fans out workers.
  It orchestrates; it does not hand-write crate code beyond trivial glue.
- **Worker sub-agent.** ONE per workpack. Spawned by the orchestrator onto its OWN lane, bound by default to its
  OWN worktree (§2b total isolation). It runs `claim`→`guard`→`closeout`, does its local due diligence, and
  transitions to `pr_ready` + mails the primary lane when it genuinely believes it is done (§2d).
- **Leaf sub-agent.** An INTRA-workpack helper spawned by a worker to parallelize disjoint sub-units WITHIN a
  single workpack's scope. Leaves share their parent worker's lane/worktree; intra-lane `claim`/`guard`/`lock`
  serializes any same-file race between sibling leaves (§2d — locking is intra-lane).
- **Swarm.** A fan-out SET of workers (or of leaves) running in parallel — not a distinct role, but the
  collective noun for a wave's worth of concurrent sub-agents. A distributed swarm can span machines via peer
  sync (§2c).
- **Lane.** A coordination-hub identity (`<agent>-<wp>`) carrying its own presence row (`worktreeRoot`/`branch`/
  `commit`). The MAIN lane is special (primary, no-claim); every other lane (A/B/C/D/EA…) is independently
  bindable to any worktree + branch. **Lane ≠ worktree.**
- **Worktree.** A physical `git worktree` checkout with its own fully isolated build/run artifacts (§2b). The
  default is one worktree per lane; the main tree is the orchestrator's home + integration branch, not a work
  location.
- **NESTING RULE (3-tier max).** Workflow nesting = **1 level only** — a workflow child cannot itself launch a
  workflow. Background sub-agents CAN spawn leaf sub-agents. The supported shape is exactly three tiers:
  **orchestrator → workpack worker → intra-workpack leaf.** No deeper. Harnesses that cannot nest at all are
  flattened by §3b.

## 3b. Capability detection + adaptive degradation — map the target model onto each harness
The §3a model is the TARGET. Real harnesses differ in which primitives they actually have. c02
(harness-autodetect) produces, at install/doctor time, a per-harness **capability manifest** — max concurrent
agents, sub-agent nesting depth, background-task support, scheduled-task/cron/automation support, cross-session/
direct messaging, implicit-invocation (Codex strong; others weaker/none). The orchestrator CONSUMES that
manifest and ADAPTS; it never assumes a primitive exists (see AUDIT_FINDINGS WAVE 5).
- **Doctrine: map, then degrade — honestly and labeled, never silently.** For each target primitive the
  orchestrator checks the manifest and either uses the native primitive or drops to a declared fallback. Every
  degradation is surfaced (logged/reported into the ledger) with WHICH primitive was missing and WHICH fallback
  was chosen — a degraded run is an honest, labeled run, not a silent pretence of full capability.
- **Adaptation table (target primitive → fallback when absent):**
  - concurrency (`maxConcurrentAgents`) → THROTTLE the swarm to the declared cap; queue the rest.
  - nesting (`subAgentNestingDepth`) → FLATTEN: if the harness cannot nest, run workers directly off the
    orchestrator with no leaf tier (3-tier collapses to 2- or 1-tier).
  - scheduled mail-check (`scheduledTasks`) → POLL: if there is no cron/scheduled-task primitive, the lane
    polls its inbox on a manual cadence instead of a standing scheduled wake (contrast Codex-native scheduling;
    for Claude, `ScheduleWakeup`/`CronCreate` per §2d when available).
  - cross-session messaging (`crossSessionMessaging`) → MANUAL / HUMAN-RELAYED handoff: if the harness cannot
    pass messages between sessions, `pr_ready` and hand-offs fall back to human-relayed relay rather than direct
    `SendMessage`.
  - background tasks (`backgroundTasks`) → run foreground / synchronous; no detached background sub-agents.
  - implicit invocation (`implicitInvocation`) → require EXPLICIT invocation of the enforcer agent.
- **Unknown ⇒ fail-closed.** A manifest field of `Unknown`/`Support::Unknown` is treated as ABSENT — the
  orchestrator degrades rather than optimistically assuming the primitive works.
- **Homes.** This subsection is the adaptation DOCTRINE. The capability manifest is produced by c02; each
  c-track adapter (c03/c06/c08/c09) may refine its own harness's declared matrix; arc-16 / this taxonomy (§3a)
  is the target model the manifest is mapped onto. Extends the reference-multiharness-install-matrix.

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
- **PROVEN LIVE 2026-07-04 — per-lane worktree binding works, not just a shared tree:** created
  `../enforcer-rust` (branch `rust-engine`, off `aa7b282`); the direct MCP `coordination_claim` call was
  refused (stale-server guard, same cause as above), so used the `ocentra_enforcer_run` CLI fallback
  end-to-end instead — `coordination init` (creates hub identity; NOTE: `ensure` alone only starts the peer
  daemon, it does NOT create hub identity — `init` is required first for a brand-new hub) → `coordination
  claim --hub enforcer-rust-build --lane orchestrator-wiretest --repo-root ..\enforcer-rust --worktree-root
  ..\enforcer-rust --paths crates/enforcer-core/src/lib.rs` → exit 0 → `coordination_health` confirmed the
  presence row recorded `worktreeRoot: C:\Projects\enforcer-rust`, `branch: rust-engine`, `commit: aa7b282`
  for that lane, distinct from the main tree → `coordination release` → exit 0. Confirms: (a) the CLI fallback
  is a fully working substitute while the MCP is stale, and (b) lanes genuinely carry independent
  worktree/branch bindings in the hub schema — the design in §1.3 (per-lane worktree choice) is not
  speculative, it is the hub's actual behavior.
