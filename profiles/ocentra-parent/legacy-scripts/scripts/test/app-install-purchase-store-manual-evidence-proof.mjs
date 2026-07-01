import { execFile } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const proofPath = resolve(repoRoot, 'test-results', 'app-install-purchase-store-manual-evidence-proof', 'proof.json');

await run('npm', ['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);
await run('npm', [
  'run',
  'test',
  '--workspace',
  '@ocentra-parent/schema-domain',
  '--',
  'tests/unit/app-install-purchase-store-manual-evidence-proof.test.ts',
]);

const moduleUrl = pathToFileURL(
  resolve(repoRoot, 'packages', 'schema-domain', 'dist', 'app-install-purchase-store-manual-evidence-proof.js')
).href;
const { AppInstallPurchaseStoreManualEvidenceProofReadModel, summarizeAppInstallPurchaseStoreManualEvidence } =
  await import(moduleUrl);

const summary = summarizeAppInstallPurchaseStoreManualEvidence(AppInstallPurchaseStoreManualEvidenceProofReadModel);

assertEqual(summary.storeRows, 5, 'store rows');
assertEqual(summary.manualEvidenceRequiredRows, 2, 'manual evidence required rows');
assertEqual(summary.policyReviewRequiredRows, 2, 'store policy review required rows');
assertEqual(summary.unavailableRows, 1, 'store unavailable rows');
assertEqual(summary.providerExecutedRows, 0, 'provider executed rows');
assertEqual(summary.storeIntegratedRows, 0, 'store integrated rows');

const proof = {
  proof: 'app-install-purchase-store-manual-evidence-proof',
  commit: await gitHead(),
  branch: await gitBranch(),
  generatedAt: new Date().toISOString(),
  summary,
  sourceProofs: {
    platformProofReadiness: 'app-install-purchase-platform-proof-readiness',
  },
  storeManualEvidenceRows: AppInstallPurchaseStoreManualEvidenceProofReadModel.storeManualEvidenceRows,
  nonClaims: AppInstallPurchaseStoreManualEvidenceProofReadModel.nonClaims,
  knownGaps: AppInstallPurchaseStoreManualEvidenceProofReadModel.knownGaps,
  docs: {
    feature: 'docs/features/app-install-purchase-approval.md updated for store manual evidence proof movement',
    expectation:
      'docs/expectations/app-install-purchase-approval.md updated for store manual evidence acceptance coverage',
    checklist:
      'docs/product-capability-checklist.md updated for the app-install store manual evidence proof while keeping provider/store execution gaps explicit',
  },
  evidence: {
    source: 'packages/schema-domain/src/app-install-purchase-store-manual-evidence-proof.ts',
    tests: 'packages/schema-domain/tests/unit/app-install-purchase-store-manual-evidence-proof.test.ts',
    output: 'test-results/app-install-purchase-store-manual-evidence-proof/proof.json',
  },
};

await mkdir(dirname(proofPath), { recursive: true });
await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`app-install-purchase-store-manual-evidence-proof-ok:${proofPath}`);

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
