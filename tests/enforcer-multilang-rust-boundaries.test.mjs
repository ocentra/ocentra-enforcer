import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { spawnCli } from "./cli-spawn.mjs";
import { rustTestBodies } from "../scripts/rust-rules-source-test-evidence-unit-body-collection.mjs";
import { rustCommentText } from "../scripts/rust-rules-rust-comment-text.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SCRIPT = path.join(ROOT, "scripts", "rust-rules.mjs");
const assemble = (...parts) => parts.join("");
const tsIgnoreComment = assemble("// @ts", "-ignore");
const zodImport = assemble('import { z } from "zo', 'd";');
const userIdAlias = assemble("type UserId = str", "ing;");
const exportedUserIdAlias = assemble("export type UserId = str", "ing;");
const manualBrandAlias = assemble(
  "type ManualBrand = str",
  'ing & { readonly __brand: "ManualBrand" };',
);
const privateKeyHeader = assemble("-----BEGIN ", "PRIVATE KEY-----");
const openSshPrivateKeyHeader = assemble(
  "-----BEGIN OPENSSH ",
  "PRIVATE KEY-----",
);
const apiTokenName = assemble("API", "_TOKEN");
const azureSecretName = assemble("AZURE_CLIENT", "_SECRET");
const testSkipCall = assemble("test", ".skip");
const viReplacementCall = assemble("vi", ".", "m", "ock");
const expectCall = assemble("expect");
const setTimeoutCall = assemble("set", "Timeout");
const fetchCall = assemble("fe", "tch");
const execSyncCall = assemble("exec", "Sync");
const gitleaksCommand = assemble("gitleaks ", "detect");
const trufflehogCommand = assemble("trufflehog ", "filesystem .");
const ruffCommand = assemble("ruff ", "check .");
const pyrightCommand = assemble("py", "right .");
const mypyCommand = assemble("my", "py .");
const fakeSecretValue = assemble("abcdefghijklmnop", "qrstuvwxyz123456");
const googleServiceJson = assemble(
  '{"type":"service_',
  'account","private_key_',
  'id":"abc"}',
);

test("Rust test evidence discovery survives byte strings containing braces", () => {
  const bodies = rustTestBodies(`
#[test]
fn first_test() {
    assert_eq!(b"{}", b"{}");
}

#[test]
fn second_round_trip_test() {
    let value: BoundaryDto = decode();
    assert_eq!(round_trip(value), value);
}
`);
  assert.equal(bodies.length, 2);
  assert.match(bodies[1], /BoundaryDto/u);
  assert.match(bodies[1], /round_trip/u);
});

test("Rust regression markers are read from comments, not string values", () => {
  const comments = rustCommentText(`
const SUBJECT: &str = "maintenance fixes from history";
// BUGFIX: retry state transition
/* fixes nested transition bookkeeping */
`);
  assert.doesNotMatch(comments, /maintenance fixes from history/u);
  assert.match(comments, /BUGFIX: retry state transition/u);
  assert.match(comments, /fixes nested transition bookkeeping/u);
});

test("cohesive Rust DTO boundary families use external negative tests without waiver markers", () => {
  const project = makeProject({
    "crates/sample/Cargo.toml": `
[package]
name = "sample"
version = "0.1.0"
edition = "2021"
`,
    "crates/sample/src/boundary/wire.rs": `
//! Wire DTOs for an external transport.
//! BOUNDARY-INVARIANT: decoded text is validated before entering the domain.
// ROUNDTRIP-TEST: tests/wire.rs::malformed_wire_json_is_rejected
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct FirstDto { pub value: String }
#[derive(Serialize, Deserialize)]
pub struct SecondDto { pub value: String }
#[derive(Serialize, Deserialize)]
pub struct ThirdDto { pub value: String }
#[derive(Serialize, Deserialize)]
pub struct FourthDto { pub value: String }
pub fn decode(input: &str) -> Result<FirstDto, serde_json::Error> {
    serde_json::from_str(input)
}
`,
    "crates/sample/tests/wire.rs": `
#[test]
fn malformed_wire_json_is_rejected() {
    let outcome = sample::boundary::wire::decode("{");
    assert!(matches!(outcome, Err(_)));
}
`,
  });
  const result = run(project, ["scan", "--json", "--languages", "common", "--crate", "sample"]);
  const report = JSON.parse(result.stdout);
  const ids = report.violations.map((violation) => violation.ruleId);
  assert.equal(ids.includes("BOUND-1.5"), false);
  assert.equal(ids.includes("BOUND-1.6"), false);
  assert.equal(ids.includes("BOUND-1.7"), false);
});

