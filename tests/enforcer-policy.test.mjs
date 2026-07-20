import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { spawnCli } from "./cli-spawn.mjs";
import { isIgnoredPath } from "../src/path-utils.mjs";
import { DEFAULT_CONFIG } from "../src/rule-metadata.mjs";
import { collectWaiverPolicyFindings } from "../src/check-policy.mjs";
import { collectCiIntegrityFindings } from "../src/check-governance.mjs";
import { policyTraversalEntries } from "../scripts/check-source-core-helpers.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SCRIPT = path.join(ROOT, "scripts", "rust-rules.mjs");
const POLICY_CLI_TIMEOUT_MS = 20_000;
const FIXED_WAIVER_NOW = new Date("2030-01-15T12:00:00.000Z");
const assemble = (...parts) => parts.join("");
const validatorNetworkSource = assemble(
  "export async function scan() { return fe",
  'tch("https://example.com"); }\n',
);

function utcDateDaysAfter(date, days) {
  const shifted = new Date(date.getTime());
  shifted.setUTCDate(shifted.getUTCDate() + days);
  return shifted.toISOString().slice(0, 10);
}

function makeProject(files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "ocentra-enforcer-policy-"));
  for (const [rel, content] of Object.entries(files)) {
    const full = path.join(dir, rel);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content.trimStart(), "utf8");
  }
  return dir;
}

function makeVerifyProject() {
  return makeProject({
    ".github/CODEOWNERS": `
rules/** @team/enforcer
scripts/** @team/enforcer
src/** @team/enforcer
mcp/** @team/enforcer
schemas/** @team/enforcer
profiles/** @team/enforcer
adapters/** @team/enforcer
.github/workflows/** @team/enforcer
`,
    "LICENSE": "MIT License\n",
    "SECURITY.md": "Report a security vulnerability privately.\n",
    "CONTRIBUTING.md": "Keep every rule, validator, fixture, registry, and schema synchronized.\n",
    "CHANGELOG.md": "# Enforcer changes\n",
    "docs/RELEASE_POLICY.md": "Signed release tags are required before publish.\n",
    "src/index.mjs": "export const ready = true;\n",
  });
}

function run(project, args, options = {}) {
  return spawnCli(process.execPath, [SCRIPT, ...args, "--root", project], {
    encoding: "utf8",
    timeout: POLICY_CLI_TIMEOUT_MS,
    ...options,
  });
}

function writeConfig(project, config) {
  fs.writeFileSync(
    path.join(project, "ocentra-enforcer.config.json"),
    `${JSON.stringify(config, null, 2)}\n`,
    "utf8",
  );
}

function violationIds(report) {
  return new Set(report.violations.map((violation) => violation.ruleId));
}

test("immutable Rust rules cannot be disabled or hidden", () => {
  const project = makeProject({
    "src/lib.rs": "pub fn f() { let _ = Some(1).unwrap(); }\n",
  });
  writeConfig(project, {
    failOn: ["error"],
    languages: ["rust"],
    rules: {
      "RR-4.1": { enabled: false },
    },
  });

  const policy = run(project, ["check", "config-lockdown", "--json"]);
  assert.notEqual(policy.status, 0, policy.stdout || policy.stderr);
  assert.equal(violationIds(JSON.parse(policy.stdout)).has("CFG-1.2"), true);

  const scan = run(project, ["scan", "--json", "--files", "src/lib.rs"]);
  assert.notEqual(scan.status, 0, scan.stdout || scan.stderr);
  assert.equal(violationIds(JSON.parse(scan.stdout)).has("RR-4.1"), true);
});

test("immutable domain rules cannot be downgraded", () => {
  const project = makeProject({
    "src/lib.rs": "pub fn find_user(id: String) -> String { id }\n",
  });
  writeConfig(project, {
    failOn: ["error"],
    languages: ["rust"],
    rules: {
      "RR-6.1": { severity: "warning" },
    },
  });

  const policy = run(project, ["check", "config-lockdown", "--json"]);
  assert.notEqual(policy.status, 0, policy.stdout || policy.stderr);
  assert.equal(violationIds(JSON.parse(policy.stdout)).has("CFG-1.3"), true);

  const scan = run(project, ["scan", "--json", "--files", "src/lib.rs"]);
  assert.notEqual(scan.status, 0, scan.stdout || scan.stderr);
  const report = JSON.parse(scan.stdout);
  const rawString = report.violations.find((violation) => violation.ruleId === "RR-6.1");
  assert.equal(rawString?.severity, "error");
});

