# "Can Any Harness Drive It?" Findings (historical record)

<!-- ai-dense -->
```yaml
status: HISTORICAL — point-in-time cross-harness drivability check against the legacy TS/.mjs engine, predating the Rust rewrite and the c-track multi-harness install
verdict: "coordination core is client-agnostic by construction (worktree+branch+file keyed, not thread-id keyed) -- carried forward unchanged into enforcer-coordination"
superseded_by: "c01-c10 (native Rust, all 11 harness adapters) plus COORDINATION.md's harness-neutral presence fields"
```
<!-- /ai-dense -->

Kept as a **historical record**: an early check of whether a non-Codex
harness could drive the coordination/enforcement core, run against the
legacy TypeScript/`.mjs` engine before the Rust rewrite and before the
multi-harness install track existed.

## Verdict (at the time, since confirmed and generalized)

**Yes — a non-Codex harness could drive the coordination/enforcement core
via the CLI, with zero code changes.** The only harness-specific surface was
a presence *label* (thread/session id), already overridable per call.
Everything that mattered for correctness (claims, guard, conflict detection,
intent queue, presence) was client-agnostic. This finding is now permanent
architecture, not a one-off observation: see `CallerContext` in
`enforcer-coordination` and [COORDINATION.md](COORDINATION.md).

## What was verified (all green, at the time)

| Check | Result |
| --- | --- |
| MCP server protocol smoke | full coordination toolset, route returned the rule set |
| MCP test suite | all green |
| Coordination init | hub created, node id + default lane assigned |
| Coordination claim (lane A, worktree A, branch main) | `writeLock` recorded |
| Coordination guard (lane B, worktree B, same branch, same file) | `branch-write-conflict`, exit 1 — fired exactly as designed |
| Coordination claim `--on-conflict intent` (lane B) | `intentQueued: true` — the defer/queue path worked |
| Coordination presence | both lanes shown with distinct thread ids |

## The one compat point that mattered

Identity resolution derived `worktreeRoot`, `branch`, `commit`, `projectId`,
and `gitRemote` from git — client-agnostic, works for any harness.
**Conflict detection keyed on worktree+branch+file, never on the thread
id.** The thread id was only a presence label, and the CLI already accepted
an override flag/env for it.

### What changed since

The suggested "neutral `clientThreadId` + `clientKind`" shim from this note
landed as permanent architecture in `enforcer-coordination`'s presence
matrix — see [COORDINATION.md](COORDINATION.md)'s Presence Matrix section.
It is no longer a suggestion; it is the current field set, with the old
harness-specific names kept only as a compatibility alias for one Rust-pack
release.

## How any harness uses coordination today

The Rust rewrite made both paths first-class rather than CLI-primary/
MCP-secondary:

1. **MCP tools** (primary agent path): `mcp__enforcer__coordination_*`,
   registered by any of the 11 c-track harness adapters at user/global
   scope.
2. **CLI** (equally first-class): `enforcer coordination ...`, for direct/
   CI/pre-commit/cargo-alias use.

The original "next steps" in this note (register the MCP server for a
non-Codex harness, re-run the claim/guard/intent flow through MCP tool
calls, prototype multi-agent orchestration with per-lane identity) are now
the enforcer's normal, generally-available operating mode — see
[RUST_ARCHITECTURE.md](../docs/plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md)
and the "How This Was Built" section of [README.md](../README.md) for the
live proof that this scaled to a full multi-agent build.
