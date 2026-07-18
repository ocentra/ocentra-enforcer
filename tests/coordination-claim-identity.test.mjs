import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  coordinationClaim,
  coordinationCloseout,
  coordinationGuard,
  coordinationInbox,
  coordinationInit,
  coordinationRelease,
  coordinationStatus,
} from "../src/coordination/api.mjs";
import { buildCoordinationContext } from "../src/coordination/vendor/context.js";
import { loadIdentity } from "../src/coordination/vendor/identity.js";
import { streamPath } from "../src/coordination/vendor/paths.js";
import {
  executeClaimCommand,
  executeReleaseCommand,
} from "../src/coordination/vendor/server.js";
import { appendEvent } from "../src/coordination/vendor/stream.js";

test("same-lane sibling threads cannot claim the same exact path", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-sibling-owner-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-sibling-owner-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "sibling-owner-hub", lane: "codex-a" });

  const owner = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "thread a owner",
    branch: "feature/shared",
    codexThreadId: "thread-a",
    codexSessionId: "shared-session",
  });
  assert.equal(owner.ok, true);

  const sibling = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "thread b overlap",
    branch: "feature/shared",
    codexThreadId: "thread-b",
    codexSessionId: "shared-session",
  });
  assert.equal(sibling.ok, false);
  assert.equal(sibling.blockers.length, 1);
  assert.equal(sibling.blockers[0].type, "write-lock-conflict");
  assert.equal(sibling.blockingOwners[0].codexThreadId, "thread-a");

  const status = await coordinationStatus({ stateRoot });
  assert.equal(status.state.ownership.activeClaims.length, 1);
  assert.equal(status.state.ownership.conflicts.length, 0);
});

test("unknown ownership requires an explicit legacy release before thread upgrade", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-owner-upgrade-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-owner-upgrade-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "owner-upgrade-hub", lane: "codex-a" });

  const legacy = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "legacy unattributed owner",
    branch: "feature/shared",
    codexThreadId: "unknown",
    codexSessionId: "unknown",
  });
  assert.equal(legacy.ok, true);

  const unsafeUpgrade = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "explicit owner must not steal legacy claim",
    branch: "feature/shared",
    codexThreadId: "thread-explicit",
  });
  assert.equal(unsafeUpgrade.ok, false);
  assert.equal(unsafeUpgrade.blockers[0].type, "write-lock-conflict");

  await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "release legacy owner deliberately",
    branch: "feature/shared",
    codexThreadId: "unknown",
    codexSessionId: "unknown",
  });

  const upgraded = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "explicit owner after legacy release",
    branch: "feature/shared",
    codexThreadId: "thread-explicit",
  });
  assert.equal(upgraded.ok, true);
  const status = await coordinationStatus({ stateRoot });
  assert.equal(status.state.ownership.activeClaims.length, 1);
  assert.equal(status.state.ownership.activeClaims[0].context.codexThreadId, "thread-explicit");
});

test("project provenance remains stable through context normalization", () => {
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-project-context-"));
  const derived = buildCoordinationContext({
    cwd: targetRoot,
    repoRoot: targetRoot,
    worktreeRoot: targetRoot,
    gitRemote: null,
  });
  assert.equal(derived.explicitProjectId, null);
  assert.equal(buildCoordinationContext(derived).explicitProjectId, null);

  const declared = buildCoordinationContext({
    cwd: targetRoot,
    repoRoot: targetRoot,
    worktreeRoot: targetRoot,
    projectId: "declared-project",
  });
  assert.equal(declared.explicitProjectId, "declared-project");
  assert.equal(buildCoordinationContext(declared).explicitProjectId, "declared-project");
});

