import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  coordinationClaim,
  coordinationGuard,
  coordinationHealth,
  coordinationIndex,
  coordinationInbox,
  coordinationInit,
  coordinationMessage,
  coordinationPeer,
  coordinationPresence,
  coordinationRelease,
  coordinationRepair,
  coordinationStreams,
  coordinationStatus,
  coordinationSync,
} from "../src/coordination/api.mjs";
import { loadIdentity } from "../src/coordination/vendor/identity.js";
import { appendEvent } from "../src/coordination/vendor/stream.js";
import {
  assertCoordinationHashCompatibility,
  coordinationHashCompatibility,
  hashForEvent,
  hashForEventWithExtensions,
} from "../src/coordination/vendor/events.js";
import { inspectLedger } from "../src/coordination/vendor/doctor.js";
import { startPeerServer } from "../src/coordination/vendor/server.js";

const PACK_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const CLI = path.join(PACK_ROOT, "scripts", "rust-rules.mjs");

test("coordination hash compatibility self-test excludes extension context", () => {
  const compatibility = coordinationHashCompatibility();
  assert.equal(compatibility.ok, true);
  assert.equal(compatibility.contextExcludedFromWireHash, true);
  assert.notEqual(compatibility.actualWireHash, compatibility.extensionHash);
  assert.doesNotThrow(() => assertCoordinationHashCompatibility());
});

test("coordination API initializes generic external state and guards exact claims", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-coord-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-coord-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"));
  fs.writeFileSync(path.join(targetRoot, "src", "lib.rs"), "fn local() {}\n");
  await coordinationInit({ stateRoot, hub: "generic-hub", lane: "codex-a" });

  const health = await coordinationHealth({ stateRoot, lane: "codex-a" });
  assert.equal(health.canInspect, true);
  assert.equal(health.mustRepairLedger, false);

  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "test claim",
  });
  const guard = await coordinationGuard({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
  });
  assert.equal(guard.ok, true);

  const denied = await coordinationGuard({
    stateRoot,
    root: targetRoot,
    lane: "codex-b",
    paths: ["src/lib.rs"],
  });
  assert.equal(denied.ok, false);
  assert.match(
    denied.result.findings.join("\n"),
    /changed path src\/lib\.rs is claimed by codex-a .* lane codex-b cannot write it/u,
  );
  assert.doesNotMatch(
    denied.result.findings.join("\n"),
    /no active ledger claim/u,
  );

  await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "test release",
  });
  const afterReleasePresence = await coordinationPresence({ stateRoot });
  assert.deepEqual(afterReleasePresence.views.byClaimedPath, {});
  const released = await coordinationGuard({
    stateRoot,
    root: targetRoot,
    lane: "codex-b",
    paths: ["src/lib.rs"],
  });
  assert.equal(released.ok, false);

  await assert.rejects(
    () =>
      coordinationClaim({
        stateRoot,
        root: targetRoot,
        lane: "codex-a",
        paths: ["src/lib.rs"],
        action: "release",
      }),
    /coordination claim does not support action="release"/u,
  );
});

test("focused coordination guard only blocks requested path conflicts", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-focused-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-focused-target-"));
  fs.mkdirSync(path.join(targetRoot, "other"), { recursive: true });
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  fs.writeFileSync(path.join(targetRoot, "other", "busy.ts"), "export const busy = true;\n");
  await coordinationInit({ stateRoot, hub: "focused-hub", lane: "codex-a" });
  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "focused owner",
  });
  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-b",
    paths: ["other/busy.ts"],
    reason: "unrelated owner b",
  });
  const config = await loadIdentity(stateRoot);
  await appendEvent(stateRoot, config, "codex-c", {
    type: "claim",
    paths: ["other/busy.ts"],
    reason: "legacy unrelated owner c",
    context: {
      repoRoot: targetRoot,
      worktreeRoot: targetRoot,
      cwd: targetRoot,
      branch: "main",
      operation: "edit",
      lockKind: "writeLock",
    },
  });

  const focused = await coordinationGuard({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    limit: 1,
  });
  assert.equal(focused.ok, true);
  assert.equal(focused.result.findings.length, 0);
  assert.equal(focused.result.globalWarningCount, 1);
  assert.equal(focused.result.globalWarnings.length, 1);
  assert.match(focused.result.globalWarnings[0], /write-lock-conflict/u);

  const focusedHealth = await coordinationHealth({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    limit: 1,
  });
  assert.equal(focusedHealth.mustWait, false);
  assert.equal(focusedHealth.canWriteClaimedPaths, true);
  assert.equal(focusedHealth.conflictCount, 0);
  assert.equal(focusedHealth.globalConflictCount, 1);

  const unfocused = await coordinationGuard({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    focused: false,
  });
  assert.equal(unfocused.ok, false);
  assert.match(unfocused.result.findings.join("\n"), /write-lock-conflict/u);
});

