# Handoff — Make Ocentra Enforcer universally Claude-drivable (continue in Claude Code)

Paste-ready context for a fresh **Claude Code** session. Goal: keep building toward the enforcer being driven by Claude (and any MCP harness), not just Codex. This was started in Cowork; we're moving to Claude Code because Cowork doesn't load `~/.claude/.mcp.json` and its permission prompts are app-managed and painful. Claude Code reads the exact config we already set up.

---

## 0. TL;DR of where we are

- Repo cloned + installed + validated. MCP server is healthy.
- The coordination core (claims / guard / conflict / intent queue / presence) is **proven drivable by a Claude lane via the CLI — zero code changes needed.**
- The enforcer's MCP server is **already registered in Claude Code's config** (`~/.claude/.mcp.json`), and Claude Code is set to `bypassPermissions`. So a Claude Code session launched in the repo should expose `mcp__ocentra-enforcer__*` tools with no prompts.
- Not yet done: verify the tools load *as MCP tools* in Claude Code (only CLI was exercised), the `clientThreadId` neutral-identity shim, and a universal/per-harness installer.

---

## 1. Machine / environment facts

- OS: Windows (machine `1K08FH4`), user `sujan.mishra`. Only a `C:\` drive.
- Node: **v24.18.0** at `C:\Program Files\nodejs\node.exe`; npm 11.16; git 2.55.
- **Engine mismatch (open item):** `package.json` engines = `node >=20 <23`, but node 24 is installed. No hard block — `mcp:smoke` and the MCP test suite pass on 24 — but decide: bump engines to allow 24, or pin. 
- Repo location: `C:\Projects\ocentra-enforcer` (cloned `--depth 1` from github.com/ocentra/ocentra-enforcer, `main`). `npm install` done (only 4 deps). This is a **shallow clone** — run `git fetch --unshallow` if you need history.
- Ledger root for tests: `C:\Projects\ocentra-enforcer\.ledger\<hub>`. A test hub `clh-test` already exists there from the CLI proof.

## 2. What's already configured for Claude Code (done in the previous session)

These live in `C:\Users\sujan.mishra\.claude\` and are the RIGHT config for Claude Code (they do nothing for Cowork):

- **`.mcp.json`** — added an `ocentra-enforcer` server alongside the existing `codebase-memory-mcp`:
  ```json
  "ocentra-enforcer": {
    "command": "node",
    "args": ["C:/Projects/ocentra-enforcer/mcp/ocentra-enforcer-mcp.mjs"],
    "env": { "OCENTRA_LEDGER_HOME": "C:/Projects/ocentra-enforcer/.ledger" }
  }
  ```
  Backup: `.mcp.json.bak-ocentra`.
- **`settings.json`** — added `permissions.defaultMode = "bypassPermissions"` + an allow-list, hooks preserved. Backup: `settings.json.bak-ocentra`.

> Note: `.mcp.json` at user scope may require approval on first use (`/mcp` or "enable all project MCP servers"). The repo also ships its own `.mcp.json` but with a **relative** path (`./mcp/...`) that only works when cwd = repo.

## 3. What was verified (all green)

| Check | How | Result |
| --- | --- | --- |
| MCP server protocol | `npm run mcp:smoke` | `ok`, `stale:false`, `writeCompatible:true`, full `ocentra_enforcer_*` toolset incl. all `coordination_*`; route returned 54 rules |
| MCP tests | `node --test tests/rust-rules-mcp.test.mjs` | 3 tests, 3 pass, 0 fail (on node 24) |
| Coordination init | `coordination init clh-test --lane claude-a --hub clh-test` | hub + node id + default lane |
| Claim | lane `claude-a`, worktree A, branch main | `writeLock` recorded |
| **Guard → conflict** | lane `claude-b`, worktree B, **same branch**, same file | **`type: branch-write-conflict`, ok:false, exit 1** |
| Intent queue (defer) | claim `--on-conflict intent` | `intentQueued:true`, `type:editIntent` |
| Presence | `coordination presence --json` | both lanes shown with distinct `codexThreadId` (`claude-a-thread`, `claude-b-thread`) |

Full write-up: `docs/CLAUDE_DRIVE_FINDINGS.md`.

## 4. Key technical finding — identity / why Claude lanes already work

`src/coordination/vendor/context.js` resolves identity like this:
- `worktreeRoot`, `branch`, `commit`, `projectId`, `gitRemote` → from **git** (client-agnostic).
- `hub`/`projectId` also honor env (`OCENTRA_COORDINATION_HUB`, `OCENTRA_PROJECT_ID`).
- `codexThreadId = input.codexThreadId ?? CODEX_THREAD_ID ?? CODEX_THREAD ?? "unknown"` (and `codexSessionId` similarly).

**Conflict detection keys on worktree + branch + file — NOT the thread id.** The thread id is only a presence label, and it's already overridable via the `--codex-thread-id` CLI flag / `CODEX_THREAD_ID` env / `input.codexThreadId`. So a Claude lane conflicts/guards correctly today.

Relevant CLI flags that exist (confirmed in `src/coordination/runner.mjs`): `--root`, `--worktree-root`, `--branch`, `--lane`, `--paths`, `--operation`, `--on-conflict`, `--reason`, `--codex-thread-id` / `--codexThreadId`.

## 5. Reproduce the proof (CLI) in Claude Code

From `C:\Projects\ocentra-enforcer` (forward slashes; `OCENTRA_PROJECT_ID` pins both lanes to one project so a same-branch/different-worktree conflict fires):

```powershell
node scripts/ocentra-enforcer.mjs coordination init clh-test --lane claude-a --hub clh-test
$env:OCENTRA_PROJECT_ID='clh-proj'
node scripts/ocentra-enforcer.mjs coordination claim --hub clh-test --lane claude-a --paths src/lib.rs --operation edit --worktree-root C:/wt/A --branch main --codex-thread-id claude-a-thread --reason "A" --json
node scripts/ocentra-enforcer.mjs coordination guard --hub clh-test --lane claude-b --paths src/lib.rs --operation commit --worktree-root C:/wt/B --branch main --codex-thread-id claude-b-thread --json   # expect type: branch-write-conflict, exit 1
node scripts/ocentra-enforcer.mjs coordination claim --hub clh-test --lane claude-b --paths src/lib.rs --operation edit --on-conflict intent --worktree-root C:/wt/B --branch main --codex-thread-id claude-b-thread --reason "queued" --json   # expect intentQueued:true
node scripts/ocentra-enforcer.mjs coordination presence --hub clh-test --json
```

## 6. Next steps (the actual roadmap)

1. **Confirm MCP tools load in Claude Code.** `cd C:\Projects\ocentra-enforcer && claude`, then `/mcp` — verify `ocentra-enforcer` is connected and `mcp__ocentra-enforcer__*` tools appear. Approve the user-scope server if prompted.
2. **MCP parity test.** Reproduce §5 through tool calls (`ocentra_enforcer_coordination_init/claim/guard`, `--on-conflict intent`) instead of the CLI. Before direct MCP coordination writes, call `ocentra_enforcer_mcp_status` and require `writeCompatible:true` (fails closed if stale).
3. **Neutral identity shim (`clientThreadId`/`clientKind`).** Add these alongside `codexThreadId`/`codexSessionId` in `context.js`, `src/coordination/vendor/presence.js`, and the schemas (`schemas/effect`, `schemas/json`), mapping to the same identity slot; keep codex fields as aliases for one release. This removes the "codex" mislabel when a Claude lane runs. It's your repo — safe to change.
4. **Universal installer + per-harness adapters.** The repo has `codex install` but no equivalent for other clients. Build `install.ps1`/`install.sh` (one-liners) plus thin adapters that write whichever client configs are present:
   - Claude Code → `~/.claude/.mcp.json` (this is what we hand-wrote — automate it)
   - Codex → `~/.codex/config.toml` (already exists)
   - Cursor / Zed / others → their own MCP config
   - Add `update` / `uninstall`. (`docs/INSTALL_REFERENCE_LESSONS.md` "Still Needed" already lists these.)
5. **Engine decision** — bump `engines` for node 24 or pin CI < 23.

## 7. Reference to copy from — codebase-memory-mcp

Repo: **https://github.com/DeusData/codebase-memory-mcp** (`DeusData/codebase-memory-mcp`). It's the model for "one stdio MCP server, ~11 harnesses" (Claude Code, Codex, Cursor, Zed, etc.), single static binary, zero deps. Your `docs/INSTALL_REFERENCE_LESSONS.md` already cites it. Portability lessons to keep: answer optional probes (`resources/list`, `resources/templates/list`, `prompts/list`) with empty results; support both Content-Length and newline-delimited JSON framing (enforcer already passes `mcp:smoke` and `mcp:smoke:ndjson`); write app config idempotently with a backup; absolute paths + forward slashes on Windows.

## 8. Gotchas / open items

- **Cowork ≠ Claude Code.** Cowork does not read `~/.claude/.mcp.json` (it uses a plugin system) and rewrites `claude_desktop_config.json` on exit — so hand-editing app config there doesn't stick. That's the whole reason we moved. (A future task could be packaging the enforcer as a Cowork plugin via the `create-cowork-plugin` skill, but that's separate.)
- Verify the MCP server resolves rule/schema paths by its own `__dirname`, not cwd, so it works when a client launches it from an arbitrary directory (it did under `mcp:smoke`, which runs from repo root — double-check under Claude Code's launch cwd).
- Full `npm test` is slow (exceeds short shell timeouts); run individual `tests/*.test.mjs` files or give it a long timeout.
- Node 24 vs engines `<23` (see §1/§6.5).