test("environment project mismatch cannot reuse a claim after normalization", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-env-project-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-env-project-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "env-project-hub", lane: "codex-a" });
  const previousProjectId = process.env.OCENTRA_PROJECT_ID;
  try {
    process.env.OCENTRA_PROJECT_ID = "project-a";
    await coordinationClaim({
      stateRoot,
      root: targetRoot,
      lane: "codex-a",
      paths: ["src/owned.ts"],
      codexThreadId: "thread-shared",
    });
    process.env.OCENTRA_PROJECT_ID = "project-b";
    const guard = await coordinationGuard({
      stateRoot,
      root: targetRoot,
      lane: "codex-a",
      paths: ["src/owned.ts"],
      operation: "commit",
      codexThreadId: "thread-shared",
    });
    assert.equal(guard.ok, false);
    assert.match(guard.result.findings.join("\n"), /outside active ledger claims/u);
  } finally {
    if (previousProjectId === undefined) delete process.env.OCENTRA_PROJECT_ID;
    else process.env.OCENTRA_PROJECT_ID = previousProjectId;
  }
});

test("derived project claims retain scoped release compatibility", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-derived-project-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-derived-project-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "derived-project-hub", lane: "codex-a" });
  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    codexThreadId: "thread-derived",
  });
  const before = await coordinationStatus({ stateRoot });
  assert.equal(before.state.ownership.activeClaims[0].context.explicitProjectId, null);
  await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    codexThreadId: "thread-derived",
  });
  assert.equal((await coordinationStatus({ stateRoot })).state.ownership.activeClaims.length, 0);
});

test("a scoped API release can clear a contextless legacy claim", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-contextless-release-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-contextless-release-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "contextless-release-hub", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  await appendEvent(stateRoot, config, "codex-a", {
    type: "claim",
    paths: ["src/owned.ts"],
    reason: "pre-context legacy claim",
  });
  const legacyStream = streamPath(stateRoot, config.nodeId, "codex-a");
  const legacyEvent = JSON.parse(fs.readFileSync(legacyStream, "utf8"));
  delete legacyEvent.context;
  fs.writeFileSync(legacyStream, `${JSON.stringify(legacyEvent)}\n`);
  const before = await coordinationStatus({ stateRoot });
  assert.equal(Object.hasOwn(before.state.ownership.activeClaims[0], "context"), false);

  const release = await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    codexThreadId: "thread-upgraded",
  });
  assert.equal(release.matchedClaimCount, 1);
  assert.deepEqual(release.releasedPaths, ["src/owned.ts"]);
  assert.equal((await coordinationStatus({ stateRoot })).state.ownership.activeClaims.length, 0);
});

test("scoped legacy release preserves an explicit owner in a preexisting duplicate", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-legacy-release-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-legacy-release-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "legacy-release-hub", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  const commonContext = {
    projectId: "same-project",
    repoRoot: targetRoot,
    worktreeRoot: targetRoot,
    cwd: targetRoot,
    branch: "feature/shared",
    operation: "edit",
    lockKind: "writeLock",
  };
  await appendEvent(stateRoot, config, "codex-a", {
    type: "claim",
    paths: ["src/owned.ts"],
    reason: "legacy unknown claim",
    context: {
      ...commonContext,
      codexThreadId: "unknown",
      codexSessionId: "unknown",
    },
  });
  await appendEvent(stateRoot, config, "codex-a", {
    type: "claim",
    paths: ["src/owned.ts"],
    reason: "preexisting explicit duplicate",
    context: {
      ...commonContext,
      codexThreadId: "thread-explicit",
      codexSessionId: "unknown",
    },
  });

  const before = await coordinationStatus({ stateRoot });
  assert.equal(before.state.ownership.activeClaims.length, 2);
  assert.equal(before.state.ownership.conflicts.length, 1);

  await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "repair only the legacy owner",
    projectId: "same-project",
    repoRoot: targetRoot,
    worktreeRoot: targetRoot,
    branch: "feature/shared",
    codexThreadId: "unknown",
    codexSessionId: "unknown",
  });
  const afterLegacyRelease = await coordinationStatus({ stateRoot });
  assert.equal(afterLegacyRelease.state.ownership.activeClaims.length, 1);
  assert.equal(afterLegacyRelease.state.ownership.conflicts.length, 0);
  assert.equal(
    afterLegacyRelease.state.ownership.activeClaims[0].context.codexThreadId,
    "thread-explicit",
  );

  await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "release explicit owner",
    projectId: "same-project",
    repoRoot: targetRoot,
    worktreeRoot: targetRoot,
    branch: "feature/shared",
    codexThreadId: "thread-explicit",
  });
  const afterExplicitRelease = await coordinationStatus({ stateRoot });
  assert.equal(afterExplicitRelease.state.ownership.activeClaims.length, 0);
});