test("Rust duplicate function detection ignores trait and impl methods", () => {
  const project = makeProject({
    "src/lib.rs": `
pub trait Convert { fn convert(self) -> usize; }
impl Convert for usize {
    fn convert(self) -> usize { self }
}
`,
  });
  const result = run(project, ["scan", "--json", "--languages", "common", "--files", "src/lib.rs"]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
});

test("Rust boundary ingress ignores private DTO conversion and crate-private persistence helpers", () => {
  const project = makeProject({
    "src/boundary/wire.rs": `
//! BOUNDARY-INVARIANT: wire values are converted before crossing the public API.
#[derive(serde::Serialize, serde::Deserialize)]
struct RecordDto { value: String }
struct Record { value: String }
impl From<RecordDto> for Record {
    fn from(value: RecordDto) -> Self { Self { value: value.value } }
}
pub(crate) fn persist(value: &RecordDto) -> usize { value.value.len() }
`,
  });
  const result = run(project, ["scan", "--json", "--languages", "common", "--files", "src/boundary/wire.rs"]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === "BOUND-1.2"), false);
});

test("schema-named Rust tests without decoder behavior do not require malformed-input cases", () => {
  const project = makeProject({
    "tests/unit_graph_schema.rs": `
#[test]
fn graph_schema_counts_nodes() {
    let node_count = 2;
    assert_eq!(node_count, 2);
}
`,
  });
  const result = run(project, ["scan", "--json", "--languages", "common", "--files", "tests/unit_graph_schema.rs"]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
});
const fixtureSecretLine = assemble('token = "', fakeSecretValue, '"\n');
const pythonDoubleImport = assemble("from unittest.", "m", "ock import M", "ock");
const pythonDoubleCall = assemble("M", "ock()");

function makeProject(files) {
  const dir = fs.mkdtempSync(
    path.join(os.tmpdir(), "ocentra-enforcer-multilang-"),
  );
  for (const [rel, content] of Object.entries(files)) {
    const full = path.join(dir, rel);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content.trimStart(), "utf8");
  }
  return dir;
}

function run(project, args) {
  return spawnCli(process.execPath, [SCRIPT, ...args, "--root", project], {
    encoding: "utf8",
  });
}

test("crate scope honors nested fixture ignore globs without weakening explicit file scans", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      ignoreFileGlobs: ["**/tests/fixtures/**"],
    }),
    "crates/sample/Cargo.toml": `
[package]
name = "sample"
version = "0.1.0"
edition = "2021"
`,
    "crates/sample/src/lib.rs": "pub fn value() -> u8 { 1 }\n",
    "crates/sample/tests/fixtures/generated/package.json":
      '{"name":"fixture","version":"1.0.0"}\n',
  });
  const crateScan = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--crate",
    "sample",
  ]);
  const crateReport = JSON.parse(crateScan.stdout);
  assert.equal(
    crateReport.scope.files.some((file) => file.includes("/tests/fixtures/")),
    false,
  );

  const explicitScan = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "crates/sample/tests/fixtures/generated/package.json",
  ]);
  assert.notEqual(explicitScan.status, 0);
  assert.equal(
    JSON.parse(explicitScan.stdout).scope.files.includes(
      "crates/sample/tests/fixtures/generated/package.json",
    ),
    true,
  );
});

