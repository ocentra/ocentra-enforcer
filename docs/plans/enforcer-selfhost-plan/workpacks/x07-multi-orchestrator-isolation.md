# x07 Multi-Orchestrator Isolation

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `Multi-Orchestrator Isolation`
> Kind: assigned workpack; read only when selected by hub or WORKPACK_INDEX.
> Read when: Only when this exact workpack is assigned or selected from WORKPACK_INDEX.md.
> Stop rule: Do not open sibling workpacks. Do not move product status unless this workpack and its proof rows say so.
> Proves: only the local scope stated by this file and its named proof/test rows.
> Does not prove: sibling workpack completion, product status, PR readiness, or broad DONE unless routed proof says so.
> Proof rule: Before DONE, select tests in TEST_PROOF_EXPECTATIONS.md and update proof rows.
<!-- /agent-capsule -->

- owns: `docs/plans/enforcer-selfhost-plan/EXECUTION_MODEL.md` (§2f, already landed by this pack), `docs/plans/enforcer-selfhost-plan/refs/orchestrator-worktree-setup.md`, `docs/plans/enforcer-selfhost-plan/refs/lessons/` (the L42 row and any future rows in this family)
- deps: none (protocol/docs pack; the T2 CLI follow-up depends on `arc-16`)
- tier: `T1` (process doctrine — a violation is a lost/corrupted commit, not a compile error, but the failure mode is real and was observed live)

Sources: [EXECUTION_MODEL §2b/§2d/§2f](../EXECUTION_MODEL.md), [refs/orchestration-lessons.md L42](../refs/orchestration-lessons.md).

## Where We Are

`EXECUTION_MODEL.md` §2b establishes TOTAL ISOLATION between lanes/worktrees as a core design principle — but it was written assuming exactly ONE orchestrator-level session ever holds the primary worktree (`C:\Projects\enforcer-rust`, checked out to `rust-build`) and spawns isolated WORKER lanes from it. It never anticipated a SECOND long-lived orchestrator-level session (a dedicated x06 memory-system orchestrator, spawned mid-session per the owner's explicit request) also being handed that same physical directory for its own housekeeping commits.

This gap was hit live on 2026-07-05: the primary orchestrator's `git commit` against the shared worktree repeatedly failed with `fatal: Unable to create '.../enforcer-rust/index.lock': File exists` because the sibling x06 orchestrator session was concurrently running its own `git add`/`commit`/`push` in the exact same local directory. Both sessions' substantive work already lands on properly isolated lane branches (`lane/x06*`) — the actual collision was narrower: ad hoc orchestrator-level HOUSEKEEPING commits (state-board docs, memory-stream appends) racing directly against the shared checkout from two processes at once. No work was lost (uncommitted edits survive a blocked commit — git never touches the working tree on a lock failure), but the primary orchestrator lost real wall-clock time repeatedly retrying, and the failure mode could plausibly compound (e.g. a partial index write) under worse timing.

A live coordination-hub claim (`enforcer_coordination_claim`/`guard`) was considered as the fix and rejected: the hub's MCP surface has been unreliable/disconnected for long stretches this entire session (see EXECUTION_MODEL §4). Building the safety mechanism for THIS failure on top of an already-flaky dependency just relocates the failure, it doesn't remove it.

## Where We Want To Be

Any session that becomes a second (or Nth) long-lived orchestrator-level participant gets its OWN dedicated local worktree directory tracking the same integration branch — never the primary orchestrator's physical checkout. This is §2b's own "total isolation" reasoning applied one level up: two processes sharing one `.git/worktrees/<name>/index` file is the root cause, and separate worktree directories remove that root cause by construction, with no locking mechanism required to work correctly for the fix to hold. Both orchestrators still push to the same remote branch from their separate local worktrees; a rejected non-fast-forward push at the REMOTE level is a well-understood, standard git failure mode (fetch, rebase/merge, retry) — categorically different from, and much safer than, local index-lock contention. `docs/plans/enforcer-selfhost-plan/refs/orchestrator-worktree-setup.md` is the short, concrete runbook a new secondary-orchestrator session (or the human spawning one) follows to set this up correctly the first time, so this is never re-discovered live under time pressure again.

## Requirement Checklist

- [x] `EXECUTION_MODEL.md` §2f states the doctrine: secondary orchestrator-level sessions require their own dedicated worktree; remote-level push races are handled by standard fetch/rebase/retry, not a new lock; practical remediation for an already-running secondary orchestrator (route housekeeping through lane branches, or migrate to a fresh worktree); a T2 follow-up notes future CLI mechanization via `arc-16`'s `lane new` once shipped, with an optional courtesy hub-claim layered on top ONLY once the hub is reliably reachable (never as the sole mechanism).
- [ ] `refs/orchestrator-worktree-setup.md`: a short runbook — the exact `git worktree add <path> <branch>` invocation, how to point a fresh Claude Code session's cwd at it, the fetch-rebase-retry push pattern, and the "route housekeeping through your own lane branch instead" fallback for a session that's already mid-flight in the shared worktree and doesn't want to migrate. **Fail condition (docs pack, so this is a review-fixture not a compiled test):** following the runbook naively still leaves two sessions sharing one `.git/worktrees/<name>/index` -> runbook is wrong, must be corrected. **Pass condition:** two sessions following the runbook independently never produce an `index.lock` collision in a live smoke test (see Acceptance below).
- [ ] Lesson L42 in `refs/orchestration-lessons.md` names this live incident with the concrete evidence (the exact lock-file error, the PIDs of concurrent `git.exe` processes observed) and its `landed-at` cites this workpack + the EXECUTION_MODEL §2f section once both exist.
- [ ] `WORKPACK_INDEX.md` / `NEXT_ACTIONS.md` register `x07` (currently stale on workpack counts generally — folding this pack's registration into whatever pass next refreshes those indices; not a blocking dependency for x07's own proof).

## Acceptance And Proof

Since this is a process/doctrine pack (no crate to compile), acceptance is a live smoke test rather than a `cargo test`: spin up two separate `git worktree add` checkouts of the same branch in two different local directories, run a scripted "housekeeping commit" loop concurrently in both (a tight loop of `git add`+`commit`+`push` with fetch-rebase-retry on rejection) for long enough to have collided under the OLD shared-worktree pattern, and confirm zero `index.lock` failures and a converged, non-diverged `origin/<branch>` at the end. Record the smoke-test transcript (or a link to it) as the proof artifact; there is no `proof/**` JSON row for this pack today because the plan's proof harness is crate-test-shaped — note this explicitly rather than fabricate a JSON artifact that doesn't fit, and flag it as a gap for whoever mechanizes cross-cutting process proof (candidate: x05/x06's lesson-and-process-proof machinery, once it exists).

## Parallel Ownership Notes

Leaf pack, disjoint from every crate-owning workpack — owns only plan/doctrine docs. Should be claimed and closed quickly relative to crate packs; it exists to prevent a repeat of a real, already-observed failure, not to unblock other work. The T2 CLI-mechanization follow-up (an `--orchestrator` mode on `arc-16`'s `lane new`) is explicitly deferred and does not block this pack's own DONE.