test("same-worktree write lock queues edit intent and notifies on release", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-intent-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-intent-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "lib.rs"), "fn local() {}\n");
  await coordinationInit({ stateRoot, hub: "intent-hub", lane: "codex-a" });

  const owner = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "owner edit",
    branch: "main",
  });
  assert.equal(owner.ok, true);

  const intent = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    reason: "next edit",
    branch: "main",
    onConflict: "intent",
  });
  assert.equal(intent.ok, false);
  assert.equal(intent.intentQueued, true);
  assert.equal(intent.event.type, "editIntent");
  assert.equal(intent.blockers[0].type, "write-lock-conflict");
  assert.equal(intent.blockingOwners.length, 1);
  assert.equal(intent.blockingOwners[0].lane, "codex-a");
  assert.match(intent.nextStep, /re-read/iu);

  const statusWithIntent = await coordinationStatus({ stateRoot });
  assert.equal(statusWithIntent.state.ownership.editIntents.length, 1);

  const release = await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "done",
  });
  assert.equal(release.ok, true);
  assert.equal(release.notificationEvents.length, 1);
  assert.equal(release.notificationEvents[0].to, "codex-b");

  const inbox = await coordinationInbox({ stateRoot, lane: "codex-b" });
  assert.match(inbox.inbox[0].body, /Re-read the file/u);

  const nextClaim = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    reason: "claim after reread",
    branch: "main",
  });
  assert.equal(nextClaim.ok, true);
  const statusAfterClaim = await coordinationStatus({ stateRoot });
  assert.equal(statusAfterClaim.state.ownership.editIntents.length, 0);
});

test("different worktree on same branch is a hard branch write conflict", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-branch-conflict-"));
  const worktreeA = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-worktree-a-"));
  const worktreeB = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-worktree-b-"));
  fs.mkdirSync(path.join(worktreeA, "src"), { recursive: true });
  fs.mkdirSync(path.join(worktreeB, "src"), { recursive: true });
  fs.writeFileSync(path.join(worktreeA, "src", "lib.rs"), "fn a() {}\n");
  fs.writeFileSync(path.join(worktreeB, "src", "lib.rs"), "fn b() {}\n");
  await coordinationInit({ stateRoot, hub: "branch-conflict-hub", lane: "codex-a" });

  await coordinationClaim({
    stateRoot,
    root: worktreeA,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "worktree a",
    projectId: "same-project",
    repoRoot: worktreeA,
    worktreeRoot: worktreeA,
    branch: "feature/shared",
  });

  const blocked = await coordinationClaim({
    stateRoot,
    root: worktreeB,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    reason: "worktree b",
    projectId: "same-project",
    repoRoot: worktreeB,
    worktreeRoot: worktreeB,
    branch: "feature/shared",
    onConflict: "intent",
  });
  assert.equal(blocked.ok, false);
  assert.equal(blocked.intentQueued, true);
  assert.equal(blocked.blockers[0].type, "branch-write-conflict");
});