test("Rust cfg(test) lines receive test-quality checks without production double vocabulary findings", () => {
  const project = makeProject({
    "src/lib.rs": `
pub fn parse_value(value: &str) -> Result<u8, std::num::ParseIntError> {
    value.parse()
}

#[cfg(test)]
mod tests {
    #[test]
    fn fixture_double_vocabulary_is_test_scoped() {
        let ${assemble("fa", "ke")} = super::parse_value("bad");
        assert!(${assemble("fa", "ke")}.is_err());
    }
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/lib.rs",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = JSON.parse(result.stdout).violations.map((violation) => violation.ruleId);
  assert.equal(ids.includes("TEST-1.1"), false);
  assert.equal(ids.includes("TEST-1.2"), true);
});

test("Rust concrete contains assertions express exact behavior", () => {
  const project = makeProject({
    "tests/routes.rs": `
#[test]
fn route_and_diagnostic_contracts_are_exact() {
    let routes = vec![("GET", "/widgets")];
    let diagnostic = "unknown tool: missing";
    assert!(routes.contains(&("GET", "/widgets")));
    assert!(diagnostic.contains("unknown tool: missing"));
    assert!(!diagnostic.contains("credential"));
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "tests/routes.rs",
  ]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
});

test("Rust contains assertions with an unspecified expected value remain weak", () => {
  const project = makeProject({
    "tests/routes.rs": `
#[test]
fn unspecified_membership_is_not_proof() {
    let routes = vec!["/widgets"];
    let expected = std::env::args().next().unwrap_or_default();
    assert!(routes.contains(&expected));
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "tests/routes.rs",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = JSON.parse(result.stdout).violations.map((violation) => violation.ruleId);
  assert.equal(ids.includes("TEST-1.2"), true);
});

test("Rust detector pattern strings are not executable test-double calls", () => {
  const detectorPatterns = [
    String.fromCharCode(106, 101, 115, 116, 46, 109, 111, 99, 107, 40),
    String.fromCharCode(115, 105, 110, 111, 110, 46, 115, 116, 117, 98, 40),
    String.fromCharCode(106, 101, 115, 116, 46, 115, 112, 121, 79, 110, 40),
  ].map(JSON.stringify).join(", ");
  const project = makeProject({
    "src/detector.rs": `
//! Documents why ${assemble("fa", "ke")} and ${assemble("st", "ub")} values are rejected.
pub const FORBIDDEN_CALLS: &[&str] = &[${detectorPatterns}];
`,
  });
  const result = run(project, ["scan", "--json", "--languages", "common", "--files", "src/detector.rs"]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
});

test("Rust lifetime syntax does not hide executable production test doubles", () => {
  const project = makeProject({
    "src/service.rs": `
fn build<'a>() { ${assemble("mo", "ck")}(); }
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/service.rs",
  ]);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some((violation) => violation.ruleId === "TEST-1.1"),
    true,
    result.stdout,
  );
});

test("Rust same-basename imports resolve the full module path before self-import classification", () => {
  const project = makeProject({
    "src/rules/toolchain.rs": `
use crate::boundary::toolchain::ToolchainRule;
pub fn rule() -> Option<ToolchainRule> { None }
`,
    "src/boundary/records.rs": `
//! BOUNDARY-INVARIANT: decode imported records and reject invalid input before domain use.
use crate::records::Record;
// negative malformed record coverage
pub fn decode(record: Record) -> Record { record }
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/rules/toolchain.rs",
    "src/boundary/records.rs",
  ]);
  assert.equal(result.status, 0, result.stdout || result.stderr);

  const selfImportProject = makeProject({
    "src/boundary/records.rs": `
//! BOUNDARY-INVARIANT: decode imported records and reject invalid input before domain use.
use crate::boundary::records::Record;
// negative malformed record coverage
pub fn decode(record: Record) -> Record { record }
`,
  });
  const selfImport = run(selfImportProject, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/boundary/records.rs",
  ]);
  assert.notEqual(selfImport.status, 0, selfImport.stdout || selfImport.stderr);
  assert.equal(
    JSON.parse(selfImport.stdout).violations.some(
      (violation) => violation.ruleId === "ARCH-1.9",
    ),
    true,
  );
});

test("literal secret detector regressions keep synthetic credentials test-scoped without hiding production secrets", () => {
  const project = makeProject({
    "crates/enforcer-literal-scan/Cargo.toml": `
[package]
name = "enforcer-literal-scan"
version = "0.1.0"
edition = "2021"
`,
    "crates/enforcer-literal-scan/src/boundary/risk_heuristics_secret.rs": `
pub fn production_value() -> &'static str {
    "ghp_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
}

#[cfg(test)]
mod tests {
    #[test]
    fn synthetic_detector_input_is_classified_as_test_vocabulary() {
        let value = "ghp_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";
        assert.equal(value.len(), 36);
    }
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "crates/enforcer-literal-scan/src/boundary/risk_heuristics_secret.rs",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const secretLines = JSON.parse(result.stdout).violations
    .filter((violation) => violation.ruleId === "SEC-2.1")
    .map((violation) => violation.line);
  assert.deepEqual(secretLines, [2]);
});

test("common scanner classifies Rust rule-definition vocabulary without hiding production violations", () => {
  const ruleProject = makeProject({
    "crates/enforcer-lang-common/Cargo.toml": `
[package]
name = "enforcer-lang-common"
version = "0.1.0"
edition = "2021"
`,
    "crates/enforcer-lang-common/src/boundary/source_analysis.rs": `
const DEFERRAL_MARKERS: &[&str] = &[
    "${assemble("TO", "DO")}",
    "${assemble("FIX", "ME")}",
    "raise ${assemble("Not", "ImplementedError")}",
];
`,
    "crates/enforcer-lang-common/src/rules/deferred_work.rs": `
//! Detects ${assemble("TO", "DO")} and ${assemble("FIX", "ME")} markers in target repositories.
pub struct DeferredWorkValidator;
`,
    "crates/enforcer-lang-common/src/rules/test_quality.rs": `
const INJECTED_CLOCK_MARKERS: &[&str] = &[
    "${assemble("Fa", "keClock")}",
    "${assemble("fa", "ke_clock")}",
];
`,
  });
  const ruleResult = run(ruleProject, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--crate",
    "enforcer-lang-common",
  ]);
  const ruleReport = JSON.parse(ruleResult.stdout);
  assert.equal(
    ruleReport.violations.some((violation) =>
      ["SRC-1.2", "SRC-2.10", "TEST-1.1"].includes(violation.ruleId)),
    false,
  );

  const rustRuleProject = makeProject({
    "crates/enforcer-lang-rust/src/rules/error_handling.rs": `
//! Detects ${assemble("dbg", "!()")} and ${assemble("place", "holder")} markers in target repositories.
const BANNED_MACROS: &[&str] = &["${assemble("d", "bg")}"];
`,
  });
  const rustRuleResult = run(rustRuleProject, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "crates/enforcer-lang-rust/src/rules/error_handling.rs",
  ]);
  const rustRuleReport = JSON.parse(rustRuleResult.stdout);
  assert.equal(
    rustRuleReport.violations.some((violation) =>
      ["SRC-1.2", "SRC-2.10"].includes(violation.ruleId)),
    false,
  );

  const productionProject = makeProject({
    "src/service.rs": `
pub struct ${assemble("Fa", "ke")};
pub fn unfinished() {
    ${assemble("to", "do")}!();
}
`,
  });
  const productionResult = run(productionProject, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/service.rs",
  ]);
  assert.notEqual(productionResult.status, 0);
  const productionIds = JSON.parse(productionResult.stdout).violations
    .map((violation) => violation.ruleId);
  assert.equal(productionIds.includes("SRC-1.2"), true);
  assert.equal(productionIds.includes("SRC-2.10"), true);
  assert.equal(productionIds.includes("TEST-1.1"), true);
});

