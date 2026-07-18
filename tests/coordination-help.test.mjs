import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { coordinationInit } from "../src/coordination/api.mjs";
import { runCoordinationCli } from "../src/coordination/runner.mjs";
import { spawnCli } from "./cli-spawn.mjs";

const PACK_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const CLI = path.join(PACK_ROOT, "scripts", "rust-rules.mjs");
const VENDOR_CLI = path.join(PACK_ROOT, "src", "coordination", "vendor", "cli.js");
const COMMAND_FAMILIES = [
  ["init"],
  ["root"],
  ["lane"],
  ["start"],
  ["msg"],
  ["message"],
  ["inbox"],
  ["ack"],
  ["handoff"],
  ["note"],
  ["claim"],
  ["release"],
  ["closeout"],
  ["resolve"],
  ["status"],
  ["heartbeat"],
  ["session", "claim"],
  ["session", "release"],
  ["task"],
  ["worker"],
  ["report"],
  ["workers"],
  ["tasks"],
  ["materialize"],
  ["presence"],
  ["index"],
  ["notify"],
  ["compact"],
  ["doctor"],
  ["health"],
  ["guard"],
  ["streams"],
  ["manifest"],
  ["sync"],
  ["peer", "add"],
  ["peer", "remove"],
  ["peer", "sync"],
  ["serve"],
  ["ensure"],
  ["repair", "legacy-hash", "--write"],
  ["repair", "sequence", "--write"],
  ["repair", "stale-claims", "--write"],
  ["lanes:init"],
  ["lanes:status"],
  ["hub:status"],
  ["ledger:doctor"],
  ["ledger:root"],
  ["ledger:install"],
  ["ledger:build"],
  ["ledger:ensure"],
  ["ledger:dashboard"],
  ["ledger:inbox"],
  ["hub:inbox"],
  ["ledger:workers"],
  ["hub:heartbeats"],
  ["ledger:free"],
  ["ledger:tasks"],
  ["ledger:message"],
  ["ledger:notify"],
  ["hub:notify"],
  ["ledger:sync"],
  ["hub:state:sync"],
  ["ledger:guard"],
  ["lanes:guard"],
  ["hub:guard"],
  ["lanes:claim"],
  ["lanes:free"],
  ["hub:message"],
  ["hub:ack"],
  ["hub:heartbeat"],
  ["hub:report"],
  ["hub:lock"],
  ["hub:unlock"],
  ["hub:watch"],
  ["hub:hook"],
  ["hub:thread-mode"],
  ["hub:thread:upgrade"],
  ["hub:thread:default"],
  ["hub:delegate:grant"],
  ["hub:delegate:revoke"],
  ["hub:lane-ledger:audit"],
];

function ledgerSnapshot(stateRoot) {
  const files = [];
  let eventCount = 0;
  const visit = (directory) => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolutePath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        visit(absolutePath);
        continue;
      }
      if (!entry.isFile()) continue;
      const relativePath = path.relative(stateRoot, absolutePath).replaceAll(path.sep, "/");
      const contents = fs.readFileSync(absolutePath);
      if (relativePath.startsWith("streams/") && relativePath.endsWith(".ndjson")) {
        eventCount += contents.toString("utf8").split(/\r?\n/gu).filter(Boolean).length;
      }
      files.push({
        path: relativePath,
        sha256: createHash("sha256").update(contents).digest("hex"),
      });
    }
  };
  visit(stateRoot);
  files.sort((left, right) => left.path.localeCompare(right.path));
  const ledgerHash = createHash("sha256");
  for (const file of files) ledgerHash.update(file.path).update("\0").update(file.sha256).update("\n");
  return { eventCount, ledgerHash: ledgerHash.digest("hex"), files };
}

async function captureHelp(args) {
  const output = [];
  const originalLog = console.log;
  console.log = (...items) => output.push(items.join(" "));
  try {
    await runCoordinationCli(args);
  } finally {
    console.log = originalLog;
  }
  return output.join("\n");
}

test("coordination help flags are read-only across every public command family", async () => {
  const stateRoot = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-global-help-"));
  await coordinationInit({ stateRoot, hub: "global-help", lane: "codex-a" });
  const before = ledgerSnapshot(stateRoot);
  for (const commandArgs of COMMAND_FAMILIES) {
    for (const flag of ["--help", "-h"]) {
      const label = [...commandArgs, flag].join(" ");
      const output = await captureHelp([
        ...commandArgs,
        "--state-root",
        stateRoot,
        "--hub",
        "global-help",
        "--lane",
        "codex-a",
        flag,
      ]);
      assert.match(output, /^Usage: ocentra-enforcer coordination/u, label);
      assert.deepEqual(ledgerSnapshot(stateRoot), before, `${label} changed the ledger hash or event count`);
    }
  }

  for (const [command, flag] of [["init", "--help"], ["lanes:free", "-h"]]) {
    const result = spawnCli(
      process.execPath,
      [CLI, "coordination", command, "--state-root", stateRoot, "--hub", "global-help", "--lane", "codex-a", flag],
      { cwd: PACK_ROOT, encoding: "utf8", timeout: 15_000 },
    );
    assert.equal(result.status, 0, `${command} ${flag}: ${result.stderr}`);
    assert.match(result.stdout, /^Usage: ocentra-enforcer coordination/u);
    assert.deepEqual(ledgerSnapshot(stateRoot), before);
  }

  for (const [command, flag] of [["init", "--help"], ["worker", "-h"]]) {
    const result = spawnCli(
      process.execPath,
      [VENDOR_CLI, command, flag],
      {
        cwd: PACK_ROOT,
        encoding: "utf8",
        timeout: 15_000,
        env: { ...process.env, LEDGER_ROOT: stateRoot },
      },
    );
    assert.equal(result.status, 0, `direct vendor ${command} ${flag}: ${result.stderr}`);
    assert.match(result.stdout, /^Usage: ocentra-enforcer coordination/u);
    assert.deepEqual(ledgerSnapshot(stateRoot), before);
  }
});