test("strict failOn cannot remove error failures", () => {
  const project = makeProject({ "src/lib.rs": "pub fn f() {}\n" });
  writeConfig(project, {
    profileName: "strict",
    failOn: [],
    languages: ["rust"],
  });

  const result = run(project, ["check", "config-lockdown", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("CFG-1.1"), true);
});

test("unsafe and dependency escape hatches require waivers", () => {
  const project = makeProject({ "src/lib.rs": "pub fn f() {}\n" });
  writeConfig(project, {
    failOn: ["error"],
    languages: ["rust"],
    allowUnsafeCode: true,
    allowBuildRs: true,
    allowGitDependencies: true,
    allowPathDependencies: true,
    publicReexportPolicy: "allow",
  });

  const result = run(project, ["check", "config-lockdown", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = violationIds(JSON.parse(result.stdout));
  assert.equal(ids.has("CFG-1.4"), true);
  assert.equal(ids.has("CFG-1.5"), true);
  assert.equal(ids.has("CFG-1.6"), true);
});

test("profile-backed waivers satisfy governed dependency escape hatches", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 2,
      profileName: "ocentra-parent",
      failOn: ["error"],
      languages: ["rust", "common"],
    }),
    "src/lib.rs": "pub fn f() {}\n",
  });

  const result = run(project, ["check", "config-lockdown", "--json"]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(violationIds(report).has("CFG-1.6"), false);
});

test("waiver policy rejects expired, broad, AI-owned, and immutable waivers", () => {
  const project = makeProject({ "src/lib.rs": "pub fn f() {}\n" });
  writeConfig(project, {
    failOn: ["error"],
    waivers: [
      {
        ruleId: "RR-4.1",
        waiverId: "WAIVER-BAD-1",
        owner: "codex",
        issue: "OCEN-1",
        reason: "temporary",
        scope: ["**/*"],
        expires: "2020-01-01",
        remediation: "",
        ciAllowed: false,
      },
    ],
  });

  const result = run(project, ["check", "waiver-policy", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = violationIds(JSON.parse(result.stdout));
  assert.equal(ids.has("WAIVER-1.2"), true);
  assert.equal(ids.has("WAIVER-1.3"), true);
  assert.equal(ids.has("WAIVER-1.4"), true);
  assert.equal(ids.has("WAIVER-1.9"), true);
  assert.equal(ids.has("WAIVER-1.10"), true);
});

test("waiver expiry validation uses an injected UTC clock", () => {
  const project = makeProject({ "src/api.ts": "export const ready = true;\n" });
  const waiver = {
    ruleId: "DOC-1.1",
    waiverId: "WAIVER-CLOCK-1",
    owner: "platform-team",
    issue: "OCEN-125",
    reason: "bounded documentation rollout",
    scope: ["src/api.ts"],
    expires: "2030-02-01",
    remediation: "Document the API before expiry.",
    ciAllowed: true,
    visible: true,
  };
  const config = { waivers: [waiver], maxWaiverDays: 90 };

  const validIds = new Set(collectWaiverPolicyFindings(project, config, {
    now: FIXED_WAIVER_NOW,
  }).map((finding) => finding.ruleId));
  const expiredIds = new Set(collectWaiverPolicyFindings(project, config, {
    now: new Date("2030-02-02T00:00:00.000Z"),
  }).map((finding) => finding.ruleId));

  assert.equal(validIds.has("WAIVER-1.3"), false);
  assert.equal(expiredIds.has("WAIVER-1.3"), true);
});

test("advisory documentation rule remains overridable", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 1,
      profileName: "strict",
      failOn: ["error"],
      languages: ["typescript", "common"],
      rules: {
        "DOC-1.1": {
          enabled: false,
          waiverId: "WAIVER-DOC-DISABLE-1",
          owner: "sujan",
          issue: "OCEN-124",
          reason: "temporary advisory documentation rollout",
          scope: ["src/api.ts"],
          expires: utcDateDaysAfter(new Date(), 30),
          remediation: "Document this API when the public surface stabilizes.",
        },
      },
    }),
    "src/api.ts": "export function makeThing(): number { return 1; }\n",
  });

  const scan = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "src/api.ts",
  ]);
  assert.equal(scan.status, 0, scan.stdout || scan.stderr);
  const report = JSON.parse(scan.stdout);
  assert.equal(report.violations.length, 0);
  assert.equal(
    report.findings.some((finding) => finding.ruleId === "DOC-1.1"),
    false,
  );
});