test("different branch same file is edit advisory but pr_ready blocker", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-merge-risk-"));
  const worktreeA = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-merge-a-"));
  const worktreeB = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-merge-b-"));
  fs.mkdirSync(path.join(worktreeA, "src"), { recursive: true });
  fs.mkdirSync(path.join(worktreeB, "src"), { recursive: true });
  fs.writeFileSync(path.join(worktreeA, "src", "lib.rs"), "fn a() {}\n");
  fs.writeFileSync(path.join(worktreeB, "src", "lib.rs"), "fn b() {}\n");
  await coordinationInit({ stateRoot, hub: "merge-risk-hub", lane: "codex-a" });

  await coordinationClaim({
    stateRoot,
    root: worktreeA,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "branch a",
    projectId: "same-project",
    repoRoot: worktreeA,
    worktreeRoot: worktreeA,
    branch: "feature/a",
  });

  const advisory = await coordinationClaim({
    stateRoot,
    root: worktreeB,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    reason: "branch b",
    projectId: "same-project",
    repoRoot: worktreeB,
    worktreeRoot: worktreeB,
    branch: "feature/b",
  });
  assert.equal(advisory.ok, true);

  const editGuard = await coordinationGuard({
    stateRoot,
    root: worktreeB,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    projectId: "same-project",
    repoRoot: worktreeB,
    worktreeRoot: worktreeB,
    branch: "feature/b",
    operation: "edit",
  });
  assert.equal(editGuard.ok, true);
  assert.equal(editGuard.result.mergeRisks.length, 1);
  assert.equal(editGuard.result.globalWarningCount, 1);

  const prReady = await coordinationGuard({
    stateRoot,
    root: worktreeB,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    projectId: "same-project",
    repoRoot: worktreeB,
    worktreeRoot: worktreeB,
    branch: "feature/b",
    operation: "pr_ready",
  });
  assert.equal(prReady.ok, false);
  assert.match(prReady.result.findings.join("\n"), /merge-risk/u);
});

test("global singleton paths hard lock across branches and worktrees", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-global-lock-"));
  const worktreeA = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-global-a-"));
  const worktreeB = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-global-b-"));
  fs.writeFileSync(path.join(worktreeA, "Cargo.lock"), "# lock a\n");
  fs.writeFileSync(path.join(worktreeB, "Cargo.lock"), "# lock b\n");
  await coordinationInit({ stateRoot, hub: "global-lock-hub", lane: "codex-a" });

  await coordinationClaim({
    stateRoot,
    root: worktreeA,
    lane: "codex-a",
    paths: ["Cargo.lock"],
    reason: "lockfile update a",
    projectId: "same-project",
    repoRoot: worktreeA,
    worktreeRoot: worktreeA,
    branch: "feature/a",
  });

  const blocked = await coordinationClaim({
    stateRoot,
    root: worktreeB,
    lane: "codex-b",
    paths: ["Cargo.lock"],
    reason: "lockfile update b",
    projectId: "same-project",
    repoRoot: worktreeB,
    worktreeRoot: worktreeB,
    branch: "feature/b",
  });
  assert.equal(blocked.ok, false);
  assert.equal(blocked.blockers[0].type, "global-write-conflict");
  assert.equal(blocked.blockingOwners[0].lockKind, "globalWriteLock");
});

test("inspect allows conflicts as warnings while commit requires same-worktree claims", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-operation-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-operation-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "lib.rs"), "fn local() {}\n");
  await coordinationInit({ stateRoot, hub: "operation-hub", lane: "codex-a" });
  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "owner",
    branch: "main",
  });

  const inspect = await coordinationGuard({
    stateRoot,
    root: targetRoot,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    operation: "inspect",
  });
  assert.equal(inspect.ok, true);
  assert.equal(inspect.result.findings.length, 0);
  assert.equal(inspect.result.globalWarningCount, 1);

  const commit = await coordinationGuard({
    stateRoot,
    root: targetRoot,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    operation: "commit",
  });
  assert.equal(commit.ok, false);
  assert.match(commit.result.findings.join("\n"), /write-lock-conflict/u);
});

test("coordination message and inbox are generic by hub/state root", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-mail-"));
  await coordinationInit({ stateRoot, hub: "generic-mail", lane: "primary" });
  await coordinationMessage({
    stateRoot,
    to: "codex-b",
    body: "Do the generic coordination slice.",
  });

  const inbox = await coordinationInbox({ stateRoot, lane: "codex-b" });
  assert.equal(inbox.ok, true);
  assert.equal(inbox.inbox.length, 1);
  assert.match(inbox.inbox[0].body, /generic coordination/u);
});

