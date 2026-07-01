# Coordination, Hub, Mail, And Ledger

This document is the detailed model for the Enforcer coordination system. The
short version: coordination is a Codex/harness concern, not product code.

## What It Is For

- Coordinate many Codex threads, humans, PCs, worktrees, and lanes touching the
  same projects.
- Prevent write collisions by requiring exact-file claims before edits and a
  focused guard before commit.
- Give every worker a visible identity: machine, user, OS, hub, project,
  worktree, branch, commit, lane, Codex thread/session, PID, task, heartbeat,
  and active claims.
- Provide mail between agents for assignments, acknowledgements, blockers,
  handoffs, PR-ready reports, and done reports.
- Keep an append-only audit trail of who claimed, released, reported, messaged,
  synced, repaired, and why.
- Return compact machine-readable safety decisions so Codex does not read giant
  terminal dumps or stale lane pages.

## What It Is Not For

- It is not product runtime state.
- It is not proof collection. Proof output lives under target repos at
  `.enforce/proofs`.
- It is not application logging. Harness command diagnostics live under target
  repos at `.enforce/runs`.
- It is not Git history, and Git is not the ledger sync mechanism.
- It is not tied to any one product repo. Every repo is a configured consumer.

## Ownership Model

Enforcer owns generic coordination:

- hubs;
- lanes;
- inbox/mail;
- exact-file claims;
- releases;
- focused guards;
- reports;
- workers and tasks;
- sessions and heartbeats;
- presence matrix;
- peer sync;
- stream repair and stale-claim repair;
- MCP and CLI surfaces.

Target repos own only configuration and thin aliases during migration. A product
repo should not contain the implementation of hub, mail, lane, worktree, or
exact-file-claim logic.

## Storage Model

Each PC installs Enforcer once. The default ledger home is inside that install:

```text
<enforcer-install>/.ledger/
```

On Sujan's current Windows machine that is:

```text
E:/ocentra-enforcer/.ledger/
```

Each hub lives below the ledger home:

```text
E:/ocentra-enforcer/.ledger/project-alpha/
E:/ocentra-enforcer/.ledger/project-beta/
```

The installer writes the MCP environment variable:

```toml
env = { OCENTRA_LEDGER_HOME = "E:/ocentra-enforcer/.ledger" }
```

Resolution rules:

- Normal setup uses `OCENTRA_LEDGER_HOME` plus `--hub <hub>`.
- `--ledger-root <path>` configures the per-PC ledger home during install.
- `--state-root <path>` and `LEDGER_ROOT` are exact hub-root overrides for
  repair, import, compatibility, or emergency operations.
- If no configured ledger home exists, Enforcer falls back to a user-local
  home, but that is legacy/fallback behavior, not the preferred install model.

The ledger folder is gitignored. It is live coordination state and should sync
through Enforcer peer sync or a user-chosen synced folder, not through Git.

## Canonical Truth

Canonical truth is append-only NDJSON streams:

```text
<ledger-home>/<hub>/streams/<nodeId>.<lane>.ndjson
```

Events include a sequence number, previous pointer, content hash, event type,
writer, lane, timestamp, and payload. Extended context is stored for presence
and diagnostics, but compatibility-sensitive hash material follows the v1
ledger envelope so legacy readers and Enforcer agree during migration.

Generated read models are disposable:

- JSON views under the hub root;
- optional SQLite hot index for fast operational queries;
- optional DuckDB for analytics/history when needed.

If a read model is deleted or corrupted, rebuild it from streams. Do not sync DB
files as the source of truth.

## Mail

Mail is the agent-to-agent communication surface.

Use it for:

- assignments;
- acknowledgement;
- blocker reports;
- handoffs;
- PR_READY and DONE reports;
- requests for owner release or path clarification.

Mail events are append-only. Inbox views are materialized from streams. Agents
should query compact inbox results through MCP or CLI instead of reading stream
files.

## Physical Locks Versus Git Coordination

Coordination has two different safety layers. They must not be collapsed into
one global repo-path claim.