test("valid waivers are visible and suppress only waivable findings", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 1,
      profileName: "strict",
      failOn: ["error", "warning"],
      languages: ["typescript", "common"],
      waivers: [
        {
          ruleId: "DOC-1.1",
          waiverId: "WAIVER-DOC-1",
          owner: "sujan",
          issue: "OCEN-123",
          reason: "temporary documentation rollout for this exact API file",
          scope: ["src/api.ts"],
          expires: utcDateDaysAfter(new Date(), 30),
          remediation: "Add concise API docs when the public surface stabilizes.",
          ciAllowed: true,
          visible: true,
        },
      ],
    }),
    "src/api.ts": "export function makeThing(): number { return 1; }\n",
  });

  const scan = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "src/api.ts",
  ]);
  assert.equal(scan.status, 0, scan.stdout || scan.stderr);
  const report = JSON.parse(scan.stdout);
  assert.equal(report.violations.length, 0);
  const waived = report.waived.find((finding) => finding.ruleId === "DOC-1.1");
  assert.equal(waived?.status, "waived");
  assert.equal(waived?.waiverId, "WAIVER-DOC-1");
  assert.equal(
    report.findings.some(
      (finding) => finding.ruleId === "DOC-1.1" && finding.status === "waived",
    ),
    true,
  );
});

function makePack(ruleOverrides, files = {}) {
  const pack = makeProject({
    "rules/common/test.md": `
# Test Rules

## Covered Rules

- \`${ruleOverrides.id ?? "TEST-9.9"}\`: fixture-only test rule.
`,
    "rules/rules.json": JSON.stringify({
      schemaVersion: 2,
      productName: "ocentra-enforcer",
      languages: ["common"],
      rules: [
        {
          id: ruleOverrides.id ?? "TEST-9.9",
          language: "common",
          family: "harness",
          severity: "error",
          title: "Fixture-only test rule",
          snippet: "Fix the fixture-only rule.",
          lockLevel: "immutable",
          canDisable: false,
          canDowngrade: false,
          requiresFailFixture: false,
          requiresPassFixture: false,
          appliesTo: ["**/*"],
          triggers: ["fixture"],
          validator: "common/fixture",
          doc: "rules/common/test.md#covered-rules",
          ...ruleOverrides,
        },
      ],
    }),
    ...files,
  });
  return pack;
}

