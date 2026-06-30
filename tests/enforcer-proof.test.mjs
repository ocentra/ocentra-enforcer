import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  claimProof,
  inventoryProofs,
  loadProofRegistry,
  proofArtifact,
  proofLastFailure,
  proofStatus,
  routeProofs,
  runProof,
} from "../src/proof.mjs";

const PACK_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const CLI = path.join(PACK_ROOT, "scripts", "rust-rules.mjs");

function makeProject() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "ocentra-enforcer-proof-"));
}

test("proof registry routes by file, capability, and explicit proof id", () => {
  const registry = loadProofRegistry(PACK_ROOT);
  assert.equal(registry.productName, "ocentra-enforcer");
  assert.equal(registry.proofs.some((proof) => proof.id === "PROOF-COMMAND-GENERIC"), true);

  const tsRoute = routeProofs(
    {
      root: PACK_ROOT,
      scope: "files",
      files: ["src/index.ts", "scripts/test/example-proof.mjs"],
    },
    PACK_ROOT,
  );
  assert.equal(tsRoute.ok, true);
  assert.equal(
    tsRoute.proofs.some((proof) => proof.id === "PARENT-PROOF-INVENTORY"),
    true,
  );
  assert.equal(
    tsRoute.proofs.some((proof) => proof.id === "PROOF-JUNIT-TEST-REPORT"),
    true,
  );

  const androidRoute = routeProofs(
    {
      root: PACK_ROOT,
      capability: "android-device",
    },
    PACK_ROOT,
  );
  assert.deepEqual(
    androidRoute.proofs.map((proof) => proof.id),
    ["PROOF-ANDROID-DEVICE"],
  );

  const explicitRoute = routeProofs(
    {
      root: PACK_ROOT,
      proofId: "PROOF-SARIF-SECURITY-REPORT",
    },
    PACK_ROOT,
  );
  assert.deepEqual(
    explicitRoute.proofs.map((proof) => proof.id),
    ["PROOF-SARIF-SECURITY-REPORT"],
  );
});

test("proof inventory classifies legacy proof scripts without writing", () => {
  const project = makeProject();
  fs.mkdirSync(path.join(project, "scripts", "test"), { recursive: true });
  fs.writeFileSync(
    path.join(project, "scripts", "test", "android-physical-proof.mjs"),
    [
      "import { spawn } from 'node:child_process';",
      "import { writeFile } from 'node:fs/promises';",
      "const serial = process.env.ANDROID_SERIAL;",
      "await writeFile('test-results/android-physical-proof/proof.json', JSON.stringify({ serial }));",
      "spawn('adb', ['devices']);",
    ].join("\n"),
  );
  fs.writeFileSync(
    path.join(project, "scripts", "test", "contract-parity-proof.mjs"),
    [
      "import { readFile } from 'node:fs/promises';",
      "await readFile('test-results/source-proof/proof.json', 'utf8');",
      "await import('../dist/schema.js');",
      "Schema.parse({});",
    ].join("\n"),
  );

  const report = inventoryProofs({ root: project });
  assert.equal(report.totals.scripts, 2);
  assert.equal(report.totals.proofNamed, 2);
  assert.equal(report.totals.spawnCommands, 1);
  assert.equal(report.totals.writesProof, 1);
  assert.equal(report.totals.manualOrDevice, 1);
  assert.equal(report.byFamily["device-manual"], 1);
  assert.equal(report.references["test-results"], 2);
  assert.equal(report.scriptRowsIncluded, false);
  assert.equal(report.scripts.length, 0);
  assert.equal(report.omittedScriptCount, 2);

  const detailed = inventoryProofs({ root: project, includeScripts: true, limit: 1 });
  assert.equal(detailed.scriptRowsIncluded, true);
  assert.equal(detailed.scripts.length, 1);
  assert.equal(detailed.omittedScriptCount, 1);
});

