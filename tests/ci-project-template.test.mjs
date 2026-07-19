import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { planImpacted } from '../scripts/ci/plan-impacted.mjs';
import { verifyWorkflowContract } from '../scripts/ci/verify-workflow-contract.mjs';

const metadata = {
  workspace_root: '/repo',
  workspace_members: ['domain 0.1', 'api 0.1', 'cli 0.1'],
  packages: [
    { id: 'domain 0.1', name: 'domain', manifest_path: '/repo/crates/domain/Cargo.toml', dependencies: [] },
    { id: 'api 0.1', name: 'api', manifest_path: '/repo/crates/api/Cargo.toml', dependencies: [{ name: 'domain', path: '/repo/crates/domain' }] },
    { id: 'cli 0.1', name: 'cli', manifest_path: '/repo/crates/cli/Cargo.toml', dependencies: [{ name: 'api', path: '/repo/crates/api' }] },
  ],
};

test('impact planning expands through reverse workspace dependencies', () => {
  const plan = planImpacted({ changedFiles: ['crates/domain/src/lib.rs'], metadata });
  assert.deepEqual(plan.packages, ['api', 'cli', 'domain']);
  assert.equal(plan.docsOnly, false);
});

test('workflow or manifest changes require full validation', () => {
  const workflow = planImpacted({ changedFiles: ['.github/workflows/ci.yml'], metadata });
  const manifest = planImpacted({ changedFiles: ['Cargo.lock'], metadata });
  assert.equal(workflow.fullRequired, true);
  assert.equal(workflow.graphContractChanged, true);
  assert.equal(manifest.fullRequired, true);
});

test('docs-only changes retain the fast path', () => {
  const plan = planImpacted({ changedFiles: ['docs/usage.md', 'README.md'], metadata });
  assert.deepEqual(plan.packages, []);
  assert.equal(plan.docsOnly, true);
  assert.equal(plan.fullRequired, false);
});

test('workflow contract validator rejects missing and mutable gates', () => {
  const root = mkdtempSync(path.join(tmpdir(), 'enforcer-ci-contract-'));
  const workflows = path.join(root, '.github', 'workflows');
  mkdirSync(workflows, { recursive: true });
  for (const name of ['ci.yml', 'dogfood.yml', 'release.yml']) {
    writeFileSync(path.join(workflows, name), 'uses: actions/checkout@v6\n');
  }
  const failures = verifyWorkflowContract(root);
  assert.ok(failures.length >= 4, 'seeded incomplete workflows must fail mechanically');
  assert.ok(failures.some((failure) => failure.includes('mutable major-version tag')));
});

test('checked-in workflows satisfy the reusable contract', () => {
  assert.deepEqual(verifyWorkflowContract(process.cwd()), []);
});
