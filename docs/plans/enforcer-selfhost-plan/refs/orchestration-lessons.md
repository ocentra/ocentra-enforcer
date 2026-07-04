# ORCHESTRATION LESSONS — live capture ledger (seed corpus for x05)

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `orchestration-lessons` (refs)
> Kind: append-only lesson ledger. The orchestrator appends a row the moment a lesson is learned live.
> Read when: authoring worker prompts, assigning models, or executing x05 (which mechanizes this ledger).
> Stop rule: append rows; never rewrite history rows. Each row must name where the lesson LANDED.
> Proof rule: a lesson without a landed artifact is NOT captured — it is a TODO wearing a hat.
<!-- /agent-capsule -->

Format: `id | date | observed (live evidence) | lesson | landed-at (durable artifact) | ships-via (harness surface)`.
The x05 workpack mechanizes this: capture tool + routing emitters + fail-closed doctor ("every lesson routes to a landed artifact").

Lessons are DUAL-DOMAIN (`harness` | `code` — RUST_ARCHITECTURE "The learning thesis"): orchestration/protocol
lessons AND coding-fault/fix-pattern lessons flow through the same loop; a `code` lesson lands as a rule-candidate
WITH fixtures or it does not land. Rows L13+ carry an explicit domain tag in the `observed` cell; of the seed rows,
L9 and L10 are `code`-domain, the rest `harness`. Learning is PROVABLE: t0 observation → t1 landed artifact →
t2 recurrence query (x06 `memory evidence`), backed by the tamper-evident proof journal.

| id | date | observed | lesson | landed-at | ships-via |
|---|---|---|---|---|---|
| L1 | 2026-07-04 | `coordination_init` re-init threw raw `EEXIST` | init must be idempotent (return existing identity, not a filesystem error) | arc-16 finding (this row) | fixed MCP tool behavior (arc-16) |
| L2 | 2026-07-04 | claim/mail `context` blocks reported `worktreeRoot=C:\Projects\enforcer-rust` while worker a01 verifiably wrote in `.claude/worktrees/agent-a34012…` (worker ground-truth mail) | hub context must record CALLER identity (worktree/branch/commit), not server-side resolution; caller identity should be required claim params | arc-16 finding (this row) | fixed MCP tool behavior (arc-16) |
| L3 | 2026-07-04 | harness-cut worktrees arrived at stale base `b4b6cf5`; naive merge of lane/d15 would have deleted the whole plan tree | worker step-0 is ALWAYS: fetch + hard-reset/branch from `origin/<integration-branch>`; orchestrator NEVER merges a lane branch without checking `merge-base` first (cherry-pick the scoped commit when base is stale) | worker prompt template; this row | c01 doctrine payload (worker-protocol snippet) |
| L4 | 2026-07-04 | wave-1 workers went silent until done | worker mail lifecycle is `started → progress → done/blocked`, never final-only | EXECUTION_MODEL §2d (f74102f) | c01 doctrine payload + b06 decision forest |
| L5 | 2026-07-04 | orchestrator was about to be sole verifier of its own integration | three-role gate: worker (never pr_ready) → orchestrator (integrates, declares pr_ready) → gatekeeper (separate mind, verifies proofs vs plan, only green produces a PR) | EXECUTION_MODEL §2d (f74102f) | c01 doctrine payload |
| L6 | 2026-07-04 | this harness has NO local undo (unlike Codex/Cursor); live `~/.claude.json` got clobbered mid-edit by the running session | commit+push is part of finishing a step; a step is not done until its bytes are on the remote; never use the working tree as memory | EXECUTION_MODEL §2e (f74102f) + NEXT_ACTIONS claiming discipline | c01 doctrine payload |
| L7 | 2026-07-04 | read-audit of transcripts: haiku full-read index files + read from the frozen worktree path; sonnet did targeted greps from its own worktree | worker read discipline: full-Read ONLY your workpack; Grep the index/proof/model files for your rows; ALL reads from YOUR worktree path | worker prompt template; this row | c01 doctrine payload (worker-protocol snippet) |
| L8 | 2026-07-04 | d15 (haiku): 84k tok/37 calls, compliant but sloppy; a01 (sonnet): 126k tok/109 calls, textbook incl. seeded-violation proof + justified deviations | model tiering works under a strong capsule: haiku = docs/fixtures/mechanical; sonnet = crate implementation; escalate only on judgment-heavy packs. Capsule strength substitutes for model strength | this row (baseline metrics) | b04 orchestrator binding + c02 capability manifest |
| L9 | 2026-07-04 | a01 correctly REFUSED the workpack's literal "drop the .mjs bin" because the live MCP/coordination servers this build runs on are those `.mjs` files | during self-replacement, never delete the surface you are currently standing on; live-surface removal happens only at the arc-21/arc-22 cutover (x03) | a01 deviation record; package.json inline comment | x03 rename/cutover migration |
| L10 | 2026-07-04 | dogfood scan emitted transient `DEP-1.1` solely because `cargo-audit` was not yet installed on the host | tool-ABSENCE is a doctor precondition failure, not a rule finding; findings must distinguish "violation" from "cannot check" | this row | arc-18 harness + d-track dependency-policy rules |
| L11 | 2026-07-04 | fresh-spawn bootstrap tax is ~identical per worker (tool loading + protocol + plan reads) — significant vs d15's 84k total | long-lived worker experiment: reuse one agent across a same-track dependency chain (a01→arc-01→arc-02), fresh spawns across tracks; measure tokens/pack + defect rate; retirement point when accumulated context outweighs bootstrap | EXECUTION_MODEL §3 once measured (pending data) | b04 orchestrator binding |

| L12 | 2026-07-04 | arc-01 flagged mid-flight: workpack says VENDOR OcentraParent logging-core, but the source (E:\OcentraParent) is physically absent on this machine; worker proposed spec-implement + attribute + reconcile-later instead of stalling | vendor-source-unreachable protocol: implement to the workpack's behavioral spec with a module-level attribution comment (origin intent + "must diff-reconcile against canonical source when reachable"), record as deviation, primary logs a tracked follow-up — never stall the foundation on an absent drive, never silently re-invent | primary ruling mail (hub seq 4) + this row; reconciliation follow-up = arc-25 vendoring pass re-checks arc-01's four modules | c01 doctrine payload (worker-protocol snippet) |

| L13 | 2026-07-04 | [harness] arc-01 claim friction: `coordination_claim` REJECTS glob paths (owns `crates/enforcer-core/**` had to be enumerated as exact files) and caps 10 files per claim (split across two claims); also the worktree fetch refspec only tracked `main` (worker had to add `rust-build` manually) | claims must accept glob/dir owns-sets (or auto-expand), the per-claim cap must batch transparently instead of forcing manual splits, and lane worktrees need a full fetch refspec at cut time | arc-16 finding (this row); worker template step-0 now includes refspec fix | fixed MCP tool behavior (arc-16) + c01 doctrine payload |

Append below this line as new lessons land. Never edit existing rows except to fill a previously-pending `landed-at`.
