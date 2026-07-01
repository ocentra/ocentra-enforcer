import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const FIXTURE_ROOT = path.join(ROOT, "tests", "fixtures", "enforcer");
const SCRIPT = path.join(ROOT, "scripts", "rust-rules.mjs");

const REQUIRED_DIRS = [
  "policy",
  "rust",
  "typescript",
  "python",
  "common",
  "ci",
  "docs",
];

const REQUIRED_FIXTURES = [
  "policy/cfg-1.1-failon-empty.fail.json",
  "policy/cfg-1.2-disable-immutable.fail.json",
  "policy/cfg-1.3-downgrade-immutable.fail.json",
  "policy/cfg-1.4-allow-unsafe-no-waiver.fail.json",
  "policy/waiver-1.1-missing-owner.fail.json",
  "policy/waiver-1.3-expired.fail.json",
  "policy/waiver-1.4-immutable-waiver.fail.json",
  "policy/enf-1.2-missing-doc-anchor.fail.json",
  "policy/enf-1.3-unregistered-scanner-rule.fail.mjs",
  "rust/rr-4.7-result-string.fail.rs",
  "rust/rr-4.8-static-str-error.fail.rs",
  "rust/rr-4.9-err-literal.fail.rs",
  "rust/rr-4.11-map-err-to-string.fail.rs",
  "rust/rr-6.27-asref-str-param.fail.rs",
  "rust/rr-6.28-into-string-param.fail.rs",
  "rust/rr-6.31-vec-string-api.fail.rs",
  "rust/rr-6.32-hashmap-string-api.fail.rs",
  "rust/rr-6.37-multiple-bool-state.fail.rs",
  "rust/rr-6.43-public-newtype-field.fail.rs",
  "rust/rr-14.16-domain-deserialize.fail.rs",
  "rust/rr-14.18-serde-untagged-no-justification.fail.rs",
  "rust/rr-3.16-transmute.fail.rs",
  "rust/rr-3.21-static-mut.fail.rs",
  "rust/rr-8.18-untracked-tokio-spawn.fail.rs",
  "rust/rr-8.20-unbounded-channel.fail.rs",
  "rust/rr-12.22-is-ok-weak-assert.fail.rs",
  "rust/rr-12.23-is-some-weak-assert.fail.rs",
  "typescript/ts-6.1-any.fail.ts",
  "typescript/ts-6.3-as-cast.fail.ts",
  "typescript/ts-6.4-double-cast.fail.ts",
  "typescript/ts-6.5-non-null.fail.ts",
  "typescript/ts-6.7-raw-string-id.fail.ts",
  "typescript/ts-6.10-record-string-domain.fail.ts",
  "typescript/ts-6.13-default-export.fail.ts",
  "typescript/ts-6.14-index-barrel.fail.ts",
  "typescript/ts-6.18-process-env-domain.fail.ts",
  "typescript/ts-6.19-json-parse-domain.fail.ts",
  "typescript/ts-6.22-floating-promise.fail.ts",
  "typescript/ts-6.24-console-log.fail.ts",
  "typescript/ts-6.25-throw-string-error.fail.ts",
  "typescript/ts-7.1-tsconfig-not-strict.fail.json",
  "python/py-4.1-any.fail.py",
  "python/py-4.2-untyped-def.fail.py",
  "python/py-4.10-mutable-default.fail.py",
  "python/py-4.11-broad-except.fail.py",
  "python/py-4.12-bare-except.fail.py",
  "python/py-4.14-print.fail.py",
  "python/py-4.15-runtime-assert.fail.py",
  "python/py-4.17-subprocess-shell.fail.py",
  "python/py-4.23-naive-datetime.fail.py",
  "python/py-4.25-requests-no-timeout.fail.py",
  "python/py-4.29-wildcard-import.fail.py",
  "python/py-6.2-weak-assert.fail.py",
  "common/sec-1.1-secret.fail.txt",
  "ci/ci-1.12-ignore-exit-code.fail.yml",
  "docs/docenf-1.1-missing-sections.fail.md",
];

