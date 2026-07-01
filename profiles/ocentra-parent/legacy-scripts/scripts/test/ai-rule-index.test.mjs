import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';

const repoRoot = resolve(import.meta.dirname, '..', '..');
const scriptPath = join(repoRoot, 'scripts', 'check-ai-rule-index.mjs');

function writeFixture(root, filePath, source) {
  const fullPath = join(root, filePath);
  mkdirSync(dirname(fullPath), { recursive: true });
  writeFileSync(fullPath, source, 'utf8');
}

function runGuard(root) {
  return spawnSync(process.execPath, [scriptPath], {
    cwd: root,
    encoding: 'utf8',
  });
}

test('ai rule index legacy command rejects unlinked rule files', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-ai-rule-index-'));
  try {
    writeFixture(root, 'AGENTS.md', '.ocentra-ai/rules/ocentra-parent-rules.mdc\n');
    writeFixture(root, '.ocentra-ai/rules/ocentra-parent-rules.mdc', '# Index\n');
    writeFixture(root, '.ocentra-ai/rules/missing-rule.mdc', '# Missing\n');

    const result = runGuard(root);

    assert.equal(result.status, 1);
    assert.match(result.stderr, /missing-rule\.mdc is not linked/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('ai rule index legacy command rejects missing AGENTS reference', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-ai-rule-index-agents-'));
  try {
    writeFixture(root, 'AGENTS.md', '# Agent guide\n');
    writeFixture(root, '.ocentra-ai/rules/ocentra-parent-rules.mdc', 'linked-rule.mdc\n');
    writeFixture(root, '.ocentra-ai/rules/linked-rule.mdc', '# Linked\n');

    const result = runGuard(root);

    assert.equal(result.status, 1);
    assert.match(result.stderr, /AGENTS\.md must reference/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('ai rule index legacy command accepts indexed granular rules', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-ai-rule-index-good-'));
  try {
    writeFixture(root, 'AGENTS.md', '.ocentra-ai/rules/ocentra-parent-rules.mdc\n');
    writeFixture(root, '.ocentra-ai/rules/ocentra-parent-rules.mdc', 'linked-rule.mdc\n');
    writeFixture(root, '.ocentra-ai/rules/linked-rule.mdc', '# Linked\n');

    const result = runGuard(root);

    assert.equal(result.status, 0);
    assert.match(result.stdout, /Ocentra Enforcer check ai-rule-index passed/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
