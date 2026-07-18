import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { syncBuiltinESMExports } from "node:module";
import test from "node:test";

import {
  coordinationAck,
  coordinationClaim,
  coordinationCompact,
  coordinationGuard,
  coordinationHealth,
  coordinationInbox,
  coordinationIndex,
  coordinationInit,
  coordinationMessage,
  coordinationPresence,
  coordinationRelease,
  coordinationStatus,
} from "../src/coordination/api.mjs";
import { loadIdentity } from "../src/coordination/vendor/identity.js";
import { appendEvent } from "../src/coordination/vendor/stream.js";
import { inspectLedger } from "../src/coordination/vendor/doctor.js";
import { materialize, materializedToJson } from "../src/coordination/vendor/materialize.js";
import { streamPath } from "../src/coordination/vendor/paths.js";
import {
  assertCoordinationHashCompatibility,
  coordinationHashCompatibility,
  hashForEvent,
} from "../src/coordination/vendor/events.js";

const PACK_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const CLI = path.join(PACK_ROOT, "scripts", "rust-rules.mjs");
const TEST_CLI_MAX_BUFFER = 32 * 1024 * 1024;

test("coordination hash compatibility self-test excludes extension context", () => {
  const compatibility = coordinationHashCompatibility();
  assert.equal(compatibility.ok, true);
  assert.equal(compatibility.contextExcludedFromWireHash, true);
  assert.notEqual(compatibility.actualWireHash, compatibility.extensionHash);
  assert.doesNotThrow(() => assertCoordinationHashCompatibility());
});

test("compacted archive uses an indexed checkpoint for hot health and claims", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-hot-read-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-hot-read-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "live.rs"), "fn live() {}\n");
  await coordinationInit({ stateRoot, hub: "hot-read-hub", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  for (let index = 0; index < 12; index += 1) {
    await appendEvent(stateRoot, config, "codex-a", {
      type: "report",
      summary: `archived history ${index}`,
    });
  }
  const compacted = await coordinationCompact({ stateRoot, keepLatest: 1 });
  assert.equal(compacted.compactedStreams.length, 1);
  const archiveDir = path.join(stateRoot, "archive", "streams");
  const archivedStream = fs.readdirSync(archiveDir)[0];
  const archiveFile = path.join(archiveDir, archivedStream, fs.readdirSync(path.join(archiveDir, archivedStream))[0]);
  fs.writeFileSync(archiveFile, "{broken archive for explicit audit}\n");

  const originalReadFile = fs.promises.readFile;
  let archiveReads = 0;
  fs.promises.readFile = async (file, ...args) => {
    if (path.resolve(String(file)).startsWith(`${path.resolve(archiveDir)}${path.sep}`)) archiveReads += 1;
    return originalReadFile(file, ...args);
  };
  syncBuiltinESMExports();
  try {
    const health = await coordinationHealth({ stateRoot, lane: "codex-a" });
    assert.equal(health.ok, true);
    assert.equal(health.dashboard.eventCount, 12);

    const owner = await coordinationClaim({
      stateRoot,
      root: targetRoot,
      lane: "codex-hot-owner",
      paths: ["src/live.rs"],
      reason: "hot owner claim",
    });
    assert.equal(owner.ok, true);
    const blocked = await coordinationClaim({
      stateRoot,
      root: targetRoot,
      lane: "codex-hot-intent",
      paths: ["src/live.rs"],
      reason: "hot queued edit",
      onConflict: "intent",
    });
    assert.equal(blocked.ok, false);
    assert.equal(blocked.intentQueued, true);
  } finally {
    fs.promises.readFile = originalReadFile;
    syncBuiltinESMExports();
  }
  assert.equal(archiveReads, 0);

  const audit = await inspectLedger(stateRoot);
  assert.equal(audit.ok, false);
  assert.ok(audit.diagnostics.some((diagnostic) => /ignored malformed final line/u.test(diagnostic.message)));
  assert.ok(audit.diagnostics.some((diagnostic) => /first event does not start a stream chain/u.test(diagnostic.message)));
});

