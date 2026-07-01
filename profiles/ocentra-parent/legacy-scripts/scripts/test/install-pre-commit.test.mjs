import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, normalize } from 'node:path';
import test from 'node:test';

import { installPreCommitHook, resolvePreCommitHookPath } from '../git-hooks/install-pre-commit.mjs';

test('pre-commit installer uses git hook path for linked worktrees', () => {
  const workspaceRoot = mkdtempSync(join(tmpdir(), 'ocentra-parent-hook-test-'));
  const repoRoot = join(workspaceRoot, 'repo');
  const worktreeRoot = join(workspaceRoot, 'worker');

  git(workspaceRoot, ['init', repoRoot]);
  git(repoRoot, ['config', 'user.email', 'test@example.invalid']);
  git(repoRoot, ['config', 'user.name', 'Ocentra Test']);
  writeFileSync(join(repoRoot, 'README.md'), '# test\n', 'utf8');
  git(repoRoot, ['add', 'README.md']);
  git(repoRoot, ['commit', '--no-verify', '-m', 'init']);
  git(repoRoot, ['worktree', 'add', '-b', 'worker', worktreeRoot]);

  const expectedHookPath = git(worktreeRoot, ['rev-parse', '--git-path', 'hooks/pre-commit']);
  const resolvedHookPath = resolvePreCommitHookPath(worktreeRoot);
  const installedHookPath = installPreCommitHook(worktreeRoot);

  assert.equal(normalize(resolvedHookPath), normalize(expectedHookPath));
  assert.equal(normalize(installedHookPath), normalize(expectedHookPath));
  assert.equal(existsSync(installedHookPath), true);
  assert.match(readFileSync(installedHookPath, 'utf8'), /run-ocentra-enforcer\.mjs coordination hub:guard/u);
  assert.equal(existsSync(join(worktreeRoot, '.git', 'hooks')), false);
});

function git(cwd, args) {
  return execFileSync('git', args, {
    cwd,
    encoding: 'utf8',
    env: cleanGitEnv(),
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim();
}

function cleanGitEnv() {
  return Object.fromEntries(Object.entries(process.env).filter(([key]) => !key.startsWith('GIT_')));
}