test("coordination presence captures PC/project/worktree/thread context and writes read index", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-presence-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-presence-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"));
  fs.writeFileSync(path.join(targetRoot, "src", "lib.rs"), "fn local() {}\n");
  await coordinationInit({ stateRoot, hub: "presence-hub", lane: "codex-a" });
  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "presence matrix claim",
    projectId: "presence-project",
    repoRoot: targetRoot,
    worktreeRoot: targetRoot,
    codexThreadId: "thread-presence",
    codexSessionId: "session-presence",
  });

  const presence = await coordinationPresence({ stateRoot });
  assert.equal(presence.ok, true);
  assert.equal(presence.rows.length, 1);
  assert.equal(presence.rows[0].projectId, "presence-project");
  assert.equal(presence.rows[0].worktreeRoot, path.resolve(targetRoot));
  assert.equal(presence.rows[0].codexThreadId, "thread-presence");
  assert.equal(
    presence.views.byClaimedPath["src/lib.rs"][0].codexSessionId,
    "session-presence",
  );

  const health = await coordinationHealth({
    stateRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
  });
  assert.equal(health.presence.rows[0].projectId, "presence-project");

  const index = await coordinationIndex({ stateRoot });
  assert.equal(index.ok, true);
  assert.equal(index.counts.presenceRows, 1);
  assert.equal(
    fs.existsSync(path.join(stateRoot, "db", "coordination-index.json")),
    true,
  );

  const streams = await coordinationStreams({ stateRoot });
  assert.equal(streams.streams.length, 1);
  assert.equal(streams.streams[0].eventCount, 1);
  assert.equal(typeof streams.streams[0].tailHash, "string");

  const streamPath = path.join(
    stateRoot,
    "streams",
    `${streams.streams[0].stream}`,
  );
  const event = JSON.parse(fs.readFileSync(streamPath, "utf8").trim());
  const { hash, ...withoutHash } = event;
  assert.equal(hash, hashForEvent(withoutHash));
  assert.notEqual(hash, hashForEventWithExtensions(withoutHash));
});

test("coordination repair fixes Enforcer context-hashed streams for legacy readers", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-repair-"));
  const streamsRoot = path.join(stateRoot, "streams");
  fs.mkdirSync(streamsRoot, { recursive: true });
  const streamPath = path.join(streamsRoot, "node_test.codex-a.ndjson");
  const firstBase = {
    id: "evt_contexthash0001",
    schema: 1,
    hub: "repair-hub",
    nodeId: "node_test",
    nodeName: "TestNode",
    lane: "codex-a",
    writer: "node_test.codex-a",
    type: "claim",
    ts: "2026-06-30T00:00:00.000Z",
    seq: 1,
    prevEventId: null,
    prevHash: null,
    paths: ["src/lib.rs"],
    reason: "legacy compatibility regression",
    context: {
      projectId: "repair-project",
      repoRoot: "C:/repo",
      worktreeRoot: "C:/repo",
      codexThreadId: "thread-repair",
    },
  };
  const first = {
    ...firstBase,
    hash: hashForEventWithExtensions(firstBase),
  };
  const secondBase = {
    id: "evt_aftercontext0002",
    schema: 1,
    hub: "repair-hub",
    nodeId: "node_test",
    nodeName: "TestNode",
    lane: "codex-a",
    writer: "node_test.codex-a",
    type: "session.claim",
    ts: "2026-06-30T00:00:01.000Z",
    seq: 2,
    prevEventId: first.id,
    prevHash: first.hash,
    sessionId: "session-repair",
    ttlSeconds: 120,
    summary: "session after context hash",
  };
  const second = {
    ...secondBase,
    hash: hashForEvent(secondBase),
  };
  fs.writeFileSync(
    streamPath,
    `${JSON.stringify(first)}\n${JSON.stringify(second)}\n`,
    "utf8",
  );

  const before = await inspectLedger(stateRoot);
  assert.equal(before.ok, false);
  assert.match(JSON.stringify(before.diagnostics), /hash-invalid/u);

  const dryRun = await coordinationRepair({ stateRoot });
  assert.equal(dryRun.dryRun, true);
  assert.equal(dryRun.repairedStreams.length, 1);
  assert.equal(dryRun.repairedEvents, 1);
  assert.equal(fs.existsSync(dryRun.repairedStreams[0].backupPath ?? ""), false);

  const repaired = await coordinationRepair({ stateRoot, write: true });
  assert.equal(repaired.ok, true);
  assert.equal(repaired.dryRun, false);
  assert.equal(repaired.repairedStreams.length, 1);
  assert.equal(fs.existsSync(repaired.repairedStreams[0].backupPath), true);
  const after = await inspectLedger(stateRoot);
  assert.equal(after.ok, true);

  const [repairedFirst, repairedSecond] = fs
    .readFileSync(streamPath, "utf8")
    .trim()
    .split(/\r?\n/u)
    .map((line) => JSON.parse(line));
  assert.equal(repairedFirst.context.projectId, "repair-project");
  assert.equal(repairedFirst.hash, hashForEvent(removeHash(repairedFirst)));
  assert.notEqual(
    repairedFirst.hash,
    hashForEventWithExtensions(removeHash(repairedFirst)),
  );
  assert.equal(repairedSecond.prevHash, repairedFirst.hash);
  assert.equal(repairedSecond.hash, hashForEvent(removeHash(repairedSecond)));
});

