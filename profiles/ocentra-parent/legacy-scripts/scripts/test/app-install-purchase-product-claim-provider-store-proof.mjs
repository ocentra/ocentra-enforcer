import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-product-claim-provider-store-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/schema-domain',
      '--',
      'tests/unit/app-install-purchase-product-claim-provider-store-proof.test.ts',
    ])
  );

  const proofModule = await loadProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseProductClaimProviderStoreProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseProductClaimProviderStoreProof(parsedReadModel);

  assert.deepEqual(summary, {
    providerStoreProductClaimRows: 5,
    providerStoreProofRequiredRows: 1,
    manualProviderStoreProofRequiredRows: 1,
    unsupportedStoreProofBlockedRows: 3,
    providerExecutedRows: 0,
    productClaimAllowedRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.providerStoreProductClaimRows.map(
      (row) =>
        `${row.platform}:${row.storeSurface}:${row.sourceProductClaimGateState}:${row.sourceProviderStorePreflightState}:${row.providerStoreProductClaimState}`
    ),
    [
      'windows:microsoft-store:product-claim-denied:preflight-ready:provider-store-proof-required',
      'macos:mac-app-store:manual-required:manual-provider-proof-required:manual-provider-store-proof-required',
      'linux:linux-package-manager:blocked:provider-unavailable:unsupported-store-proof-blocked',
      'android:google-play:blocked:manual-provider-proof-required:unsupported-store-proof-blocked',
      'ios:apple-app-store:blocked:manual-provider-proof-required:unsupported-store-proof-blocked',
    ]
  );

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-product-claim-provider-store-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    checklistState: 'updated-docs-product-capability-checklist-app-install-row',
    evidence: {
      providerStoreProductClaimContract:
        'packages/schema-domain/src/app-install-purchase-product-claim-provider-store-proof.ts',
      sourceProductClaimGateContract: 'packages/schema-domain/src/app-install-purchase-product-claim-gate-proof.ts',
      sourceProviderStorePreflightContract:
        'packages/schema-domain/src/app-install-purchase-provider-store-execution-preflight-proof.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-product-claim-provider-store-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistDoc: 'docs/product-capability-checklist.md',
      packageExport: '@ocentra-parent/schema-domain/app-install-purchase-product-claim-provider-store-proof',
      output: relative(repoRoot, proofPath),
    },
    providerStoreProductClaimSummary: summary,
    providerStoreProductClaimRows: parsedReadModel.providerStoreProductClaimRows,
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-product-claim-provider-store-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-product-claim-provider-store-proof.js'
  );
  return import(pathToFileURL(modulePath).href);
}

async function gitHead() {
  const output = await commandOutput('git', ['rev-parse', 'HEAD']);
  return output.trim();
}

async function commandOutput(command, args) {
  const chunks = [];
  const child = spawn(command, args, { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
  child.stdout.on('data', (chunk) => chunks.push(chunk));
  child.stderr.on('data', (chunk) => chunks.push(chunk));
  const exitCode = await new Promise((resolve) => {
    child.on('close', resolve);
  });
  const output = Buffer.concat(chunks).toString('utf8');
  if (exitCode !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${exitCode}\n${output}`);
  }
  return output;
}

async function runCommand(command, args) {
  const startedAt = new Date().toISOString();
  const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit' });
  const exitCode = await new Promise((resolve) => {
    child.on('close', resolve);
  });
  commands.push({ command: `${command} ${args.join(' ')}`, startedAt, exitCode });
  if (exitCode !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${exitCode}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
