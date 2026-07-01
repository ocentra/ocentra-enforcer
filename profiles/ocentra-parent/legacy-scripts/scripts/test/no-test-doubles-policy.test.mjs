import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { test } from 'node:test';

const repoRoot = resolve(import.meta.dirname, '..', '..');
const scriptPath = join(repoRoot, 'scripts', 'check-no-test-doubles.mjs');

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

test('test-double legacy command delegates to Enforcer and rejects common bypass APIs', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-test-doubles-'));
  try {
    writeFixture(root, 'packages/example/tests/bad.test.ts', "vi.mock('@scope/module')\nconst replacement = vi.fn()\n");

    const result = runGuard(root, ['packages/example/tests/bad.test.ts']);

    assert.equal(result.status, 1);
    assert.match(result.stderr, /test double|mock/iu);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('test-double legacy command accepts real assertions', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-test-doubles-good-'));
  try {
    writeFixture(
      root,
      'packages/example/tests/contract.test.ts',
      "import assert from 'node:assert/strict';\nassert.equal(1, 1);\n"
    );

    const result = runGuard(root, ['--files', 'packages/example/tests/contract.test.ts']);

    assert.equal(result.status, 0);
    assert.match(result.stdout, /Ocentra Enforcer check no-test-doubles passed/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
