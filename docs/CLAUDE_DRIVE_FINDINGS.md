# Ocentra Enforcer — "Can Claude drive it?" findings

Date: 2026-07-03 · Machine: 1K08FH4 · Repo: C:\Projects\ocentra-enforcer (cloned from github.com/ocentra/ocentra-enforcer, main)

## Verdict

**Yes — Claude can drive the coordination/enforcement core today, via the CLI, with zero code changes.** The only Codex-specific surface is a *label* (`codexThreadId`/`codexSessionId`) that is already overridable per call. Everything that matters for correctness (claims, guard, conflict detection, intent queue, presence) is client-agnostic.

## Environment

- node **v24.18.0**, npm 11.16, git 2.55. Only a `C:\` drive.
- `package.json` engines = `node >=20 <23` → **you are outside the supported range (24).** No hard block; MCP smoke + MCP test suite pass anyway. *Compat note: bump engines to allow 24, or keep CI pinned <23.*
- `npm install` = 4 packages, 5s. Minimal deps.

## What was verified (all green)

| Check | Result |
| --- | --- |
| `npm run mcp:smoke` | `ok`, `stale:false`, `writeCompatible:true`, full `ocentra_enforcer_*` toolset incl. all `coordination_*`, route returned 54 rules |
| `tests/rust-rules-mcp.test.mjs` | 3 tests, 3 pass, 0 fail (on node 24) |
| `coordination init` | hub created, node id + default lane assigned |
| `coordination claim` (lane A, worktree A, branch main) | `writeLock` recorded |
| `coordination guard` (lane B, worktree B, **same branch**, same file) | **`type: branch-write-conflict`**, `ok:false`, exit 1 — fires exactly as designed |
| `coordination claim --on-conflict intent` (lane B) | `intentQueued:true`, `type:editIntent` — the "defer"/queue path works |
| `coordination presence --json` | shows **both** lanes with distinct thread ids (`claude-a-thread`, `claude-b-thread`) |

Test staged with explicit flags (`--worktree-root`, `--branch`, `--codex-thread-id`) + `OCENTRA_PROJECT_ID` pinned, so no git-worktree gymnastics were needed to force "same project / different worktree / same branch."

## The one compat point that matters

Identity is resolved in `src/coordination/vendor/context.js`:

- `worktreeRoot`, `branch`, `commit`, `projectId`, `gitRemote` → derived from **git** (client-agnostic; works for anyone). `hub`/`projectId` also honor env overrides.
- `codexThreadId` = `input.codexThreadId ?? CODEX_THREAD_ID ?? CODEX_THREAD ?? "unknown"`
- `codexSessionId` = same shape, else `"unknown"`

**Conflict detection keys on worktree+branch+file, NOT on the thread id.** So a Claude lane already conflicts/guards correctly. The thread id is only a *presence label*, and the CLI already accepts `--codex-thread-id` / `--codexThreadId` (and env), so Claude lanes populate presence today.

### Suggested change (small, cosmetic-correctness — it's your repo)

Add a neutral `clientThreadId` + `clientKind` (`codex` | `claude` | ...) alongside the codex fields in `context.js`, `presence.js`, and the schemas, mapping to the same identity slot. Then presence/matrix aren't mislabeled "codex" when a Claude lane is running. Low effort; no behavior change. Until then, `--codex-thread-id claude-<x>` is a fine stopgap.

## How Claude uses it — two modes

1. **CLI (works now, used here):** shell out to `node scripts/ocentra-enforcer.mjs coordination …` per lane. Good enough for orchestration; conflict/guard/intent all usable.
2. **As MCP tools (not yet wired here):** register the stdio server `mcp/ocentra-enforcer-mcp.mjs` as a Claude MCP server, then call `ocentra_enforcer_coordination_*` directly. Requires adding it to the Claude client's MCP config — not doable mid-session, needs a config step.

## Next steps

1. Register `ocentra-enforcer-mcp.mjs` as a Claude MCP server; re-run the same claim/guard/intent flow through MCP tool calls (parity check vs CLI).
2. Orchestration prototype: spawn N Claude sub-agents, each with its own lane + `clientThreadId`; each claims its exact files, runs `coordination guard` before writing, releases on done. A PreToolUse-style guard hook maps 1:1 onto `coordination guard` → allow/deny/defer.
3. (Optional) engines bump for node 24; add the `clientThreadId`/`clientKind` shim.
