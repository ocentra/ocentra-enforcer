import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SCRIPT = path.join(ROOT, "scripts", "rust-rules.mjs");

function makeProject(files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "ocentra-enforcer-edge-"));
  writeFiles(dir, files);
  return dir;
}

function writeFiles(root, files) {
  for (const [rel, content] of Object.entries(files)) {
    const full = path.join(root, rel);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content.trimStart(), "utf8");
  }
}

function run(project, args) {
  return spawnSync(process.execPath, [SCRIPT, ...args, "--root", project], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function report(result) {
  assert.ok(
    result.stdout.trim().startsWith("{"),
    result.stdout || result.stderr,
  );
  return JSON.parse(result.stdout);
}

test("source-shape path and glob overrides relax only matching files", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      profileName: "shape-edge",
      sourceShapePolicies: [
        {
          roots: ["src"],
          extensions: [".ts"],
          kind: "typescript",
          maxExports: 0,
          maxLines: 10,
          maxFunctionLines: 20,
        },
      ],
      sourceShapeOverrides: [
        { path: "src/allowed.ts", maxExports: 2 },
        { glob: "src/generated/*.ts", maxExports: 2 },
      ],
    }),
    "src/allowed.ts": "export const one = 1;\nexport const two = 2;\n",
    "src/generated/api.ts": "export const generatedOne = 1;\n",
    "src/denied.ts": "export const denied = 1;\n",
  });

  const result = run(project, [
    "check",
    "source-shape",
    "--json",
    "--files",
    "src/allowed.ts",
    "src/generated/api.ts",
    "src/denied.ts",
  ]);

  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const parsed = report(result);
  assert.deepEqual(
    parsed.violations.map((violation) => violation.file),
    ["src/denied.ts"],
  );
  assert.match(parsed.violations[0].detail, /exports/u);
});

test("strict required-tests rejects deepest empty placeholder trees once", () => {
  const project = makeProject({
    "packages/app/package.json": JSON.stringify({ name: "@edge/app" }),
    "packages/app/src/index.ts": "export const value = 1;\n",
    "packages/app/tests/unit/index.test.ts":
      'test("value", () => expect(1).toBe(1));\n',
    "packages/app/tests/contract/.gitkeep": "",
    "packages/app/tests/e2e/nested/.gitkeep": "",
    "packages/app/tests/integration/real.test.ts":
      'test("real", () => expect(2).toBe(2));\n',
  });
  fs.mkdirSync(path.join(project, "packages/app/tests/manual"), {
    recursive: true,
  });

  const result = run(project, [
    "check",
    "required-tests",
    "--strict-empty-test-trees",
    "--json",
  ]);

  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const parsed = report(result);
  const details = parsed.violations.map((violation) => violation.detail).sort();
  assert.equal(
    details.some((detail) => detail.includes("packages/app/tests/contract")),
    true,
  );
  assert.equal(
    details.some((detail) =>
      detail.includes(
        "packages/app/tests/contract: empty test/proof category tree contains only 1 .gitkeep placeholder file",
      ),
    ),
    true,
  );
  assert.equal(
    details.some((detail) => detail.includes("packages/app/tests/e2e/nested")),
    true,
  );
  assert.equal(
    details.some((detail) =>
      detail.includes(
        "packages/app/tests/manual: empty test/proof category tree has no files",
      ),
    ),
    true,
  );
  assert.equal(
    details.some((detail) => detail.includes("packages/app/tests:")),
    false,
  );
  assert.equal(
    details.some((detail) => detail.includes("packages/app/tests/integration")),
    false,
  );
});

test("single-source contracts enforce required mirror coverage and allow explicit coverage exceptions", () => {
  const project = makeProject({
    "crates/schema/src/constants/covered.rs":
      'pub const KIND: &str = "edge.kind";\n',
    "crates/schema/src/constants/missing.rs":
      'pub const OTHER_KIND: &str = "edge.other";\n',
    "contracts.json": JSON.stringify({
      requiredMirrorRoots: ["crates/schema/src/constants"],
      contracts: [
        {
          name: "kinds",
          ownerPath: "crates/schema/src/constants/covered.rs",
          values: [{ name: "kind", rustConst: "KIND" }],
          scanRoots: ["src"],
        },
      ],
    }),
  });

  const missing = run(project, [
    "check",
    "single-source-contracts",
    "--check-config",
    "contracts.json",
    "--json",
  ]);
  assert.notEqual(missing.status, 0, missing.stdout || missing.stderr);
  assert.equal(
    report(missing).violations.some(
      (violation) =>
        violation.file === "crates/schema/src/constants/missing.rs",
    ),
    true,
  );

  fs.writeFileSync(
    path.join(project, "contracts.json"),
    JSON.stringify({
      requiredMirrorRoots: ["crates/schema/src/constants"],
      contracts: [
        {
          name: "kinds",
          ownerPath: "crates/schema/src/constants/covered.rs",
          values: [{ name: "kind", rustConst: "KIND" }],
          scanRoots: ["src"],
          allowedPaths: ["crates/schema/src/constants/missing.rs"],
        },
      ],
    }),
    "utf8",
  );

  const allowed = run(project, [
    "check",
    "single-source-contracts",
    "--check-config",
    "contracts.json",
    "--json",
  ]);
  assert.equal(allowed.status, 0, allowed.stdout || allowed.stderr);
  assert.equal(report(allowed).ok, true);
});

