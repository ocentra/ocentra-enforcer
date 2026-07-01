import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import test from 'node:test';

const script = 'scripts/check-no-placeholder-implementation.mjs';

test('placeholder guard allows Rust serde attributes that use approved temporary-override domain literals', () => {
  const tempDir = mkdtempSync(path.join(process.cwd(), 'crates', 'placeholder-guard-'));
  const tempFile = path.join(tempDir, 'policy_request_fixture.rs');

  try {
    writeFileSync(
      tempFile,
      [
        '#![forbid(unsafe_code)]',
        '',
        '#[derive(Debug)]',
        'pub enum PolicyRequestKind {',
        '    #[serde(rename = "temporary-override")]',
        '    TemporaryOverride,',
        '}',
        '',
      ].join('\n'),
      'utf8'
    );

    const result = spawnSync(process.execPath, [script, '--files', tempFile], {
      cwd: process.cwd(),
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    });

    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /Ocentra Enforcer check placeholder-implementation passed/u);
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

test('placeholder guard still rejects temporary markers in real comments', () => {
  const tempDir = mkdtempSync(path.join(process.cwd(), 'crates', 'placeholder-guard-'));
  const tempFile = path.join(tempDir, 'placeholder_comment_fixture.rs');

  try {
    writeFileSync(
      tempFile,
      [
        '#![forbid(unsafe_code)]',
        '',
        '// temporary workaround until ownership is implemented',
        'pub struct PolicySource;',
        '',
      ].join('\n'),
      'utf8'
    );

    const result = spawnSync(process.execPath, [script, '--files', tempFile], {
      cwd: process.cwd(),
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
    });

    assert.equal(result.status, 1);
    assert.match(result.stderr, /temporary marker found in production source/u);
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }
});
