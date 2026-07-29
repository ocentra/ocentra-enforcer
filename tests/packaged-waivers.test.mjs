import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { spawnCli } from "./cli-spawn.mjs";

import {
  applyPackagedWaivers,
  loadPackagedWaiverRegistry,
} from "../src/packaged-waivers.mjs";
import { decodePackagedWaiverDocument } from "../src/decoder-packaged-waivers.mjs";
import { MalformedPackagedWaiverDocumentError } from "../src/error-malformed-packaged-waiver-document.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const CLI = path.join(ROOT, "scripts", "ocentra-enforcer.mjs");

const rules = [
  { id: "CFG-1.6", severity: "error", lockLevel: "waiver-required" },
  { id: "DOC-1.1", severity: "warning", lockLevel: "advisory" },
];

function registryFile(document) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-packaged-waivers-"));
  const file = path.join(directory, "waivers.json");
  fs.writeFileSync(file, JSON.stringify(document));
  return file;
}

function projectFile(project, relativePath, content) {
  const target = path.join(project, relativePath);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, content, "utf8");
}

test("packaged waiver decoder returns typed evidence for malformed JSON", () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-packaged-waivers-"));
  const file = path.join(directory, "waivers.json");
  fs.writeFileSync(file, '{"waivers":[', "utf8");

  assert.throws(
    () => decodePackagedWaiverDocument(file),
    (error) => {
      assert.equal(error instanceof MalformedPackagedWaiverDocumentError, true);
      assert.equal(error.name, "MalformedPackagedWaiverDocumentError");
      assert.equal(error.registryPath, file);
      assert.equal(error.cause instanceof SyntaxError, true);
      assert.equal(
        error.message,
        `Cannot load packaged waiver registry ${file}: ${error.cause.message}`,
      );
      return true;
    },
  );
});

test("path-scoped packaged waiver only marks its exact finding", () => {
  const waivers = loadPackagedWaiverRegistry(registryFile({
    waivers: [{ path: "src/lib.rs", ruleId: "CFG-1.6", owner: "maintainer", reason: "fixture proof", expires: "2026-12-31" }],
  }), rules, { today: "2026-07-10" });
  const result = applyPackagedWaivers([
    { ruleId: "CFG-1.6", file: "src/lib.rs", status: "open" },
    { ruleId: "CFG-1.6", file: "src/other.rs", status: "open" },
    { ruleId: "CFG-1.6", status: "open" },
  ], waivers, { today: "2026-07-10" });
  assert.equal(result.active.length, 2);
  assert.equal(result.waived.length, 1);
  assert.equal(result.waived[0].status, "waived");
  assert.equal(result.waived[0].waiverSource, "packaged-registry");
  assert.equal(result.active.some((finding) => finding.file === "src/other.rs"), true);
  assert.equal(result.active.some((finding) => !finding.file), true);
});

test("scan --json applies a project-local exact waiver without waiving other findings", () => {
  const project = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-project-waivers-"));
  projectFile(project, "ocentra-enforcer.config.json", `${JSON.stringify({
    schemaVersion: 1,
    profileName: "strict",
    failOn: ["error", "warning"],
    languages: ["typescript", "common"],
  }, null, 2)}\n`);
  projectFile(project, ".enforce/waivers.json", `${JSON.stringify({
    waivers: [{
      path: "src/api.ts",
      ruleId: "DOC-1.1",
      owner: "platform-team",
      reason: "project-local scanner fixture",
      expires: "2099-12-31",
    }],
  }, null, 2)}\n`);
  projectFile(project, "src/api.ts", "export function makeThing(): number { return 1; }\n");
  projectFile(project, "src/other.ts", "export function makeOtherThing(): number { return 1; }\n");

  const scan = spawnCli(process.execPath, [
    CLI,
    "scan",
    "--json",
    "--root",
    project,
    "--languages",
    "typescript,common",
    "--files",
    "src/api.ts,src/other.ts",
  ], { encoding: "utf8" });
  assert.notEqual(scan.status, 0, scan.stdout || scan.stderr);
  const report = JSON.parse(scan.stdout);
  const waived = report.waived.find((finding) => finding.ruleId === "DOC-1.1" && finding.file === "src/api.ts");

  assert.equal(waived?.status, "waived");
  assert.equal(waived?.waiverId, "PROJECT-WAIVER:DOC-1.1:src/api.ts");
  assert.equal(waived?.waiverSource, "project-registry");
  assert.equal(
    report.violations.some((finding) => finding.ruleId === "DOC-1.1" && finding.file === "src/api.ts"),
    false,
  );
  assert.equal(
    report.violations.some((finding) => finding.ruleId === "DOC-1.1" && finding.file === "src/other.ts"),
    true,
  );
});

test("scan --json treats an absent project-local waiver registry as a no-op", () => {
  const project = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-no-project-waivers-"));
  projectFile(project, "ocentra-enforcer.config.json", `${JSON.stringify({
    schemaVersion: 1,
    profileName: "strict",
    failOn: ["error"],
    languages: ["typescript", "common"],
  }, null, 2)}\n`);
  projectFile(project, "src/api.ts", "export function makeThing(): number { return 1; }\n");

  const scan = spawnCli(process.execPath, [
    CLI,
    "scan",
    "--json",
    "--root",
    project,
    "--languages",
    "typescript,common",
    "--files",
    "src/api.ts",
  ], { encoding: "utf8" });
  assert.equal(scan.status, 0, scan.stdout || scan.stderr);
  const report = JSON.parse(scan.stdout);

  assert.equal(fs.existsSync(path.join(project, ".enforce", "waivers.json")), false);
  assert.equal(report.warnings.some((finding) => finding.ruleId === "DOC-1.1"), true);
  assert.deepEqual(report.waived, []);
});

test("invalid, expired, broad, and non-waivable packaged waivers fail closed", () => {
  for (const waiver of [
    { path: "src/**", ruleId: "CFG-1.6", owner: "maintainer", reason: "bad", expires: "2026-12-31" },
    { path: "src/lib.rs", ruleId: "CFG-1.6", owner: "maintainer", reason: "bad", expires: "2026-07-09" },
    { path: "src/lib.rs", ruleId: "CFG-1.6", owner: "maintainer", reason: "bad", expires: "2026-02-30" },
    { path: "src/lib.rs", ruleId: "RR-6.1", owner: "maintainer", reason: "bad", expires: "2026-12-31" },
    { path: "src/lib.rs", ruleId: "CFG-1.6", owner: "", reason: "bad", expires: "2026-12-31" },
  ]) {
    assert.throws(() => loadPackagedWaiverRegistry(registryFile({ waivers: [waiver] }), rules, { today: "2026-07-10" }));
  }
  const result = applyPackagedWaivers([{ ruleId: "CFG-1.6", status: "open" }], [], { today: "2026-07-10" });
  assert.deepEqual(result.active, [{ ruleId: "CFG-1.6", status: "open" }]);
});

test("aggregate findings remain active instead of aborting exact-file waiver matching", () => {
  const result = applyPackagedWaivers([
    { ruleId: "CFG-1.6", file: "crates/enforcer-memory", status: "open" },
  ], [], { today: "2026-07-10" });

  assert.deepEqual(result.active, [
    { ruleId: "CFG-1.6", file: "crates/enforcer-memory", status: "open" },
  ]);
  assert.equal(result.waived.length, 0);
});