test("coordination repair fixes sequence breaks without Parent wrappers", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-seq-repair-"));
  const streamsRoot = path.join(stateRoot, "streams");
  fs.mkdirSync(streamsRoot, { recursive: true });
  const streamPath = path.join(streamsRoot, "node_seq.codex-a.ndjson");
  const firstBase = {
    id: "evt_sequence000001",
    schema: 1,
    hub: "sequence-hub",
    nodeId: "node_seq",
    nodeName: "TestNode",
    lane: "codex-a",
    writer: "node_seq.codex-a",
    type: "claim",
    ts: "2026-06-30T00:00:00.000Z",
    seq: 1,
    prevEventId: null,
    prevHash: null,
    paths: ["src/lib.rs"],
  };
  const first = {
    ...firstBase,
    hash: hashForEvent(firstBase),
  };
  const secondBase = {
    id: "evt_sequence000002",
    schema: 1,
    hub: "sequence-hub",
    nodeId: "node_seq",
    nodeName: "TestNode",
    lane: "codex-a",
    writer: "node_seq.codex-a",
    type: "release",
    ts: "2026-06-30T00:00:01.000Z",
    seq: 4,
    prevEventId: first.id,
    prevHash: first.hash,
    paths: ["src/lib.rs"],
  };
  const second = {
    ...secondBase,
    hash: hashForEvent(secondBase),
  };
  fs.writeFileSync(
    streamPath,
    `${JSON.stringify(first)}\n${JSON.stringify(second)}\n`,
    "utf8",
  );

  const before = await inspectLedger(stateRoot);
  assert.equal(before.ok, false);
  assert.match(JSON.stringify(before.diagnostics), /sequence break/u);

  const dryRun = await coordinationRepair({ stateRoot, action: "sequence" });
  assert.equal(dryRun.dryRun, true);
  assert.equal(dryRun.sequenceRepairs, 1);
  assert.equal(dryRun.repairedStreams.length, 1);

  const repaired = await coordinationRepair({
    stateRoot,
    action: "sequence",
    write: true,
  });
  assert.equal(repaired.ok, true);
  assert.equal(repaired.sequenceRepairs, 1);
  assert.equal(fs.existsSync(repaired.repairedStreams[0].backupPath), true);
  const after = await inspectLedger(stateRoot);
  assert.equal(after.ok, true);

  const [repairedFirst, repairedSecond] = fs
    .readFileSync(streamPath, "utf8")
    .trim()
    .split(/\r?\n/u)
    .map((line) => JSON.parse(line));
  assert.equal(repairedFirst.seq, 1);
  assert.equal(repairedSecond.seq, 2);
  assert.equal(repairedSecond.prevHash, repairedFirst.hash);
  assert.equal(repairedSecond.hash, hashForEvent(removeHash(repairedSecond)));
});

