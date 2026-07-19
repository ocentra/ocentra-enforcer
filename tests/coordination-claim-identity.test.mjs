import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  coordinationClaim,
  coordinationIndex,
  coordinationInit,
  coordinationRelease,
  coordinationStatus,
} from "../src/coordination/api.mjs";
import { loadIdentity } from "../src/coordination/vendor/identity.js";
import { materialize, materializedToJson } from "../src/coordination/vendor/materialize.js";
import { applyReleaseEvent } from "../src/coordination/vendor/materialize-claim-identity.js";
import {
  executeClaimCommand,
  executeReleaseCommand,
} from "../src/coordination/vendor/server.js";
import { appendEvent } from "../src/coordination/vendor/stream.js";

const PACK_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const CLI = path.join(PACK_ROOT, "scripts", "rust-rules.mjs");
const MATERIALIZER_VERSION = 2;

test("same-lane sibling threads cannot claim the same exact path", async () => {
  const { stateRoot, targetRoot } = await initializedRoots("sibling-owner");
  const owner = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "thread-a",
  });
  assert.equal(owner.ok, true);

  const sibling = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "thread-b",
  });
  assert.equal(sibling.ok, false);
  assert.equal(sibling.blockers.length, 1);
  assert.equal(sibling.blockingOwners[0].codexThreadId, "thread-a");
  assert.equal((await coordinationStatus({ stateRoot })).state.ownership.activeClaims.length, 1);
});

test("explicit owner cannot reuse or release an unknown-owner claim", async () => {
  const { stateRoot, targetRoot } = await initializedRoots("unknown-owner");
  const legacy = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "unknown",
    codexSessionId: "unknown",
  });
  assert.equal(legacy.ok, true);

  const takeover = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "thread-explicit",
  });
  assert.equal(takeover.ok, false);

  const noOpRelease = await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "thread-explicit",
  });
  assert.deepEqual(noOpRelease.releasedPaths, []);
  assert.equal((await coordinationStatus({ stateRoot })).state.ownership.activeClaims.length, 1);

  const legacyRelease = await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "unknown",
    codexSessionId: "unknown",
  });
  assert.deepEqual(legacyRelease.releasedPaths, ["src/owned.ts"]);
  assert.equal((await coordinationStatus({ stateRoot })).state.ownership.activeClaims.length, 0);
});

test("scoped release preserves sibling paths in a legacy grouped claim", async () => {
  const { stateRoot, targetRoot } = await initializedRoots("grouped-release");
  fs.writeFileSync(path.join(targetRoot, "src", "sibling.ts"), "export const sibling = true;\n");
  const config = await loadIdentity(stateRoot);
  const grouped = await appendEvent(stateRoot, config, "codex-a", {
    type: "claim",
    paths: ["src/owned.ts", "src/sibling.ts"],
    context: claimContext(targetRoot, {
      branch: "feature/grouped",
      codexThreadId: "thread-grouped",
    }),
  });

  const release = await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/grouped",
    codexThreadId: "thread-grouped",
  });
  assert.deepEqual(release.releasedPaths, ["src/owned.ts"]);
  assert.deepEqual(release.event.paths, ["src/owned.ts"]);

  const remaining = (await coordinationStatus({ stateRoot })).state.ownership.activeClaims;
  assert.equal(remaining.length, 1);
  assert.equal(remaining[0].eventId, grouped.id);
  assert.deepEqual(remaining[0].paths, ["src/sibling.ts"]);
});

test("scoped release cannot cross an explicit project or branch identity", async () => {
  const { stateRoot, targetRoot } = await initializedRoots("scoped-release");
  const config = await loadIdentity(stateRoot);
  for (const [projectId, branch] of [
    ["project-a", "feature/one"],
    ["project-a", "feature/two"],
    ["project-b", "feature/one"],
  ]) {
    await appendEvent(stateRoot, config, "codex-a", {
      type: "claim",
      paths: ["src/owned.ts"],
      context: claimContext(targetRoot, {
        projectId,
        explicitProjectId: projectId,
        branch,
        codexThreadId: "thread-shared",
      }),
    });
  }

  const release = await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    projectId: "project-a",
    branch: "feature/one",
    codexThreadId: "thread-shared",
  });
  assert.equal(release.matchedClaimCount, 1);
  const remaining = (await coordinationStatus({ stateRoot })).state.ownership.activeClaims;
  assert.deepEqual(
    remaining.map((claim) => [claim.context.projectId, claim.context.branch]).sort(),
    [
      ["project-a", "feature/two"],
      ["project-b", "feature/one"],
    ],
  );
});

