import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { decodeEnforcerConfig } from "../schemas/effect/enforcer-schemas.mjs";
import { spawnCli } from "./cli-spawn.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SCRIPT = path.join(ROOT, "scripts", "rust-rules.mjs");

function makeWorkspace({
  allowedBuildRsPaths = [],
  allowBuildRs = false,
  denyToml = '[advisories]\nyanked = "deny"\nunmaintained = "deny"\n',
} = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "build-rs-policy-"));
  const crates = ["approved", "sibling"];
  fs.writeFileSync(
    path.join(root, "Cargo.toml"),
    `[workspace]\nmembers = ["crates/approved", "crates/sibling"]\nresolver = "2"\n`,
    "utf8",
  );
  fs.writeFileSync(path.join(root, "Cargo.lock"), "", "utf8");
  fs.writeFileSync(
    path.join(root, "rust-toolchain.toml"),
    '[toolchain]\nchannel = "stable"\ncomponents = ["rustfmt", "clippy"]\n',
    "utf8",
  );
  fs.writeFileSync(path.join(root, "clippy.toml"), "# fixture\n", "utf8");
  fs.writeFileSync(path.join(root, "deny.toml"), denyToml, "utf8");
  fs.writeFileSync(path.join(root, "OWNERS"), "@ocentra/enforcer\n", "utf8");
  fs.writeFileSync(
    path.join(root, "rust-rules.config.json"),
    JSON.stringify({
      schemaVersion: 2,
      profileName: "strict",
      requireCargoDeny: false,
      allowBuildRs,
      allowedBuildRsPaths,
    }),
    "utf8",
  );
  for (const crateName of crates) {
    const crateRoot = path.join(root, "crates", crateName);
    fs.mkdirSync(path.join(crateRoot, "src"), { recursive: true });
    fs.writeFileSync(
      path.join(crateRoot, "Cargo.toml"),
      `[package]\nname = "${crateName}"\nversion = "0.1.0"\nedition = "2021"\nrust-version = "1.85"\n`,
      "utf8",
    );
    fs.writeFileSync(
      path.join(crateRoot, "src", "lib.rs"),
      "#![forbid(unsafe_code)]\n#![deny(warnings)]\n",
      "utf8",
    );
  }
  return root;
}

function scan(root) {
  const result = spawnCli(
    process.execPath,
    [SCRIPT, "scan", "--root", root, "--languages", "rust", "--workspace", "--json"],
    { encoding: "utf8" },
  );
  const report = JSON.parse(result.stdout);
  return {
    result,
    buildScriptFindings: report.violations.filter(
      (violation) => violation.ruleId === "RR-7.5",
    ),
    advisoryPolicyFindings: report.violations.filter(
      (violation) => violation.ruleId === "RR-9.23" || violation.ruleId === "RR-9.24",
    ),
  };
}

function writeBuildScript(root, crateName) {
  fs.writeFileSync(
    path.join(root, "crates", crateName, "build.rs"),
    'fn main() { println!("cargo:rerun-if-changed=build.rs"); }\n',
    "utf8",
  );
}

test("RR-7.5 rejects an unlisted build script", () => {
  const root = makeWorkspace();
  writeBuildScript(root, "approved");
  const { buildScriptFindings } = scan(root);
  assert.deepEqual(
    buildScriptFindings.map((finding) => finding.file),
    ["crates/approved/build.rs"],
  );
});

test("RR-7.5 accepts only an exact listed build script path", () => {
  const root = makeWorkspace({
    allowedBuildRsPaths: ["crates/approved/build.rs"],
  });
  writeBuildScript(root, "approved");
  const { buildScriptFindings } = scan(root);
  assert.deepEqual(buildScriptFindings, []);
});

test("RR-7.5 exact approval does not allow a sibling build script", () => {
  const root = makeWorkspace({
    allowedBuildRsPaths: ["crates/approved/build.rs"],
  });
  writeBuildScript(root, "approved");
  writeBuildScript(root, "sibling");
  const { buildScriptFindings } = scan(root);
  assert.deepEqual(
    buildScriptFindings.map((finding) => finding.file),
    ["crates/sibling/build.rs"],
  );
});

test("RR-7.5 rejects near-match and non-canonical approval paths", () => {
  for (const candidate of [
    "crates/approved/build.rs.backup",
    "crates/approved",
    "crates/*/build.rs",
    "./crates/approved/build.rs",
    "crates/sibling/../approved/build.rs",
    "crates\\approved\\build.rs",
    "/crates/approved/build.rs",
    "C:\\crates\\approved\\build.rs",
  ]) {
    const root = makeWorkspace({ allowedBuildRsPaths: [candidate] });
    writeBuildScript(root, "approved");
    const { buildScriptFindings } = scan(root);
    assert.equal(buildScriptFindings.length, 1, candidate);
  }
});

test("allowedBuildRsPaths schema accepts only string arrays", () => {
  assert.deepEqual(
    decodeEnforcerConfig({
      allowedBuildRsPaths: ["crates/approved/build.rs"],
    }).allowedBuildRsPaths,
    ["crates/approved/build.rs"],
  );
  for (const allowedBuildRsPaths of [
    "crates/approved/build.rs",
    ["crates/approved/build.rs", 7],
    { approved: "crates/approved/build.rs" },
  ]) {
    assert.throws(
      () => decodeEnforcerConfig({ allowedBuildRsPaths }),
      /enforcer config schema validation failed/u,
    );
  }
});

test("legacy allowBuildRs remains backward compatible", () => {
  const root = makeWorkspace({ allowBuildRs: true });
  writeBuildScript(root, "approved");
  writeBuildScript(root, "sibling");
  const { buildScriptFindings } = scan(root);
  assert.deepEqual(buildScriptFindings, []);
});

test("RR-9.24 accepts current all and legacy deny unmaintained policies", () => {
  for (const unmaintained of ["all", "deny"]) {
    const root = makeWorkspace({
      denyToml: `[advisories]\nyanked = "deny"\nunmaintained = "${unmaintained}"\n`,
    });
    const { advisoryPolicyFindings } = scan(root);
    assert.deepEqual(advisoryPolicyFindings, [], unmaintained);
  }
});

test("RR-9.24 rejects weak, commented, misplaced, and missing policies", () => {
  for (const [name, denyToml] of [
    ["none", '[advisories]\nyanked = "deny"\nunmaintained = "none"\n'],
    ["workspace", '[advisories]\nyanked = "deny"\nunmaintained = "workspace"\n'],
    ["transitive", '[advisories]\nyanked = "deny"\nunmaintained = "transitive"\n'],
    ["warn", '[advisories]\nyanked = "deny"\nunmaintained = "warn"\n'],
    ["commented", '[advisories]\nyanked = "deny"\n# unmaintained = "all"\n'],
    ["misplaced", '[advisories]\nyanked = "deny"\n[licenses]\nunmaintained = "all"\n'],
    ["missing", '[advisories]\nyanked = "deny"\n'],
  ]) {
    const root = makeWorkspace({ denyToml });
    const { advisoryPolicyFindings } = scan(root);
    assert.deepEqual(
      advisoryPolicyFindings.map((finding) => finding.ruleId),
      ["RR-9.24"],
      name,
    );
  }
});