test("historical unscoped release events retain writer-wide replay semantics", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-historical-release-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-historical-release-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "historical-release-hub", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  for (const codexThreadId of ["thread-a", "thread-b"]) {
    await appendEvent(stateRoot, config, "codex-a", {
      type: "claim",
      paths: ["src/owned.ts"],
      context: {
        projectId: "same-project",
        repoRoot: targetRoot,
        worktreeRoot: targetRoot,
        branch: "feature/shared",
        codexThreadId,
        operation: "edit",
        lockKind: "writeLock",
      },
    });
  }
  await appendEvent(stateRoot, config, "codex-a", {
    type: "release",
    paths: ["src/owned.ts"],
  });
  const after = await coordinationStatus({ stateRoot });
  assert.equal(after.state.ownership.activeClaims.length, 0);
});

test("scoped release never crosses an explicit owner's branch", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-branch-release-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-branch-release-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "branch-release-hub", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  for (const branch of ["feature/one", "feature/two"]) {
    await appendEvent(stateRoot, config, "codex-a", {
      type: "claim",
      paths: ["src/owned.ts"],
      reason: `${branch} preexisting claim`,
      context: {
        projectId: "same-project",
        repoRoot: targetRoot,
        worktreeRoot: targetRoot,
        cwd: targetRoot,
        branch,
        codexThreadId: "thread-explicit",
        codexSessionId: "unknown",
        operation: "edit",
        lockKind: "writeLock",
      },
    });
  }
  const before = await coordinationStatus({ stateRoot });
  assert.equal(before.state.ownership.activeClaims.length, 2);

  await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "release feature one only",
    projectId: "same-project",
    repoRoot: targetRoot,
    worktreeRoot: targetRoot,
    branch: "feature/one",
    codexThreadId: "thread-explicit",
  });
  const after = await coordinationStatus({ stateRoot });
  assert.equal(after.state.ownership.activeClaims.length, 1);
  assert.equal(after.state.ownership.activeClaims[0].context.branch, "feature/two");
});

test("thread-scoped closeout preserves a same-lane sibling claim", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-closeout-owner-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-closeout-owner-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "closeout-owner-hub", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  for (const codexThreadId of ["thread-a", "thread-b"]) {
    await appendEvent(stateRoot, config, "codex-a", {
      type: "claim",
      paths: ["src/owned.ts"],
      reason: `${codexThreadId} preexisting claim`,
      context: {
        projectId: "same-project",
        repoRoot: targetRoot,
        worktreeRoot: targetRoot,
        cwd: targetRoot,
        branch: "feature/shared",
        codexThreadId,
        codexSessionId: "unknown",
        operation: "edit",
        lockKind: "writeLock",
      },
    });
  }

  const closeout = await coordinationCloseout({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    codexThreadId: "thread-a",
    reason: "close thread a only",
  });
  assert.equal(closeout.ok, true);
  assert.equal(closeout.initialClaimCount, 1);
  const after = await coordinationStatus({ stateRoot });
  assert.equal(after.state.ownership.activeClaims.length, 1);
  assert.equal(after.state.ownership.activeClaims[0].context.codexThreadId, "thread-b");
});