test("historical contextless releases retain writer-wide replay compatibility", async () => {
  const { stateRoot, targetRoot } = await initializedRoots("historical-release");
  const config = await loadIdentity(stateRoot);
  for (const codexThreadId of ["thread-a", "thread-b"]) {
    await appendEvent(stateRoot, config, "codex-a", {
      type: "claim",
      paths: ["src/owned.ts"],
      context: claimContext(targetRoot, { codexThreadId }),
    });
  }
  await appendEvent(stateRoot, config, "codex-a", {
    type: "release",
    paths: ["src/owned.ts"],
  });
  assert.equal((await coordinationStatus({ stateRoot })).state.ownership.activeClaims.length, 0);
});

test("multi-path release evaluates each active claim once", () => {
  const activeClaimCount = 128;
  const releasedClaimIndex = 64;
  const activeClaims = new Map();
  for (let index = 0; index < activeClaimCount; index += 1) {
    const claim = {
      writer: "node_release_perf.codex-a",
      lane: "codex-a",
      paths: [`src/claimed-${index}.ts`],
      eventId: `claim-${index}`,
      context: claimContext("C:/release-performance", {
        branch: "feature/release-performance",
        codexThreadId: "thread-release-performance",
      }),
    };
    activeClaims.set(claim.eventId, claim);
  }
  const releasePaths = Array.from(
    { length: activeClaimCount },
    (_, index) => `src/unclaimed-${index}.ts`,
  );
  releasePaths[releasePaths.length - 1] = `src/claimed-${releasedClaimIndex}.ts`;
  let overlapCallCount = 0;

  applyReleaseEvent(
    activeClaims,
    {
      writer: "node_release_perf.codex-a",
      lane: "codex-a",
      paths: releasePaths,
      eventId: "release-event",
      context: {
        ...claimContext("C:/release-performance", {
          branch: "feature/release-performance",
          codexThreadId: "thread-release-performance",
        }),
        explicitReleaseScope: true,
        releaseClaimEventIds: [`claim-${releasedClaimIndex}`],
      },
    },
    (activePaths, requestedPaths) => {
      overlapCallCount += 1;
      return activePaths.filter((activePath) => requestedPaths.includes(activePath));
    },
  );

  assert.equal(activeClaims.size, activeClaimCount - 1);
  assert.equal(activeClaims.has(`claim-${releasedClaimIndex}`), false);
  assert.equal(overlapCallCount, activeClaimCount);
});

test("same-lane sibling intents survive another owner's claim refresh", async () => {
  const { stateRoot, targetRoot } = await initializedRoots("sibling-intents");
  const base = {
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
  };
  assert.equal((await coordinationClaim({ ...base, codexThreadId: "thread-owner" })).ok, true);
  assert.equal((await coordinationClaim({
    ...base,
    codexThreadId: "thread-waiter-a",
    onConflict: "intent",
  })).intentQueued, true);
  assert.equal((await coordinationClaim({
    ...base,
    codexThreadId: "thread-waiter-b",
    onConflict: "intent",
  })).intentQueued, true);
  assert.equal((await coordinationClaim({ ...base, codexThreadId: "thread-owner" })).ok, true);

  const status = await coordinationStatus({ stateRoot });
  assert.equal(status.state.ownership.editIntents.length, 2);
  assert.deepEqual(
    status.state.ownership.editIntents.map((intent) => intent.context.codexThreadId).sort(),
    ["thread-waiter-a", "thread-waiter-b"],
  );
});