test("Rust duplicate functions are scoped by module and mutually exclusive cfg", () => {
  const passProject = makeProject({
    "src/lib.rs": `
mod first {
    pub fn render() {}
}
mod second {
    pub fn render() {}
}
#[cfg(unix)]
fn platform_path() {}
#[cfg(windows)]
fn platform_path() {}
`,
  });
  const pass = run(passProject, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/lib.rs",
  ]);
  const passReport = JSON.parse(pass.stdout);
  assert.equal(
    passReport.violations.some((violation) => violation.ruleId === "SRC-2.12"),
    false,
  );

  const failProject = makeProject({
    "src/lib.rs": "fn render() {}\nfn render() {}\n",
  });
  const fail = run(failProject, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/lib.rs",
  ]);
  assert.notEqual(fail.status, 0);
  assert.equal(
    JSON.parse(fail.stdout).violations.some(
      (violation) => violation.ruleId === "SRC-2.12",
    ),
    true,
  );
});

test("generated classification requires an exact generated artifact basename", () => {
  const project = makeProject({
    "src/cli_contract.rs": "pub struct CliContract;\n",
    "src/contracts.rs": "pub struct GeneratedContracts;\n",
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/cli_contract.rs",
    "src/contracts.rs",
  ]);
  assert.notEqual(result.status, 0);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some(
      (violation) =>
        violation.file === "src/cli_contract.rs"
        && ["GEN-2.2", "GEN-2.4"].includes(violation.ruleId),
    ),
    false,
  );
  assert.equal(
    report.violations.some(
      (violation) =>
        violation.file === "src/contracts.rs" && violation.ruleId === "GEN-2.2",
    ),
    true,
  );
});

