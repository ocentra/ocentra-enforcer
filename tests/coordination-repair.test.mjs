import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  coordinationClaim,
  coordinationCloseout,
  coordinationHealth,
  coordinationIndex,
  coordinationInit,
  coordinationPresence,
  coordinationRelease,
  coordinationRepair,
  coordinationStatus,
  coordinationStreams,
} from "../src/coordination/api.mjs";
import { loadIdentity } from "../src/coordination/vendor/identity.js";
import { appendEvent } from "../src/coordination/vendor/stream.js";
import {
  hashForEvent,
  hashForEventWithExtensions,
} from "../src/coordination/vendor/events.js";
import { inspectLedger } from "../src/coordination/vendor/doctor.js";

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

test("coordination keeps same-lane claims separated by project and worktree", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-presence-split-"));
  const parentRoot = makeClaimTarget("parent");
  const enforcerRoot = makeClaimTarget("enforcer");
  await coordinationInit({ stateRoot, hub: "presence-split-hub", lane: "codex-a" });
  await claimSplitTarget(stateRoot, parentRoot, "parent-project", "thread-parent");
  await claimSplitTarget(stateRoot, enforcerRoot, "enforcer-project", "thread-enforcer");

  const claimedBefore = await coordinationStatus({ stateRoot });
  assert.equal(claimedBefore.state.ownership.activeClaims.length, 2);

  const parentHealth = await coordinationHealth({
    stateRoot,
    root: parentRoot,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    operation: "edit",
    projectId: "parent-project",
    codexThreadId: "thread-parent-reader",
  });
  assert.equal(parentHealth.canLockPaths, false);
  assert.match(parentHealth.guard.findings.join("\n"), /write-lock-conflict/u);

  await coordinationRelease({
    stateRoot,
    cwd: parentRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    codexThreadId: "thread-parent",
    reason: "release parent only",
  });

  const claimedAfter = await coordinationStatus({ stateRoot });
  assert.equal(claimedAfter.state.ownership.activeClaims.length, 1);
  assert.equal(claimedAfter.state.ownership.activeClaims[0].context.projectId, "enforcer-project");
  const presence = await coordinationPresence({ stateRoot });
  assert.equal(presence.views.byClaimedPath["src/lib.rs"].length, 1);
  assert.equal(presence.views.byClaimedPath["src/lib.rs"][0].projectId, "enforcer-project");

  const parentEdit = await coordinationHealth({
    stateRoot,
    root: parentRoot,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    operation: "edit",
    projectId: "parent-project",
    codexThreadId: "thread-parent-reader",
  });
  assert.equal(parentEdit.canLockPaths, true);
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
  assert.deepEqual(dryRun.repairedStreams[0].backupPaths, []);

  const repaired = await coordinationRepair({ stateRoot, write: true });
  assert.equal(repaired.ok, true);
  assert.equal(repaired.dryRun, false);
  assert.equal(repaired.repairedStreams.length, 1);
  assert.equal(fs.existsSync(repaired.repairedStreams[0].backupPaths[0]), true);
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

test("coordination repair fixes compacted context-hashed archive segments", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-archive-repair-"));
  const init = await coordinationInit({
    stateRoot,
    hub: "archive-repair-hub",
    lane: "codex-a",
  });
  const streamPath = path.join(
    stateRoot,
    "streams",
    `${init.nodeId}.codex-a.ndjson`,
  );
  fs.mkdirSync(path.dirname(streamPath), { recursive: true });
  const firstBase = {
    id: "evt_archivecontext1",
    schema: 1,
    hub: "archive-repair-hub",
    nodeId: init.nodeId,
    nodeName: init.nodeName,
    lane: "codex-a",
    writer: `${init.nodeId}.codex-a`,
    type: "message",
    ts: "2026-06-30T00:00:00.000Z",
    seq: 1,
    prevEventId: null,
    prevHash: null,
    to: "codex-b",
    body: "stale mcp message",
    context: {
      projectId: "archive-repair",
      codexThreadId: "thread-archive",
    },
  };
  const first = {
    ...firstBase,
    hash: hashForEventWithExtensions(firstBase),
  };
  const secondBase = {
    id: "evt_archiveclaim02",
    schema: 1,
    hub: "archive-repair-hub",
    nodeId: init.nodeId,
    nodeName: init.nodeName,
    lane: "codex-a",
    writer: `${init.nodeId}.codex-a`,
    type: "claim",
    ts: "2026-06-30T00:00:01.000Z",
    seq: 2,
    prevEventId: first.id,
    prevHash: first.hash,
    paths: ["src/lib.rs"],
    reason: "claim after stale mcp message",
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

  const archiveDir = path.join(
    stateRoot,
    "archive",
    "streams",
    path.basename(streamPath),
  );
  fs.mkdirSync(archiveDir, { recursive: true });
  fs.writeFileSync(
    path.join(archiveDir, "20260630T000000000Z.ndjson"),
    `${JSON.stringify(first)}\n`,
    "utf8",
  );
  fs.writeFileSync(streamPath, `${JSON.stringify(second)}\n`, "utf8");
  await assert.rejects(
    coordinationRelease({
      stateRoot,
      lane: "codex-a",
      paths: ["src/lib.rs"],
      reason: "should refuse before repair",
    }),
    /hash mismatch/u,
  );

  const repaired = await coordinationRepair({ stateRoot, write: true });
  assert.equal(repaired.ok, true);
  assert.equal(repaired.repairedEvents, 1);
  assert.equal(repaired.rehashedEvents, 2);
  const inspection = await inspectLedger(stateRoot);
  assert.equal(inspection.ok, true);

  const release = await coordinationRelease({
    stateRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "release after archive repair",
  });
  assert.equal(release.ok, true);
  assert.equal(release.event.type, "release");
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
  assert.equal(fs.existsSync(repaired.repairedStreams[0].backupPaths[0]), true);
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

test("coordination closeout releases lane claims and verifies zero active claims", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-closeout-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-closeout-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "lib.rs"), "fn local() {}\n");
  fs.writeFileSync(path.join(targetRoot, "src", "other.rs"), "fn other() {}\n");
  await coordinationInit({ stateRoot, hub: "closeout-hub", lane: "codex-a" });
  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs", "src/other.rs"],
    reason: "closeout claim",
    codexThreadId: "thread-closeout",
  });

  const result = await coordinationCloseout({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    codexThreadId: "thread-closeout",
    reason: "test closeout",
  });
  assert.equal(result.ok, true);
  assert.equal(result.initialClaimCount, 2);
  assert.equal(result.remainingClaimCount, 0);
  assert.equal(result.index.counts.activeClaims, 0);

  const presence = await coordinationPresence({ stateRoot });
  assert.deepEqual(presence.views.byClaimedPath, {});
});

