import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const scannerPath = fileURLToPath(new URL('../security/scan-staged-secrets.mjs', import.meta.url));

function withTempRepo(testBody) {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-parent-secret-scan-'));
  try {
    return testBody(root);
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
}

function runScanner(cwd, args = []) {
  return spawnSync(process.execPath, [scannerPath, ...args], {
    cwd,
    encoding: 'utf8',
    env: cleanGitEnv(),
  });
}

function combinedOutput(result) {
  return `${result.stdout}\n${result.stderr}`;
}

function runGit(cwd, args) {
  return spawnSync('git', args, {
    cwd,
    encoding: 'utf8',
    env: cleanGitEnv(),
  });
}

function cleanGitEnv() {
  return Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith('GIT_')));
}

test('repository secret scan rejects forbidden sensitive filenames', () => {
  withTempRepo((root) => {
    writeFileSync(join(root, '.env'), 'OCENTRA_PARENT_TOKEN=local-only\n', 'utf8');

    const result = runScanner(root, ['--repo']);

    assert.notEqual(result.status, 0);
    assert.match(combinedOutput(result), /\.env:1: error SEC-1\.2/u);
    assert.match(combinedOutput(result), /Reason: forbidden sensitive file path/u);
  });
});

test('staged secret scan rejects forbidden sensitive filenames', () => {
  withTempRepo((root) => {
    assert.equal(runGit(root, ['init']).status, 0);
    writeFileSync(join(root, '.env'), 'OCENTRA_PARENT_TOKEN=local-only\n', 'utf8');
    assert.equal(runGit(root, ['add', '.env']).status, 0);

    const result = runScanner(root);

    assert.notEqual(result.status, 0);
    assert.match(combinedOutput(result), /\.env:1: error SEC-1\.2/u);
    assert.match(combinedOutput(result), /Reason: forbidden sensitive file path/u);
  });
});

test('repository secret scan allows environment templates while still scanning their contents', () => {
  withTempRepo((root) => {
    writeFileSync(join(root, '.env.example'), 'OCENTRA_PARENT_AGENT_PORT=4477\n', 'utf8');

    const result = runScanner(root, ['--repo']);

    assert.equal(result.status, 0);
    assert.match(result.stdout, /Ocentra Enforcer check secrets passed/u);
  });
});

test('staged secret scan allows risk-benefit proof slugs', () => {
  withTempRepo((root) => {
    assert.equal(runGit(root, ['init']).status, 0);
    writeFileSync(
      join(root, 'proof.md'),
      'output/browser-plan-proof/game-11-game-risk-benefit-signal-model/\n',
      'utf8'
    );
    assert.equal(runGit(root, ['add', 'proof.md']).status, 0);

    const result = runScanner(root);

    assert.equal(result.status, 0);
    assert.match(result.stdout, /Ocentra Enforcer check secrets passed/u);
  });
});

test('staged secret scan rejects OpenAI keys at token boundaries', () => {
  withTempRepo((root) => {
    assert.equal(runGit(root, ['init']).status, 0);
    writeFileSync(join(root, 'secret.md'), `OPENAI_API_KEY=${'sk-' + 'abcdefghijklmnopqrstuvwxyz123456'}\n`, 'utf8');
    assert.equal(runGit(root, ['add', 'secret.md']).status, 0);

    const result = runScanner(root);

    assert.notEqual(result.status, 0);
    assert.match(combinedOutput(result), /secret\.md:1: error SEC-1\.1/u);
    assert.match(combinedOutput(result), /Reason: OpenAI key found/u);
  });
});
