# Manager Runbook

## Role

The visible manager coordinates; it does not redesign architecture. It reads plan state, checks mail/locks, requests one ready workpack from the boss, assigns bounded child packets, verifies evidence, and returns an integration recommendation.

## Dispatch loop

1. Sync branch/ref and read coordination health.
2. Check `WORKPACK_INDEX.md` and dependency proof.
3. Ask the boss for one exact workpack instance.
4. Reserve separate worktrees/branches for permitted child packets.
5. Require route, claim, guard, and `<lane> started` mail before edits.
6. Poll compact status/mail; do not edit a child's files.
7. On completion, reproduce the smallest decisive gate independently.
8. Reject packets with broader diffs, missing negative proof, unsupported-as-clean behavior, or shared-file edits by non-integrators.
9. Mail the boss a table: packet, SHA, changed files, gates/run IDs, claims released, recommendation.

## Parallel rule

One active workpack is permitted at a time. Only a workpack explicitly marked parallel-safe may have at most three implementation children concurrently. Their exact owns sets must be disjoint in physical and branch paths. Different branches make overlap a merge risk rather than a free pass; overlapping paths still require serialization and `pr_ready` review. Same branch plus different worktrees on the same path is a hard branch-write conflict. UL07, shared registries, contracts, and UL14 are never child-parallel.

## Model boundary

Small-model children may perform UL00 evidence packets, post-architecture fixture/rule migrations, framework-adapter fixtures, and bounded language onboarding. They may not own UL01-UL06 architecture, shared registries, graph semantics, process policy, or closure claims.

## Escalation packet

Mail the boss:

```text
workpack / packet:
base SHA / head SHA:
exact owns:
decision needed:
evidence and run IDs:
smallest next safe action:
```
