import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-child-device-delivery-readiness-proof');
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
      'tests/unit/app-install-purchase-child-device-delivery-readiness-proof.test.ts',
    ])
  );

  const proofModule = await loadProofModule();
  const readModel = proofModule.AppInstallPurchaseChildDeviceDeliveryReadinessProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseChildDeviceDeliveryReadinessProof(readModel);

  assert.deepEqual(summary, {
    childDeviceDeliveryReadinessRows: 5,
    deliveryEvidenceReadyRows: 1,
    manualProofRequiredRows: 1,
    platformUnavailableRows: 1,
    policyBlockedRows: 2,
    childDeviceDeliveredRows: 0,
  });
  assert.equal(readModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(readModel.nonClaims.includes('no-runtime-writer-delivery'), true);
  assert.equal(readModel.nonClaims.includes('no-provider-api-execution'), true);
  assert.equal(readModel.nonClaims.includes('no-platform-adapter-implementation'), true);
  assert.equal(readModel.nonClaims.includes('no-app-blocking'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-child-device-delivery-readiness-proof',
    commands,
    evidence: {
      childDeviceDeliveryReadinessContract:
        'packages/schema-domain/src/app-install-purchase-child-device-delivery-readiness-proof.ts',
      sourceChildDeliveryRuntimeWriterContract:
        'packages/schema-domain/src/app-install-purchase-child-device-delivery-runtime-writer-proof.ts',
      sourcePackageSourceAdapterExecutionContract:
        'packages/schema-domain/src/app-install-purchase-package-source-adapter-execution-proof.ts',
      sourcePlatformLimitationActionContract:
        'packages/schema-domain/src/app-install-purchase-platform-limitation-action-proof.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-child-device-delivery-readiness-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      output: relative(repoRoot, proofPath),
    },
    summary,
    rows: readModel.childDeviceDeliveryReadinessRows.map((row) => ({
      platform: row.platform,
      childDeviceDeliveryReadinessState: row.childDeviceDeliveryReadinessState,
      sourceChildDeliveryRuntimeWriterRowIds: row.sourceChildDeliveryRuntimeWriterRowIds,
      sourcePackageSourceAdapterExecutionRowId: row.sourcePackageSourceAdapterExecutionRowId,
      sourcePlatformLimitationActionRowId: row.sourcePlatformLimitationActionRowId,
      requiredDeliveryProofRefs: row.requiredDeliveryProofRefs,
      parentVisibleStatusRefs: row.parentVisibleStatusRefs,
      childDeviceDeliveryClaim: row.childDeviceDeliveryClaim,
      runtimeWriterExecutionClaim: row.runtimeWriterExecutionClaim,
      runtimeWriterDeliveryClaim: row.runtimeWriterDeliveryClaim,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      appBlockingClaim: row.appBlockingClaim,
      childDataCustody: row.childDataCustody,
      ocentraHostedFamilyDataCustodyClaim: row.ocentraHostedFamilyDataCustodyClaim,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: readModel.nonClaims,
    knownGaps: readModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-child-device-delivery-readiness-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-child-device-delivery-readiness-proof.js'
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
  await commandOutput(command, args);
  commands.push(`${command} ${args.join(' ')}`);
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