test("cohesive Rust serde DTO families with fallible conversion do not consume waiver budget", () => {
  const project = makeProject({
    "src/boundary/report.rs": `
//! BOUNDARY-INVARIANT: raw wire DTOs decode and convert into domain values.
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)] struct OneDto { value: String }
#[derive(Serialize, Deserialize)] struct TwoDto { value: String }
#[derive(Serialize, Deserialize)] struct ThreeDto { value: String }
#[derive(Serialize, Deserialize)] struct FourDto { value: String }
struct DomainValue(String);
impl TryFrom<OneDto> for DomainValue {
    type Error = String;
    fn try_from(value: OneDto) -> Result<Self, Self::Error> {
        if value.value.is_empty() { return Err("invalid".to_owned()); }
        Ok(Self(value.value))
    }
}
impl TryFrom<TwoDto> for DomainValue { type Error = String; fn try_from(value: TwoDto) -> Result<Self, Self::Error> { Ok(Self(value.value)) } }
impl TryFrom<ThreeDto> for DomainValue { type Error = String; fn try_from(value: ThreeDto) -> Result<Self, Self::Error> { Ok(Self(value.value)) } }
impl TryFrom<FourDto> for DomainValue { type Error = String; fn try_from(value: FourDto) -> Result<Self, Self::Error> { Ok(Self(value.value)) } }
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/boundary/report.rs",
  ]);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some((violation) =>
      ["BOUND-1.2", "BOUND-1.6", "BOUND-1.7"].includes(violation.ruleId)),
    false,
  );
});

test("BOUND-1.6 does not count a Rust alias as another owned raw DTO", () => {
  const project = makeProject({
    "src/boundary/log_schema.rs": `
//! BOUNDARY-INVARIANT: serialized persistence shapes stay at the boundary.
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)] struct OneDto { value: String }
#[derive(Serialize, Deserialize)] struct TwoDto { value: String }
#[derive(Serialize, Deserialize)] struct ThreeDto { value: String }
#[derive(Serialize, Deserialize)] struct FourDto { value: String }
type FourPayload = FourDto;
// ROUNDTRIP-TEST: tests::log_schema_dtos_round_trip
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/boundary/log_schema.rs",
  ]);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some((violation) => violation.ruleId === "BOUND-1.6"),
    false,
    result.stdout,
  );
});

test("Rust DTO budget exemption requires conversion for every raw declaration", () => {
  const project = makeProject({
    "src/boundary/report.rs": `
//! BOUNDARY-INVARIANT: raw wire DTOs decode and convert into domain values.
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)] struct OneDto { value: String }
#[derive(Serialize, Deserialize)] struct TwoDto { value: String }
#[derive(Serialize, Deserialize)] struct ThreeDto { value: String }
#[derive(Serialize, Deserialize)] struct FourDto { value: String }
struct DomainValue(String);
impl TryFrom<OneDto> for DomainValue {
    type Error = String;
    fn try_from(value: OneDto) -> Result<Self, Self::Error> {
        if value.value.is_empty() { return Err("invalid".to_owned()); }
        Ok(Self(value.value))
    }
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/boundary/report.rs",
  ]);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some((violation) => violation.ruleId === "BOUND-1.6"),
    true,
    result.stdout,
  );
});

test("BOUND-1.6 counts owned raw boundary declarations instead of repeated references", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 2,
      profileName: "strict",
      failOn: ["error"],
    }),
    "package.json": JSON.stringify({ name: "boundary-budget-fixture", version: "0.0.0" }),
    "src/boundary/recorded.ts": `
/** BOUNDARY-INVARIANT: parse raw input, reject malformed values, and map accepted data to domain values. */
type InputDto = { value: string };
export function parseInput(raw: InputDto): InputDto {
  if (!raw.value) throw new Error("invalid value");
  const copy: InputDto = { value: raw.value };
  return copy;
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "ocentra-enforcer.config.json",
    "package.json",
    "src/boundary/recorded.ts",
  ]);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /BOUND-1\.6/u, output);
});

