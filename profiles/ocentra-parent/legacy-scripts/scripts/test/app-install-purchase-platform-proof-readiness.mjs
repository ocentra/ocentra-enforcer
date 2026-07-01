import { execFile } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const proofPath = resolve(repoRoot, 'test-results', 'app-install-purchase-platform-proof-readiness', 'proof.json');

await run('npm', ['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);
await run('npm', [
  'run',
  'test',
  '--workspace',
  '@ocentra-parent/schema-domain',
  '--',
  'tests/unit/app-install-purchase-platform-proof-readiness.test.ts',
]);

const moduleUrl = pathToFileURL(
  resolve(repoRoot, 'packages', 'schema-domain', 'dist', 'app-install-purchase-platform-proof-readiness.js')
).href;
const { AppInstallPurchasePlatformProofReadinessProofReadModel, summarizeAppInstallPurchasePlatformProofReadiness } =
  await import(moduleUrl);

const summary = summarizeAppInstallPurchasePlatformProofReadiness(
  AppInstallPurchasePlatformProofReadinessProofReadModel
);

assertEqual(summary.platformRows, 5, 'platform rows');
assertEqual(summary.manualProofRequiredRows, 2, 'manual-proof-required rows');
assertEqual(summary.policyBlockedRows, 2, 'policy-blocked rows');
assertEqual(summary.unavailableRows, 1, 'unavailable rows');
assertEqual(summary.providerExecutedRows, 0, 'provider executed rows');
assertEqual(summary.adapterImplementedRows, 0, 'adapter implemented rows');

const proof = {
  proof: 'app-install-purchase-platform-proof-readiness',
  commit: await gitHead(),
  branch: await gitBranch(),
  generatedAt: new Date().toISOString(),
  summary,
  sourceProofs: {
    limitationSummary: 'app-install-purchase-limitation-summary-proof',
  },
  nonClaims: AppInstallPurchasePlatformProofReadinessProofReadModel.nonClaims,
  knownGaps: AppInstallPurchasePlatformProofReadinessProofReadModel.knownGaps,
  docs: {
    feature: 'docs/features/app-install-purchase-approval.md',
    expectation: 'docs/expectations/app-install-purchase-approval.md',
    checklist: 'docs/product-capability-checklist.md unchanged; product-status checklist remains primary-sequenced',
  },
  evidence: {
    source: 'packages/schema-domain/src/app-install-purchase-platform-proof-readiness.ts',
    tests: 'packages/schema-domain/tests/unit/app-install-purchase-platform-proof-readiness.test.ts',
    output: 'test-results/app-install-purchase-platform-proof-readiness/proof.json',
  },
};

await mkdir(dirname(proofPath), { recursive: true });
await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`app-install-purchase-platform-proof-readiness-ok:${proofPath}`);

async function run(command, args) {
  await execFileAsync(command, args, { cwd: repoRoot, shell: process.platform === 'win32' });
}

async function gitHead() {
  const { stdout } = await execFileAsync('git', ['rev-parse', 'HEAD'], { cwd: repoRoot });
  return stdout.trim();
}

async function gitBranch() {
  const { stdout } = await execFileAsync('git', ['branch', '--show-current'], { cwd: repoRoot });
  return stdout.trim();
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, got ${actual}`);
  }
}
