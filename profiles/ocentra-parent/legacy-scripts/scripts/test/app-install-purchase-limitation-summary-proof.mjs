import { execFile } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const proofPath = resolve(repoRoot, 'test-results', 'app-install-purchase-limitation-summary-proof', 'proof.json');

await run('npm', ['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);
await run('npm', [
  'run',
  'test',
  '--workspace',
  '@ocentra-parent/schema-domain',
  '--',
  'tests/unit/app-install-purchase-limitation-summary-proof.test.ts',
]);

const moduleUrl = pathToFileURL(
  resolve(repoRoot, 'packages', 'schema-domain', 'dist', 'app-install-purchase-limitation-summary-proof.js')
).href;
const { AppInstallPurchaseLimitationSummaryProofReadModel, summarizeAppInstallPurchaseLimitationSummaryProof } =
  await import(moduleUrl);

const summary = summarizeAppInstallPurchaseLimitationSummaryProof(AppInstallPurchaseLimitationSummaryProofReadModel);

assertEqual(summary.limitationSummaryRows, 3, 'limitation summary rows');
assertEqual(summary.readyRows, 1, 'ready rows');
assertEqual(summary.manualRequiredRows, 1, 'manual-required rows');
assertEqual(summary.unavailableRows, 1, 'unavailable rows');
assertEqual(summary.sourceProviderStoreRows, 5, 'source provider/store rows');
assertEqual(summary.sourceReportStatusRows, 4, 'source report status rows');
assertEqual(summary.providerExecutedRows, 0, 'provider executed rows');
assertEqual(summary.externallyDeliveredRows, 0, 'externally delivered rows');

const proof = {
  proof: 'app-install-purchase-limitation-summary-proof',
  commit: await gitHead(),
  branch: await gitBranch(),
  generatedAt: new Date().toISOString(),
  summary,
  sourceProofs: {
    providerStoreReportStatus: 'app-install-purchase-provider-store-report-status-proof',
    reportStatusReadModelHandoff: 'app-install-purchase-report-status-read-model-handoff-proof',
  },
  nonClaims: AppInstallPurchaseLimitationSummaryProofReadModel.nonClaims,
  knownGaps: AppInstallPurchaseLimitationSummaryProofReadModel.knownGaps,
  docs: {
    feature: 'docs/features/app-install-purchase-approval.md',
    expectation: 'docs/expectations/app-install-purchase-approval.md',
    checklist:
      'docs/product-capability-checklist.md update deferred to primary/shared checklist sequencing for product status row changes',
  },
  evidence: {
    source: 'packages/schema-domain/src/app-install-purchase-limitation-summary-proof.ts',
    tests: 'packages/schema-domain/tests/unit/app-install-purchase-limitation-summary-proof.test.ts',
    output: 'test-results/app-install-purchase-limitation-summary-proof/proof.json',
  },
};

await mkdir(dirname(proofPath), { recursive: true });
await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`app-install-purchase-limitation-summary-proof-ok:${proofPath}`);

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