test("proof run stores canonical files, bounded artifacts, status, and claims", () => {
  const project = makeProject();
  const run = runProof(
    {
      root: project,
      proofId: "PROOF-COMMAND-GENERIC",
      runId: "proof-pass",
      command: [process.execPath, "-e", "console.log('proof-ok')"],
      tags: ["unit"],
    },
    PACK_ROOT,
  );
  assert.equal(run.ok, true);
  assert.equal(run.proofRun.status, "passed");
  assert.equal(
    fs.existsSync(path.join(project, ".enforce", "proofs", "runs", "proof-pass", "proof-run.json")),
    true,
  );
  assert.equal(
    run.proofRun.artifacts.some((artifact) => artifact.name === "attestation.json"),
    true,
  );

  const status = proofStatus({ root: project, proofId: "PROOF-COMMAND-GENERIC" });
  assert.equal(status.runs[0].runId, "proof-pass");

  const artifact = proofArtifact({ root: project, runId: "proof-pass", artifact: "summary" });
  assert.match(artifact.text, /Proof PROOF-COMMAND-GENERIC/u);

  const claim = claimProof(
    {
      root: project,
      proofId: "PROOF-COMMAND-GENERIC",
      prReady: true,
    },
    PACK_ROOT,
  );
  assert.equal(claim.ok, true);
  assert.equal(claim.claim.accepted[0].runId, "proof-pass");
});

test("manual-required and failed proofs are not accepted for claims", () => {
  const project = makeProject();
  const manual = runProof(
    {
      root: project,
      proofId: "PROOF-ANDROID-DEVICE",
      runId: "proof-manual",
      capability: "manual-required",
    },
    PACK_ROOT,
  );
  assert.equal(manual.ok, false);
  assert.equal(manual.proofRun.status, "manual-required");

  const failure = proofLastFailure({ root: project });
  assert.equal(failure.found, true);
  assert.equal(failure.proofRun.runId, "proof-manual");

  const claim = claimProof(
    {
      root: project,
      proofId: "PROOF-ANDROID-DEVICE",
      prReady: true,
    },
    PACK_ROOT,
  );
  assert.equal(claim.ok, false);
  assert.equal(
    claim.claim.violations.some((violation) => violation.code === "proof-not-passed"),
    true,
  );
});

test("proof CLI exposes route, run, inventory, and claim", () => {
  const project = makeProject();
  fs.mkdirSync(path.join(project, "scripts", "test"), { recursive: true });
  fs.writeFileSync(
    path.join(project, "scripts", "test", "tiny-proof.mjs"),
    "console.log('test-results/tiny-proof/proof.json');\n",
  );

  const route = spawnSync(
    process.execPath,
    [CLI, "proof", "route", "--root", project, "--files", "scripts/test/tiny-proof.mjs", "--json"],
    { encoding: "utf8" },
  );
  assert.equal(route.status, 0, route.stderr);
  assert.equal(
    JSON.parse(route.stdout).proofs.some((proof) => proof.id === "PARENT-PROOF-INVENTORY"),
    true,
  );

  const run = spawnSync(
    process.execPath,
    [
      CLI,
      "proof",
      "run",
      "--root",
      project,
      "--proof",
      "PROOF-COMMAND-GENERIC",
      "--run-id",
      "proof-cli",
      "--json",
      "--",
      process.execPath,
      "-e",
      "process.exit(0)",
    ],
    { encoding: "utf8" },
  );
  assert.equal(run.status, 0, run.stderr);
  assert.equal(JSON.parse(run.stdout).proofRun.runId, "proof-cli");

  const inventory = spawnSync(process.execPath, [CLI, "proof", "inventory", "--root", project, "--json"], {
    encoding: "utf8",
  });
  assert.equal(inventory.status, 0, inventory.stderr);
  const inventoryJson = JSON.parse(inventory.stdout);
  assert.equal(inventoryJson.totals.scripts, 1);
  assert.equal(inventoryJson.scripts.length, 0);
  assert.equal(inventoryJson.omittedScriptCount, 1);

  const detailedInventory = spawnSync(
    process.execPath,
    [CLI, "proof", "inventory", "--root", project, "--include-scripts", "--limit", "1", "--json"],
    { encoding: "utf8" },
  );
  assert.equal(detailedInventory.status, 0, detailedInventory.stderr);
  assert.equal(JSON.parse(detailedInventory.stdout).scripts.length, 1);

  const claim = spawnSync(
    process.execPath,
    [CLI, "proof", "claim", "--root", project, "--proof", "PROOF-COMMAND-GENERIC", "--pr-ready", "--json"],
    { encoding: "utf8" },
  );
  assert.equal(claim.status, 0, claim.stderr);
  assert.equal(JSON.parse(claim.stdout).ok, true);
});
