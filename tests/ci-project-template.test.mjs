import assert from 'node:assert/strict';
import { cpSync, mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
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

test('workflow contract rejects caching generated Cargo target output', () => {
  const root = mkdtempSync(path.join(tmpdir(), 'enforcer-ci-target-cache-'));
  cpSync(path.join(process.cwd(), '.github'), path.join(root, '.github'), { recursive: true });
  const workflow = path.join(root, '.github', 'workflows', 'ci.yml');
  const cachedTarget = readFileSync(workflow, 'utf8').replace(
    '            ~/.cargo/git\n',
    '            ~/.cargo/git\n            target\n',
  );
  writeFileSync(workflow, cachedTarget);
  const failures = verifyWorkflowContract(root);
  assert.ok(failures.some((failure) => failure.includes('generated target output')));
});

test('workflow contract rejects a branch-local scanner labelled as frozen', () => {
  const root = mkdtempSync(path.join(tmpdir(), 'enforcer-ci-frozen-scanner-'));
  cpSync(path.join(process.cwd(), '.github'), path.join(root, '.github'), { recursive: true });
  const dogfood = path.join(root, '.github', 'workflows', 'dogfood.yml');
  const localScanner = readFileSync(dogfood, 'utf8').replace(
    'node "$FROZEN_SCANNER_DIR/scripts/ocentra-enforcer.mjs" scan --root "$GITHUB_WORKSPACE" --languages rust --workspace',
    'node scripts/ocentra-enforcer.mjs scan --root . --languages rust --workspace',
  );
  writeFileSync(dogfood, localScanner);
  const failures = verifyWorkflowContract(root);
  assert.ok(failures.some((failure) => failure.includes('branch-local scanner')));
});

test('workflow contract rejects legacy frozen verification outside the Rust scan', () => {
  const root = mkdtempSync(path.join(tmpdir(), 'enforcer-ci-frozen-verify-'));
  cpSync(path.join(process.cwd(), '.github'), path.join(root, '.github'), { recursive: true });
  const dogfood = path.join(root, '.github', 'workflows', 'dogfood.yml');
  const legacyVerify = readFileSync(dogfood, 'utf8').replace(
    'node "$FROZEN_SCANNER_DIR/scripts/ocentra-enforcer.mjs" scan --root "$GITHUB_WORKSPACE" --languages rust --workspace',
    'node "$FROZEN_SCANNER_DIR/scripts/ocentra-enforcer.mjs" scan --root "$GITHUB_WORKSPACE" --languages rust --workspace\n          node "$FROZEN_SCANNER_DIR/scripts/ocentra-enforcer.mjs" verify ci --root "$GITHUB_WORKSPACE" --profile strict',
  );
  writeFileSync(dogfood, legacyVerify);
  const failures = verifyWorkflowContract(root);
  assert.ok(failures.some((failure) => failure.includes('legacy verify profile')));
});

test('workflow contract requires a shell that expands frozen scanner variables', () => {
  const root = mkdtempSync(path.join(tmpdir(), 'enforcer-ci-frozen-shell-'));
  cpSync(path.join(process.cwd(), '.github'), path.join(root, '.github'), { recursive: true });
  const dogfood = path.join(root, '.github', 'workflows', 'dogfood.yml');
  const powerShellGate = readFileSync(dogfood, 'utf8').replace(
    '      - name: Frozen Enforcer Rust workspace scan\n        shell: bash',
    '      - name: Frozen Enforcer full workspace gate',
  );
  writeFileSync(dogfood, powerShellGate);
  const failures = verifyWorkflowContract(root);
  assert.ok(failures.some((failure) => failure.includes('Frozen Enforcer Rust workspace scan')));
});

test('workflow contract rejects an invalidly indented reusable setup action', () => {
  const root = mkdtempSync(path.join(tmpdir(), 'enforcer-ci-action-indent-'));
  cpSync(path.join(process.cwd(), '.github'), path.join(root, '.github'), { recursive: true });
  const action = path.join(root, '.github', 'actions', 'setup-enforcer', 'action.yml');
  const malformed = readFileSync(action, 'utf8').replace(
    '    - name: Install locked Node dependencies',
    '  - name: Install locked Node dependencies',
  );
  writeFileSync(action, malformed);
  const failures = verifyWorkflowContract(root);
  assert.ok(failures.some((failure) => failure.includes('actions/setup-enforcer/action.yml')));
});

test('workflow contract keeps release publishing off the integration branch', () => {
  const root = mkdtempSync(path.join(tmpdir(), 'enforcer-ci-release-trigger-'));
  cpSync(path.join(process.cwd(), '.github'), path.join(root, '.github'), { recursive: true });
  const release = path.join(root, '.github', 'workflows', 'release.yml');
  const integrationRelease = readFileSync(release, 'utf8').replace(
    '    branches: [main]\n    tags:',
    '    branches: [main, rust-build]\n    tags:',
  );
  writeFileSync(release, integrationRelease);
  const failures = verifyWorkflowContract(root);
  assert.ok(failures.some((failure) => failure.includes('workflows/release.yml')));
});

test('workflow contract keeps release ancestry and publishing tag-only', () => {
  const root = mkdtempSync(path.join(tmpdir(), 'enforcer-ci-release-tag-only-'));
  cpSync(path.join(process.cwd(), '.github'), path.join(root, '.github'), { recursive: true });
  const release = path.join(root, '.github', 'workflows', 'release.yml');
  const pullRequestPublish = readFileSync(release, 'utf8')
    .replace(
      "- name: Require the release tag commit to be on main\n        if: startsWith(github.ref, 'refs/tags/v')",
      '- name: Require the release tag commit to be on main',
    )
    .replace(
      "build:\n    if: startsWith(github.ref, 'refs/tags/v')",
      'build:',
    )
    .replace(
      "publish:\n    if: startsWith(github.ref, 'refs/tags/v')",
      'publish:',
    );
  writeFileSync(release, pullRequestPublish);
  const failures = verifyWorkflowContract(root);
  assert.ok(failures.some((failure) => failure.includes('release tag commit')));
  assert.ok(failures.some((failure) => failure.includes("build:\n")));
  assert.ok(failures.some((failure) => failure.includes("publish:\n")));
});
