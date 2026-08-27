import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { scanRustDocumentationHints } from "../src/documentation-hints.mjs";
import {
  makeProject,
  parseReport,
  pythonDoubleCall,
  pythonDoubleImport,
  run,
} from "./enforcer-multilang-test-support.mjs";

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
const fixtureSecretLine = assemble('token = "', fakeSecretValue, '"\n');
test("BOUND-1.2 accepts a raw boundary type converted by an adjacent boundary module", () => {
  const project = makeProject({
    "src/boundary/report.ts": `
/** BOUNDARY-INVARIANT: transport reports are converted before domain use. */
export interface BranchReportDto { rawPayload: Record<string, string>; }
export const rawInput = "transport";
// negative malformed report is rejected.
`,
    "src/boundary/report-payload.ts": `
import type { BranchReportDto } from "./report";
export function toDomain(value: BranchReportDto): { checked: true } { return { checked: true }; }
`,
  });
  const pass = run(project, ["scan", "--json", "--languages", "typescript,common", "--files", "src/boundary/report.ts"]);
  assert.equal(pass.status, 0, pass.stdout || pass.stderr);
  fs.unlinkSync(path.join(project, "src", "boundary", "report-payload.ts"));
  const fail = run(project, ["scan", "--json", "--languages", "typescript,common", "--files", "src/boundary/report.ts"]);
  assert.notEqual(fail.status, 0, fail.stdout || fail.stderr);
  assert.equal(parseReport(fail).violations.some((finding) => finding.ruleId === "BOUND-1.2"), true);
});

test("BOUND-1.3 distinguishes raw JSON decoding from a domain decision", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 2,
      profileName: "strict",
      failOn: ["error"],
    }),
    "package.json": JSON.stringify({ name: "boundary-decision-fixture", version: "0.0.0" }),
    "src/boundary/decode.rs": `
/// BOUNDARY-INVARIANT: raw MCP input is decoded before domain use.
// malformed input is rejected.
fn decode(raw: &serde_json::Value) {
    match raw.get("domain") {
        Some(value) => parse(value),
        None => reject(),
    }
}
`,
    "src/boundary/authorize.rs": `
/// BOUNDARY-INVARIANT: requests are decoded before domain use.
// malformed input is rejected.
fn authorize(account: Account) {
    match account.role {
        Role::Admin => allow(),
        Role::Reader => reject(),
    }
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "rust,common",
    "--files",
    "ocentra-enforcer.config.json",
    "package.json",
    "src/boundary/decode.rs",
    "src/boundary/authorize.rs",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const violations = parseReport(result).violations;
  assert.equal(
    violations.some((violation) => violation.ruleId === "BOUND-1.3" && violation.file === "src/boundary/decode.rs"),
    false,
    "raw JSON field lookup must not be treated as a domain decision",
  );
  assert.equal(
    violations.some((violation) => violation.ruleId === "BOUND-1.3" && violation.file === "src/boundary/authorize.rs"),
    true,
    "typed role branching must remain a boundary domain-decision violation",
  );
});

test("DOC-1.1 associates rustdoc across attributes and ignores restricted visibility", () => {
  const findings = scanRustDocumentationHints(
    (violations, root, file, line, ruleId, detail, source) => violations.push({ ruleId, line, detail, source }),
    ".",
    "src/example.rs",
    [
      "/// Public record documentation.",
      "#[derive(Clone)]",
      "pub struct PublicRecord;",
      "pub(crate) fn crate_helper() {}",
      "pub(super) struct ParentHelper;",
      "pub fn undocumented() {}",
    ],
  );
  assert.deepEqual(findings.map((finding) => finding.line), [6]);
});

test("TypeScript and common scanners catch source, test, generated, and secret policy violations", () => {
  const project = makeProject({
    "src/index.ts": `
${zodImport}
export { Thing } from "./thing";
${userIdAlias}
${manualBrandAlias}
${tsIgnoreComment}
const apiKey = "sk_test_1234567890abcdef";
`,
    "src/generated.ts": `
// @generated by a tool
export const generatedValue = 1;
`,
    "tests/example.test.ts": `