test("rule coverage rejects scanner-emitted unregistered rule IDs", () => {
  const pack = makePack({}, {
    "src/scanner.mjs": 'export const ruleId = "RR-99.1";\n',
  });
  const result = run(pack, ["check", "rule-coverage", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("ENF-1.3"), true);
});

test("rule coverage rejects registry docs with missing anchors", () => {
  const pack = makePack({
    doc: "rules/common/test.md#missing-anchor",
  });
  const result = run(pack, ["check", "rule-coverage", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("ENF-1.2"), true);
});

test("rule coverage rejects required fixture evidence that is missing", () => {
  const pack = makePack({
    requiresFailFixture: true,
    requiresPassFixture: true,
  });
  const result = run(pack, ["check", "rule-coverage", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("ENF-1.4"), true);
});

test("rule coverage rejects missing rule ID lock file", () => {
  const pack = makePack({});
  const result = run(pack, ["check", "rule-coverage", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("ENF-1.5"), true);
});

test("rule coverage rejects registry rule IDs missing from the lock file", () => {
  const pack = makePack(
    {},
    {
      "rules/rule-id-lock.json": JSON.stringify({
        schemaVersion: 1,
        ruleIds: ["AI-1.1"],
      }),
    },
  );
  const result = run(pack, ["check", "rule-coverage", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(violationIds(report).has("ENF-1.5"), true);
  assert.match(
    report.violations.map((violation) => violation.detail).join("\n"),
    /registry rule ID TEST-9\.9 is missing from rules\/rule-id-lock\.json/u,
  );
});

test("rule coverage rejects registry metadata drift from validator metadata", () => {
  const pack = makePack(
    {
      id: "NPM-1.3",
      title: "Wrong title",
      snippet: "Use exact package versions; avoid ^, ~, *, latest, git:, file:, and path ranges.",
    },
    {
      "rules/rule-id-lock.json": JSON.stringify({
        schemaVersion: 1,
        ruleIds: ["NPM-1.3"],
      }),
    },
  );
  const result = run(pack, ["check", "rule-coverage", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("ENF-1.7"), true);
});

test("rule coverage rejects non-deterministic registry ordering", () => {
  const pack = makeProject({
    "rules/common/test.md": `
# Test Rules

## Covered Rules

- \`SEC-9.9\`: last rule.
- \`AI-9.9\`: first rule.
`,
    "rules/rule-id-lock.json": JSON.stringify({
      schemaVersion: 1,
      ruleIds: ["AI-9.9", "SEC-9.9"],
    }),
    "rules/rules.json": JSON.stringify({
      schemaVersion: 2,
      productName: "ocentra-enforcer",
      languages: ["common"],
      rules: [
        {
          id: "SEC-9.9",
          language: "common",
          family: "harness",
          severity: "error",
          title: "Last rule",
          snippet: "Last.",
          lockLevel: "immutable",
          canDisable: false,
          canDowngrade: false,
          requiresFailFixture: false,
          requiresPassFixture: false,
          appliesTo: ["**/*"],
          triggers: ["fixture"],
          validator: "common/fixture",
          doc: "rules/common/test.md#covered-rules",
        },
        {
          id: "AI-9.9",
          language: "common",
          family: "harness",
          severity: "error",
          title: "First rule",
          snippet: "First.",
          lockLevel: "immutable",
          canDisable: false,
          canDowngrade: false,
          requiresFailFixture: false,
          requiresPassFixture: false,
          appliesTo: ["**/*"],
          triggers: ["fixture"],
          validator: "common/fixture",
          doc: "rules/common/test.md#covered-rules",
        },
      ],
    }),
  });
  const result = run(pack, ["check", "rule-coverage", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("ENF-1.9"), true);
});

test("rule coverage rejects network access in validator source", () => {
  const pack = makePack(
    {},
    {
      "rules/rule-id-lock.json": JSON.stringify({
        schemaVersion: 1,
        ruleIds: ["TEST-9.9"],
      }),
      "src/checks.mjs": validatorNetworkSource,
    },
  );
  const result = run(pack, ["check", "rule-coverage", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("ENF-1.11"), true);
});

test("rule coverage rejects temporary bypass comments in enforcer source", () => {
  const pack = makePack(
    {},
    {
      "rules/rule-id-lock.json": JSON.stringify({
        schemaVersion: 1,
        ruleIds: ["TEST-9.9"],
      }),
      "src/checks.mjs": "// TODO bypass this later\nexport const ok = true;\n",
    },
  );
  const result = run(pack, ["check", "rule-coverage", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("ENF-1.13"), true);
});

test("docs completeness rejects routed docs without repair sections", () => {
  const pack = makePack(
    {},
    {
      "rules/rule-id-lock.json": JSON.stringify({
        schemaVersion: 1,
        ruleIds: ["TEST-9.9"],
      }),
    },
  );
  const result = run(pack, ["check", "docs-completeness", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("DOCENF-1.1"), true);
});

test("docs completeness rejects source docs without fail/pass code examples", () => {
  const pack = makeProject({
    "rules/common/test.md": `
# Test Rules

## Covered Rules

- \`RR-9.9\`: source rule.

## Fails

- Bad code fails.

## Passes

- Good code passes.

## Fix Recipe

1. Fix it.

## Validator

- scanner: test
`,
    "rules/rule-id-lock.json": JSON.stringify({
      schemaVersion: 1,
      ruleIds: ["RR-9.9"],
    }),
    "rules/rules.json": JSON.stringify({
      schemaVersion: 2,
      productName: "ocentra-enforcer",
      languages: ["rust"],
      rules: [
        {
          id: "RR-9.9",
          language: "rust",
          family: "source",
          severity: "error",
          title: "Source rule",
          snippet: "Fix source rule.",
          lockLevel: "immutable",
          canDisable: false,
          canDowngrade: false,
          requiresFailFixture: false,
          requiresPassFixture: false,
          appliesTo: ["**/*.rs"],
          triggers: ["fixture"],
          validator: "rust/source",
          doc: "rules/common/test.md#covered-rules",
        },
      ],
    }),
  });
  const result = run(pack, ["check", "docs-completeness", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("DOCENF-1.2"), true);
});

test("package determinism rejects loose npm metadata", () => {
  const project = makeProject({
    "package.json": JSON.stringify(
      {
        name: "loose-package",
        version: "1.0.0",
        packageManager: "npm@latest",
        engines: { node: ">=20" },
        dependencies: {
          effect: "^3.21.4",
          "openai-js": "1.0.0",
          "from-git": "github:owner/repo",
          "from-file": "file:../local-package",
          "left-pad": "latest",
        },
      },
      null,
      2,
    ),
    "package-lock.json": JSON.stringify(
      {
        name: "loose-package",
        lockfileVersion: 3,
        packages: {
          "": { name: "loose-package", version: "1.0.0" },
          "node_modules/install-script": {
            version: "1.0.0",
            hasInstallScript: true,
          },
        },
      },
      null,
      2,
    ),
  });

  const result = run(project, ["check", "package-determinism", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = violationIds(JSON.parse(result.stdout));
  assert.equal(ids.has("NPM-1.3"), true);
  assert.equal(ids.has("NPM-1.4"), true);
  assert.equal(ids.has("NPM-1.5"), true);
  assert.equal(ids.has("NPM-1.6"), true);
  assert.equal(ids.has("NPM-1.7"), true);
  assert.equal(ids.has("NPM-1.8"), true);
  assert.equal(ids.has("NPM-1.11"), true);
  assert.equal(ids.has("NPM-1.13"), true);
  for (const violation of JSON.parse(result.stdout).violations) {
    assert.equal(typeof violation.doc, "string");
    assert.equal("source" in violation, true);
  }
});

test("ci integrity rejects weak workflow gates", () => {
  const project = makeProject({
    ".github/workflows/ci.yml": `
name: Weak CI
on:
  push:
    branches: [dev]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@main
      - run: npm install
      - run: node scripts/rust-rules.mjs scan --root .
      - continue-on-error: true
        run: npm test || true
`,
    "tests/json-cli.test.mjs": `
import { spawnSync } from "node:child_process";

const result = spawnSync(process.execPath, ["scripts/emit-json.mjs"], {
  encoding: "utf8",
});
JSON.parse(result.stdout);
`,
  });

  const result = run(project, ["check", "ci-integrity", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = violationIds(JSON.parse(result.stdout));
  for (const ruleId of ["CI-1.1", "NPM-1.2", "CI-1.3", "CI-1.4", "CI-1.5", "CI-1.6", "CI-1.7", "CI-1.8", "CI-1.9", "CI-1.10", "CI-1.11", "CI-1.12", "CI-1.13", "CI-1.14", "CI-1.15", "CI-1.16", "CI-1.17", "CI-1.18", "CI-1.19", "CI-1.20", "CI-1.21"]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});

test("ci integrity rejects implicit shell execution in harness helpers", () => {
  const pack = makePack(
    {},
    {
      "rules/rule-id-lock.json": JSON.stringify({
        schemaVersion: 1,
        ruleIds: ["TEST-9.9"],
      }),
      "src/checks.mjs":
        'import { spawnSync } from "node:child_process";\nspawnSync("npm", ["test"], { shell: process.platform === "win32" });\n',
    },
  );
  const result = run(pack, ["check", "ci-integrity", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("HAR-2.15"), true);
});

test("ci integrity recognizes local composite actions and reusable dogfood roles", () => {
  const project = makeProject({
    "scripts/ci-local.mjs": `
      npm test
      npm run test:policy
      npm run test:multilang
      npm run test:mcp
      npm run enforcer:self
      npm run enforcer:verify:ci
    `,
    ".github/workflows/ci.yml": `
      on:
        pull_request:
        push:
          branches: [main]
      permissions:
        contents: read
      jobs:
        workspace:
          strategy:
            matrix:
              os: [ubuntu-latest, windows-latest, macos-latest]
          runs-on: \${{ matrix.os }}
          steps:
            - uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5
            - uses: ./.github/actions/setup-enforcer
            - run: npm run ci:local
    `,
    ".github/workflows/dogfood.yml": `
      on:
        workflow_call:
      permissions:
        contents: read
      jobs:
        dogfood:
          runs-on: ubuntu-latest
          steps:
            - uses: actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5
            - uses: ./.github/actions/setup-enforcer
    `,
  });

  const findings = collectCiIntegrityFindings(project);
  const ruleIds = new Set(findings.map((finding) => finding.ruleId));
  for (const ruleId of ["CI-1.13", "CI-1.15", "CI-1.16", "CI-1.17"]) {
    assert.equal(ruleIds.has(ruleId), false, `${ruleId} must honor workflow role semantics`);
  }
});

test("ci integrity accepts CI-safe subprocess JSON capture", () => {
  const project = makeProject({
    "tests/json-cli.test.mjs": `
import { spawnSync } from "node:child_process";

const result = spawnSync(process.execPath, ["scripts/emit-json.mjs"], {
  encoding: "utf8",
  maxBuffer: 32 * 1024 * 1024,
});
JSON.parse(result.stdout);
`,
    "tests/file-backed-json-cli.test.mjs": `
import fs from "node:fs";
import { spawnSync } from "node:child_process";

const stdoutFd = fs.openSync("stdout.log", "w");
const stderrFd = fs.openSync("stderr.log", "w");
const result = spawnSync(process.execPath, ["scripts/emit-json.mjs"], {
  stdio: ["ignore", stdoutFd, stderrFd],
});
const stdout = fs.readFileSync("stdout.log", "utf8");
JSON.parse(stdout);
`,
  });

  const result = run(project, ["check", "ci-integrity", "--json"]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  assert.equal(violationIds(JSON.parse(result.stdout)).has("CI-1.21"), false);
});

test("repo governance rejects missing ownership and package determinism", () => {
  const project = makeProject({
    ".github/CODEOWNERS": `
rules/** @team/enforcer
`,
    "package.json": JSON.stringify(
      {
        name: "weak-repo",
        version: "1.0.0",
        engines: { node: ">=20" },
        dependencies: { effect: "^3.21.4" },
      },
      null,
      2,
    ),
  });

  const result = run(project, ["check", "repo-governance", "--json"]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = violationIds(JSON.parse(result.stdout));
  for (const ruleId of ["REPO-1.3", "REPO-1.4", "REPO-1.5", "REPO-1.6", "REPO-1.7", "REPO-1.8", "REPO-1.9", "REPO-1.10", "REPO-1.11", "REPO-1.12", "REPO-1.13", "REPO-1.14"]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});

test("mutation-risk rejects policy-critical changed files and ignores ordinary docs", () => {
  const project = makeProject({
    "src/checks.mjs": "export const changed = true;\n",
    "docs/note.md": "# Note\n",
  });

  const critical = run(project, [
    "check",
    "mutation-risk",
    "--json",
    "--files",
    "src/checks.mjs",
  ]);
  assert.notEqual(critical.status, 0, critical.stdout || critical.stderr);
  const criticalReport = JSON.parse(critical.stdout);
  assert.equal(violationIds(criticalReport).has("ENF-2.1"), true);
  assert.match(JSON.stringify(criticalReport.violations[0]), /rule-coverage/u);
  assert.match(JSON.stringify(criticalReport.violations[0]), /test:mcp/u);

  const ordinary = run(project, [
    "check",
    "mutation-risk",
    "--json",
    "--files",
    "docs/note.md",
  ]);
  assert.equal(ordinary.status, 0, ordinary.stdout || ordinary.stderr);
  assert.equal(JSON.parse(ordinary.stdout).ok, true);
});

test("verify local is the canonical default parity gate", () => {
  const project = makeVerifyProject();
  const result = run(project, ["verify", "--json"]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.equal(report.verifyMode, "local");
  const checkNames = new Set(report.checks.map((check) => check.check));
  assert.equal(checkNames.has("verify"), true);
  assert.equal(checkNames.has("rule-coverage"), true);
  assert.equal(checkNames.has("policy-integrity"), true);
  assert.equal(checkNames.has("ci-integrity"), true);
  assert.equal(checkNames.has("repo-governance"), true);
  assert.equal(checkNames.has("package-determinism"), true);
  assert.equal(Array.isArray(report.warnings), true);
  const packageManifest = JSON.parse(fs.readFileSync(path.join(ROOT, "package.json"), "utf8"));
  assert.equal(packageManifest.scripts["enforcer:verify:local"].includes("--root ."), true);
  assert.equal(fs.readFileSync(path.join(ROOT, "scripts", "ci-local.mjs"), "utf8").includes("enforcer:verify:ci"), true);
});

test("verify fast and ci select the expected gate sets", () => {
  const project = makeVerifyProject();
  const fast = run(project, ["verify", "fast", "--json"]);
  assert.equal(fast.status, 0, fast.stdout || fast.stderr);
  const fastChecks = new Set(JSON.parse(fast.stdout).checks.map((check) => check.check));
  assert.equal(fastChecks.has("rule-coverage"), true);
  assert.equal(fastChecks.has("policy-integrity"), true);
  assert.equal(fastChecks.has("ci-integrity"), false);
  assert.equal(fastChecks.has("secrets"), false);

  const ci = run(project, ["verify", "ci", "--json"]);
  assert.equal(ci.status, 0, ci.stdout || ci.stderr);
  const ciReport = JSON.parse(ci.stdout);
  assert.equal(ciReport.verifyMode, "ci");
  assert.equal(ciReport.ok, true);
  const ciChecks = new Set(ciReport.checks.map((check) => check.check));
  assert.equal(ciChecks.has("rule-coverage"), true);
  assert.equal(ciChecks.has("policy-integrity"), true);
  assert.equal(ciChecks.has("ci-integrity"), true);
  assert.equal(ciChecks.has("repo-governance"), true);
  assert.equal(ciChecks.has("package-determinism"), true);
  assert.equal(ciChecks.has("secrets"), true);
  assert.equal(ciChecks.has("dependency-policy"), true);
  assert.equal(ciChecks.has("sbom"), true);
  assert.equal(ciReport.checks.every((check) => check.ok === true), true);
});

test("enforcer self source-shape enforces governed pack-local ceilings", () => {
  const result = run(ROOT, [
    "check",
    "source-shape",
    "--root",
    ".",
    "--workspace",
    "--json",
  ]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.equal(report.violations.length, 0);
});

test("source-shape workspace traversal stays inside policy roots", () => {
  const project = makeProject({
    "src/index.mjs": "export const ready = true;\n",
    "target-policy-hang/generated.mjs": "export const ignored = true;\n",
    ".tmp-policy-hang/generated.mjs": "export const ignored = true;\n",
  });
  writeConfig(project, {
    schemaVersion: 2,
    profileName: "strict",
    failOn: ["error"],
    languages: ["typescript", "common"],
    sourceShapePolicies: [
      {
        roots: ["src"],
        extensions: [".mjs"],
        kind: "typescript",
        maxClasses: 1,
        maxExports: 5,
        maxFunctionLines: 80,
        maxLines: 100,
      },
    ],
  });

  assert.deepEqual(
    policyTraversalEntries(project, {}, { roots: ["src"] }, { mode: "all" }),
    ["src"],
  );
  assert.equal(isIgnoredPath("target-policy-hang/generated.mjs"), true);
  assert.equal(isIgnoredPath(".tmp-policy-hang/generated.mjs"), true);
  assert.equal(DEFAULT_CONFIG.ignoreDirs.includes(".tmp"), true);
  assert.equal(isIgnoredPath("targeted-policy-hang/generated.mjs"), false);
  assert.equal(isIgnoredPath("target_policy_hang/generated.mjs"), false);
  assert.equal(isIgnoredPath(".tmpkeeper/generated.mjs"), false);

  const result = run(project, ["check", "source-shape", "--json"], {
    timeout: 10_000,
  });
  assert.equal(result.error, undefined, result.stderr);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  assert.equal(JSON.parse(result.stdout).ok, true);
});

test("policy child timeout returns a compact diagnostic", () => {
  const result = spawnCli(
    process.execPath,
    ["--input-type=module", "--eval", "setInterval(() => {}, 1_000)"],
    { encoding: "utf8", timeout: 100 },
  );
  assert.equal(result.error?.code, "ETIMEDOUT");
  assert.match(result.stderr, /spawnCli timed out after 100ms/u);
  assert.equal(result.stderr.length < 500, true);
});