test("coordination repair stale-claims resolves exact-path conflicts append-only", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-claim-repair-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-claim-repair-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "lib.rs"), "fn local() {}\n");
  await coordinationInit({ stateRoot, hub: "claim-repair-hub", lane: "codex-a" });
  const ownerClaim = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "owner claim",
  });
  const config = await loadIdentity(stateRoot);
  await appendEvent(stateRoot, config, "codex-b", {
    type: "claim",
    paths: ["src/lib.rs"],
    reason: "stale competing claim",
    context: {
      repoRoot: targetRoot,
      worktreeRoot: targetRoot,
      cwd: targetRoot,
      operation: "edit",
      lockKind: "writeLock",
    },
  });

  const before = await coordinationStatus({ stateRoot });
  assert.equal(before.state.ownership.conflicts.length, 1);

  const dryRun = await coordinationRepair({
    stateRoot,
    action: "stale-claims",
    paths: ["src/lib.rs"],
    owner: ownerClaim.event.writer,
  });
  assert.equal(dryRun.dryRun, true);
  assert.equal(dryRun.matchingConflictCount, 1);
  assert.equal(dryRun.matchingClaimCount, 2);
  assert.match(dryRun.suggestedCommands[1], /--write/u);

  const repaired = await coordinationRepair({
    stateRoot,
    action: "stale-claims",
    paths: ["src/lib.rs"],
    owner: ownerClaim.event.writer,
    lane: "codex-a",
    write: true,
  });
  assert.equal(repaired.ok, true);
  assert.equal(repaired.event.type, "claim.resolve");
  assert.equal(repaired.event.owner, ownerClaim.event.writer);
  assert.equal(repaired.resolvedConflictCount, 1);
  assert.equal(repaired.remainingConflictCount, 0);

  const after = await coordinationStatus({ stateRoot });
  assert.equal(after.state.ownership.conflicts.length, 0);
  assert.equal(after.state.ownership.activeClaims.length, 1);
  assert.equal(after.state.ownership.activeClaims[0].writer, ownerClaim.event.writer);
});

test("coordination repair stale-claims reports stream repair prerequisite", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-claim-prereq-"));
  const streamsRoot = path.join(stateRoot, "streams");
  fs.mkdirSync(streamsRoot, { recursive: true });
  const streamPath = path.join(streamsRoot, "node_bad.codex-a.ndjson");
  const event = {
    id: "evt_badclaimprereq",
    schema: 1,
    hub: "claim-prereq-hub",
    nodeId: "node_bad",
    nodeName: "TestNode",
    lane: "codex-a",
    writer: "node_bad.codex-a",
    type: "claim",
    ts: "2026-06-30T00:00:00.000Z",
    seq: 1,
    prevEventId: null,
    prevHash: null,
    paths: ["src/lib.rs"],
    hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
  };
  fs.writeFileSync(streamPath, `${JSON.stringify(event)}\n`, "utf8");

  const result = await coordinationRepair({
    stateRoot,
    action: "stale-claims",
    paths: ["src/lib.rs"],
  });
  assert.equal(result.ok, false);
  assert.match(result.error, /hash mismatch|hash/u);
  assert.match(result.nextStep, /coordination repair all/u);
});

test("coordination health reports stream repair prerequisite instead of throwing", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-health-prereq-"));
  const streamsRoot = path.join(stateRoot, "streams");
  fs.mkdirSync(streamsRoot, { recursive: true });
  const streamPath = path.join(streamsRoot, "node_bad.codex-a.ndjson");
  const event = {
    id: "evt_badhealthprereq",
    schema: 1,
    hub: "health-prereq-hub",
    nodeId: "node_bad",
    nodeName: "TestNode",
    lane: "codex-a",
    writer: "node_bad.codex-a",
    type: "claim",
    ts: "2026-06-30T00:00:00.000Z",
    seq: 1,
    prevEventId: null,
    prevHash: null,
    paths: ["src/lib.rs"],
    hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
  };
  fs.writeFileSync(streamPath, `${JSON.stringify(event)}\n`, "utf8");

  const result = await coordinationHealth({
    stateRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
  });
  assert.equal(result.ok, false);
  assert.equal(result.mustRepairLedger, true);
  assert.equal(result.canWriteClaimedPaths, false);
  assert.match(result.guard.error, /hash mismatch|hash/u);
  assert.match(result.nextStep, /coordination repair all/u);
});

