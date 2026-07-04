import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { createMcpClient } from "./mcp-client.mjs";

const PACK_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const SERVER_PATH = path.join(PACK_ROOT, "mcp", "ocentra-enforcer-mcp.mjs");
const CLI = path.join(PACK_ROOT, "scripts", "rust-rules.mjs");

test("MCP status detects stale server code and blocks coordination writes", async (t) => {
  const launcherRoot = fs.mkdtempSync(
    path.join(os.tmpdir(), "ocentra-enforcer-mcp-stale-"),
  );
  const watchedFile = path.join(launcherRoot, "watched.txt");
  fs.writeFileSync(watchedFile, "before\n");
  const server = spawnStaleAwareServer(launcherRoot, watchedFile);
  t.after(() => {
    server.kill();
  });

  const client = createMcpClient(server);
  await initializeServer(client);
  await assertFreshStatus(client);
  fs.writeFileSync(watchedFile, "after\n");
  const staleReport = await assertStaleStatus(client);
  assert.equal(staleReport.changedFiles.length, 1);

  const roots = makeStaleRoots();
  await assertStaleClaimFallback(client, roots);
  await assertStaleReleaseFallback(client, roots);
  await assertStaleReleaseOwnedFallback(client, roots);
  await assertStaleMessageFallback(client, roots);
});

function spawnStaleAwareServer(launcherRoot, watchedFile) {
  return spawn(process.execPath, [SERVER_PATH], {
    cwd: launcherRoot,
    stdio: ["pipe", "pipe", "pipe"],
    env: {
      ...process.env,
      OCENTRA_ENFORCER_MCP_FINGERPRINT_EXTRA: watchedFile,
    },
  });
}

async function initializeServer(client) {
  const initialized = await client.request(1, "initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
  });
  assert.equal(initialized.result.serverInfo.name, "ocentra-enforcer");
}

async function assertFreshStatus(client) {
  const freshStatus = await client.request(2, "tools/call", {
    name: "ocentra_enforcer_mcp_status",
    arguments: {},
  });
  assert.equal(freshStatus.result.isError, false);
  assert.equal(JSON.parse(freshStatus.result.content[0].text).stale, false);
}

async function assertStaleStatus(client) {
  const staleStatus = await client.request(3, "tools/call", {
    name: "ocentra_enforcer_mcp_status",
    arguments: {},
  });
  assert.equal(staleStatus.result.isError, true);
  const report = JSON.parse(staleStatus.result.content[0].text);
  assert.equal(report.stale, true);
  assert.equal(report.reloadRequired, true);
  assert.equal(report.writeCompatible, false);
  assert.equal(report.directWritesAllowed, false);
  assert.equal(report.hashCompatible, true);
  return report;
}

function makeStaleRoots() {
  return {
    stateRoot: fs.mkdtempSync(path.join(os.tmpdir(), "mcp-stale-ledger-")),
    targetRoot: fs.mkdtempSync(path.join(os.tmpdir(), "mcp-stale-target-")),
  };
}

async function assertStaleClaimFallback(client, roots) {
  const staleClaim = await callStaleClaim(client, roots);
  assert.equal(staleClaim.result.isError, true);
  assert.match(staleClaim.result.content[0].text, /MCP server is stale/u);
  const report = JSON.parse(staleClaim.result.content[0].text);
  assert.equal(report.reloadRequired, true);
  assert.equal(report.writeCapable, false);
  assert.equal(report.directWritesAllowed, false);
  assert.equal(report.fallbackAvailable, true);
  assert.equal(report.fallback.recommendedTool, "ocentra_enforcer_run");
  assert.equal(report.fallback.cwd, PACK_ROOT);
  assert.deepEqual(report.fallback.command, claimFallbackCommand(roots));
  assert.deepEqual(report.fallback.enforcerRunArguments.command, report.fallback.command);
  assert.equal(
    report.fallback.enforcerRunArguments.tool,
    "ocentra-enforcer-coordination-fallback",
  );
  assert.match(report.fallback.commandLine, /coordination claim/u);
  assert.match(report.nextStep, /ocentra_enforcer_run/u);
}