| Situation | Hard lock? | Record |
| --- | ---: | --- |
| Same project, same worktree, same file | Yes | `writeLock` plus optional `editIntent` queue |
| Same worktree, different files | No | Exact file claims only |
| Different worktree, same branch, same file | Yes by default | `branchWriteConflict` and optional `editIntent` |
| Different worktree, same branch, different files | No | Branch presence and commit/push coordination |
| Different branch, same file | Advisory for edit | `mergeRisk`; `pr_ready` blocks unless waived |
| Different branch, different file | No | Presence only |
| Protected singleton path, any branch/worktree | Yes | `globalWriteLock` |
| Generated/source claim group | Yes for the group | `claimGroup` shared by related paths |

This split avoids the old false READ-ONLY failure mode where any two lanes
touching the same repo-relative path blocked each other, even when they were on
different branches and only needed merge-risk awareness.

## Claims And Guard

Normal claims are `writeLock` records for exact-path physical edits. They are
intentionally narrower than repo, branch, lane, or directory ownership.
Singleton paths promote to `globalWriteLock`; branch ownership uses
`branchLease`; broad plan ownership uses `workReservation`.

Expected workflow:

1. Inspect inbox and active claims.
2. Claim exact files before editing.
   Use `--operation edit --on-conflict intent` when another writer may already
   own the file.
3. Edit only claimed files.
4. Run a focused guard on changed paths before commit with `--operation commit`.
5. Release paths when done or blocked.

If a claim is blocked and `onConflict=intent`, Enforcer appends an `editIntent`
instead of creating a second lock. On release, Enforcer sends mail to the next
queued lane. The next writer must re-read the file before claiming and editing.

Guard is operation-aware:

| Operation | Contract |
| --- | --- |
| `inspect` | Allow unless streams cannot be read; return conflicts as warnings. |
| `edit` | Block same-worktree, same-branch, and global hard conflicts; allow merge risks as warnings. |
| `commit` | Require this lane owns changed files in this worktree. |
| `push` | Check branch lease/remote coordination, not physical file locks. |
| `rebase` / `merge` | Report merge risks, branch conflicts, stale locks, and dirty-worktree metadata. |
| `pr_ready` | Block unresolved hard conflicts, queued intents, and merge risks unless explicitly waived. |

Focused guard answers:

- `canInspect`;
- `canLockPaths`;
- `canWriteClaimedPaths`;
- `mustWait`;
- `mustRepairLedger`;
- `conflictingPaths`;
- stale sessions;
- stale locks;
- global warnings.

Path-specific blockers should block the current worker. Global corruption or old
sequence issues should appear as repair warnings unless they prevent trusted
materialization.

## Presence Matrix

Presence tells every participant who is active and where.

Rows include:

- `nodeId`;
- `nodeName`;
- `machine`;
- `user`;
- `os`;
- `hub`;
- `projectId`;
- `repoRoot`;
- `worktreeRoot`;
- `gitRemote`;
- `branch`;
- `commit`;
- `cwd`;
- `lane`;
- `codexThreadId`;
- `codexSessionId`;
- `pid`;
- `state`;
- `lastSeenAt`;
- `heartbeatExpiresAt`;
- `activeTask`;
- `activeClaims`;
- `unreadInboxCount`;
- `peerUrl`;
- `syncStatus`.

Views group by PC, project, worktree, lane, thread, claimed path, and
stale/offline state. This is how a coordinator answers questions like:

- Which thread owns this path?
- Which PC has a stale session?
- Which worktree is a child lane using?
- Can this worker write the files it changed?
- Is a conflict path-specific or only historical noise?

## Peer Sync

LAN/WAN sync is stream replication, not database replication.

The sync contract:

- Each stream has a manifest with byte length, event count, seq range, tail
  hash, and archive segment hashes.
- Peers compare manifests.
- If prefixes match, only missing suffix bytes/events are transferred.
- If prefixes diverge, Enforcer writes conflict copies instead of overwriting.
- After sync, read models are rebuilt from streams.

WAN transport is not NAT traversal in v1. Use a supported secure transport:

- Tailscale;
- WireGuard;
- Cloudflare Tunnel;
- direct HTTPS;
- another user-managed private network.

The peer API should be token-protected. Sync should be safe to retry and should
not require raw file-share access when HTTP peer sync is available.

## Repair

Repair is append-only or backup-before-write.

