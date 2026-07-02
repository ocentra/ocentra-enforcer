import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const SCRIPT = path.join(ROOT, 'scripts', 'rust-rules.mjs');
const TEST_CLI_MAX_BUFFER = 32 * 1024 * 1024;

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
    maxBuffer: TEST_CLI_MAX_BUFFER,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function git(project, args) {
  const result = spawnSync('git', args, {
    cwd: project,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  assert.equal(result.status, 0, `git ${args.join(' ')}\n${result.stdout}\n${result.stderr}`);
  return result;
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

test('check no-naked-domain-strings ignores generated TypeScript DTO folders', () => {
  const project = makeProject({
    'packages/schema-domain/src/generated/contracts.ts': 'export type GeneratedDeviceId = string;\n',
  });
  const result = run(project, [
    'check',
    'no-naked-domain-strings',
    '--json',
    '--files',
    'packages/schema-domain/src/generated/contracts.ts',
  ]);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.deepEqual(report.violations, []);
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

test('check source-shape respects file scope instead of scanning the whole repo', () => {
  const project = makeProject({
    'ocentra-enforcer.config.json': JSON.stringify({
      profileName: 'shape-scope-test',
      sourceShapePolicies: [
        {
          roots: ['src'],
          extensions: ['.ts'],
          kind: 'typescript',
          maxExports: 0,
        },
      ],
    }),
    'src/good.ts': 'const localValue = 1;\n',
    'src/bad.ts': 'export const leaked = 1;\n',
  });
  const result = run(project, ['check', 'source-shape', '--json', '--files', 'src/good.ts']);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.deepEqual(report.scope.files, ['src/good.ts']);
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

test('check required-tests rejects inline tests in source files', () => {
  const project = makeProject({
    'packages/app/package.json': JSON.stringify({ name: '@fixture/app' }),
    'packages/app/src/index.ts': `
export const value = 1;
describe("value", () => {});
`,
    'packages/app/tests/index.test.ts': 'test("value", () => expect(1).toBe(1));\n',
    'packages/python/package.json': JSON.stringify({ name: '@fixture/python' }),
    'packages/python/src/module.py': `
def value():
    return 1

def test_value():
    assert value() == 1
`,
    'packages/python/tests/module.test.ts': 'test("placeholder", () => expect(1).toBe(1));\n',
    'crates/core/Cargo.toml': '[package]\nname = "core"\nversion = "0.1.0"\nedition = "2021"\n',
    'crates/core/src/lib.rs': `
pub fn value() -> u8 {
    1
}

#[cfg(test)]
mod tests {}
`,
    'crates/core/tests/value.rs': '#[test]\nfn value_is_stable() { assert_eq!(1, 1); }\n',
  });
  const result = run(project, ['check', 'required-tests', '--json']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  const inlineFindings = report.violations.filter((violation) => violation.ruleId === 'TEST-2.2');
  assert.equal(inlineFindings.length, 3);
  assert.equal(inlineFindings.some((violation) => violation.file === 'packages/app/src/index.ts'), true);
  assert.equal(inlineFindings.some((violation) => violation.file === 'packages/python/src/module.py'), true);
  assert.equal(inlineFindings.some((violation) => violation.file === 'crates/core/src/lib.rs'), true);
});

test('check required-tests requires organized Rust tests instead of inline modules', () => {
  const project = makeProject({
    'crates/core/Cargo.toml': '[package]\nname = "core"\nversion = "0.1.0"\nedition = "2021"\n',
    'crates/core/src/lib.rs': `
pub fn value() -> u8 {
    1
}

#[cfg(test)]
mod tests {}
`,
  });
  const result = run(project, ['check', 'required-tests', '--json']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'TEST-2.1'), true);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'TEST-2.2'), true);
});

test('check required-tests limits package discovery to touched project roots', () => {
  const project = makeProject({
    'packages/covered/package.json': JSON.stringify({ name: '@fixture/covered' }),
    'packages/covered/src/index.ts': 'export const value = 1;\n',
    'packages/covered/tests/index.test.ts': 'test("value", () => expect(1).toBe(1));\n',
    'packages/missing/package.json': JSON.stringify({ name: '@fixture/missing' }),
    'packages/missing/src/index.ts': 'export const value = 1;\n',
  });
  const result = run(project, ['check', 'required-tests', '--json', '--files', 'packages/covered/src/index.ts']);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.equal(report.scope.files.includes('packages/covered/src/index.ts'), true);
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

test('check single-source-contracts supports source object values and file scope', () => {
  const project = makeProject({
    'config/kinds.ts': `
export const RuntimeKinds = {
  parentReady: "parent.ready"
} as const;
`,
    'contracts.json': JSON.stringify({
      contracts: [
        {
          name: 'runtimeKinds',
          ownerPath: 'config/kinds.ts',
          values: [{ name: 'parentReady', sourceObjectPath: 'RuntimeKinds.parentReady' }],
          scanRoots: ['src'],
        },
      ],
    }),
    'src/good.ts': 'export const localValue = "not copied";\n',
    'src/bad.ts': 'export const copied = "parent.ready";\n',
  });
  const good = run(project, ['check', 'single-source-contracts', '--check-config', 'contracts.json', '--json', '--files', 'src/good.ts']);
  assert.equal(good.status, 0, good.stdout || good.stderr);
  const bad = run(project, ['check', 'single-source-contracts', '--check-config', 'contracts.json', '--json', '--files', 'src/bad.ts']);
  assert.notEqual(bad.status, 0, bad.stdout || bad.stderr);
  const report = JSON.parse(bad.stdout);
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

test('check generated-artifacts --tracked catches tracked output paths', () => {
  const project = makeProject({
    'package.json': JSON.stringify({ name: 'generated-fixture', version: '1.0.0' }),
    'output/proof.json': '{}\n',
  });
  git(project, ['init', '-q']);
  git(project, ['add', 'package.json', 'output/proof.json']);
  const result = run(project, ['check', 'generated-artifacts', '--tracked', '--json']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'GEN-1.2'), true);
});

test('check generated-artifacts --tracked ignores untracked output paths', () => {
  const project = makeProject({
    'package.json': JSON.stringify({ name: 'generated-untracked-fixture', version: '1.0.0' }),
    'test-results/proof.json': '{}\n',
  });
  git(project, ['init', '-q']);
  git(project, ['add', 'package.json']);
  const result = run(project, ['check', 'generated-artifacts', '--tracked', '--json']);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
});

test('check secrets --staged scans only staged files', () => {
  const project = makeProject({
    '.env.local': ['API_', 'KEY="abcdefghijklmnop"', '\n'].join(''),
    'src/app.ts': 'export const safe = 1;\n',
  });
  git(project, ['init', '-q']);
  git(project, ['add', '.env.local']);
  const result = run(project, ['check', 'secrets', '--staged', '--json']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'SEC-1.1'), true);
  assert.deepEqual(report.scope.files, ['.env.local']);
});

test('check secrets catches unquoted OpenAI keys', () => {
  const project = makeProject({
    'secret.md': `OPENAI_API_KEY=${'sk-' + 'abcdefghijklmnopqrstuvwxyz123456'}\n`,
  });
  git(project, ['init', '-q']);
  git(project, ['add', 'secret.md']);
  const result = run(project, ['check', 'secrets', '--staged', '--json']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'SEC-1.1' && /OpenAI key/u.test(violation.detail)), true);
});

test('check secrets --staged does not scan the workspace when no files are staged', () => {
  const project = makeProject({
    'src/app.ts': ['const api', 'Key = "abcdefghijklmnop";\n'].join(''),
  });
  git(project, ['init', '-q']);
  const result = run(project, ['check', 'secrets', '--staged', '--json']);
  assert.equal(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.ok, true);
  assert.deepEqual(report.violations, []);
});

test('check import-boundaries catches configured forbidden imports', () => {
  const project = makeProject({
    'ocentra-enforcer.config.json': JSON.stringify({
      profileName: 'import-boundary-test',
      importBoundaryPolicies: [
        {
          roots: ['apps/web'],
          forbiddenImports: ['@domain/*'],
          message: 'apps/web must not import domain internals directly',
        },
      ],
    }),
    'apps/web/src/index.ts': 'import { value } from "@domain/core";\nexport const result = value;\n',
  });
  const result = run(project, ['check', 'import-boundaries', '--json', '--files', 'apps/web/src/index.ts']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.profileName, 'import-boundary-test');
  assert.equal(report.violations.some((violation) => violation.ruleId === 'TS-4.1'), true);
});

test('check architecture-policy aggregates configured reusable checks', () => {
  const project = makeProject({
    'ocentra-enforcer.config.json': JSON.stringify({
      profileName: 'architecture-policy-test',
      architecturePolicyChecks: ['no-zod-source'],
    }),
    'src/schema.ts': ['import { z } from "zo', 'd";\nexport const value = z.string();\n'].join(''),
  });
  const result = run(project, ['check', 'architecture-policy', '--json', '--files', 'src/schema.ts']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.check, 'architecture-policy');
  assert.deepEqual(report.checks, [{ check: 'no-zod-source', ok: false, violations: 1 }]);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'TS-1.2'), true);
});

test('architecture check now routes to full architecture-policy instead of reexports only', () => {
  const project = makeProject({
    'ocentra-enforcer.config.json': JSON.stringify({
      profileName: 'architecture-policy-test',
      architecturePolicyChecks: ['no-zod-source'],
    }),
    'src/schema.ts': ['import { z } from "zo', 'd";\nexport const value = z.string();\n'].join(''),
  });
  const result = run(project, ['architecture', 'check', '--json', '--language', 'rust', '--files', 'src/schema.ts']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.check, 'architecture-policy');
  assert.deepEqual(report.checks, [{ check: 'no-zod-source', ok: false, violations: 1 }]);
  assert.equal(report.violations.some((violation) => violation.ruleId === 'TS-1.2'), true);
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