${viReplacementCall}("bad");
${testSkipCall}("bad", () => {
  ${expectCall}(true).toBe(true);
});
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "src",
    "tests",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = parseReport(result);
  assert.deepEqual(report.languages, ["typescript", "common"]);
  const ids = report.violations.map((violation) => violation.ruleId).sort();
  assert.equal(ids.includes("TS-1.1"), true);
  assert.equal(ids.includes("TS-1.2"), true);
  assert.equal(ids.includes("TS-1.3"), true);
  assert.equal(ids.includes("TS-2.1"), true);
  assert.equal(ids.includes("TS-3.1"), true);
  assert.equal(ids.includes("TEST-1.1"), true);
  assert.equal(ids.includes("SEC-1.1"), true);
  assert.equal(ids.includes("GEN-1.1"), true);
});

test("common scanner catches sensitive paths, generated output paths, and unguarded platform-specific commands", () => {
  const project = makeProject({
    ".env": "OPENAI_API_KEY=local-only\n",
    "test-results/report.json": '{"ok":true}\n',
    "scripts/run.mjs": `
import { spawnSync } from "node:child_process";
spawnSync("cmd", ["/c", "npm", "test"]);
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "common",
    "--files",
    ".env",
    "test-results",
    "scripts",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = parseReport(result);
  const ids = report.violations.map((violation) => violation.ruleId).sort();
  assert.equal(ids.includes("SEC-1.2"), true);
  assert.equal(ids.includes("GEN-1.2"), true);
  assert.equal(ids.includes("PORT-1.1"), true);
});

test("common scanner catches expanded secret, generated, and source-shape rules", () => {
  const project = makeProject({
    "secrets.txt": `
github = "${"ghp"}_abcdefghijklmnopqrstuvwxyz123456"
aws = "${"AKIA"}1234567890ABCDEF"
stripe = "${"sk"}_${"live"}_1234567890abcdefghijklmnop"
generic_secret = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890"
jwt_like = "eyJhbGciOiJIUzI1NiJ9.abcdefghijklmnopqrstuvwxyz.abcdefghijklmnopqrstuvwxyz"
${azureSecretName}="abcdefghijklmnopqrstuvwxyz123456"
SLACK_BOT_TOKEN="${"xoxb"}-1234567890-abcdefghijklmnop"
${apiTokenName}="abcdefghijklmnopqrstuvwxyz123456"
NPM_TOKEN="${"npm"}_abcdefghijklmnopqrstuvwxyz123456"
${privateKeyHeader}
`,
    "google-services.json": googleServiceJson,
    ".env.example": `${apiTokenName}="abcdefghijklmnopqrstuvwxyz123456"\n`,
    ".env.template": `${apiTokenName}="abcdefghijklmnopqrstuvwxyz123456"\n`,
    "id_rsa": `${openSshPrivateKeyHeader}\nabc\n`,
    "tests/snapshot.test.ts": `
expect(value).toMatchInlineSnapshot("2026-01-01T00:00:00.000Z ${"ghp"}_abcdefghijklmnopqrstuvwxyz123456");
`,
    "fixtures/creds.txt": fixtureSecretLine,
    "output/proof.json": '{"ok":true}\n',
    "src/generated/file.ts": `// @generated
${tsIgnoreComment}
export const value = 1;
`,
    "src/domain/generated.ts": `// @generated
export const domainGenerated = 1;
`,
    "src/generated/contracts.ts": `// @generated
// SOURCE_OF_TRUTH: generated output
export const contractValue = 1;
`,
    "src/generated/schema.json": `{"generated": true}
`,
    "src/utils.ts": `// hack quick fix for now
throw new Error("not implemented");
export const value = 1;
`,
    "src/domain/bad.ts": `
// copied from legacy module
import { Widget } from "../ui/widget";
import { readUser } from "../data/repo";
import { send } from "../infra/client";
export function duplicateName() {
  return Widget;
}
export function duplicateName() {
  return readUser(send);
}
`,
    "src/internal/api.ts": `
export function leakInternal() {
  return 1;
}
`,
    "scripts/security-scan.mjs": `
${execSyncCall}("${gitleaksCommand}");
${execSyncCall}("${trufflehogCommand}");
${execSyncCall}("${ruffCommand}");
${execSyncCall}("${pyrightCommand}");
${execSyncCall}("${mypyCommand}");
${execSyncCall}("npm install");
`,
    "eslint.config.mjs": `
export default [];
`,
    "tests/user-codec.test.ts": `
test("codec parses valid payload", () => {
  expect(parseUser({ id: "1" })).toEqual({ id: "1" });
});
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "secrets.txt",
    "google-services.json",
    ".env.example",
    ".env.template",
    "id_rsa",
    "tests/snapshot.test.ts",
    "fixtures/creds.txt",
    "output",
    "src",
    "scripts/security-scan.mjs",
    "eslint.config.mjs",
    "tests/user-codec.test.ts",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(parseReport(result).violations.map((violation) => violation.ruleId));
  for (const ruleId of [
    "SEC-2.1",
    "SEC-2.2",
    "SEC-2.3",
    "SEC-2.4",
    "SEC-2.5",
    "SEC-2.6",
    "SEC-2.7",
    "SEC-2.8",
    "SEC-2.9",
    "SEC-2.10",
    "SEC-2.11",
    "SEC-2.12",
    "SEC-2.13",
    "SEC-2.14",
    "SEC-2.16",
    "SEC-2.17",
    "SEC-2.18",
    "SEC-2.19",
    "SEC-2.20",
    "GEN-2.1",
    "GEN-2.2",
    "GEN-2.3",
    "GEN-2.4",
    "GEN-2.5",
    "GEN-2.6",
    "GEN-2.7",
    "GEN-2.8",
    "GEN-2.9",
    "GEN-2.10",
    "SRC-2.8",
    "SRC-2.9",
    "SRC-2.10",
    "SRC-2.11",
    "SRC-2.12",
    "SRC-2.13",
    "SRC-2.14",
    "SRC-2.15",
    "TS-7.12",
    "TS-7.13",
    "TS-8.10",
    "PY-5.5",
    "PY-5.6",
  ]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
  for (const violation of parseReport(result).violations.filter((finding) => finding.ruleId.startsWith("SEC-"))) {
    assert.equal(/ghp_|AKIA|sk_live_|eyJ/.test(violation.source ?? ""), false, `${violation.ruleId} source is redacted`);
  }
});

test("Python scanner catches lint/type suppressions and skipped tests", () => {
  const project = makeProject({
    "src/app.py": `
value = dynamic()  # type: ignore
other = 1  # noqa
UserId = str
`,
    "tests/test_app.py": `
import pytest

@pytest.mark.skip("not today")
def test_app():
    assert True
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "python,common",
    "--files",
    "src",
    "tests",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = parseReport(result);
  const ids = report.violations.map((violation) => violation.ruleId).sort();
  assert.equal(ids.includes("PY-1.1"), true);
  assert.equal(ids.includes("PY-1.2"), true);
  assert.equal(ids.includes("PY-1.3"), true);
  assert.equal(ids.includes("PY-2.1"), true);
  assert.equal(ids.includes("TEST-1.2"), true);
});

test("TypeScript scanner catches strict source slop rules", () => {
  const project = makeProject({
    "src/domain.ts": `
import { spawnSync } from "node:child_process";
${exportedUserIdAlias}
export type UsersById = Record<string, User>;
export type Users = Map<string, User>;
export type UserNames = string[];
export type Patch = Partial<User>;
export type Payload = Record<string, unknown>;
declare global {
  interface Window { bad: string }
}
export namespace BadNamespace {
  export const value = 1;
}
export enum BadEnum {
  One = "one",
}
export interface User {
  name?: string;
  createdAt: Date;
}
class Box {
  value!: string;
}
async function saveUserAsync(): Promise<void> {}
export function take(raw: unknown) {
  return raw;
}
export function timed(count: number, enabled: boolean, at: Date): Promise<unknown> {
  ${setTimeoutCall}(() => {}, 1);
  return Promise.resolve(at);
}
export const api = { parse: true };
export default function parse(raw: string): any {
  const parsed = JSON.parse(raw) as unknown as User;
  const url = process.env.API_URL!;
  const copied = { ...rawDto, ...userAny };
  const maybe = undefined;
  let temp = 1;
  sharedCache.add(temp);
  import("./late");
  eval("1 + 1");
  saveUserAsync();
  saveUserAsync().catch(() => {});
  console.log(url);
  if (!url) throw "missing url";
  if (maybe) return undefined;
  if (!raw) return null;
  return parsed as User;
}
`,
    "tests/domain.test.ts": `
import { test, expect, vi } from "vitest";

${testSkipCall}("skipped", () => {});
test("weak", () => {
  ${expectCall}.any(String);
  ${expectCall}(value).toBeTruthy();
  ${fetchCall}("/unit");
  ${setTimeoutCall}(() => {}, 1);
  ${viReplacementCall}("x");
  ${expectCall}(new Date().toISOString()).toMatchInlineSnapshot("2026-01-01T00:00:00.000Z");
});
test("empty", () => {});
test("no assertion", () => {
  const x = 1;
});
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "src/domain.ts",
    "tests/domain.test.ts",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(parseReport(result).violations.map((violation) => violation.ruleId));
  for (const ruleId of [
    "TS-6.1",
    "TS-6.2",
    "TS-6.3",
    "TS-6.4",
    "TS-6.5",
    "TS-6.6",
    "TS-6.7",
    "TS-6.8",
    "TS-6.9",
    "TS-6.10",
    "TS-6.11",
    "TS-6.12",
    "TS-6.13",
    "TS-6.15",
    "TS-6.16",
    "TS-6.17",
    "TS-6.18",
    "TS-6.19",
    "TS-6.20",
    "TS-6.21",
    "TS-6.22",
    "TS-6.23",
    "TS-6.24",
    "TS-6.25",
    "TS-6.26",
    "TS-6.27",
    "TS-6.28",
    "TS-6.29",
    "TS-6.30",
    "TS-6.31",
    "TS-6.32",
    "TS-6.33",
    "TS-6.34",
    "TS-6.35",
    "TS-6.36",
    "TS-6.37",
    "TS-6.38",
    "TS-6.39",
    "TS-6.40",
    "TS-8.1",
    "TS-8.2",
    "TS-8.3",
    "TS-8.4",
    "TS-8.5",
    "TS-8.6",
    "TS-8.7",
    "TS-8.8",
    "TS-8.9",
  ]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});

test("TypeScript optional-field and let-const rules stay scoped to domain-like paths", () => {
  const project = makeProject({
    "src/ui/component.ts": `
type UiShape = {
  optional?: string;
};

export function render() {
  let value = 1;
  return value;
}
`,
    "src/domain.ts": `
type DomainShape = {
  optional?: string;
};

export function build() {
  let value = 1;
  return value;
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript",
    "--files",
    "src/ui/component.ts",
    "src/domain.ts",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = parseReport(result);
  const uiViolations = report.violations.filter((violation) => violation.file === "src/ui/component.ts");
  assert.equal(uiViolations.some((violation) => violation.ruleId === "TS-6.28"), false);
  assert.equal(uiViolations.some((violation) => violation.ruleId === "TS-6.39"), false);
  const domainViolations = report.violations.filter((violation) => violation.file === "src/domain.ts");
  assert.equal(domainViolations.some((violation) => violation.ruleId === "TS-6.28"), true);
  assert.equal(domainViolations.some((violation) => violation.ruleId === "TS-6.39"), true);
});

test("TypeScript scanner catches index barrels", () => {
  const project = makeProject({
    "src/index.ts": `
export { UserId } from "./user-id";
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "src/index.ts",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(parseReport(result).violations.map((violation) => violation.ruleId));
  assert.equal(ids.has("TS-6.14"), true);
});

test("TypeScript scanner catches non-strict tsconfig", () => {
  const project = makeProject({
    "tsconfig.json": `
{
  "compilerOptions": {
    "strict": false
  }
}
`,
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "tsconfig.json",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(parseReport(result).violations.map((violation) => violation.ruleId));
  assert.equal(ids.has("TS-7.1"), true);
  for (const ruleId of ["TS-7.2", "TS-7.3", "TS-7.4", "TS-7.5", "TS-7.6", "TS-7.7", "TS-7.8", "TS-7.9"]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});

test("TypeScript scanner catches package toolchain policy", () => {
  const project = makeProject({
    "package.json": `
{
  "dependencies": {
    "left-pad": "^1.3.0",
    "zod": "latest"
  }
}
`,
    "packages/no-lock/package.json": `
{
  "dependencies": {
    "zod": "latest"
  }
}
`,
    "yarn.lock": "# duplicate package manager lock\n",
    "pnpm-lock.yaml": "lockfileVersion: '9.0'\n",
  });
  const result = run(project, [
    "scan",
    "--json",
    "--languages",
    "typescript,common",
    "--files",
    "package.json",
    "packages/no-lock/package.json",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(parseReport(result).violations.map((violation) => violation.ruleId));
  for (const ruleId of ["TS-7.10", "TS-7.11", "TS-7.14", "TS-7.15"]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});

test("common scanner catches boundary and architecture violations", () => {
  const project = makeProject({
    "ocentra-enforcer.config.json": JSON.stringify({
      schemaVersion: 2,
      profileName: "strict",
      failOn: ["error"],
      importBoundaryPolicies: [{ roots: ["src"], forbiddenImports: ["../infra"] }],
    }),
    "package.json": JSON.stringify({ name: "architecture-fixture", version: "0.0.0" }),
    "src/boundary/helpers.ts": `
type RawUserDto = { role: string };
type RawOrgDto = { id: string };
type AccountPayload = { id: string };
type LoginRequest = { id: string };
export function leak(rawInput: RawUserDto): RawUserDto {
  if (rawInput.role === "business-admin") return rawInput;
  return rawInput;
}
export function convert(rawInput: RawUserDto): string {
  return rawInput.role;
}
`,
    "src/domain/model.ts": `
import { leak } from "../boundary/helpers";
import { Widget } from "../ui/widget";
import { connect } from "../db/client";
import { get } from "../http/client";
import { adapter } from "../adapters/user";
import { initialize } from "../infra/config";
import { fixture } from "../test-support/user";
export function domainValue() {
  return [leak, Widget, connect, get, adapter, initialize, fixture];
}
`,
    "src/generated/contract.ts": `
// @generated
// sourceHash: abc
import { InternalSecret } from "../domain/internal/secret";
export const generatedContract = InternalSecret;
`,
    "src/cli.ts": `
import { domainValue } from "./domain/model";
import { connect } from "./infra/db";
domainValue(connect);
`,
    "src/cycle.ts": `
import { cycle } from "./cycle";
export function cycle() {
  return cycle;
}
`,
    "src/public-api.ts": `
export { unstableThing } from "./internal/unstable";
export type InternalUser = import("./internal/user").InternalUser;
export const a1 = 1;
export const a2 = 2;
export const a3 = 3;
export const a4 = 4;
export const a5 = 5;
export const a6 = 6;
export const a7 = 7;
export const a8 = 8;
export const a9 = 9;
export const a10 = 10;
export const a11 = 11;
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
    "src/boundary/helpers.ts",
    "src/domain/model.ts",
    "src/generated/contract.ts",
    "src/cli.ts",
    "src/cycle.ts",
    "src/public-api.ts",
  ]);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const ids = new Set(parseReport(result).violations.map((violation) => violation.ruleId));
  for (const ruleId of [
    "BOUND-1.1",
    "BOUND-1.2",
    "BOUND-1.3",
    "BOUND-1.4",
    "BOUND-1.5",
    "BOUND-1.6",
    "BOUND-1.8",
    "BOUND-1.9",
    "BOUND-1.10",
    "ARCH-1.1",
    "ARCH-1.2",
    "ARCH-1.3",
    "ARCH-1.4",
    "ARCH-1.5",
    "ARCH-1.6",
    "ARCH-1.7",
    "ARCH-1.8",
    "ARCH-1.9",
    "ARCH-1.10",
    "ARCH-1.11",
    "ARCH-1.12",
    "ARCH-1.13",
    "ARCH-1.14",
    "ARCH-1.15",
  ]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
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
  const commentOnlyIds = new Set(parseReport(commentOnly).violations.map((violation) => violation.ruleId));
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
  const selfImportIds = new Set(parseReport(selfImport).violations.map((violation) => violation.ruleId));
  assert.equal(selfImportIds.has("ARCH-1.9"), true);
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
  const ids = new Set(parseReport(result).violations.map((violation) => violation.ruleId));
  for (const ruleId of ["PY-5.1", "PY-5.2", "PY-5.3", "PY-5.4", "PY-5.7", "PY-5.8", "PY-5.9", "PY-5.10"]) {
    assert.equal(ids.has(ruleId), true, `${ruleId} should fail`);
  }
});