Supported repair surfaces:

- `coordination repair legacy-hash`: repair early context-hashed Enforcer events
  for v1 ledger compatibility.
- `coordination repair sequence`: repair sequence and previous-pointer breaks
  where safe.
- `coordination repair all`: run the compatible repair set.
- `coordination repair stale-claims`: append `claim.resolve` events for exact
  paths after stream health is trusted.
- `coordination closeout`: release all claims for a selected lane/thread scope,
  repair stale selected-owner claims, rebuild the read index, and fail if any
  matching claim remains.

Always dry-run first. The write form should report backup paths or appended
repair events.

## CLI Shape

Common commands:

```powershell
ocentra-enforcer coordination root --hub project-alpha
ocentra-enforcer coordination init project-alpha --hub project-alpha --lane codex-a
ocentra-enforcer coordination presence --hub project-alpha --json
ocentra-enforcer coordination inbox --hub project-alpha --lane codex-a --json
ocentra-enforcer coordination message --hub project-alpha --from codex-a --to codex-b --subject "..." --body "..."
ocentra-enforcer coordination claim --hub project-alpha --lane codex-a --paths src/lib.rs --operation edit --on-conflict intent --reason "exact file claim"
ocentra-enforcer coordination guard --hub project-alpha --lane codex-a --paths src/lib.rs --operation commit --json
ocentra-enforcer coordination release --hub project-alpha --lane codex-a --paths src/lib.rs --reason "done"
ocentra-enforcer coordination closeout --hub project-alpha --lane codex-a --thread-id <codex-thread-id> --reason "done" --json
ocentra-enforcer coordination manifest --hub project-alpha --json
ocentra-enforcer coordination peer list --hub project-alpha --json
ocentra-enforcer coordination sync --hub project-alpha --peer office --json
```

Use `--state-root <exact-hub-root>` only when operating on a specific legacy or
repair root. Normal commands should rely on `OCENTRA_LEDGER_HOME` plus `--hub`.

## MCP Shape

Codex should prefer MCP tools over raw terminal output:

- `ocentra_enforcer_coordination_health`;
- `ocentra_enforcer_coordination_presence`;
- `ocentra_enforcer_coordination_inbox`;
- `ocentra_enforcer_coordination_mail`;
- `ocentra_enforcer_coordination_claim`;
- `ocentra_enforcer_coordination_release`;
- `ocentra_enforcer_coordination_closeout`;
- `ocentra_enforcer_coordination_guard`;
- `ocentra_enforcer_coordination_report`;
- `ocentra_enforcer_coordination_message`;
- `ocentra_enforcer_coordination_workers`;
- `ocentra_enforcer_coordination_tasks`;
- `ocentra_enforcer_coordination_streams`;
- `ocentra_enforcer_coordination_peer`;
- `ocentra_enforcer_coordination_sync`;
- `ocentra_enforcer_coordination_repair`.

MCP results should stay compact. Raw streams and large worker dumps are fallback
debug artifacts, not the normal agent workflow.

Before an agent reports `DONE` or `PR_READY`, it should call closeout for its
lane/thread scope. A successful closeout means no matching active claims remain.
If closeout fails, the task is not done; inspect `remainingClaims` and resolve
the exact owner/path state first.

## Safe Subagent Model

Use unique child lanes for parallel workers:

```text
codex-a-parser
codex-a-ui
codex-a-proof
```

The coordinator can keep the parent lane. Child lanes claim exact paths and use
focused guard decisions. Avoid multiple sessions writing under the same lane
unless there is a single active lease owner or the project has fully moved to
Enforcer claim checks without legacy wrapper lease enforcement.

## Done Criteria For Migration

The coordination system is deletion-ready for a legacy product repo only when:

- Enforcer can init, message, inbox, ack, claim, guard, release, report, and
  show presence without using product repo wrappers.
- MCP and CLI return equivalent or better compact decisions than old scripts.
- Peer sync and manifest checks work from external ledger roots.
- Hash and sequence repair are available before stale-claim cleanup.
- Product repo wrappers are thin aliases only.
- The product repo no longer owns the implementation of hub, lane, mail,
  claims, workers, tasks, or peer sync.