test("transport command boundaries cannot claim or release for a sibling owner", async () => {
  const { stateRoot, targetRoot } = await initializedRoots("transport-owner");
  const owner = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "transport-thread-a",
  });
  assert.equal(owner.ok, true);

  const rejected = await executeClaimCommand(stateRoot, {
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "transport-thread-b",
  });
  assert.equal(rejected.status, 409);
  assert.equal(rejected.result.ok, false);

  const noOpRelease = await executeReleaseCommand(stateRoot, {
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "transport-thread-b",
  });
  assert.equal(noOpRelease.status, 200);
  assert.deepEqual(noOpRelease.result.releasedPaths, []);
  assert.equal((await coordinationStatus({ stateRoot })).state.ownership.activeClaims.length, 1);
});

test("concurrent processes acquire one exact-path owner atomically", { timeout: 60_000 }, async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-atomic-process-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-atomic-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  await coordinationInit({ stateRoot, hub: "atomic-process", lane: "primary" });
  const iterations = 8;
  const races = [];
  for (let index = 0; index < iterations; index += 1) {
    const relativePath = `src/owned-${index}.ts`;
    fs.writeFileSync(path.join(targetRoot, relativePath), `export const owned${index} = true;\n`);
    races.push(Promise.all([
      spawnClaim({ stateRoot, targetRoot, relativePath, lane: `racer-a-${index}`, threadId: `thread-a-${index}` }),
      spawnClaim({ stateRoot, targetRoot, relativePath, lane: `racer-b-${index}`, threadId: `thread-b-${index}` }),
    ]));
  }

  const results = await Promise.all(races);
  const status = await coordinationStatus({ stateRoot });
  for (let index = 0; index < iterations; index += 1) {
    const pair = results[index];
    assert.deepEqual(pair.map((result) => result.code).sort(), [0, 1], pair.map((result) => result.stderr).join("\n"));
    assert.deepEqual(pair.map((result) => result.body.ok).sort(), [false, true]);
    const relativePath = `src/owned-${index}.ts`;
    const owners = status.state.ownership.activeClaims.filter((claim) => claim.paths.includes(relativePath));
    assert.equal(owners.length, 1, `${relativePath} has ${owners.length} active owners`);
  }
});

test("claim acquisition recovers a transaction lock left by a stopped local process", async () => {
  const { stateRoot, targetRoot } = await initializedRoots("stopped-owner-lock");
  const lockPath = writeStaleOwnershipLock(stateRoot);

  const claim = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "thread-after-restart",
  });
  assert.equal(claim.ok, true);
  assert.equal(fs.existsSync(lockPath), false);
});

test("competing stale-lock cleaners still produce one active owner", async () => {
  const { stateRoot, targetRoot } = await initializedRoots("stale-cleaner-race");
  writeStaleOwnershipLock(stateRoot);
  const request = {
    stateRoot,
    root: targetRoot,
    paths: ["src/owned.ts"],
    branch: "feature/shared",
  };
  const results = await Promise.all([
    coordinationClaim({ ...request, lane: "cleaner-a", codexThreadId: "cleaner-thread-a" }),
    coordinationClaim({ ...request, lane: "cleaner-b", codexThreadId: "cleaner-thread-b" }),
  ]);
  assert.deepEqual(results.map((result) => result.ok).sort(), [false, true]);
  assert.equal((await coordinationStatus({ stateRoot })).state.ownership.activeClaims.length, 1);
});

