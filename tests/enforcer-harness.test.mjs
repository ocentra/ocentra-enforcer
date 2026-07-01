import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SCRIPT = path.join(ROOT, "scripts", "rust-rules.mjs");
const TEST_CLI_MAX_BUFFER = 32 * 1024 * 1024;

function makeProject() {
  return fs.mkdtempSync(path.join(os.tmpdir(), "ocentra-enforcer-harness-"));
}

function run(project, args) {
  const delimiter = args.indexOf("--");
  const cliArgs =
    delimiter === -1
      ? [SCRIPT, ...args, "--root", project]
      : [
          SCRIPT,
          ...args.slice(0, delimiter),
          "--root",
          project,
          ...args.slice(delimiter),
        ];
  return spawnSync(process.execPath, cliArgs, {
    encoding: "utf8",
    maxBuffer: TEST_CLI_MAX_BUFFER,
    stdio: ["ignore", "pipe", "pipe"],
  });
}

test("harness persists raw logs, NDJSON diagnostics, summaries, and last failure", () => {
  const project = makeProject();
  const tscLine =
    "src/app.ts(7,5): error TS2322: Type string is not assignable to type number.";
  const result = run(project, [
    "run",
    "--json",
    "--tool",
    "tsc",
    "--package-name",
    "web-app",
    "--tag",
    "typecheck",
    "--",
    process.execPath,
    "-e",
    `console.error(${JSON.stringify(tscLine)}); process.exit(1);`,
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, false);
  assert.equal(report.summary.status, "failed");
  assert.equal(report.summary.storage.root, ".enforce");
  assert.equal(report.summary.packageName, "web-app");
  assert.deepEqual(report.summary.tags, ["typecheck"]);
  assert.equal(
    report.diagnostics.some((diagnostic) => diagnostic.ruleId === "TS2322"),
    true,
  );

  const runDir = path.join(project, ".enforce", "runs", report.summary.runId);
  assert.equal(fs.existsSync(path.join(runDir, "raw", "stderr.log")), true);
  assert.equal(fs.existsSync(path.join(runDir, "diagnostics.ndjson")), true);
  assert.equal(
    fs.existsSync(path.join(project, ".enforce", "db", "ingest-manifest.json")),
    true,
  );

  const last = run(project, [
    "runs",
    "last-failure",
    "--json",
    "--package-name",
    "web-app",
    "--tag",
    "typecheck",
  ]);
  assert.equal(last.status, 0, last.stdout || last.stderr);
  const lastReport = JSON.parse(last.stdout);
  assert.equal(lastReport.found, true);
  assert.equal(lastReport.run.runId, report.summary.runId);

  const artifact = run(project, [
    "runs",
    "artifact",
    "--json",
    "--run-id",
    report.summary.runId,
    "--artifact",
    "stderr",
  ]);
  const artifactReport = JSON.parse(artifact.stdout);
  assert.match(artifactReport.text, /TS2322/u);
});

test("harness retention prunes old successful runs but keeps bounded recent metadata", () => {
  const project = makeProject();
  fs.writeFileSync(
    path.join(project, "ocentra-enforcer.config.json"),
    JSON.stringify({
      harness: {
        storageDir: ".enforce",
        maxRuns: 2,
        maxRunsPerTool: 1,
        maxFailedRuns: 1,
        pruneAfterDays: null,
      },
    }),
    "utf8",
  );

  for (const runId of ["run-a", "run-b", "run-c"]) {
    const result = run(project, [
      "run",
      "--json",
      "--tool",
      "node",
      "--run-id",
      runId,
      "--crate-name",
      "agent-protocol",
      "--",
      process.execPath,
      "-e",
      "process.exit(0);",
    ]);
    assert.equal(result.status, 0, result.stdout || result.stderr);
  }

  const list = run(project, [
    "runs",
    "list",
    "--json",
    "--tool",
    "node",
    "--crate-name",
    "agent-protocol",
  ]);
  assert.equal(list.status, 0, list.stdout || list.stderr);
  const runs = JSON.parse(list.stdout).runs;
  assert.equal(runs.length <= 2, true);
  assert.equal(
    runs.some((entry) => entry.runId === "run-c"),
    true,
  );
  assert.equal(
    fs.existsSync(path.join(project, ".enforce", "runs", "run-a")),
    false,
  );

  const prune = run(project, ["runs", "prune", "--json"]);
  assert.equal(prune.status, 0, prune.stdout || prune.stderr);
  assert.equal(Array.isArray(JSON.parse(prune.stdout).removed), true);
});

test("harness parses ESLint JSON and Ruff JSON diagnostics", () => {
  const project = makeProject();
  const eslintJson = JSON.stringify([
    {
      filePath: path.join(project, "src", "index.ts"),
      messages: [
        {
          ruleId: "no-console",
          severity: 2,
          message: "Unexpected console statement.",
          line: 3,
        },
      ],
    },
  ]);
  const eslintRun = run(project, [
    "run",
    "--json",
    "--tool",
    "eslint",
    "--",
    process.execPath,
    "-e",
    `console.log(${JSON.stringify(eslintJson)}); process.exit(1);`,
  ]);
  const eslintReport = JSON.parse(eslintRun.stdout);
  assert.equal(
    eslintReport.diagnostics.some(
      (diagnostic) => diagnostic.ruleId === "no-console",
    ),
    true,
  );

  const ruffJson = JSON.stringify([
    {
      filename: path.join(project, "app.py"),
      code: "F401",
      message: "imported but unused",
      location: { row: 1, column: 1 },
    },
  ]);
  const ruffRun = run(project, [
    "run",
    "--json",
    "--tool",
    "ruff",
    "--",
    process.execPath,
    "-e",
    `console.log(${JSON.stringify(ruffJson)}); process.exit(1);`,
  ]);
  const ruffReport = JSON.parse(ruffRun.stdout);
  assert.equal(
    ruffReport.diagnostics.some((diagnostic) => diagnostic.ruleId === "F401"),
    true,
  );
});

test("harness parses rustc JSON, Pyright JSON, pytest text, and dedupes repeats", () => {
  const project = makeProject();
  const rustcMessage = JSON.stringify({
    reason: "compiler-message",
    message: {
      level: "error",
      message: "borrowed value does not live long enough",
      code: { code: "E0597" },
      spans: [
        {
          is_primary: true,
          file_name: path.join(project, "src", "lib.rs"),
          line_start: 4,
        },
      ],
    },
  });
  const rustcRun = run(project, [
    "run",
    "--json",
    "--tool",
    "cargo-check",
    "--",
    process.execPath,
    "-e",
    `console.error(${JSON.stringify(`${rustcMessage}\n${rustcMessage}`)}); process.exit(1);`,
  ]);
  const rustcReport = JSON.parse(rustcRun.stdout);
  assert.equal(
    rustcReport.diagnostics.filter(
      (diagnostic) => diagnostic.ruleId === "E0597",
    ).length,
    1,
  );

  const pyrightJson = JSON.stringify({
    generalDiagnostics: [
      {
        file: path.join(project, "app.py"),
        severity: "error",
        message: 'Type "str" is not assignable to declared type "int"',
        range: { start: { line: 2 } },
      },
    ],
  });
  const pyrightRun = run(project, [
    "run",
    "--json",
    "--tool",
    "pyright",
    "--",
    process.execPath,
    "-e",
    `console.log(${JSON.stringify(pyrightJson)}); process.exit(1);`,
  ]);
  const pyrightReport = JSON.parse(pyrightRun.stdout);
  assert.equal(
    pyrightReport.diagnostics.some(
      (diagnostic) =>
        diagnostic.ruleId === "pyright" && diagnostic.file === "app.py",
    ),
    true,
  );

  const pytestRun = run(project, [
    "run",
    "--json",
    "--tool",
    "pytest",
    "--",
    process.execPath,
    "-e",
    'console.error("FAILED tests/test_app.py::test_app - AssertionError: bad value"); process.exit(1);',
  ]);
  const pytestReport = JSON.parse(pytestRun.stdout);
  assert.equal(
    pytestReport.diagnostics.some(
      (diagnostic) =>
        diagnostic.ruleId === "pytest" &&
        diagnostic.file === "tests/test_app.py",
    ),
    true,
  );
});