test("explicit project boundaries are decisive for claims, guards, and releases", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-project-owner-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-project-owner-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "claim.ts"), "export const claim = true;\n");
  fs.writeFileSync(path.join(targetRoot, "src", "guard.ts"), "export const guard = true;\n");
  await coordinationInit({ stateRoot, hub: "project-owner-hub", lane: "codex-a" });

  const projectA = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/claim.ts"],
    reason: "project a owner",
    projectId: "project-a",
    branch: "feature/shared",
    codexThreadId: "thread-a",
  });
  assert.equal(projectA.ok, true);
  const projectB = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/claim.ts"],
    reason: "separate explicit project namespace",
    projectId: "project-b",
    branch: "feature/shared",
    codexThreadId: "thread-b",
  });
  assert.equal(projectB.ok, true);

  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/guard.ts"],
    reason: "project a guard owner",
    projectId: "project-a",
    branch: "feature/shared",
    codexThreadId: "thread-shared",
  });
  const wrongProjectGuard = await coordinationGuard({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/guard.ts"],
    operation: "commit",
    projectId: "project-b",
    branch: "feature/shared",
    codexThreadId: "thread-shared",
  });
  assert.equal(wrongProjectGuard.ok, false);
  assert.match(wrongProjectGuard.result.findings.join("\n"), /outside active ledger claims/u);

  await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/guard.ts"],
    reason: "wrong project release",
    projectId: "project-b",
    branch: "feature/shared",
    codexThreadId: "thread-shared",
  });
  const afterWrongRelease = await coordinationStatus({ stateRoot });
  assert.equal(
    afterWrongRelease.state.ownership.activeClaims.some(
      (claim) => claim.paths.includes("src/guard.ts") && claim.context.projectId === "project-a",
    ),
    true,
  );
});

test("pathless release reports only claims matched by its derived worktree scope", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-pathless-release-"));
  const worktreeA = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-pathless-release-a-"));
  const worktreeB = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-pathless-release-b-"));
  await coordinationInit({ stateRoot, hub: "pathless-release-hub", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  for (const [worktreeRoot, claimedPath] of [[worktreeA, "src/a.ts"], [worktreeB, "src/b.ts"]]) {
    await appendEvent(stateRoot, config, "codex-a", {
      type: "claim",
      paths: [claimedPath],
      context: buildCoordinationContext({
        cwd: worktreeRoot,
        repoRoot: worktreeRoot,
        worktreeRoot,
        gitRemote: "https://github.com/example/shared.git",
        branch: "feature/shared",
        codexThreadId: "thread-shared",
        operation: "edit",
        lockKind: "writeLock",
      }),
    });
  }

  const previousCwd = process.cwd();
  let release;
  try {
    process.chdir(worktreeA);
    release = await coordinationRelease({
      stateRoot,
      lane: "codex-a",
      codexThreadId: "thread-shared",
    });
  } finally {
    process.chdir(previousCwd);
  }
  assert.equal(release.matchedClaimCount, 1);
  assert.deepEqual(release.event.paths, ["src/a.ts"]);
  assert.deepEqual(release.releasedPaths, ["src/a.ts"]);
  const after = await coordinationStatus({ stateRoot });
  assert.equal(after.state.ownership.activeClaims.length, 1);
  assert.deepEqual(after.state.ownership.activeClaims[0].paths, ["src/b.ts"]);
});

test("no-op release stays silent and owner release wakes a same-lane sibling intent", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-sibling-intent-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-sibling-intent-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "sibling-intent-hub", lane: "codex-a" });
  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "thread-owner",
  });
  const intent = await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "thread-waiting",
    onConflict: "intent",
  });
  assert.equal(intent.intentQueued, true);

  const noOp = await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "thread-unrelated",
  });
  assert.equal(noOp.matchedClaimCount, 0);
  assert.deepEqual(noOp.releasedPaths, []);
  assert.deepEqual(noOp.notificationEvents, []);
  assert.equal((await coordinationInbox({ stateRoot, lane: "codex-a" })).inbox.length, 0);

  const ownerRelease = await coordinationRelease({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    branch: "feature/shared",
    codexThreadId: "thread-owner",
  });
  assert.equal(ownerRelease.matchedClaimCount, 1);
  assert.deepEqual(ownerRelease.releasedPaths, ["src/owned.ts"]);
  assert.equal(ownerRelease.notificationEvents.length, 1);
  assert.equal(ownerRelease.notificationEvents[0].to, "codex-a");
  assert.equal(ownerRelease.notificationEvents[0].context.editIntentId, intent.event.id);
  const inbox = await coordinationInbox({ stateRoot, lane: "codex-a" });
  assert.match(inbox.inbox[0].body, /Re-read the file/u);
});