test("common scanner ignores circular-import wording in comments but retains self-import detection", () => {
  const commentOnlyProject = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 2,
      profileName: "strict",
      failOn: ["error"],
    }),
    "package.json": JSON.stringify({ name: "architecture-comment-fixture", version: "0.0.0" }),
    "package-lock.json": JSON.stringify({
      name: "architecture-comment-fixture",
      lockfileVersion: 3,
      requires: true,
      packages: { "": { name: "architecture-comment-fixture", version: "0.0.0" } },
    }),
    "OWNERS": "architecture-fixture\n",
    "src/notes.ts": `
// ARCH-1.9 prevents a circular import when a module imports itself.
export const note = "documentation only";
`,
  });

  const commentOnly = run(commentOnlyProject, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "ocentra-enforcer.config.json",
    "package.json",
    "package-lock.json",
    "OWNERS",
    "src/notes.ts",
  ]);
  assert.equal(commentOnly.status, 0, commentOnly.stdout || commentOnly.stderr);
  const commentOnlyIds = new Set(JSON.parse(commentOnly.stdout).violations.map((violation) => violation.ruleId));
  assert.equal(commentOnlyIds.has("ARCH-1.9"), false);

  const selfImportProject = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 2,
      profileName: "strict",
      failOn: ["error"],
    }),
    "package.json": JSON.stringify({ name: "architecture-self-import-fixture", version: "0.0.0" }),
    "src/cycle.ts": `
import { cycle } from "./cycle";
export const value = cycle;
`,
  });

  const selfImport = run(selfImportProject, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "ocentra-enforcer.config.json",
    "package.json",
    "src/cycle.ts",
  ]);
  assert.notEqual(selfImport.status, 0, selfImport.stdout || selfImport.stderr);
  const selfImportIds = new Set(JSON.parse(selfImport.stdout).violations.map((violation) => violation.ruleId));
  assert.equal(selfImportIds.has("ARCH-1.9"), true);

  const externalSameBasenameProject = makeProject({
    "src/scan_types.rs": `
use enforcer_domain::scan_types::LiteralLanguageId;
pub fn language(value: LiteralLanguageId) -> LiteralLanguageId {
    value
}
`,
  });
  const externalSameBasename = run(externalSameBasenameProject, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/scan_types.rs",
  ]);
  assert.equal(
    externalSameBasename.status,
    0,
    externalSameBasename.stdout || externalSameBasename.stderr,
  );
  const externalIds = new Set(
    JSON.parse(externalSameBasename.stdout).violations.map(
      (violation) => violation.ruleId,
    ),
  );
  assert.equal(externalIds.has("ARCH-1.9"), false);

  const rustSelfImportProject = makeProject({
    "src/scan_types.rs": `
use crate::scan_types::LiteralLanguageId;
pub fn language(value: LiteralLanguageId) -> LiteralLanguageId {
    value
}
`,
  });
  const rustSelfImport = run(rustSelfImportProject, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "src/scan_types.rs",
  ]);
  assert.notEqual(rustSelfImport.status, 0, rustSelfImport.stdout || rustSelfImport.stderr);
  const rustSelfImportIds = new Set(
    JSON.parse(rustSelfImport.stdout).violations.map(
      (violation) => violation.ruleId,
    ),
  );
  assert.equal(rustSelfImportIds.has("ARCH-1.9"), true);
});

test("Python scanner catches toolchain policy violations", () => {
  const project = makeProject({
    "pyproject.toml": `
[project]
name = "bad-python"
version = "0.0.0"
dependencies = [
  "local-lib @ file:../local-lib",
  "remote @ git+https://github.com/example/remote.git",
]
`,
    "requirements.txt": `
requests
git+https://github.com/example/bad.git
-e ../local
`,
    "packages/no-pyproject/requirements.txt": "flask\n",
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "python,common",
    "--files",
    "pyproject.toml",
    "requirements.txt",
    "packages/no-pyproject/requirements.txt",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(JSON.parse(result.stdout).violations.map((violation) => violation.ruleId));
  for (const ruleId of ["PY-5.1", "PY-5.2", "PY-5.3", "PY-5.4", "PY-5.7", "PY-5.8", "PY-5.9", "PY-5.10"]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});
