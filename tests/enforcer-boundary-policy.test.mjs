import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SCRIPT = path.join(ROOT, "scripts", "rust-rules.mjs");

function makeProject(files) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-boundary-policy-"));
  for (const [relativePath, content] of Object.entries(files)) {
    const absolutePath = path.join(root, relativePath);
    fs.mkdirSync(path.dirname(absolutePath), { recursive: true });
    fs.writeFileSync(absolutePath, content, "utf8");
  }
  return root;
}

function scan(files) {
  const root = makeProject(files);
  const result = spawnSync(
    process.execPath,
    [
      SCRIPT,
      "scan",
      "--root",
      root,
      "--json",
      "--languages",
      "common",
      "--files",
      ...Object.keys(files),
    ],
    { encoding: "utf8", maxBuffer: 32 * 1024 * 1024 },
  );
  assert.match(result.stdout, /^\s*\{/u, result.stdout || result.stderr);
  return JSON.parse(result.stdout).violations;
}

function config(rawTypeBoundaryGlobs) {
  return JSON.stringify({
    schemaVersion: 2,
    profileName: "strict",
    failOn: ["error"],
    rawTypeBoundaryGlobs,
    boundaryOwnerNote: "Boundary DTO ownership is reviewed by the domain team.",
  });
}

test("boundary ownership globs reject catch-all scopes and accept named scopes", () => {
  const broad = scan({
    "ocentra-enforcer.config.json": config(["src/**"]),
  });
  const narrow = scan({
    "ocentra-enforcer.config.json": config(["src/boundary/**"]),
  });

  assert.equal(broad.some((finding) => finding.ruleId === "BOUND-1.7"), true);
  assert.equal(narrow.some((finding) => finding.ruleId === "BOUND-1.7"), false);
});

test("decoder source does not require a waiver-shaped marker", () => {
  const findings = scan({
    "ocentra-enforcer.config.json": config(["src/boundary/**"]),
    "src/decoder-packaged-waivers.mjs": `
/** Decodes a package document at its transport boundary. */
export function decodeDocument(raw) {
  return JSON.parse(raw);
}
`,
  });

  assert.equal(findings.some((finding) => finding.ruleId === "BOUND-1.7"), false);
});