test("coordination sync converges local roots and transfers HTTP suffixes only", async () => {
  const leftRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-sync-left-"));
  const rightRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-sync-right-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-sync-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"));
  fs.writeFileSync(path.join(targetRoot, "src", "lib.rs"), "fn local() {}\n");
  await coordinationInit({ stateRoot: leftRoot, hub: "sync-hub", lane: "codex-a" });
  await coordinationInit({ stateRoot: rightRoot, hub: "sync-hub", lane: "codex-b" });
  await coordinationClaim({
    stateRoot: leftRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "sync seed",
    projectId: "sync-project",
    codexThreadId: "thread-sync-a",
  });

  const localSync = await coordinationSync({ stateRoot: rightRoot, peer: leftRoot });
  assert.equal(localSync.ok, true);
  assert.equal(localSync.result.imported, 1);
  let rightPresence = await coordinationPresence({ stateRoot: rightRoot });
  assert.equal(rightPresence.views.byClaimedPath["src/lib.rs"][0].lane, "codex-a");

  await coordinationMessage({
    stateRoot: leftRoot,
    to: "codex-b",
    body: "suffix only",
    projectId: "sync-project",
    codexThreadId: "thread-sync-a",
  });
  const server = await startPeerServer(leftRoot, { host: "127.0.0.1", port: 0 });
  try {
    const httpSync = await coordinationSync({
      stateRoot: rightRoot,
      peer: server.url,
    });
    assert.equal(httpSync.ok, true);
    assert.equal(httpSync.result.imported, 1);
    assert.equal(httpSync.result.transferredLines, 1);
  } finally {
    await server.close();
  }
  const inbox = await coordinationInbox({ stateRoot: rightRoot, lane: "codex-b" });
  assert.equal(inbox.inbox.length, 1);
  assert.equal(inbox.inbox[0].body, "suffix only");

  const peer = await coordinationPeer({
    stateRoot: rightRoot,
    action: "add",
    name: "left",
    url: "http://127.0.0.1:8787",
    mode: "pull",
  });
  assert.equal(peer.registry.peers[0].mode, "pull");
  const peers = await coordinationPeer({ stateRoot: rightRoot, action: "list" });
  assert.equal(peers.registry.peers[0].name, "left");
});

test("coordination rejects folder, glob, duplicate, and overbroad claims", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-exact-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-exact-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"));
  fs.writeFileSync(path.join(targetRoot, "src", "lib.rs"), "fn local() {}\n");
  await coordinationInit({ stateRoot, hub: "exact-hub", lane: "codex-a" });

  await assert.rejects(
    () =>
      coordinationClaim({
        stateRoot,
        root: targetRoot,
        lane: "codex-a",
        paths: ["src"],
      }),
    /exact files/u,
  );
  await assert.rejects(
    () =>
      coordinationClaim({
        stateRoot,
        root: targetRoot,
        lane: "codex-a",
        paths: ["src/*.rs"],
      }),
    /exact files/u,
  );
  await assert.rejects(
    () =>
      coordinationClaim({
        stateRoot,
        root: targetRoot,
        lane: "codex-a",
        paths: ["src/lib.rs", "src/lib.rs"],
      }),
    /duplicate claim path/u,
  );
});

test("coordination CLI supports --hub without Parent repo wiring", () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-cli-"));
  const result = spawnSync(
    process.execPath,
    [CLI, "coordination", "root", "--hub", "portable-hub"],
    {
      cwd: PACK_ROOT,
      encoding: "utf8",
      env: { ...process.env, LEDGER_ROOT: stateRoot },
    },
  );
  assert.equal(result.status, 0, result.stderr);
  const parsed = JSON.parse(result.stdout);
  assert.equal(parsed.root, stateRoot);
});