test("coordination closeout stale repair removes only selected owners", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-closeout-stale-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-closeout-stale-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "lib.rs"), "fn local() {}\n");
  await coordinationInit({ stateRoot, hub: "closeout-stale-hub", lane: "codex-a" });
  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "owner a",
    codexThreadId: "thread-a",
  });
  const stale = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-b",
    paths: ["src/lib.rs"],
    reason: "owner b",
    codexThreadId: "thread-b",
    operation: "inspect",
  });
  assert.equal(stale.ok, true);

  const result = await coordinationCloseout({
    stateRoot,
    root: targetRoot,
    lane: "codex-b",
    codexThreadId: "thread-b",
    releaseOwned: false,
    reason: "stale-only closeout",
  });
  assert.equal(result.ok, true);
  assert.equal(result.releaseEvents.length, 0);
  assert.equal(result.staleRepairClaimCount, 1);
  assert.equal(result.remainingClaimCount, 0);

  const status = await coordinationStatus({ stateRoot });
  assert.equal(status.state.ownership.activeClaims.length, 1);
  assert.equal(status.state.ownership.activeClaims[0].lane, "codex-a");
});

test("coordination release without explicit paths releases lane-owned claims in scope", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-release-owned-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-release-owned-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "lib.rs"), "fn local() {}\n");
  fs.writeFileSync(path.join(targetRoot, "src", "other.rs"), "fn other() {}\n");
  await coordinationInit({ stateRoot, hub: "release-owned-hub", lane: "codex-a" });
  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/lib.rs", "src/other.rs"],
    reason: "release owned claim",
    codexThreadId: "thread-release-owned",
  });

  const result = await coordinationRelease({
    stateRoot,
    cwd: targetRoot,
    lane: "codex-a",
    codexThreadId: "thread-release-owned",
    reason: "release all owned",
  });
  assert.equal(result.ok, true);
  assert.equal(result.matchedClaimCount, 2);
  assert.deepEqual(
    [...result.releasedPaths].sort(),
    ["src/lib.rs", "src/other.rs"],
  );

  const presence = await coordinationPresence({ stateRoot });
  assert.deepEqual(presence.views.byClaimedPath, {});
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

function makeClaimTarget(name) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), `enforcer-${name}-target-`));
  fs.mkdirSync(path.join(root, "src"), { recursive: true });
  fs.writeFileSync(path.join(root, "src", "lib.rs"), "fn local() {}\n");
  return root;
}

async function claimSplitTarget(stateRoot, root, projectId, codexThreadId) {
  return coordinationClaim({
    stateRoot,
    root,
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: `${projectId} claim`,
    projectId,
    repoRoot: root,
    worktreeRoot: root,
    codexThreadId,
  });
}

function removeHash(event) {
  const { hash: _hash, ...withoutHash } = event;
  return withoutHash;
}
