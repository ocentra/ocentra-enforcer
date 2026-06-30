import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const SCRIPT = path.join(ROOT, 'scripts', 'rust-rules.mjs');

function makeProject(files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ocentra-enforcer-checks-'));
  for (const [rel, content] of Object.entries(files)) {
    const full = path.join(dir, rel);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content.trimStart(), 'utf8');
  }
  return dir;
}

function run(project, args) {
  return spawnSync(process.execPath, [SCRIPT, ...args, '--root', project], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

test('check no-zod-source runs a filtered scanner-backed rule set', () => {
  const project = makeProject({
    'src/index.ts': ['import { z } from "zo', 'd";\nexport const value = z.string();\n'].join(''),
  });
  const result = run(project, ['check', 'no-zod-source', '--json', '--files', 'src/index.ts']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.command, 'check');
  assert.equal(report.check, 'no-zod-source');
  assert.deepEqual([...new Set(report.violations.map((violation) => violation.ruleId))], ['TS-1.2']);
});

test('check source-shape applies project-configured shape policies', () => {
  const project = makeProject({
    'ocentra-enforcer.config.json': JSON.stringify({
      profileName: 'shape-test',
      sourceShapePolicies: [
        {
          roots: ['src'],
          extensions: ['.ts'],
          kind: 'typescript',
          maxClasses: 0,
          maxExports: 0,
          maxFunctionLines: 1,
          maxLines: 2,
        },
      ],
    }),
    'src/app.ts': `
export function tooLong() {
  const value = 1;
  return value;
}
`,
  });
  const result = run(project, ['check', 'source-shape', '--json']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.profileName, 'shape-test');
  assert.equal(report.violations.some((violation) => violation.ruleId === 'SRC-1.1'), true);
});

test('check required-tests catches source workspaces without test scaffolds', () => {
  const project = makeProject({
    'packages/app/package.json': JSON.stringify({ name: '@fixture/app' }),
    'packages/app/src/index.ts': 'export const value = 1;\n',
  });
  const result = run(project, ['check', 'required-tests', '--json']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'TEST-2.1'), true);
});

test('check single-source-contracts accepts the migrated Ocentra Parent contract config shape', () => {
  const project = makeProject({
    'config/owner.json': JSON.stringify({ ports: { portal: '4478' } }),
    'contracts.json': JSON.stringify({
      contracts: [
        {
          name: 'ports',
          ownerPath: 'config/owner.json',
          values: [{ name: 'portal', jsonPath: 'ports.portal' }],
          scanRoots: ['src'],
        },
      ],
    }),
    'src/app.ts': 'export const copiedPort = "4478";\n',
  });
  const result = run(project, ['check', 'single-source-contracts', '--check-config', 'contracts.json', '--json']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'CONTRACT-1.1'), true);
});

test('check sbom supports dry-run without writing generated artifacts', () => {
  const project = makeProject({
    'package.json': JSON.stringify({ name: 'sbom-fixture', version: '1.0.0' }),
  });
  const result = run(project, ['check', 'sbom', '--dry-run', '--json']);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.equal(fs.existsSync(path.join(project, 'target')), false);
});

test('check weak-assertions catches low-value assertions', () => {
  const project = makeProject({
    'tests/example.test.ts': `
test("value", () => {
  expect(result).${'toBe' + 'Truthy'}();
});
`,
  });
  const result = run(project, ['check', 'weak-assertions', '--json', '--files', 'tests/example.test.ts']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'TEST-1.2'), true);
});

test('check skipped-focused-tests catches Rust ignored tests', () => {
  const project = makeProject({
    'tests/ignored_test.rs': `
#[test]
#[ignore]
fn ignored() {}
`,
  });
  const result = run(project, ['check', 'skipped-focused-tests', '--json', '--files', 'tests/ignored_test.rs']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'TEST-1.3'), true);
});

test('check validation-bypass catches formatter and lint bypass comments', () => {
  const project = makeProject({
    'src/app.ts': ['// prettier-', 'ignore\nexport const value = 1;\n'].join(''),
  });
  const result = run(project, ['check', 'validation-bypass', '--json', '--files', 'src/app.ts']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'TS-2.1'), true);
});

test('check placeholder-implementation catches production placeholders', () => {
  const project = makeProject({
    'src/app.ts': `
export function incomplete() {
  throw new Error("not implemented");
}
`,
  });
  const result = run(project, ['check', 'placeholder-implementation', '--json', '--files', 'src/app.ts']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'SRC-1.2'), true);
});
