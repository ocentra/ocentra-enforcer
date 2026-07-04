# arc-16 Crate enforcer-coordination

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Crate enforcer-coordination`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `crates/enforcer-coordination/**`
- deps: `arc-01`, `arc-02`, `arc-25`
- tier: `P1`

Sources: [PLAN_STATE](../PLAN_STATE.md), [PLAN_EXECUTION_BLUEPRINT](../PLAN_EXECUTION_BLUEPRINT.md), [TEST_PROOF_EXPECTATIONS](../TEST_PROOF_EXPECTATIONS.md), [RUST_ARCHITECTURE](../RUST_ARCHITECTURE.md).

## Where We Are
The multi-agent coordination hub is a vendored JS tree at `src/coordination/vendor/*.js` (domain, paths, root, events, identity, claim-policy, lock-policy, stream, materialize, presence, health-presence, guard, repair, server, daemon, cli, notify, peers, retention, read-index, manifest, dashboard, context). It is not Rust and not a workspace crate.

## Where We Want To Be
`enforcer-coordination` is the Rust coordination engine per RUST_ARCHITECTURE.md: hub/lane/claim/guard/ledger/presence/sync built on `enforcer-domain` (`HubName`, `LaneId`), porting the entire `src/coordination/vendor/*.js` tree to Rust. This is the crate that MUST port `src/coordination/vendor/*.js` -> Rust.

## Requirement Checklist
- [ ] Port `src/coordination/vendor/*.js` to Rust in `crates/enforcer-coordination` per RUST_ARCHITECTURE.md — explicitly: this crate ports `src/coordination/vendor/*.js` -> Rust.
- [ ] Cover the subsystems: hub, lane, claim + claim-policy, guard, ledger/events + materialize/read-index, presence + health-presence, sync + stream, plus identity, lock-policy, repair, retention, peers, notify, manifest, dashboard, context, root/paths.
- [ ] **Preserve the multi-machine/multi-project semantics exactly** (see EXECUTION_MODEL.md §2c — verified against the vendored `.js` source, not new scope): `presence` groups rows `byPc`/`byProject`/`byWorktree`/`byLane`/`byThread` with heartbeat-expiry + stale/offline detection; `peers` is a named registry (name, URL, `pull`/`push`/`both` mode, optional token-env); `sync` (http + local transports) pulls a peer's event-stream manifest, appends ONLY new lines (never rewrites), and on divergence writes a `<stream>.conflict.<ts>` file rather than silently merging. This is what makes the hub a genuinely distributed, cross-machine, cross-project coordination plane — it must not regress to single-machine/single-project during the port.
- [ ] **Preserve `pr_ready` as a first-class gated state + the `"primary"` lane's special status** (EXECUTION_MODEL.md §2d): `WorkerState`/`TaskState` include `pr_ready`; the `pr_ready` operation is guarded on `allowMergeRisks`; a literally-named `"primary"` lane carries `allowPrimaryWithoutClaims`. Only the primary lane's workflow is meant to open a PR — worker lanes transition to `pr_ready` + mail, they never open a PR themselves. This is a protocol invariant, not incidental behavior — do not drop it while porting.
- [ ] **Decide + document the claim-conflict scoping** (EXECUTION_MODEL.md §2d — VERIFIED GAP: today's `detectConflicts()` keys claims by bare normalized relative path, no `worktreeRoot`): either (i) keep one-hub-per-worktree/lane as the operational convention (default, matches current usage), or (ii) key claim conflicts by `(worktreeRoot, normalizedRelPath)` if a workpack needs one shared hub across many worktrees. Pick one explicitly in the Rust port; do not silently inherit the ambiguity.
- [ ] **Lane-worktree spawn primitive (product capability, not just our own bootstrap trick — see EXECUTION_MODEL.md §1.3/§2b):** `enforcer coordination lane new <lane-id> [--branch <name>] [--worktree-path <path>]` — creates a fresh `git worktree` for the lane (defaults to a sibling directory named after the lane), runs hub `init` for it, and returns `{lane, hub, worktreeRoot, branch}` as JSON. Contract is TOTAL ISOLATION by construction: it never points at or shares another lane's `target/`/`node_modules`/build cache; each spawned worktree is a bare git checkout only — installing/building toolchain state is the calling agent's job. Also exposed as an MCP tool (consumed by arc-21) so any harness (Claude Code, Cursor, Codex, Windsurf, ...) driving parallel workers gets this pattern without hand-rolling `git worktree` calls.
- [ ] `enforcer coordination lane park <lane-id>` / `lane rm <lane-id> [--clean]` — park leaves the worktree + its build artifacts on disk indefinitely (no-op, cheap); `rm --clean` deletes the worktree (and prunes it from git) as a normal, zero-drama operation, never a failure path.
- [ ] Build all coordination identifiers/records on `enforcer-domain` newtypes (`HubName`, `LaneId`, etc.); parse-at-boundary for the on-disk ledger/index.
- [ ] `cargo test -p enforcer-coordination` passes with fail/pass fixtures for the load-bearing invariants (claim conflict rejected vs. granted; guard fires on a corrupt ledger vs. passes on a healthy one; sync/materialize round-trip; lane-worktree spawn creates an isolated worktree and `lane rm --clean` removes it cleanly).
- [ ] Clean `cargo clippy` / `cargo fmt --check`.

## Acceptance And Proof
Tier P1. Proof row asserts `cargo test -p enforcer-coordination` exits 0 — claim/guard/sync invariants proven with fail/pass fixtures, and behavior parity with the ported `vendor/*.js` semantics. Record the artifact path in TEST_PROOF_EXPECTATIONS.md.

## Parallel Ownership Notes
Owns only `crates/enforcer-coordination/**` (the Rust port). Deps arc-01/02 only, so it can proceed in parallel with the rules/validator/lang track. Parallel-safe with arc-15/arc-17/arc-18 — disjoint crate trees. Consumed by arc-21 (mcp) for the coordination tool surface.
