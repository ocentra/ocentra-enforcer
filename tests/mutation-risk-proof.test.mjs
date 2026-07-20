import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { collectMutationRiskFindings } from "../src/check-governance.mjs";
import { spawnCli } from "./cli-spawn.mjs";

function makeProject() {
  const project = fs.mkdtempSync(path.join(os.tmpdir(), "ocentra-enforcer-mutation-proof-"));
  fs.mkdirSync(path.join(project, "src"), { recursive: true });
  fs.writeFileSync(path.join(project, "src", "checks.mjs"), "export const changed = true;\n");
  runGit(project, ["init"]);
  runGit(project, ["config", "user.email", "enforcer@example.test"]);
  runGit(project, ["config", "user.name", "Enforcer Test"]);
  runGit(project, ["add", "."]);
  runGit(project, ["commit", "-m", "fixture"]);
  return project;
}

function runGit(project, args) {
  const result = spawnCli("git", args, { cwd: project, encoding: "utf8" });
  assert.equal(result.status, 0, result.stdout || result.stderr);
  return result.stdout.trim();
}

function writeProof(project, commit) {
  const runId = "mutation-risk-proof";
  const runDir = path.join(project, ".enforce", "proofs", "runs", runId);
  fs.mkdirSync(runDir, { recursive: true });
  const proofPath = path.join(runDir, "proof-run.json");
  fs.writeFileSync(
    proofPath,
    `${JSON.stringify({
      runId,
      proofId: "PROOF-MUTATION-RISK-CI",
      status: "passed",
      git: { commit },
      command: [process.execPath, "scripts/ci-local.mjs"],
    })}\n`,
  );
  const manifestDir = path.join(project, ".enforce", "proofs", "db");
  fs.mkdirSync(manifestDir, { recursive: true });
  fs.writeFileSync(
    path.join(manifestDir, "proof-manifest.json"),
    `${JSON.stringify({ schemaVersion: 1, runs: [{ runId }] })}\n`,
  );
  return proofPath;
}

test("mutation-risk accepts only a passed current-commit canonical CI proof", () => {
  const project = makeProject();
  const proofPath = writeProof(project, runGit(project, ["rev-parse", "HEAD"]));
  const scope = { mode: "files", files: ["src/checks.mjs"] };
  assert.deepEqual(collectMutationRiskFindings(project, scope), []);

  const stale = JSON.parse(fs.readFileSync(proofPath, "utf8"));
  stale.git.commit = "0000000000000000000000000000000000000000";
  fs.writeFileSync(proofPath, `${JSON.stringify(stale)}\n`);
  assert.equal(
    collectMutationRiskFindings(project, scope).some((entry) => entry.ruleId === "ENF-2.1"),
    true,
  );

  stale.git.commit = runGit(project, ["rev-parse", "HEAD"]);
  stale.command = [process.execPath, "scripts/not-ci-local.mjs"];
  fs.writeFileSync(proofPath, `${JSON.stringify(stale)}\n`);
  assert.equal(
    collectMutationRiskFindings(project, scope).some((entry) => entry.ruleId === "ENF-2.1"),
    true,
  );
});
