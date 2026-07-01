import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

const RepoRoot = resolve(import.meta.dirname, '..', '..');
const GuardScript = join(RepoRoot, 'scripts', 'check-no-naked-domain-strings.mjs');

function runGuard(cwd) {
  return spawnSync(process.execPath, [GuardScript], {
    cwd,
    encoding: 'utf8',
  });
}

function writeFixture(root, path, source) {
  const fullPath = join(root, path);
  mkdirSync(dirname(fullPath), { recursive: true });
  writeFileSync(fullPath, source, 'utf8');
}

test('naked domain string guard rejects manual source aliases', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-naked-domain-manual-'));
  try {
    writeFixture(root, 'packages/manual-domain/src/contracts.ts', 'export type DeviceId = string;\n');

    const result = runGuard(root);

    assert.equal(result.status, 1);
    assert.match(result.stderr, /naked domain string alias DeviceId/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('naked domain string guard skips Rust-generated DTO folders', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-naked-domain-generated-'));
  try {
    writeFixture(
      root,
      'packages/schema-domain/src/generated/contracts.ts',
      'export type GeneratedDeviceId = string;\n'
    );

    const result = runGuard(root);

    assert.equal(result.status, 0);
    assert.match(result.stdout, /Ocentra Enforcer check no-naked-domain-strings passed/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
