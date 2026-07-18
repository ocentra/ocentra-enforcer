import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { DEFAULT_CONFIG } from "../src/rule-metadata.mjs";
import {
  normalizeConfig,
} from "../scripts/rust-rules-scan-core.mjs";
import { scanRustFile } from "../scripts/rust-rules-source-scan.mjs";

function performanceFixture() {
  const lines = [
    "async fn scan_values() {",
    "    let _ = std::fs::read(\"input\");",
    "    for value in values { compute(value); }",
    "    loop { poll().await; }",
    "}",
    "struct ApiSecret { value: String }",
    "fn retry_values() { for retry in 0..3 { run(retry); } }",
  ];
  for (let index = 0; index < 1_500; index += 1) {
    lines.push(`let ordinary_${index} = ${index};`);
  }
  return `${lines.join("\n")}\n`;
}

function parserHeavyFixture() {
  const lines = [];
  for (let index = 0; index < 40; index += 1) {
    lines.push(`pub fn parse_value_${index}(input: &str) -> usize { input.len() }`);
  }
  return `${lines.join("\n")}\n`;
}

function writeParserEvidenceCrate(root) {
  fs.writeFileSync(path.join(root, "Cargo.toml"), "[package]\nname = \"parser-evidence-performance\"\nversion = \"0.1.0\"\n", "utf8");
  const testsRoot = path.join(root, "tests");
  fs.mkdirSync(testsRoot, { recursive: true });
  for (let index = 0; index < 12; index += 1) {
    fs.writeFileSync(
      path.join(testsRoot, `evidence_${index}.rs`),
      `#[test]\nfn rejects_invalid_${index}() { assert!(parse_other_${index}(\"invalid\").is_err()); }\n`,
      "utf8",
    );
  }
}

test("cached whole-file Rust predicates preserve findings and remove quadratic work", {
  timeout: 30_000,
}, () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-rust-performance-"));
  const filePath = path.join(root, "large_test.rs");
  fs.writeFileSync(filePath, performanceFixture(), "utf8");
  const config = normalizeConfig(DEFAULT_CONFIG);
  const legacyTimings = {};
  const cachedTimings = {};

  const legacy = scanRustFile(root, filePath, config, {
    cacheFilePredicates: false,
    timings: legacyTimings,
  });
  const cached = scanRustFile(root, filePath, config, {
    timings: cachedTimings,
  });

  assert.deepEqual(cached, legacy);
  assert.equal(cachedTimings.lineRules < legacyTimings.lineRules, true, JSON.stringify({
    cached: cachedTimings,
    legacy: legacyTimings,
  }));
});

test("cached crate evidence preserves finding bytes and avoids per-parser test-tree reparsing", {
  timeout: 30_000,
}, () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-proof-evidence-performance-"));
  writeParserEvidenceCrate(root);
  const filePath = path.join(root, "src", "lib.rs");
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, parserHeavyFixture(), "utf8");
  const config = normalizeConfig(DEFAULT_CONFIG);
  const legacyTimings = {};
  const cachedTimings = {};

  const legacy = scanRustFile(root, filePath, config, {
    cacheProofEvidence: false,
    timings: legacyTimings,
  });
  const cached = scanRustFile(root, filePath, config, { timings: cachedTimings });

  assert.equal(JSON.stringify(cached), JSON.stringify(legacy));
  assert.equal(cachedTimings.lateRules < legacyTimings.lateRules, true, JSON.stringify({
    cached: cachedTimings,
    legacy: legacyTimings,
  }));
});

test("workspace scans read each crate test evidence tree once without changing finding bytes", {
  timeout: 30_000,
}, () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-crate-evidence-performance-"));
  writeParserEvidenceCrate(root);
  const config = normalizeConfig(DEFAULT_CONFIG);
  const sourceFiles = [];
  for (let index = 0; index < 6; index += 1) {
    const filePath = path.join(root, "src", `parser_${index}.rs`);
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, parserHeavyFixture(), "utf8");
    sourceFiles.push(filePath);
  }

  const legacy = sourceFiles.map((filePath) => scanRustFile(root, filePath, config));
  const proofEvidenceCache = new Map();
  const cached = sourceFiles.map((filePath) =>
    scanRustFile(root, filePath, config, { proofEvidenceCache }));

  assert.equal(JSON.stringify(cached), JSON.stringify(legacy));
  assert.equal(proofEvidenceCache.size, 1);
});
