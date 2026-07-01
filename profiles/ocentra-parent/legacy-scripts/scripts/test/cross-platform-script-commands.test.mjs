import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { test } from 'node:test';

const repoRoot = resolve(import.meta.dirname, '..', '..');
const scriptPath = join(repoRoot, 'scripts', 'check-cross-platform-script-commands.mjs');

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

test('cross-platform legacy command delegates to Enforcer and rejects unguarded Windows npm shell invocations', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-cross-platform-'));
  try {
    writeFixture(root, 'scripts/test/bad-proof.mjs', "runCommand('cmd', ['/c', 'npm', 'run', 'build']);\n");

    const result = runGuard(root, ['scripts/test/bad-proof.mjs']);

    assert.equal(result.status, 1);
    assert.match(result.stderr, /PORT-1\.1|platform guard|cmd/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('cross-platform legacy command accepts explicit Windows-only branches', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-cross-platform-good-'));
  try {
    writeFixture(
      root,
      'scripts/test/windows-proof.mjs',
      [
        "if (process.platform === 'win32') {",
        "  runCommand('cmd', ['/c', 'npm', 'run', 'build']);",
        '}',
        "run('npm', ['run', 'build']);",
      ].join('\n')
    );

    const result = runGuard(root, ['--files', 'scripts/test/windows-proof.mjs']);

    assert.equal(result.status, 0);
    assert.match(result.stdout, /Ocentra Enforcer check cross-platform-script-commands passed/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