test("enforcer fixture tree has deterministic language and policy slices", () => {
  for (const dir of REQUIRED_DIRS) {
    assert.equal(fs.existsSync(path.join(FIXTURE_ROOT, dir)), true, `${dir} exists`);
  }
  for (const rel of REQUIRED_FIXTURES) {
    const full = path.join(FIXTURE_ROOT, rel);
    assert.equal(fs.existsSync(full), true, `${rel} exists`);
    const text = fs.readFileSync(full, "utf8");
    assert.match(text, /\b(?:RR|TS|PY|SEC|CI|DOCENF|CFG|ENF|WAIVER)-[0-9]+\.[0-9]+\b/u);
  }
});

test("language fail fixtures emit their encoded rule IDs", () => {
  const project = makeTempProject();
  const rustFiles = REQUIRED_FIXTURES.filter((rel) => rel.startsWith("rust/"));
  const tsFiles = REQUIRED_FIXTURES.filter((rel) => rel.startsWith("typescript/"));
  const pyFiles = REQUIRED_FIXTURES.filter((rel) => rel.startsWith("python/"));

  copyFixtureGroup(project, rustFiles, "rust");
  copyFixtureGroup(project, tsFiles, "typescript");
  copyFixtureGroup(project, pyFiles, "python");

  assertFixtureRules(project, rustFiles, ["scan", "--json", "--languages", "rust", "--files", "rust"]);
  assertFixtureRules(project, tsFiles, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "typescript",
    "tsconfig.json",
  ]);
  assertFixtureRules(project, pyFiles, [
    "scan",
    "--json",
    "--languages",
    "python,common",
    "--files",
    "python",
  ]);
});

test("common and CI fail fixtures emit their encoded rule IDs", () => {
  const project = makeTempProject();
  copyFixture(project, "common/sec-1.1-secret.fail.txt", "common/sec-1.1-secret.fail.txt");
  copyFixture(project, "ci/ci-1.12-ignore-exit-code.fail.yml", ".github/workflows/ci.yml");
  fs.writeFileSync(
    path.join(project, "package.json"),
    JSON.stringify(
      {
        name: "fixture-project",
        version: "0.0.0",
        scripts: {
          "ci:local": "npm test && node scripts/rust-rules.mjs check ci-integrity --root .",
          test: "node --test",
        },
      },
      null,
      2,
    ),
  );

  assertFixtureRules(project, ["common/sec-1.1-secret.fail.txt"], [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    "common/sec-1.1-secret.fail.txt",
  ]);
  assertFixtureRules(project, ["ci/ci-1.12-ignore-exit-code.fail.yml"], [
    "check",
    "ci-integrity",
    "--json",
  ]);
});

function makeTempProject() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "ocentra-enforcer-fixtures-"));
}

function copyFixtureGroup(project, rels, language) {
  rels.forEach((rel, index) => {
    const ext = path.extname(rel);
    let target = path.join(language, `fixture-${index}${ext}`);
    if (rel.endsWith("ts-6.14-index-barrel.fail.ts")) target = path.join(language, "index.ts");
    if (rel.endsWith("ts-7.1-tsconfig-not-strict.fail.json")) target = "tsconfig.json";
    if (rel.endsWith("py-6.2-weak-assert.fail.py")) target = path.join(language, "tests", "test_fixture.py");
    copyFixture(project, rel, target);
  });
}

function copyFixture(project, sourceRel, targetRel) {
  const target = path.join(project, targetRel);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(path.join(FIXTURE_ROOT, sourceRel), target);
}

function assertFixtureRules(project, fixtureRels, args) {
  const result = spawnSync(process.execPath, [SCRIPT, ...args, "--root", project], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  const actualIds = new Set(report.violations.map((violation) => violation.ruleId));
  for (const rel of fixtureRels) {
    const ruleId = ruleIdFromFixture(rel);
    assert.equal(actualIds.has(ruleId), true, `${ruleId} emitted for ${rel}`);
  }
}

function ruleIdFromFixture(rel) {
  const name = path.basename(rel);
  const match = name.match(/^([a-z]+-\d+\.\d+)/iu);
  assert.ok(match, `fixture name has rule id: ${rel}`);
  return match[1].toUpperCase();
}
