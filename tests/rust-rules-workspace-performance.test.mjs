import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { DEFAULT_CONFIG } from "../src/rule-metadata.mjs";
import { normalizeConfig } from "../scripts/rust-rules-scan-core.mjs";
import { runScanner } from "../scripts/rust-rules-cargo-runner.mjs";
import { balancedWorkspacePartitions } from "../scripts/rust-rules-workspace-partitioning.mjs";
import {
  parallelRustFileFindings,
  serialRustFileFindings,
} from "../scripts/rust-rules-workspace-scan.mjs";

function writeWorkspace(root, crateCount = 4, filesPerCrate = 10) {
  fs.writeFileSync(
    path.join(root, "Cargo.toml"),
    '[workspace]\nmembers = ["crates/*"]\nresolver = "2"\n',
    "utf8",
  );
  fs.writeFileSync(path.join(root, "Cargo.lock"), "version = 4\n", "utf8");
  const files = [];
  for (let crateIndex = 0; crateIndex < crateCount; crateIndex += 1) {
    const crateRoot = path.join(root, "crates", `crate-${crateIndex}`);
    fs.mkdirSync(path.join(crateRoot, "src"), { recursive: true });
    fs.writeFileSync(
      path.join(crateRoot, "Cargo.toml"),
      `[package]\nname = "scan-perf-${crateIndex}"\nversion = "0.1.0"\n`,
      "utf8",
    );
    for (let fileIndex = 0; fileIndex < filesPerCrate; fileIndex += 1) {
      const filePath = path.join(crateRoot, "src", `module_${fileIndex}.rs`);
      const lines = [
        `pub fn parse_${crateIndex}_${fileIndex}(input: &str) -> usize {`,
        "    input.len()",
        "}",
      ];
      for (let lineIndex = 0; lineIndex < 350; lineIndex += 1) {
        lines.push(`fn ordinary_${lineIndex}() -> usize { ${lineIndex} }`);
      }
      fs.writeFileSync(filePath, `${lines.join("\n")}\n`, "utf8");
      files.push(filePath);
    }
  }
  return files;
}

function writeCrossFileEvidenceCrates(root) {
  const files = [];
  for (const suffix of ["Alpha", "Beta"]) {
    const crateRoot = path.join(root, "crates", suffix.toLowerCase());
    const boundaryRoot = path.join(crateRoot, "src", "boundary");
    const testsRoot = path.join(crateRoot, "tests");
    fs.mkdirSync(boundaryRoot, { recursive: true });
    fs.mkdirSync(testsRoot, { recursive: true });
    fs.writeFileSync(
      path.join(crateRoot, "Cargo.toml"),
      `[package]\nname = "evidence-${suffix.toLowerCase()}"\nversion = "0.1.0"\n`,
      "utf8",
    );
    const sourcePath = path.join(boundaryRoot, "wire.rs");
    fs.writeFileSync(
      sourcePath,
      `#[derive(serde::Serialize, serde::Deserialize)]\npub struct Evidence${suffix}Dto {\n    pub value: String,\n}\n`,
      "utf8",
    );
    fs.writeFileSync(
      path.join(testsRoot, "round_trip.rs"),
      `#[test]\nfn evidence_${suffix.toLowerCase()}_dto_round_trip() {\n    let value = Evidence${suffix}Dto { value: String::new() };\n    let encoded = serde_json::to_string(&value).unwrap();\n    let decoded: Evidence${suffix}Dto = serde_json::from_str(&encoded).unwrap();\n    assert_eq!(decoded.value, value.value);\n}\n`,
      "utf8",
    );
    files.push(sourcePath);
  }
  return files;
}

test("workspace partitions keep every Cargo crate together exactly once", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-workspace-partition-"));
  const files = writeWorkspace(root, 5, 3);
  const partitions = balancedWorkspacePartitions(root, files, 3);
  const seen = new Set();
  const crateOwners = new Map();
  partitions.forEach((partition, partitionIndex) => {
    for (const entry of partition.entries) {
      assert.equal(seen.has(entry.index), false);
      seen.add(entry.index);
      const owner = path.dirname(path.dirname(entry.filePath));
      assert.equal(crateOwners.get(owner) ?? partitionIndex, partitionIndex);
      crateOwners.set(owner, partitionIndex);
    }
  });
  assert.equal(seen.size, files.length);
});

test("the actual workspace runner is byte-equivalent in parallel and serial modes", {
  timeout: 60_000,
}, (context) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-workspace-equivalence-"));
  const files = writeWorkspace(root);
  const config = normalizeConfig(DEFAULT_CONFIG);
  const scope = { mode: "all", files };
  const serialStartedAt = performance.now();
  const serial = runScanner(root, config, scope, { forceSerial: true });
  const serialMs = performance.now() - serialStartedAt;
  const parallelStartedAt = performance.now();
  const parallel = runScanner(root, config, scope, { workerCount: 4 });
  const parallelMs = performance.now() - parallelStartedAt;
  assert.equal(JSON.stringify(parallel), JSON.stringify(serial));
  context.diagnostic(JSON.stringify({ serialMs, parallelMs }));
});

test("parallel crate groups preserve cross-file DTO round-trip evidence", {
  timeout: 30_000,
}, () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-workspace-evidence-"));
  const files = writeCrossFileEvidenceCrates(root);
  const config = normalizeConfig(DEFAULT_CONFIG);
  const serial = serialRustFileFindings(root, config, files);
  const parallel = parallelRustFileFindings(root, config, files, 2);
  assert.equal(JSON.stringify(parallel), JSON.stringify(serial));
  const findings = parallel.flatMap((row) => row.findings);
  assert.equal(findings.some((finding) => finding.ruleId === "RR-14.25"), false);
});

test("representative workspace parallel path completes within the regression budget", {
  timeout: 60_000,
}, () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-workspace-budget-"));
  const files = writeWorkspace(root);
  const config = normalizeConfig(DEFAULT_CONFIG);
  const startedAt = performance.now();
  const results = parallelRustFileFindings(root, config, files, 4);
  const elapsedMs = performance.now() - startedAt;
  assert.equal(results.length, files.length);
  assert.equal(elapsedMs < 30_000, true, `parallel workspace fixture took ${elapsedMs}ms`);
});
