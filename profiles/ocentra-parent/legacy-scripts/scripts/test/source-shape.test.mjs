import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { test } from 'node:test';

const repoRoot = resolve(import.meta.dirname, '..', '..');
const scriptPath = join(repoRoot, 'scripts', 'check-source-shape.mjs');

function writeFixture(root, filePath, source) {
  const fullPath = join(root, filePath);
  mkdirSync(dirname(fullPath), { recursive: true });
  writeFileSync(fullPath, source, 'utf8');
}

function runGuard(root, args = []) {
  return spawnSync(process.execPath, [scriptPath, ...args], {
    cwd: root,
    encoding: 'utf8',
  });
}

test('source shape legacy command delegates to Enforcer and rejects oversized TypeScript files', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-source-shape-'));
  try {
    writeFixture(
      root,
      'apps/portal/src/oversized.ts',
      Array.from({ length: 1001 }, () => 'const value = 1;').join('\n')
    );

    const result = runGuard(root, ['apps/portal/src/oversized.ts']);

    assert.equal(result.status, 1);
    assert.match(result.stderr, /file has 1001 lines/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('source shape legacy command accepts scoped files through Enforcer', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-source-shape-good-'));
  try {
    writeFixture(root, 'apps/portal/src/good.ts', 'export function ok() {\n  return 1;\n}\n');

    const result = runGuard(root, ['--files', 'apps/portal/src/good.ts']);

    assert.equal(result.status, 0);
    assert.match(result.stdout, /Ocentra Enforcer check source-shape passed/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