test("pre-version live index is rebuilt with branch-complete ownership after restart", async () => {
  const { stateRoot, targetRoot } = await initializedRoots("legacy-index");
  const config = await loadIdentity(stateRoot);
  const claimContext = {
    projectId: "claim-index-project",
    repoRoot: targetRoot,
    worktreeRoot: targetRoot,
    codexThreadId: "thread-shared",
    codexSessionId: "session-shared",
    operation: "edit",
    lockKind: "writeLock",
    onConflict: "fail",
  };
  await appendEvent(stateRoot, config, "codex-a", {
    type: "claim",
    paths: ["src/owned.ts"],
    context: { ...claimContext, branch: "feature/one" },
  });
  await appendEvent(stateRoot, config, "codex-a", {
    type: "claim",
    paths: ["src/owned.ts"],
    context: { ...claimContext, branch: "feature/two" },
  });
  const canonical = materializedToJson(await materialize(stateRoot));
  assert.equal(canonical.ownership.activeClaims.length, 2);

  await coordinationIndex({ stateRoot });
  const indexPath = path.join(stateRoot, "db", "coordination-index.json");
  const legacyIndex = JSON.parse(fs.readFileSync(indexPath, "utf8"));
  assert.equal(legacyIndex.materializerVersion, MATERIALIZER_VERSION);
  delete legacyIndex.materializerVersion;
  legacyIndex.state.ownership.activeClaims = legacyIndex.state.ownership.activeClaims.filter(
    (claim) => claim.context.branch === "feature/two",
  );
  fs.writeFileSync(indexPath, `${JSON.stringify(legacyIndex, null, 2)}\n`);

  const restarted = await coordinationStatus({ stateRoot });
  assert.equal(restarted.state.ownership.activeClaims.length, 2);
  assert.deepEqual(
    restarted.state.ownership.activeClaims.map((claim) => claim.context.branch).sort(),
    ["feature/one", "feature/two"],
  );

  await coordinationIndex({ stateRoot });
  const mismatchedIndex = JSON.parse(fs.readFileSync(indexPath, "utf8"));
  mismatchedIndex.materializerVersion = MATERIALIZER_VERSION - 1;
  mismatchedIndex.state.ownership.activeClaims = mismatchedIndex.state.ownership.activeClaims.filter(
    (claim) => claim.context.branch === "feature/two",
  );
  fs.writeFileSync(indexPath, `${JSON.stringify(mismatchedIndex, null, 2)}\n`);
  const mismatchedRestart = await coordinationStatus({ stateRoot });
  assert.deepEqual(
    mismatchedRestart.state.ownership.activeClaims.map((claim) => claim.context.branch).sort(),
    ["feature/one", "feature/two"],
  );
});

async function initializedRoots(label) {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), `enforcer-${label}-`));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), `enforcer-${label}-target-`));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: label, lane: "codex-a" });
  return { stateRoot, targetRoot };
}

function spawnClaim({ stateRoot, targetRoot, relativePath, lane, threadId }) {
  const args = [
    CLI,
    "coordination",
    "claim",
    "--state-root",
    stateRoot,
    "--root",
    targetRoot,
    "--lane",
    lane,
    "--paths",
    relativePath,
    "--project-id",
    "atomic-process-project",
    "--branch",
    "feature/shared",
    "--codex-thread-id",
    threadId,
    "--json",
  ];
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, args, {
      cwd: PACK_ROOT,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.on("error", reject);
    child.on("close", (code) => {
      try {
        resolve({ code, body: JSON.parse(stdout), stderr });
      } catch (error) {
        reject(new Error(`claim process emitted invalid JSON (${code}): ${stdout}\n${stderr}`, { cause: error }));
      }
    });
  });
}

function claimContext(targetRoot, overrides = {}) {
  return {
    projectId: "claim-identity-project",
    explicitProjectId: "claim-identity-project",
    repoRoot: targetRoot,
    worktreeRoot: targetRoot,
    branch: "feature/shared",
    codexThreadId: "thread-owner",
    operation: "edit",
    lockKind: "writeLock",
    ...overrides,
  };
}

function writeStaleOwnershipLock(stateRoot) {
  const lockPath = path.join(stateRoot, "streams", ".ownership.lock");
  fs.mkdirSync(lockPath, { recursive: true });
  fs.writeFileSync(path.join(lockPath, "stopped-owner.owner.json"), JSON.stringify({
    token: "stopped-owner",
    pid: 2_147_483_647,
    host: os.hostname(),
    acquiredAt: Date.now(),
  }));
  const staleAt = new Date(Date.now() - 120_000);
  fs.utimesSync(lockPath, staleAt, staleAt);
  return lockPath;
}