test("coordination CLI supports state-root and public claim/release flags", () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-cli-state-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-cli-target-"));
  fs.mkdirSync(path.join(targetRoot, "docs"));
  fs.writeFileSync(path.join(targetRoot, "docs", "proof.md"), "# proof\n");

  const init = spawnSync(
    process.execPath,
    [CLI, "coordination", "init", "portable-hub", "--state-root", stateRoot, "--lane", "codex-a"],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(init.status, 0, init.stderr);

  const claim = spawnSync(
    process.execPath,
    [
      CLI,
      "coordination",
      "claim",
      "--state-root",
      stateRoot,
      "--hub",
      "portable-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--paths",
      "docs/proof.md",
      "--reason",
      "cli exact-file claim",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(claim.status, 0, claim.stderr);
  assert.equal(JSON.parse(claim.stdout).event.type, "claim");

  const health = spawnSync(
    process.execPath,
    [
      CLI,
      "coordination",
      "health",
      "--state-root",
      stateRoot,
      "--hub",
      "portable-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--paths",
      "docs/proof.md",
      "--limit",
      "3",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(health.status, 0, health.stderr);
  const healthReport = JSON.parse(health.stdout);
  assert.equal(healthReport.canInspect, true);
  assert.equal(healthReport.canWriteClaimedPaths, true);

  const guard = spawnSync(
    process.execPath,
    [
      CLI,
      "coordination",
      "guard",
      "--state-root",
      stateRoot,
      "--hub",
      "portable-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--paths",
      "docs/proof.md",
      "--limit",
      "3",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(guard.status, 0, guard.stderr);
  assert.equal(JSON.parse(guard.stdout).result.focused, true);

  const deniedGuard = spawnSync(
    process.execPath,
    [
      CLI,
      "coordination",
      "guard",
      "--state-root",
      stateRoot,
      "--hub",
      "portable-hub",
      "--lane",
      "codex-b",
      "--root",
      targetRoot,
      "--paths",
      "docs/proof.md",
      "--limit",
      "3",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.notEqual(deniedGuard.status, 0, deniedGuard.stdout || deniedGuard.stderr);
  const deniedGuardReport = JSON.parse(deniedGuard.stdout);
  assert.equal(deniedGuardReport.ok, false);
  assert.match(deniedGuardReport.result.findings.join("\n"), /write-lock-conflict/u);

  const release = spawnSync(
    process.execPath,
    [
      CLI,
      "coordination",
      "release",
      "--state-root",
      stateRoot,
      "--hub",
      "portable-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--paths",
      "docs/proof.md",
      "--reason",
      "cli exact-file release",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(release.status, 0, release.stderr);
  assert.equal(JSON.parse(release.stdout).event.type, "release");

  const presence = spawnSync(
    process.execPath,
    [CLI, "coordination", "presence", "--state-root", stateRoot, "--hub", "portable-hub"],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(presence.status, 0, presence.stderr);
  assert.deepEqual(JSON.parse(presence.stdout).views.byClaimedPath, {});

  const repair = spawnSync(
    process.execPath,
    [
      CLI,
      "coordination",
      "repair",
      "legacy-hash",
      "--state-root",
      stateRoot,
      "--hub",
      "portable-hub",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(repair.status, 0, repair.stderr);
  assert.equal(JSON.parse(repair.stdout).dryRun, true);

  const sequenceRepair = spawnSync(
    process.execPath,
    [
      CLI,
      "coordination",
      "repair",
      "sequence",
      "--state-root",
      stateRoot,
      "--hub",
      "portable-hub",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(sequenceRepair.status, 0, sequenceRepair.stderr);
  assert.equal(JSON.parse(sequenceRepair.stdout).dryRun, true);

  const staleClaimRepair = spawnSync(
    process.execPath,
    [
      CLI,
      "coordination",
      "repair",
      "stale-claims",
      "--state-root",
      stateRoot,
      "--hub",
      "portable-hub",
      "--paths",
      "docs/proof.md",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(staleClaimRepair.status, 0, staleClaimRepair.stderr);
  assert.equal(JSON.parse(staleClaimRepair.stdout).action, "stale-claims");
});

test("architecture CLI flags Rust public re-exports and skips clean files", () => {
  const project = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-arch-"));
  fs.mkdirSync(path.join(project, "src"));
  fs.writeFileSync(path.join(project, "src", "clean.rs"), "fn local() {}\n");
  fs.writeFileSync(
    path.join(project, "src", "bad.rs"),
    "pub use crate::inner::Thing;\n",
  );

  const clean = spawnSync(
    process.execPath,
    [
      CLI,
      "architecture",
      "check",
      "--language",
      "rust",
      "--scope",
      "files",
      "--files",
      "src/clean.rs",
      "--root",
      project,
      "--json",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(clean.status, 0, clean.stderr);

  const bad = spawnSync(
    process.execPath,
    [
      CLI,
      "architecture",
      "check",
      "--language",
      "rust",
      "--scope",
      "files",
      "--files",
      "src/bad.rs",
      "--root",
      project,
      "--json",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(bad.status, 1);
  assert.match(bad.stdout, /RR-7\.3/u);
});

function removeHash(event) {
  const { hash: _hash, ...withoutHash } = event;
  return withoutHash;
}
