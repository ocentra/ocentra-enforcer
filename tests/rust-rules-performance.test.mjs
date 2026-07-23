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
import { maskRustCode } from "../scripts/rust-rules-path-core.mjs";
import { propertyTestBodies } from "../scripts/rust-rules-source-test-evidence-property-bodies.mjs";
import { roundTripFactoryDescriptors } from "../scripts/rust-rules-source-test-evidence-roundtrip-associated-descriptors.mjs";
import { roundTripHelperDescriptors } from "../scripts/rust-rules-source-test-evidence-roundtrip-helper-validation.mjs";
import { roundTripPersistenceDescriptors } from "../scripts/rust-rules-source-test-evidence-persistence-descriptors.mjs";
import { rustTestBodies } from "../scripts/rust-rules-source-test-evidence-unit-body-collection.mjs";
import { applyBoundaryTransportRules } from "../scripts/rust-rules-source-late-boundaries.mjs";
import { applyProofEvidenceRules } from "../scripts/rust-rules-source-late-test-evidence.mjs";

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
  // Keep enough ordinary lines to amplify the repeated whole-source predicates
  // used by the uncached path above normal timer noise on CI runners.
  for (let index = 0; index < 10_000; index += 1) {
    lines.push(`let ordinary_${index} = ${index};`);
  }
  return `${lines.join("\n")}\n`;
}

function parserHeavyFixture(parserCount = 40) {
  const lines = [];
  for (let index = 0; index < parserCount; index += 1) {
    lines.push(`pub fn parse_value_${index}(input: &str) -> usize { input.len() }`);
  }
  return `${lines.join("\n")}\n`;
}

function writeParserEvidenceCrate(root, evidenceCount = 12) {
  fs.writeFileSync(path.join(root, "Cargo.toml"), "[package]\nname = \"parser-evidence-performance\"\nversion = \"0.1.0\"\n", "utf8");
  const testsRoot = path.join(root, "tests");
  fs.mkdirSync(testsRoot, { recursive: true });
  for (let index = 0; index < evidenceCount; index += 1) {
    fs.writeFileSync(
      path.join(testsRoot, `evidence_${index}.rs`),
      index === 0
        ? "#[test]\nfn parse_value_0_rejects_invalid() { assert_eq!(parse_value_0(\"invalid\"), 7); }\n"
        : `#[test]\nfn rejects_invalid_${index}() { assert!(parse_other_${index}(\"invalid\").is_err()); }\n`,
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
  // Individual phases include scheduler, filesystem, and parser noise on
  // hosted runners.  Compare the complete scan instead of asserting that a
  // single phase wins on every OS; the cache must preserve findings and reduce
  // total work for this representative fixture.
  assert.equal(cachedTimings.total < legacyTimings.total, true, JSON.stringify({
    phase: "total",
    cached: cachedTimings,
    legacy: legacyTimings,
  }));
});

test("evidence collectors preserve outputs when a crate-cached Rust mask is supplied", () => {
  const source = `
#[test]
fn parser_rejects_invalid() { assert!(parse_value("invalid").is_err()); }
impl ValueDto { fn new() -> Self { Self {} } }
fn save_value(value: ValueDto) { let _ = serde_json::to_string(&value); }
fn load_value() -> ValueDto { serde_json::from_str("{}").unwrap() }
`;
  const masked = maskRustCode(source);
  for (const collector of [
    rustTestBodies,
    propertyTestBodies,
    roundTripFactoryDescriptors,
    roundTripHelperDescriptors,
    roundTripPersistenceDescriptors,
  ]) {
    assert.deepEqual(collector(source, masked), collector(source));
  }
});

test("boundary DTO checks reuse the proof evidence context without changing findings", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-boundary-evidence-context-"));
  fs.writeFileSync(path.join(root, "Cargo.toml"), "[package]\nname = \"boundary-evidence-context\"\nversion = \"0.1.0\"\n", "utf8");
  const filePath = path.join(root, "src", "wire.rs");
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  const source = "#[derive(serde::Serialize, serde::Deserialize)]\npub struct ValueDto { pub value: String }\n";
  fs.writeFileSync(filePath, source, "utf8");
  fs.mkdirSync(path.join(root, "tests"), { recursive: true });
  fs.writeFileSync(path.join(root, "tests", "round_trip.rs"), "#[test]\nfn value_dto_round_trip() { let value = ValueDto { value: String::new() }; let encoded = serde_json::to_string(&value).unwrap(); let decoded: ValueDto = serde_json::from_str(&encoded).unwrap(); assert_eq!(decoded.value, value.value); }\n", "utf8");
  const proofEvidenceCache = new Map();
  const context = {
    root,
    filePath,
    source,
    masked: maskRustCode(source),
    originalLines: source.split(/\r?\n/u),
    violations: [],
    isBoundary: false,
    isConfigurationBoundary: false,
    isTestSource: false,
    proofEvidenceCache,
  };
  applyProofEvidenceRules(context);
  const cachedContext = proofEvidenceCache.get(`evidence:${root}`)?.context;
  assert.strictEqual(context.evidenceContext, cachedContext);
  const beforeBoundary = [...context.violations];
  applyBoundaryTransportRules(context);
  assert.equal(context.violations.some((finding) => finding.ruleId === "RR-14.25"), false);
  assert.deepEqual(context.violations.slice(0, beforeBoundary.length), beforeBoundary);
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
  const evidenceContext = proofEvidenceCache.get(`evidence:${root}`)?.context;
  assert.equal(evidenceContext.parserTestBodiesByTarget.get("parse_value_0")?.length, 1);
  assert.equal(evidenceContext.parserTestBodiesByTarget.get("parse_value_39")?.length ?? 0, 0);
  assert.equal(proofEvidenceCache.size, 1);
});

test("one heavy crate keeps indexed parser evidence byte-equivalent for positive and negative targets", {
  timeout: 30_000,
}, () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "enforcer-parser-index-skew-"));
  writeParserEvidenceCrate(root, 48);
  const filePath = path.join(root, "src", "lib.rs");
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, parserHeavyFixture(32), "utf8");
  const config = normalizeConfig(DEFAULT_CONFIG);
  const legacy = scanRustFile(root, filePath, config, { cacheProofEvidence: false });
  const proofEvidenceCache = new Map();
  const indexed = scanRustFile(root, filePath, config, { proofEvidenceCache });

  assert.equal(JSON.stringify(indexed), JSON.stringify(legacy));
  assert.equal(indexed.some((finding) => finding.ruleId === "RR-12.16" && finding.detail.includes("parse_value_0 lacks")), false);
  assert.equal(indexed.some((finding) => finding.ruleId === "RR-12.16" && finding.detail.includes("parse_value_31 lacks")), true);
  assert.equal(proofEvidenceCache.size, 1);
  assert.equal(proofEvidenceCache.get(`evidence:${root}`)?.context?.parserTestBodiesByTarget instanceof Map, true);
});
