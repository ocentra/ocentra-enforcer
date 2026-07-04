# Handoff — Make Enforcer Universally Drivable (historical record)

<!-- ai-dense -->
```yaml
status: HISTORICAL — a point-in-time handoff note from the TS/.mjs-engine era proving the coordination core was harness-agnostic even before the Rust rewrite or the multi-harness install track (c-track) existed
superseded_by: "c01-c10 (multi-harness install, all 11 adapters, native Rust) now cover everything this note called 'next steps'"
identity_finding: "conflict detection keys on worktree+branch+file, never a harness-specific thread-id label -- carried forward into enforcer-coordination's CallerContext (see COORDINATION.md)"
```
<!-- /ai-dense -->

Paste-ready context from a fresh Claude Code session, kept here as a
**historical record** of an early cross-harness drivability check performed
against the legacy TypeScript/`.mjs` engine, before the Rust rewrite and
before the multi-harness install track (c01-c10) existed. The specific
commands and file paths below describe that point-in-time environment;
current setup is [INSTALL.md](../INSTALL.md) and
[CODEX_SETUP.md](CODEX_SETUP.md).

---

## 0. TL;DR of where things stood at the time

- The coordination core (claims / guard / conflict / intent queue /
  presence) was proven drivable by a non-Codex harness lane via the CLI,
  with zero code changes needed.
- The enforcer's MCP server was registered in the harness's own user-level
  config, the same file/registry `codebase-memory-mcp` uses.
- Not yet done at the time: verifying the tools load as MCP tools inside
  that harness's UI (only the CLI had been exercised), a harness-neutral
  identity shim, and a universal per-harness installer.

## 1. Key technical finding — identity is harness-agnostic

Coordination identity resolution derives `worktreeRoot`, `branch`,
`commit`, `projectId`, and `gitRemote` from git — client-agnostic by
construction. The harness-specific thread/session id was, even then, only a
presence *label*, already overridable per call.

**Conflict detection keys on worktree + branch + file — never the thread
id.** This finding carried forward directly into the Rust rewrite: see
`CallerContext` in `enforcer-coordination` and
[COORDINATION.md](COORDINATION.md)'s presence-matrix section, which now
exposes a harness-neutral `clientThreadId`/`clientSessionId` pair.

## 2. What was verified (all green, at the time)

| Check | Result |
| --- | --- |
| MCP server protocol smoke | `ok`, full coordination toolset, route returned the rule set |
| MCP test suite | all green |
| Coordination init | hub + node id + default lane |
| Claim | `writeLock` recorded |
| Guard -> conflict (same branch, different worktree, same file) | `branch-write-conflict`, exit 1 |
| Intent queue (defer) | `intentQueued: true` |
| Presence | both lanes shown with distinct thread ids |

## 3. Roadmap items from this note, and their current status

1. Confirm MCP tools load inside the harness's own UI, not just the CLI —
   **done**: every c-track adapter (c03/c06/c07/c08/c09) now verifies this
   per harness as part of its own acceptance proof.
2. MCP parity test (reproduce the CLI flow through tool calls) — **done**:
   the Rust `enforcer-coordination` crate's MCP surface is the primary
   surface now, CLI is the secondary/equal path per
   [RUST_ARCHITECTURE.md](../docs/plans/enforcer-selfhost-plan/RUST_ARCHITECTURE.md).
3. Neutral identity shim (`clientThreadId`/`clientKind`) — **done**: landed
   as the harness-neutral presence fields described in
   [COORDINATION.md](COORDINATION.md).
4. Universal installer + per-harness adapters — **done**: `enforcer install`
   plus the c-track adapters cover all 11 harnesses at user/global scope by
   default.
5. Toolchain/engine version decision — **moot**: the engine is now a
   compiled Rust binary; there is no Node engine-version constraint to
   manage for the enforcer's own implementation.

## 4. Reference kept for context — codebase-memory-mcp

Repo: `DeusData/codebase-memory-mcp`. It remains the model for "one stdio
MCP server, ~11 harnesses, single static binary, zero deps" that the Rust
rewrite adopted as its own distribution model (see RUST_ARCHITECTURE.md
"Distribution (codebase-memory model)"). Portability lessons kept: answer
optional probes (`resources/list`, `resources/templates/list`,
`prompts/list`) with empty results; support both Content-Length and
newline-delimited JSON framing; write app config idempotently with a
backup; absolute paths + forward slashes on Windows.