test("import-boundary allowed imports and root scope prevent false positives", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      profileName: "import-edge",
      importBoundaryPolicies: [
        {
          roots: ["apps/web"],
          forbiddenImports: ["@domain/*"],
          allowedImports: ["@domain/public"],
        },
      ],
    }),
    "apps/web/src/allowed.ts":
      'import { value } from "@domain/public";\nexport const result = value;\n',
    "apps/admin/src/sibling.ts":
      'import { value } from "@domain/private";\nexport const result = value;\n',
    "apps/web/src/denied.ts":
      'import { value } from "@domain/private";\nexport const result = value;\n',
  });

  const scopedPass = run(project, [
    "check",
    "import-boundaries",
    "--json",
    "--files",
    "apps/web/src/allowed.ts",
    "apps/admin/src/sibling.ts",
  ]);
  assert.equal(scopedPass.status, 0, scopedPass.stdout || scopedPass.stderr);

  const denied = run(project, [
    "check",
    "import-boundaries",
    "--json",
    "--files",
    "apps/web/src/denied.ts",
  ]);
  assert.notEqual(denied.status, 0, denied.stdout || denied.stderr);
  assert.equal(
    report(denied).violations.some(
      (violation) => violation.ruleId === "TS-4.1",
    ),
    true,
  );
});

test("generated marker scanning is source-only while secret scanning covers markdown", () => {
  const project = makeProject({
    "docs/policy.md":
      "Policy text may mention @generated and Generated by as examples.\n",
    "src/generated.ts": "// @generated by tool\nexport const value = 1;\n",
    "notes.md": `OPENAI_API_KEY=${"sk-" + "abcdefghijklmnopqrstuvwxyz123456"}\n`,
  });

  const docs = run(project, [
    "check",
    "generated-artifacts",
    "--json",
    "--files",
    "docs/policy.md",
  ]);
  assert.equal(docs.status, 0, docs.stdout || docs.stderr);

  const source = run(project, [
    "check",
    "generated-artifacts",
    "--json",
    "--files",
    "src/generated.ts",
  ]);
  assert.notEqual(source.status, 0, source.stdout || source.stderr);
  assert.equal(
    report(source).violations.some(
      (violation) => violation.ruleId === "GEN-1.1",
    ),
    true,
  );

  const secret = run(project, [
    "check",
    "secrets",
    "--json",
    "--files",
    "notes.md",
  ]);
  assert.notEqual(secret.status, 0, secret.stdout || secret.stderr);
  assert.equal(
    report(secret).violations.some(
      (violation) => violation.ruleId === "SEC-1.1",
    ),
    true,
  );
});

test("warning failOn can promote advisories without changing rule severity", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      profileName: "warning-fail",
      failOn: ["warning"],
    }),
    "src/api.ts": "export function missingDocs() {\n  return 1;\n}\n",
  });

  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "src/api.ts",
  ]);

  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const parsed = report(result);
  assert.equal(
    parsed.violations.some(
      (violation) =>
        violation.ruleId === "DOC-1.1" && violation.severity === "warning",
    ),
    true,
  );
  assert.deepEqual(parsed.failOn, ["warning"]);
});

test("disabled rules suppress otherwise failing findings", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      profileName: "disabled-rule",
      rules: {
        "TS-2.1": { enabled: false },
        "DOC-1.1": { enabled: false },
      },
    }),
    "src/api.ts": [
      "// @ts-",
      "ignore\nexport function ignored() {\n  return dynamicValue;\n}\n",
    ].join(""),
  });

  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "src/api.ts",
  ]);

  assert.equal(result.status, 0, result.stdout || result.stderr);
  const parsed = report(result);
  assert.deepEqual(parsed.violations, []);
  assert.deepEqual(parsed.warnings, []);
});

test("route supports explicit non-Rust rule IDs and config files without loading unknown docs", () => {
  const project = makeProject({
    "README.md": "# edge\n",
  });

  const explicitTs = run(project, ["route", "--json", "--rule-id", "TS-4.1"]);
  assert.equal(explicitTs.status, 0, explicitTs.stdout || explicitTs.stderr);
  assert.deepEqual(report(explicitTs).docs, [
    "rules/typescript/source.md#covered-rules",
  ]);

  const explicitPy = run(project, ["route", "--json", "--rule-id", "PY-3.2"]);
  assert.equal(explicitPy.status, 0, explicitPy.stdout || explicitPy.stderr);
  assert.deepEqual(report(explicitPy).docs, [
    "rules/python/toolchain.md#covered-rules",
  ]);

  const pyConfig = run(project, [
    "route",
    "--json",
    "--files",
    "pyproject.toml",
    "ruff.toml",
  ]);
  assert.equal(pyConfig.status, 0, pyConfig.stdout || pyConfig.stderr);
  assert.equal(
    report(pyConfig).docs.includes("rules/python/toolchain.md#covered-rules"),
    true,
  );

  const unknown = run(project, ["route", "--json", "--files", "README.md"]);
  assert.equal(unknown.status, 0, unknown.stdout || unknown.stderr);
  assert.deepEqual(report(unknown).docs, []);
});