function callStaleClaim(client, roots) {
  return client.request(4, "tools/call", {
    name: "ocentra_enforcer_coordination_claim",
    arguments: {
      stateRoot: roots.stateRoot,
      hub: "stale-hub",
      root: roots.targetRoot,
      lane: "codex-a",
      paths: ["src/lib.rs"],
      reason: "must fail closed",
      codexThreadId: "thread-stale",
    },
  });
}

async function assertStaleReleaseFallback(client, roots) {
  const staleRelease = await client.request(5, "tools/call", {
    name: "ocentra_enforcer_coordination_release",
    arguments: {
      stateRoot: roots.stateRoot,
      hub: "stale-hub",
      root: roots.targetRoot,
      lane: "codex-a",
      paths: ["src/lib.rs"],
      reason: "must fail closed release",
      codexThreadId: "thread-stale",
    },
  });
  assert.equal(staleRelease.result.isError, true);
  const report = JSON.parse(staleRelease.result.content[0].text);
  assert.deepEqual(report.fallback.command, releaseFallbackCommand(roots));
}

async function assertStaleReleaseOwnedFallback(client, roots) {
  const staleReleaseOwned = await client.request(51, "tools/call", {
    name: "ocentra_enforcer_coordination_release",
    arguments: {
      stateRoot: roots.stateRoot,
      hub: "stale-hub",
      lane: "codex-a",
      cwd: roots.targetRoot,
      operation: "edit",
      reason: "release stale owned",
      codexThreadId: "thread-stale",
    },
  });
  assert.equal(staleReleaseOwned.result.isError, true);
  const report = JSON.parse(staleReleaseOwned.result.content[0].text);
  assert.deepEqual(report.fallback.command, releaseOwnedFallbackCommand(roots));
}

async function assertStaleMessageFallback(client, roots) {
  const staleMessage = await client.request(6, "tools/call", {
    name: "ocentra_enforcer_coordination_message",
    arguments: {
      stateRoot: roots.stateRoot,
      hub: "stale-hub",
      from: "codex-a",
      to: "codex-b",
      subject: "Fallback subject",
      body: "Fallback body.",
    },
  });
  assert.equal(staleMessage.result.isError, true);
  const report = JSON.parse(staleMessage.result.content[0].text);
  assert.deepEqual(report.fallback.command, messageFallbackCommand(roots));
}

function claimFallbackCommand(roots) {
  return [
    process.execPath,
    CLI,
    "coordination",
    "claim",
    "--state-root",
    roots.stateRoot,
    "--hub",
    "stale-hub",
    "--lane",
    "codex-a",
    "--root",
    roots.targetRoot,
    "--codex-thread-id",
    "thread-stale",
    "--reason",
    "must fail closed",
    "--paths",
    "src/lib.rs",
    "--json",
  ];
}

function releaseFallbackCommand(roots) {
  return [
    process.execPath,
    CLI,
    "coordination",
    "release",
    "--state-root",
    roots.stateRoot,
    "--hub",
    "stale-hub",
    "--lane",
    "codex-a",
    "--root",
    roots.targetRoot,
    "--codex-thread-id",
    "thread-stale",
    "--reason",
    "must fail closed release",
    "--paths",
    "src/lib.rs",
    "--json",
  ];
}

function releaseOwnedFallbackCommand(roots) {
  return [
    process.execPath,
    CLI,
    "coordination",
    "release",
    "--state-root",
    roots.stateRoot,
    "--hub",
    "stale-hub",
    "--lane",
    "codex-a",
    "--cwd",
    roots.targetRoot,
    "--codex-thread-id",
    "thread-stale",
    "--operation",
    "edit",
    "--reason",
    "release stale owned",
    "--json",
  ];
}

function messageFallbackCommand(roots) {
  return [
    process.execPath,
    CLI,
    "coordination",
    "message",
    "--state-root",
    roots.stateRoot,
    "--hub",
    "stale-hub",
    "--from",
    "codex-a",
    "--to",
    "codex-b",
    "--subject",
    "Fallback subject",
    "--body",
    "Fallback body.",
    "--json",
  ];
}