test("scoped release stays inside its writer and worktree", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-release-boundary-"));
  const worktreeA = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-release-boundary-a-"));
  const worktreeB = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-release-boundary-b-"));
  for (const root of [worktreeA, worktreeB]) {
    fs.mkdirSync(path.join(root, "src"), { recursive: true });
    fs.writeFileSync(path.join(root, "src", "owned.ts"), "export const owned = true;\n");
  }
  await coordinationInit({ stateRoot, hub: "release-boundary-hub", lane: "codex-a" });
  const config = await loadIdentity(stateRoot);
  const claimContext = (worktreeRoot) => ({
    projectId: "same-project",
    explicitProjectId: "same-project",
    repoRoot: worktreeRoot,
    worktreeRoot,
    branch: "feature/shared",
    codexThreadId: "thread-shared",
    operation: "edit",
    lockKind: "writeLock",
  });
  for (const worktreeRoot of [worktreeA, worktreeB]) {
    await appendEvent(stateRoot, config, "codex-a", {
      type: "claim",
      paths: ["src/owned.ts"],
      context: claimContext(worktreeRoot),
    });
  }
  await appendEvent(stateRoot, { ...config, nodeId: "node_other_writer" }, "codex-a", {
    type: "claim",
    paths: ["src/owned.ts"],
    context: claimContext(worktreeA),
  });

  await coordinationRelease({
    stateRoot,
    root: worktreeA,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    projectId: "same-project",
    branch: "feature/shared",
    codexThreadId: "thread-shared",
  });
  const after = await coordinationStatus({ stateRoot });
  assert.equal(after.state.ownership.activeClaims.length, 2);
  assert.equal(
    after.state.ownership.activeClaims.some((claim) => claim.context.worktreeRoot === worktreeB),
    true,
  );
  assert.equal(
    after.state.ownership.activeClaims.some((claim) => claim.writer === "node_other_writer.codex-a"),
    true,
  );
});

test("claim command handlers preserve sibling ownership", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-command-owner-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-command-owner-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"), { recursive: true });
  fs.writeFileSync(path.join(targetRoot, "src", "owned.ts"), "export const owned = true;\n");
  await coordinationInit({ stateRoot, hub: "command-owner-hub", lane: "codex-a" });
  await coordinationClaim({
    stateRoot,
    root: targetRoot,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    projectId: "same-project",
    branch: "feature/shared",
    codexThreadId: "thread-a",
  });
  const commandBody = (codexThreadId) => ({
    lane: "codex-a",
    root: targetRoot,
    paths: ["src/owned.ts"],
    projectId: "same-project",
    branch: "feature/shared",
    codexThreadId,
  });
  const siblingRelease = await executeReleaseCommand(stateRoot, commandBody("thread-b"));
  assert.equal(siblingRelease.status, 200);
  assert.deepEqual(siblingRelease.result.releasedPaths, []);
  assert.equal((await coordinationStatus({ stateRoot })).state.ownership.activeClaims.length, 1);

  const siblingClaim = await executeClaimCommand(stateRoot, commandBody("thread-b"));
  assert.equal(siblingClaim.status, 409);
  assert.equal(siblingClaim.result.blockingOwners[0].codexThreadId, "thread-a");

  const ownerRelease = await executeReleaseCommand(stateRoot, commandBody("thread-a"));
  assert.equal(ownerRelease.status, 200);
  assert.deepEqual(ownerRelease.result.releasedPaths, ["src/owned.ts"]);
  assert.equal((await coordinationStatus({ stateRoot })).state.ownership.activeClaims.length, 0);
});

