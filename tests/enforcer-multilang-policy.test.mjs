import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { spawnCli } from "./cli-spawn.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SCRIPT = path.join(ROOT, "scripts", "rust-rules.mjs");
const assemble = (...parts) => parts.join("");
const exportedUserIdAlias = assemble("export type UserId = str", "ing;\n");
const tsIgnoreComment = assemble("// @ts", "-ignore");

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

test("no-naked-domain-strings check covers Rust, TypeScript, and Python rule docs", () => {
  const project = makeProject({
    "src/lib.rs": `
pub fn find_user(id: String) -> String {
    id
}
`,
    "src/index.ts": exportedUserIdAlias,
    "src/model.py": "UserId = str\n",
  });
  const result = run(project, [
    "check",
    "no-naked-domain-strings",
    "--json",
    "--files",
    "src/lib.rs",
    "src/index.ts",
    "src/model.py",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  const ids = new Set(report.violations.map((violation) => violation.ruleId));
  assert.equal(ids.has("RR-6.1"), true);
  assert.equal(ids.has("TS-1.3"), true);
  assert.equal(ids.has("PY-1.3"), true);
  const docsByRule = new Map(
    report.violations.map((violation) => [violation.ruleId, violation.doc]),
  );
  assert.equal(docsByRule.get("RR-6.1"), "rules/rust/domain.md#covered-rules");
  assert.equal(
    docsByRule.get("TS-1.3"),
    "rules/typescript/source.md#covered-rules",
  );
  assert.equal(
    docsByRule.get("PY-1.3"),
    "rules/python/source.md#covered-rules",
  );
});

test("source-shape default policy covers Python files", () => {
  const project = makeProject({
    "src/app.py": Array.from(
      { length: 31 },
      (_, index) => `def fn_${index}():\n    return ${index}\n`,
    ).join("\n"),
  });
  const result = run(project, [
    "check",
    "source-shape",
    "--json",
    "--files",
    "src/app.py",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some(
      (violation) =>
        violation.ruleId === "SRC-1.1" && /functions/u.test(violation.detail),
    ),
    true,
  );
});

test("source-shape emits explicit SRC-2 rule IDs for every shape budget", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      sourceShapePolicies: [
        {
          roots: ["src"],
          extensions: [".ts"],
          kind: "typescript",
          maxClasses: 1,
          maxExports: 1,
          maxFunctionLines: 4,
          maxLines: 12,
          maxNestingDepth: 1,
          maxBranches: 2,
        },
        {
          roots: ["src"],
          extensions: [".rs"],
          kind: "rust",
          maxFunctionLines: 4,
          maxFunctions: 20,
          maxLines: 200,
          maxTypes: 1,
        },
      ],
    }),
    "src/shape.ts": `
export class First {
  value = 1;
}
export class Second {
  value = 2;
}
export const one = 1;
export const two = 2;
export function complex(input: number): number {
  if (input > 0) {
    if (input > 1) {
      if (input > 2) {
        return input;
      }
    }
  }
  if (input < 0) return 0;
  return input === 1 ? 1 : 2;
}
`,
    "src/types.rs": `
pub struct First;
pub struct Second;
`,
  });
  const result = run(project, [
    "check",
    "source-shape",
    "--json",
    "--files",
    "src/shape.ts",
    "src/types.rs",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  const ids = new Set(report.violations.map((violation) => violation.ruleId));
  for (const ruleId of [
    "SRC-2.1",
    "SRC-2.2",
    "SRC-2.3",
    "SRC-2.4",
    "SRC-2.5",
    "SRC-2.6",
    "SRC-2.7",
  ]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});

test("explain returns routed docs for TypeScript and Python rules", () => {
  const project = makeProject({ "README.md": "# fixture\n" });
  const ts = run(project, ["explain", "TS-1.3", "--json"]);
  assert.equal(ts.status, 0, ts.stdout || ts.stderr);
  assert.equal(
    JSON.parse(ts.stdout).anchor,
    "rules/typescript/source.md#covered-rules",
  );

  const py = run(project, ["explain", "PY-1.3", "--json"]);
  assert.equal(py.status, 0, py.stdout || py.stderr);
  assert.equal(
    JSON.parse(py.stdout).anchor,
    "rules/python/source.md#covered-rules",
  );
});

test("documentation advisory warnings do not fail by default", () => {
  const project = makeProject({
    "src/api.ts": `
export function makeThing(): number {
  return 1;
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "src",
  ]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.deepEqual(report.violations, []);
  assert.equal(
    report.warnings.some(
      (finding) =>
        finding.ruleId === "DOC-1.1" && finding.severity === "warning",
    ),
    true,
  );
});

test("profile can promote advisory documentation warnings to hard failures", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      rules: {
        "DOC-1.1": { severity: "error" },
      },
    }),
    "src/api.ts": `
export function makeThing() {
  return 1;
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "src",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some(
      (finding) => finding.ruleId === "DOC-1.1" && finding.severity === "error",
    ),
    true,
  );
});

test("profile cannot downgrade an immutable hard rule to warning", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      rules: {
        "TS-2.1": { severity: "warning" },
      },
    }),
    "src/api.ts": `
${tsIgnoreComment}
const value = dynamicValue;
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "src",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some(
      (finding) =>
        finding.ruleId === "TS-2.1" && finding.severity === "error",
    ),
    true,
  );
  const policy = run(project, ["check", "config-lockdown", "--json"]);
  assert.notEqual(policy.status, 0, policy.stdout || policy.stderr);
  assert.equal(
    JSON.parse(policy.stdout).violations.some(
      (finding) =>
        finding.ruleId === "CFG-1.3" && /TS-2.1/u.test(finding.detail),
    ),
    true,
  );
});

test("config-lockdown catches unknown keys, boundary notes, profiles, and self-check state", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 2,
      profileName: "unknown-profile",
      failOn: ["error"],
      rawTypeBoundaryGlobs: ["src/domain/**"],
      configChangeRequiresSelfCheck: true,
      policyIntegrityChecked: false,
      typoPolicyKey: true,
    }),
  });
  const result = run(project, ["check", "config-lockdown", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(JSON.parse(result.stdout).violations.map((violation) => violation.ruleId));
  for (const ruleId of ["CFG-1.7", "CFG-1.9", "CFG-1.11", "CFG-1.12"]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});

test("config-lockdown requires explicit config identity", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      failOn: ["error"],
    }),
  });
  const result = run(project, ["check", "config-lockdown", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(JSON.parse(result.stdout).violations.map((violation) => violation.ruleId));
  assert.equal(ids.has("CFG-1.10"), true);
});

test("waiver-policy catches hidden, over-budget, and long-lived waivers", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 2,
      profileName: "strict",
      failOn: ["error"],
      maxActiveWaivers: 0,
      maxWaiverDays: 1,
      waivers: [
        {
          ruleId: "DOC-1.1",
          waiverId: "WAIVER-DOC-TEST",
          owner: "platform-team",
          issue: "https://example.test/issues/1",
          reason: "bounded fixture",
          scope: ["src/api.ts"],
          expires: "2099-01-01",
          remediation: "remove fixture waiver",
          ciAllowed: true,
          visible: false,
        },
      ],
    }),
  });
  const result = run(project, ["check", "waiver-policy", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(JSON.parse(result.stdout).violations.map((violation) => violation.ruleId));
  for (const ruleId of ["WAIVER-1.6", "WAIVER-1.7", "WAIVER-1.8"]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});

test("route command returns TypeScript, Python, and common docs without loading unknown files", () => {
  const project = makeProject({ "README.md": "# fixture\n" });
  const tsRoute = run(project, [
    "route",
    "--json",
    "--files",
    "src/index.ts",
    "tests/example.test.ts",
  ]);
  assert.equal(tsRoute.status, 0, tsRoute.stdout || tsRoute.stderr);
  const tsReport = JSON.parse(tsRoute.stdout);
  assert.equal(
    tsReport.docs.includes("rules/typescript/source.md#covered-rules"),
    true,
  );
  assert.equal(
    tsReport.docs.includes("rules/typescript/tests.md#covered-rules"),
    true,
  );
  assert.equal(
    tsReport.docs.includes("rules/common/security.md#covered-rules"),
    true,
  );

  const pyRoute = run(project, [
    "route",
    "--json",
    "--files",
    "tests/test_app.py",
  ]);
  const pyReport = JSON.parse(pyRoute.stdout);
  assert.equal(
    pyReport.docs.includes("rules/python/source.md#covered-rules"),
    true,
  );
  assert.equal(
    pyReport.docs.includes("rules/python/tests.md#covered-rules"),
    true,
  );

  const unknownRoute = run(project, [
    "route",
    "--json",
    "--files",
    "README.md",
  ]);
  const unknownReport = JSON.parse(unknownRoute.stdout);
  assert.deepEqual(unknownReport.docs, []);
  assert.deepEqual(unknownReport.rules, []);
});
