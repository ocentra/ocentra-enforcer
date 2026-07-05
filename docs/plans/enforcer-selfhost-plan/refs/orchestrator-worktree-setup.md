# Orchestrator worktree setup — for any SECOND (or Nth) long-lived orchestrator session

<!-- agent-capsule -->
> Agent Capsule
> Plan: `enforcer-selfhost-plan`
> Doc: `orchestrator-worktree-setup` (refs)
> Kind: runbook. Follow this BEFORE a new secondary orchestrator-level session starts doing any git work.
> Read when: spawning a second dedicated orchestrator session (e.g. an x0N-scope orchestrator) alongside an
> already-running primary orchestrator; or diagnosing a `.git/worktrees/<name>/index.lock` collision.
> Proves: nothing by itself — see x07's Acceptance section for the actual smoke test.
<!-- /agent-capsule -->

## Why this exists

Two orchestrator-level sessions must never run git commands (`add`/`commit`/`push`/`checkout`) directly against
the SAME physical local directory. Worker lanes already get this right (one isolated worktree per lane, per
`EXECUTION_MODEL.md` §2b) — this runbook extends the same rule to orchestrator-level sessions themselves, per
§2f. See §2f for the full reasoning; this file is just the concrete steps.

## Setup (run once, before the secondary orchestrator does anything)

1. Pick a path for the new worktree, distinct from the primary's (e.g. if the primary lives at
   `C:\Projects\enforcer-rust`, use `C:\Projects\enforcer-rust-x06` for an x06-scope orchestrator — name the
   suffix after the scope, not a generic counter, so it's self-documenting).
2. From ANY existing worktree of the repo (the primary's, or any lane's), run:
   ```
   git worktree add C:\Projects\enforcer-rust-x06 rust-build
   ```
   This creates a brand-new physical checkout with its OWN `.git/worktrees/enforcer-rust-x06/index` — no file
   is shared with the primary's `.git/worktrees/enforcer-rust/index`, so there is nothing left to lock across
   the two sessions.
3. Point the new orchestrator session's working directory at that new path. It operates exactly like the
   primary from here — spawn workers into their own isolated worktrees underneath it, cherry-pick their
   verified lane branches in, commit, push.

## The push pattern (both orchestrators, always)

Since both orchestrators push to the same remote branch (`origin/rust-build`) from different local worktrees,
a push can be rejected as non-fast-forward if the OTHER orchestrator pushed first. This is normal, expected,
and NOT the failure this runbook prevents (that failure was local index-lock contention, which this setup
already eliminates). Handle it with the standard git pattern, every time:
```
git fetch origin
git rebase origin/rust-build   # or merge, if rebase conflicts with in-flight local commits
git push origin rust-build
```
If the rebase/merge produces a real conflict (two orchestrators touched the same file), resolve it the same
way any cherry-pick conflict is resolved elsewhere in this plan — by content, not by blindly taking one side.

## If a secondary orchestrator is ALREADY running in the shared worktree (no time to migrate right now)

Don't fight the lock repeatedly. Instead:
- Stop doing ad hoc commits directly in the shared checkout for anything beyond what's strictly necessary.
- Route your actual output through your own lane branch(es) (which you should already be using for
  substantive work) and mail/report to the primary orchestrator for integration via the normal zero-trust
  verify-then-cherry-pick pattern (§2d) — the same treatment every worker lane gets.
- Migrate to a dedicated worktree (steps above) at the next natural pause, rather than continuing indefinitely
  in the shared directory.