test("indexed hot state falls back when a same-size checkpoint stream tail changes", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-checkpoint-tail-"));
  await coordinationInit({ stateRoot, hub: "checkpoint-tail", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  await appendEvent(stateRoot, config, "codex-a", { type: "report", summary: "checkpoint-before" });
  await coordinationIndex({ stateRoot });
  const stream = streamPath(stateRoot, config.nodeId, "codex-a");
  const original = fs.readFileSync(stream, "utf8");
  const event = JSON.parse(original);
  const replacement = { ...event, summary: "checkpoint-after " };
  replacement.hash = hashForEvent(replacement);
  const rewritten = `${JSON.stringify(replacement)}\n`;
  assert.equal(Buffer.byteLength(rewritten), Buffer.byteLength(original));
  fs.writeFileSync(stream, rewritten);

  const hot = await coordinationStatus({ stateRoot });
  assert.equal(hot.state.workers[event.writer].summary, "checkpoint-after ");
});

test("indexed hot state falls back when a non-tail context rewrite preserves wire hashes", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-checkpoint-prefix-"));
  await coordinationInit({ stateRoot, hub: "checkpoint-prefix", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  await appendEvent(stateRoot, config, "codex-a", {
    type: "claim",
    paths: ["src/prefix.rs"],
    reason: "prefix claim",
    context: { branch: "before" },
  });
  await appendEvent(stateRoot, config, "codex-a", { type: "report", summary: "checkpoint tail" });
  await coordinationIndex({ stateRoot });
  const stream = streamPath(stateRoot, config.nodeId, "codex-a");
  const original = fs.readFileSync(stream, "utf8");
  const events = original.trim().split("\n").map((line) => JSON.parse(line));
  const originalHash = events[0].hash;
  events[0].context.branch = "after_";
  const rewritten = `${events.map((event) => JSON.stringify(event)).join("\n")}\n`;
  assert.equal(events[0].hash, originalHash);
  assert.equal(Buffer.byteLength(rewritten), Buffer.byteLength(original));
  fs.writeFileSync(stream, rewritten);

  const hot = await coordinationStatus({ stateRoot });
  const canonical = materializedToJson(await materialize(stateRoot));
  assert.deepEqual(withoutGeneratedAt(hot.state), withoutGeneratedAt(canonical));
  assert.equal(hot.state.ownership.activeClaims[0].context.branch, "after_");
});

test("index rebuild retries when a same-size non-tail rewrite changes the checkpoint prefix", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-checkpoint-race-"));
  await coordinationInit({ stateRoot, hub: "checkpoint-race", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  await appendEvent(stateRoot, config, "codex-a", {
    type: "claim",
    paths: ["src/race.rs"],
    reason: "race claim",
    context: { branch: "before" },
  });
  await appendEvent(stateRoot, config, "codex-a", { type: "report", summary: "checkpoint tail" });
  const stream = streamPath(stateRoot, config.nodeId, "codex-a");
  const originalOpen = fs.promises.open;
  let streamOpens = 0;
  fs.promises.open = async (file, ...args) => {
    if (path.resolve(String(file)) === path.resolve(stream)) {
      streamOpens += 1;
      if (streamOpens === 2) {
        const events = fs.readFileSync(stream, "utf8").trim().split("\n").map((line) => JSON.parse(line));
        events[0].context.branch = "after_";
        fs.writeFileSync(stream, `${events.map((event) => JSON.stringify(event)).join("\n")}\n`);
      }
    }
    return originalOpen(file, ...args);
  };
  syncBuiltinESMExports();
  try {
    await coordinationIndex({ stateRoot });
  } finally {
    fs.promises.open = originalOpen;
    syncBuiltinESMExports();
  }
  const hot = await coordinationStatus({ stateRoot });
  assert.ok(streamOpens >= 4);
  assert.equal(hot.state.ownership.activeClaims[0].context.branch, "after_");
});

test("indexed hot state falls back when a delta predates the checkpoint order cursor", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-order-cursor-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-order-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"));
  fs.writeFileSync(path.join(targetRoot, "src", "order.rs"), "fn order() {}\n");
  await coordinationInit({ stateRoot, hub: "order-cursor", lane: "codex-a" });
  await coordinationClaim({ stateRoot, root: targetRoot, lane: "codex-a", paths: ["src/order.rs"], reason: "initial claim" });
  await coordinationRelease({ stateRoot, root: targetRoot, lane: "codex-a", paths: ["src/order.rs"], reason: "future release" });
  const config = await loadIdentity(stateRoot);
  const stream = streamPath(stateRoot, config.nodeId, "codex-a");
  const events = fs.readFileSync(stream, "utf8").trim().split("\n").map((line) => JSON.parse(line));
  events.at(-1).ts = "2999-01-01T00:00:00.000Z";
  events.at(-1).hash = hashForEvent(events.at(-1));
  fs.writeFileSync(stream, `${events.map((event) => JSON.stringify(event)).join("\n")}\n`);
  await coordinationIndex({ stateRoot });
  await coordinationClaim({ stateRoot, root: targetRoot, lane: "codex-a", paths: ["src/order.rs"], reason: "normal claim" });

  const hot = await coordinationStatus({ stateRoot });
  const canonical = materializedToJson(await materialize(stateRoot));
  assert.deepEqual(withoutGeneratedAt(hot.state), withoutGeneratedAt(canonical));
  assert.equal(hot.state.ownership.activeClaims.length, 0);
});

test("indexed hot state deduplicates post-checkpoint events and matches representative deltas", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-delta-parity-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-delta-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"));
  fs.writeFileSync(path.join(targetRoot, "src", "delta.rs"), "fn delta() {}\n");
  await coordinationInit({ stateRoot, hub: "delta-parity", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  await appendEvent(stateRoot, config, "codex-a", { type: "report", summary: "checkpoint" });
  await coordinationIndex({ stateRoot });
  const message = await coordinationMessage({ stateRoot, from: "codex-a", to: "codex-b", body: "delta message" });
  await coordinationAck({ stateRoot, lane: "codex-b", messageId: message.event.id });
  await appendEvent(stateRoot, config, "codex-b", { type: "task.update", taskId: "delta-task", taskState: "started", summary: "delta task" });
  await coordinationClaim({ stateRoot, root: targetRoot, lane: "codex-a", paths: ["src/delta.rs"], reason: "delta claim" });
  fs.writeFileSync(
    path.join(stateRoot, "streams", "duplicate-message.ndjson"),
    `${JSON.stringify(message.event)}\n`,
  );

  const hot = await coordinationStatus({ stateRoot });
  const canonical = materializedToJson(await materialize(stateRoot));
  assert.deepEqual(withoutGeneratedAt(hot.state), withoutGeneratedAt(canonical));
  assert.equal(hot.state.dashboard.duplicateCount, 1);
  assert.equal(hot.state.lanes["codex-b"].inbox.length, 1);
  assert.equal(hot.state.lanes["codex-b"].inbox[0].ackedBy.length, 1);
  assert.equal(hot.state.tasks["delta-task"].active, true);
  assert.equal(hot.state.ownership.activeClaims.length, 1);
});

test("indexed hot state refreshes TTL-derived heartbeat and session state without deltas", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-ttl-parity-"));
  await coordinationInit({ stateRoot, hub: "ttl-parity", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  await appendEvent(stateRoot, config, "codex-a", { type: "heartbeat", state: "online", summary: "short heartbeat", ttlSeconds: 1 });
  await appendEvent(stateRoot, config, "codex-a", { type: "session.claim", sessionId: "short-session", ttlSeconds: 1 });
  const stream = streamPath(stateRoot, config.nodeId, "codex-a");
  const events = fs.readFileSync(stream, "utf8").trim().split("\n").map((line) => JSON.parse(line));
  events[0].ts = "2000-01-01T00:00:00.000Z";
  events[0].hash = hashForEvent(events[0]);
  events[1].ts = "2000-01-01T00:00:00.000Z";
  events[1].prevHash = events[0].hash;
  events[1].hash = hashForEvent(events[1]);
  fs.writeFileSync(stream, `${events.map((event) => JSON.stringify(event)).join("\n")}\n`);
  await coordinationIndex({ stateRoot });
  const indexPath = path.join(stateRoot, "db", "coordination-index.json");
  const index = JSON.parse(fs.readFileSync(indexPath, "utf8"));
  index.state.lanes["codex-a"].heartbeat.stale = false;
  index.state.workers[`${config.nodeId}.codex-a`].heartbeat.stale = false;
  index.state.workers[`${config.nodeId}.codex-a`].state = "idle";
  index.state.workers[`${config.nodeId}.codex-a`].free = true;
  index.state.sessions["codex-a"] = {
    lane: "codex-a",
    writer: `${config.nodeId}.codex-a`,
    sessionId: "short-session",
    claimedAt: events[1].ts,
    ttlSeconds: 1,
    expiresAt: "2000-01-01T00:00:01.000Z",
    stale: false,
    eventId: events[1].id,
  };
  fs.writeFileSync(indexPath, `${JSON.stringify(index, null, 2)}\n`);

  const hot = await coordinationStatus({ stateRoot });
  const canonical = materializedToJson(await materialize(stateRoot));
  assert.deepEqual(withoutGeneratedAt(hot.state), withoutGeneratedAt(canonical));
  const worker = hot.state.workers[`${config.nodeId}.codex-a`];
  assert.equal(hot.state.lanes["codex-a"].heartbeat.stale, true);
  assert.equal(worker.state, "offline");
  assert.equal(worker.free, false);
  assert.deepEqual(hot.state.sessions, {});
});

test("transient live-stream ENOENT falls back and index rebuild retries", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-live-race-"));
  await coordinationInit({ stateRoot, hub: "live-race", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  await appendEvent(stateRoot, config, "codex-a", { type: "report", summary: "race checkpoint" });
  await coordinationIndex({ stateRoot });
  const stream = streamPath(stateRoot, config.nodeId, "codex-a");
  const originalOpen = fs.promises.open;
  let failures = 2;
  fs.promises.open = async (file, ...args) => {
    if (failures > 0 && path.resolve(String(file)) === path.resolve(stream)) {
      failures -= 1;
      const error = new Error("stream rotated");
      error.code = "ENOENT";
      throw error;
    }
    return originalOpen(file, ...args);
  };
  syncBuiltinESMExports();
  try {
    const hot = await coordinationStatus({ stateRoot });
    const canonical = materializedToJson(await materialize(stateRoot));
    assert.deepEqual(withoutGeneratedAt(hot.state), withoutGeneratedAt(canonical));
    const rebuilt = await coordinationIndex({ stateRoot });
    assert.equal(rebuilt.ok, true);
  } finally {
    fs.promises.open = originalOpen;
    syncBuiltinESMExports();
  }
});

function withoutGeneratedAt(state) {
  const copy = structuredClone(state);
  delete copy.dashboard.generatedAt;
  return copy;
}

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
    from: "codex-a",
    to: "codex-b",
    subject: "Assignment",
    body: "Do the generic coordination slice.",
  });

  const inbox = await coordinationInbox({ stateRoot, lane: "codex-b" });
  assert.equal(inbox.ok, true);
  assert.equal(inbox.inbox.length, 1);
  assert.match(inbox.inbox[0].from, /\.codex-a$/u);
  assert.match(inbox.inbox[0].body, /Assignment/u);
  assert.match(inbox.inbox[0].body, /generic coordination/u);
});

test("coordination CLI message alias supports flag and positional shapes", () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-mail-cli-"));
  const init = spawnSync(process.execPath, [
    CLI,
    "coordination",
    "init",
    "cli-mail",
    "--state-root",
    stateRoot,
    "--lane",
    "primary",
  ], { encoding: "utf8", maxBuffer: TEST_CLI_MAX_BUFFER });
  assert.equal(init.status, 0, init.stderr);

  const flagged = spawnSync(process.execPath, [
    CLI,
    "coordination",
    "message",
    "--state-root",
    stateRoot,
    "--from",
    "codex-a",
    "--to",
    "codex-b",
    "--subject",
    "Flagged Subject",
    "--body",
    "Flagged body.",
    "--json",
  ], { encoding: "utf8" });
  assert.equal(flagged.status, 0, flagged.stderr);
  const flaggedEvent = JSON.parse(flagged.stdout).event;
  assert.equal(flaggedEvent.lane, "codex-a");
  assert.equal(flaggedEvent.to, "codex-b");
  assert.equal(flaggedEvent.body, "Flagged Subject\n\nFlagged body.");

  const positional = spawnSync(process.execPath, [
    CLI,
    "coordination",
    "msg",
    "--state-root",
    stateRoot,
    "codex-b",
    "Positional body.",
  ], { encoding: "utf8" });
  assert.equal(positional.status, 0, positional.stderr);

  const inbox = spawnSync(process.execPath, [
    CLI,
    "coordination",
    "inbox",
    "--state-root",
    stateRoot,
    "codex-b",
  ], { encoding: "utf8" });
  assert.equal(inbox.status, 0, inbox.stderr);
  const messages = JSON.parse(inbox.stdout);
  assert.equal(messages.length, 2);
  assert.deepEqual(
    messages.map((message) => message.body),
    ["Flagged Subject\n\nFlagged body.", "Positional body."],
  );
});