test("owner identity never crosses writer, worktree, or branch context", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-owner-context-"));
  const worktreeA = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-owner-context-a-"));
  const worktreeB = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-owner-context-b-"));
  fs.mkdirSync(path.join(worktreeA, "src"), { recursive: true });
  fs.mkdirSync(path.join(worktreeB, "src"), { recursive: true });
  fs.writeFileSync(path.join(worktreeA, "src", "owned.ts"), "export const owned = 'a';\n");
  fs.writeFileSync(path.join(worktreeB, "src", "owned.ts"), "export const owned = 'b';\n");
  await coordinationInit({ stateRoot, hub: "owner-context-hub", lane: "codex-a" });

  const first = await coordinationClaim({
    stateRoot,
    root: worktreeA,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "first worktree owner",
    projectId: "same-project",
    repoRoot: worktreeA,
    worktreeRoot: worktreeA,
    branch: "feature/shared",
    codexThreadId: "thread-shared",
  });
  assert.equal(first.ok, true);

  const otherWorktree = await coordinationClaim({
    stateRoot,
    root: worktreeB,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "same thread cannot alias another worktree",
    projectId: "same-project",
    repoRoot: worktreeB,
    worktreeRoot: worktreeB,
    branch: "feature/shared",
    codexThreadId: "thread-shared",
  });
  assert.equal(otherWorktree.ok, false);
  assert.equal(otherWorktree.blockers[0].type, "branch-write-conflict");

  const otherBranch = await coordinationClaim({
    stateRoot,
    root: worktreeB,
    lane: "codex-a",
    paths: ["src/owned.ts"],
    reason: "same thread on another branch remains visible",
    projectId: "same-project",
    repoRoot: worktreeB,
    worktreeRoot: worktreeB,
    branch: "feature/other",
    codexThreadId: "thread-shared",
  });
  assert.equal(otherBranch.ok, true);
  const branchStatus = await coordinationStatus({ stateRoot });
  assert.equal(branchStatus.state.ownership.activeClaims.length, 2);
  assert.equal(branchStatus.state.ownership.mergeRisks.length, 1);

  const config = await loadIdentity(stateRoot);
  const otherWriterConfig = {
    ...config,
    nodeId: "node_other_writer",
    nodeName: "OtherWriter",
  };
  await appendEvent(stateRoot, otherWriterConfig, "codex-a", {
    type: "claim",
    paths: ["src/writer-owned.ts"],
    reason: "other writer owner",
    context: {
      projectId: "same-project",
      repoRoot: worktreeA,
      worktreeRoot: worktreeA,
      cwd: worktreeA,
      branch: "feature/shared",
      codexThreadId: "thread-shared",
      codexSessionId: "unknown",
      operation: "edit",
      lockKind: "writeLock",
    },
  });
  fs.writeFileSync(path.join(worktreeA, "src", "writer-owned.ts"), "export const writerOwned = true;\n");
  const otherWriter = await coordinationClaim({
    stateRoot,
    root: worktreeA,
    lane: "codex-a",
    paths: ["src/writer-owned.ts"],
    reason: "different writer cannot alias same thread",
    projectId: "same-project",
    repoRoot: worktreeA,
    worktreeRoot: worktreeA,
    branch: "feature/shared",
    codexThreadId: "thread-shared",
  });
  assert.equal(otherWriter.ok, false);
  assert.equal(otherWriter.blockers[0].type, "write-lock-conflict");
  assert.equal(otherWriter.blockingOwners[0].writer, "node_other_writer.codex-a");
});
