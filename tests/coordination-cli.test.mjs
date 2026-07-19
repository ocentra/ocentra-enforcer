import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { spawnCli } from "./cli-spawn.mjs";

import {
  coordinationClaim,
  coordinationInbox,
  coordinationInit,
  coordinationMessage,
  coordinationPeer,
  coordinationPresence,
  coordinationSync,
} from "../src/coordination/api.mjs";
import { startPeerServer } from "../src/coordination/vendor/server.js";

const PACK_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const CLI = path.join(PACK_ROOT, "scripts", "rust-rules.mjs");

test("CLI closeout all-owned remains lane-scoped and exact closeout reaches zero", () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-cli-closeout-scope-"));
  const targetRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-cli-closeout-target-"));
  fs.mkdirSync(path.join(targetRoot, "src"));
  fs.writeFileSync(path.join(targetRoot, "src", "alpha.rs"), "fn alpha() {}\n");
  fs.writeFileSync(path.join(targetRoot, "src", "beta.rs"), "fn beta() {}\n");

  const run = (...args) =>
    spawnCli(
      process.execPath,
      [CLI, "coordination", ...args, "--state-root", stateRoot, "--hub", "scope-hub"],
      { cwd: targetRoot, encoding: "utf8" },
    );

  const init = run("init", "--lane", "lane-alpha");
  assert.equal(init.status, 0, init.stderr);
  for (const [lane, file] of [
    ["lane-alpha", "src/alpha.rs"],
    ["lane-beta", "src/beta.rs"],
  ]) {
    const claim = run(
      "claim",
      "--lane",
      lane,
      "--root",
      targetRoot,
      "--paths",
      file,
      "--reason",
      `${lane} claim`,
    );
    assert.equal(claim.status, 0, claim.stderr);
  }

  const scoped = run(
    "closeout",
    "--lane",
    "lane-alpha",
    "--root",
    targetRoot,
    "--all-owned",
    "--no-repair-stale",
    "--reason",
    "alpha only",
  );
  assert.equal(scoped.status, 0, scoped.stderr);
  const scopedReport = JSON.parse(scoped.stdout);
  assert.equal(scopedReport.filters.includeAllLanes, false);
  assert.equal(scopedReport.initialClaimCount, 1);
  assert.equal(scopedReport.releasedClaimCount, 1);

  const afterScoped = run("presence");
  assert.equal(afterScoped.status, 0, afterScoped.stderr);
  const scopedClaims = JSON.parse(afterScoped.stdout).views.byClaimedPath;
  assert.equal(scopedClaims["src/alpha.rs"], undefined);
  assert.equal(scopedClaims["src/beta.rs"][0].lane, "lane-beta");

  const exact = run(
    "closeout",
    "--lane",
    "lane-beta",
    "--root",
    targetRoot,
    "--no-repair-stale",
    "--reason",
    "beta exact closeout",
  );
  assert.equal(exact.status, 0, exact.stderr);
  const exactReport = JSON.parse(exact.stdout);
  assert.equal(exactReport.filters.includeAllLanes, false);
  assert.equal(exactReport.remainingClaimCount, 0);

  const afterExact = run("presence");
  assert.equal(afterExact.status, 0, afterExact.stderr);
  assert.deepEqual(JSON.parse(afterExact.stdout).views.byClaimedPath, {});
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
      "--paths",
      "README.md",
      "--reason",
      "compat smoke",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(claim.status, 0, claim.stderr);
  assert.equal(JSON.parse(claim.stdout).event.type, "claim");

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
      "--paths",
      "README.md",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(guard.status, 0, guard.stderr);
  assert.equal(JSON.parse(guard.stdout).result.ok, true);

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
      "--paths",
      "README.md",
    ],
    { cwd: PACK_ROOT, encoding: "utf8" },
  );
  assert.equal(release.status, 0, release.stderr);
  assert.equal(JSON.parse(release.stdout).event.type, "release");
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
