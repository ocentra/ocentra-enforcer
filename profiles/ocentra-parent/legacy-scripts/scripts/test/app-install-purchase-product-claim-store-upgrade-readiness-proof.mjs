import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-product-claim-store-upgrade-readiness-proof');
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
      'tests/unit/app-install-purchase-product-claim-store-upgrade-readiness-proof.test.ts',
    ])
  );

  const proofModule = await loadProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseProductClaimStoreUpgradeReadinessProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseProductClaimStoreUpgradeReadinessProof(parsedReadModel);

  assert.deepEqual(summary, {
    storeUpgradeReadinessRows: 5,
    productClaimStoreUpgradeBlockedRows: 1,
    manualStoreUpgradeRequiredRows: 1,
    unsupportedStoreUpgradeBlockedRows: 3,
    providerExecutedRows: 0,
    portalUiClaimedRows: 0,
    productClaimApprovedRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.storeUpgradeReadinessRows.map(
      (row) =>
        `${row.platform}:${row.storeSurface}:${row.sourceProductClaimGateState}:${row.sourcePortalTestReadinessState}:${row.sourceProviderStoreProductClaimState}:${row.storeUpgradeReadinessState}`
    ),
    [
      'windows:microsoft-store:product-claim-denied:portal-test-ready:provider-store-proof-required:product-claim-store-upgrade-blocked',
      'macos:mac-app-store:manual-required:manual-portal-test-required:manual-provider-store-proof-required:manual-store-upgrade-required',
      'linux:linux-package-manager:blocked:unsupported-portal-test-blocked:unsupported-store-proof-blocked:unsupported-store-upgrade-blocked',
      'android:google-play:blocked:unsupported-portal-test-blocked:unsupported-store-proof-blocked:unsupported-store-upgrade-blocked',
      'ios:apple-app-store:blocked:unsupported-portal-test-blocked:unsupported-store-proof-blocked:unsupported-store-upgrade-blocked',
    ]
  );

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-product-claim-store-upgrade-readiness-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    checklistState: 'updated-docs-product-capability-checklist-app-install-row',
    evidence: {
      storeUpgradeReadinessContract:
        'packages/schema-domain/src/app-install-purchase-product-claim-store-upgrade-readiness-proof.ts',
      sourceProductClaimGateContract: 'packages/schema-domain/src/app-install-purchase-product-claim-gate-proof.ts',
      sourcePortalTestReadinessContract:
        'packages/schema-domain/src/app-install-purchase-product-claim-portal-test-readiness-proof.ts',
      sourceProviderStoreContract:
        'packages/schema-domain/src/app-install-purchase-product-claim-provider-store-proof.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-product-claim-store-upgrade-readiness-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistDoc: 'docs/product-capability-checklist.md',
      packageExport: '@ocentra-parent/schema-domain/app-install-purchase-product-claim-store-upgrade-readiness-proof',
      output: relative(repoRoot, proofPath),
    },
    storeUpgradeReadinessSummary: summary,
    storeUpgradeReadinessRows: parsedReadModel.storeUpgradeReadinessRows,
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-product-claim-store-upgrade-readiness-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-product-claim-store-upgrade-readiness-proof.js'
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
