import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { spawnCli } from "./cli-spawn.mjs";

import {
  coordinationAck,
  coordinationClaim,
  coordinationCompact,
  coordinationInbox,
  coordinationInit,
  coordinationMessage,
  coordinationPeer,
  coordinationPresence,
  coordinationStreams,
  coordinationSync,
  coordinationTaskUpdate,
  coordinationTasks,
  coordinationWorkerUpdate,
  coordinationWorkers,
} from "../src/coordination/api.mjs";
import { startPeerServer } from "../src/coordination/vendor/server.js";

const PACK_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const CLI = path.join(PACK_ROOT, "scripts", "rust-rules.mjs");

function spawnCompat(stateRoot, command, extraArgs = []) {
  const startedAt = performance.now();
  const result = spawnCli(
    process.execPath,
    [
      CLI,
      "coordination",
      command,
      "--state-root",
      stateRoot,
      "--hub",
      "compat-status",
      ...extraArgs,
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  return { result, elapsedMs: performance.now() - startedAt };
}

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
  const result = spawnCli(
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

  const init = spawnCli(
    process.execPath,
    [CLI, "coordination", "init", "portable-hub", "--state-root", stateRoot, "--lane", "codex-a"],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(init.status, 0, init.stderr);

  const claim = spawnCli(
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

  const health = spawnCli(
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

  const guard = spawnCli(
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

  const deniedGuard = spawnCli(
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

  const release = spawnCli(
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

  const claimForOwnedRelease = spawnCli(
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
      "cli release owned claim",
      "--codex-thread-id",
      "thread-cli-release-owned",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(claimForOwnedRelease.status, 0, claimForOwnedRelease.stderr);

  const releaseOwned = spawnCli(
    process.execPath,
    [
      CLI,
      "coordination",
      "release",
      "codex-a",
      "--state-root",
      stateRoot,
      "--hub",
      "portable-hub",
      "--cwd",
      targetRoot,
      "--codex-thread-id",
      "thread-cli-release-owned",
      "--reason",
      "cli release owned",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(releaseOwned.status, 0, releaseOwned.stderr);
  const releaseOwnedReport = JSON.parse(releaseOwned.stdout);
  assert.equal(releaseOwnedReport.event.type, "release");
  assert.equal(releaseOwnedReport.matchedClaimCount, 1);

  const claimForCloseout = spawnCli(
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
      "cli closeout claim",
      "--codex-thread-id",
      "thread-cli-closeout",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(claimForCloseout.status, 0, claimForCloseout.stderr);

  const closeout = spawnCli(
    process.execPath,
    [
      CLI,
      "coordination",
      "closeout",
      "--state-root",
      stateRoot,
      "--hub",
      "portable-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--thread-id",
      "thread-cli-closeout",
      "--reason",
      "cli closeout",
    ],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(closeout.status, 0, closeout.stderr);
  assert.equal(JSON.parse(closeout.stdout).remainingClaimCount, 0);

  const presence = spawnCli(
    process.execPath,
    [CLI, "coordination", "presence", "--state-root", stateRoot, "--hub", "portable-hub"],
    { cwd: targetRoot, encoding: "utf8" },
  );
  assert.equal(presence.status, 0, presence.stderr);
  assert.deepEqual(JSON.parse(presence.stdout).views.byClaimedPath, {});

  const repair = spawnCli(
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

  const sequenceRepair = spawnCli(
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

  const staleClaimRepair = spawnCli(
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

test("coordination CLI owns hub compatibility aliases without product repo wrappers", () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-compat-cli-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-compat-target-"));
  fs.writeFileSync(path.join(targetRoot, "README.md"), "compat\n");

  const init = spawnCli(
    process.execPath,
    [CLI, "coordination", "init", "compat-hub", "--state-root", stateRoot, "--hub", "compat-hub", "--lane", "codex-a"],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(init.status, 0, init.stderr);

  const claim = spawnCli(
    process.execPath,
    [
      CLI,
      "coordination",
      "hub:lock",
      "--state-root",
      stateRoot,
      "--hub",
      "compat-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--codex-thread-id",
      "compat-thread-a",
      "--paths",
      "README.md",
      "--reason",
      "compat smoke",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(claim.status, 0, claim.stderr);
  const claimResult = JSON.parse(claim.stdout);
  assert.equal(claimResult.event.type, "claim");
  assert.equal(claimResult.event.context.repoRoot, targetRoot);
  assert.equal(claimResult.event.context.worktreeRoot, targetRoot);
  assert.equal(claimResult.event.context.codexThreadId, "compat-thread-a");

  const siblingClaim = spawnCli(
    process.execPath,
    [
      CLI,
      "coordination",
      "hub:lock",
      "--state-root",
      stateRoot,
      "--hub",
      "compat-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--codex-thread-id",
      "compat-thread-b",
      "--paths",
      "README.md",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(siblingClaim.status, 1, siblingClaim.stderr);
  assert.equal(JSON.parse(siblingClaim.stdout).blockingOwners[0].codexThreadId, "compat-thread-a");

  const guard = spawnCli(
    process.execPath,
    [
      CLI,
      "coordination",
      "hub:guard",
      "--state-root",
      stateRoot,
      "--hub",
      "compat-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--codex-thread-id",
      "compat-thread-a",
      "--paths",
      "README.md",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(guard.status, 0, guard.stderr);
  assert.equal(JSON.parse(guard.stdout).result.ok, true);

  const siblingRelease = spawnCli(
    process.execPath,
    [
      CLI,
      "coordination",
      "hub:unlock",
      "--state-root",
      stateRoot,
      "--hub",
      "compat-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--codex-thread-id",
      "compat-thread-b",
      "--paths",
      "README.md",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(siblingRelease.status, 0, siblingRelease.stderr);

  const stillBlocked = spawnCli(
    process.execPath,
    [
      CLI,
      "coordination",
      "hub:lock",
      "--state-root",
      stateRoot,
      "--hub",
      "compat-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--codex-thread-id",
      "compat-thread-b",
      "--paths",
      "README.md",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(stillBlocked.status, 1, stillBlocked.stderr);

  const release = spawnCli(
    process.execPath,
    [
      CLI,
      "coordination",
      "hub:unlock",
      "--state-root",
      stateRoot,
      "--hub",
      "compat-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--codex-thread-id",
      "compat-thread-a",
      "--paths",
      "README.md",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(release.status, 0, release.stderr);
  assert.equal(JSON.parse(release.stdout).event.type, "release");

  const transferredClaim = spawnCli(
    process.execPath,
    [
      CLI,
      "coordination",
      "hub:lock",
      "--state-root",
      stateRoot,
      "--hub",
      "compat-hub",
      "--lane",
      "codex-a",
      "--root",
      targetRoot,
      "--codex-thread-id",
      "compat-thread-b",
      "--paths",
      "README.md",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(transferredClaim.status, 0, transferredClaim.stderr);
});

test("coordination compatibility help is read-only while bare ack keeps ack-latest compatibility", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-compat-help-"));
  await coordinationInit({ stateRoot, hub: "compat-help", lane: "codex-a" });
  await coordinationMessage({ stateRoot, hub: "compat-help", from: "primary", to: "codex-a", body: "leave unread" });
  const before = await coordinationStreams({ stateRoot });
  const beforeEventCount = before.streams.reduce((sum, stream) => sum + stream.eventCount, 0);

  for (const [command, flag] of [["hub:ack", "--help"], ["hub:lock", "--help"], ["hub:unlock", "-h"]]) {
    const result = spawnCli(
      process.execPath,
      [CLI, "coordination", command, "--state-root", stateRoot, "--hub", "compat-help", "--lane", "codex-a", flag],
      { cwd: PACK_ROOT, encoding: "utf8" },
    );
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /^Usage: ocentra-enforcer coordination/u);
  }

  for (const command of ["hub:lock", "hub:unlock"]) {
    const result = spawnCli(
      process.execPath,
      [CLI, "coordination", command, "--state-root", stateRoot, "--hub", "compat-help", "--lane", "codex-a"],
      { cwd: PACK_ROOT, encoding: "utf8" },
    );
    assert.notEqual(result.status, 0, `${command} unexpectedly succeeded: ${result.stdout}`);
  }

  const after = await coordinationStreams({ stateRoot });
  const afterEventCount = after.streams.reduce((sum, stream) => sum + stream.eventCount, 0);
  const unreadInbox = await coordinationInbox({ stateRoot, lane: "codex-a", all: true });
  assert.equal(afterEventCount, beforeEventCount);
  assert.deepEqual(unreadInbox.inbox[0].ackedBy, []);

  const bareAck = spawnCli(
    process.execPath,
    [CLI, "coordination", "hub:ack", "--state-root", stateRoot, "--hub", "compat-help", "--lane", "codex-a"],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(bareAck.status, 0, bareAck.stderr);
  const acknowledgedInbox = await coordinationInbox({ stateRoot, lane: "codex-a", all: true });
  assert.equal(acknowledgedInbox.inbox[0].ackedBy.length, 1);
  assert.match(acknowledgedInbox.inbox[0].ackedBy[0], /\.codex-a$/u);
});

test("coordination compatibility reads use the indexed live checkpoint", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-compat-status-"));
  await coordinationInit({ stateRoot, hub: "compat-status", lane: "codex-a" });
  let firstMessage;
  for (let index = 0; index < 12; index += 1) {
    const message = await coordinationMessage({
      stateRoot,
      to: "codex-a",
      body: `indexed status ${index}`,
    });
    firstMessage ??= message;
  }
  await coordinationAck({ stateRoot, lane: "codex-a", messageId: firstMessage.event.id });
  await coordinationWorkerUpdate({
    stateRoot,
    lane: "codex-a",
    state: "working",
    summary: "indexed worker",
  });
  await coordinationTaskUpdate({
    stateRoot,
    lane: "codex-a",
    taskId: "indexed-task",
    state: "started",
    summary: "indexed task",
  });
  const compacted = await coordinationCompact({ stateRoot, keepLatest: 1 });
  assert.equal(compacted.compactedStreams.length, 1);
  const archiveRoot = path.join(stateRoot, "archive", "streams");
  const streamDirectory = path.join(archiveRoot, fs.readdirSync(archiveRoot)[0]);
  const archiveFile = path.join(streamDirectory, fs.readdirSync(streamDirectory)[0]);
  fs.writeFileSync(archiveFile, "{broken archive reserved for explicit audit}\n");

  for (const command of ["hub:status", "lanes:status"]) {
    const { result, elapsedMs } = spawnCompat(stateRoot, command);
    assert.equal(result.status, 0, result.stderr);
    const report = JSON.parse(result.stdout);
    assert.deepEqual(Object.keys(report), [
      "ok",
      "diagnostics",
      "warnings",
      "conflicts",
      "dashboard",
    ]);
    assert.equal(report.ok, true);
    assert.deepEqual(report.diagnostics, []);
    assert.equal(report.dashboard.eventCount, 15);
    assert.ok(elapsedMs < 5_000, `indexed ${command} took ${elapsedMs.toFixed(1)}ms`);
  }

  const expectedWorkers = (await coordinationWorkers({ stateRoot })).workers;
  const expectedTasks = (await coordinationTasks({ stateRoot })).tasks;
  const expectedInbox = (await coordinationInbox({ stateRoot, lane: "codex-a" })).inbox;
  for (const [command, expected] of [
    ["ledger:workers", expectedWorkers],
    ["hub:heartbeats", expectedWorkers],
    ["ledger:tasks", expectedTasks],
    ["ledger:inbox", expectedInbox],
    ["hub:inbox", expectedInbox],
    ["hub:watch", expectedInbox],
  ]) {
    const { result, elapsedMs } = spawnCompat(stateRoot, command, ["--lane", "codex-a"]);
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(JSON.parse(result.stdout), expected);
    assert.ok(elapsedMs < 5_000, `indexed ${command} took ${elapsedMs.toFixed(1)}ms`);
  }
  const allInbox = spawnCompat(stateRoot, "hub:inbox", ["--lane", "codex-a", "--all"]);
  assert.equal(JSON.parse(allInbox.result.stdout).length, 12);

  const deepDoctor = spawnCompat(stateRoot, "ledger:doctor").result;
  assert.equal(deepDoctor.status, 0, deepDoctor.stderr);
  const deepReport = JSON.parse(deepDoctor.stdout);
  assert.equal(deepReport.ok, false);
  assert.ok(
    deepReport.diagnostics.some((diagnostic) =>
      /first event does not start a stream chain/u.test(diagnostic.message),
    ),
  );

  const streamRoot = path.join(stateRoot, "streams");
  const retainedEvent = fs.readFileSync(path.join(streamRoot, fs.readdirSync(streamRoot)[0]), "utf8");
  fs.writeFileSync(archiveFile, retainedEvent);
  fs.rmSync(path.join(stateRoot, "db", "coordination-index.json"));
  const fallbackReport = JSON.parse(spawnCompat(stateRoot, "hub:status").result.stdout);
  assert.equal(fallbackReport.ok, false);
  assert.ok(
    fallbackReport.diagnostics.some((diagnostic) =>
      /first event does not start a stream chain/u.test(diagnostic.message),
    ),
  );
});

test("indexed compatibility reads tolerate an incomplete live append", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-compat-tail-"));
  await coordinationInit({ stateRoot, hub: "compat-status", lane: "codex-a" });
  await coordinationMessage({ stateRoot, to: "codex-a", body: "indexed inbox" });
  await coordinationWorkerUpdate({
    stateRoot,
    lane: "codex-a",
    state: "working",
    summary: "indexed worker",
  });
  await coordinationTaskUpdate({
    stateRoot,
    lane: "codex-a",
    taskId: "indexed-task",
    state: "started",
    summary: "indexed task",
  });
  await coordinationCompact({ stateRoot, keepLatest: 1 });
  const expected = {
    "ledger:workers": (await coordinationWorkers({ stateRoot })).workers,
    "ledger:tasks": (await coordinationTasks({ stateRoot })).tasks,
    "hub:inbox": (await coordinationInbox({ stateRoot, lane: "codex-a" })).inbox,
  };
  const streamRoot = path.join(stateRoot, "streams");
  fs.appendFileSync(path.join(streamRoot, fs.readdirSync(streamRoot)[0]), "{\"partial\":");

  const status = spawnCompat(stateRoot, "hub:status").result;
  assert.equal(status.status, 0, status.stderr);
  const statusReport = JSON.parse(status.stdout);
  assert.equal(statusReport.ok, false);
  assert.ok(statusReport.warnings.some((warning) => /ignored malformed final line/u.test(warning)));
  for (const [command, value] of Object.entries(expected)) {
    const result = spawnCompat(stateRoot, command, ["--lane", "codex-a"]).result;
    assert.equal(result.status, 0, result.stderr);
    assert.deepEqual(JSON.parse(result.stdout), value);
  }
});

test("architecture CLI flags Rust public re-exports and skips clean files", () => {
  const project = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-arch-"));
  fs.mkdirSync(path.join(project, "src"));
  fs.writeFileSync(path.join(project, "src", "clean.rs"), "fn local() {}\n");
  fs.writeFileSync(
    path.join(project, "src", "bad.rs"),
    "pub use crate::inner::Thing;\n",
  );

  const clean = spawnCli(
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

  const bad = spawnCli(
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
