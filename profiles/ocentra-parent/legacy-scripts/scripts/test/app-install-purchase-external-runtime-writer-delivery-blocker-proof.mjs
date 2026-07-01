import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-external-runtime-writer-delivery-blocker-proof');
const proofPath = join(outputDir, 'proof.json');
const externalRuntimeHelpersContract = 'packages/schema-domain/src/generated/app-install-purchase-external-runtime-helpers.ts';
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
      'tests/unit/app-install-purchase-external-runtime-writer-delivery-blocker-proof.test.ts',
    ])
  );

  const proofModule = await loadExternalRuntimeWriterDeliveryBlockerProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseExternalRuntimeWriterDeliveryBlockerProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseExternalRuntimeWriterDeliveryBlockerProof(parsedReadModel);

  assert.deepEqual(summary, {
    externalRuntimeWriterDeliveryBlockerRows: 4,
    blockedRuntimePrerequisiteRows: 3,
    manualRequiredRows: 1,
    deliveryAttemptStartedRows: 0,
    externalRuntimeWriterDeliveredRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.externalRuntimeWriterDeliveryBlockerRows.map(
      (row) => `${row.sourceDecisionAction}:${row.deliveryBlockerState}`
    ),
    [
      'approve:blocked-runtime-prerequisites-missing',
      'deny:blocked-runtime-prerequisites-missing',
      'time-box:blocked-runtime-prerequisites-missing',
      'review-needed:manual-required',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-external-runtime-writer-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-external-runtime-writer-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-parent-action-runtime-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-provider-api-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-platform-interception'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-app-blocking'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-ocentra-hosted-family-data-custody'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: parsedReadModel.updatedAt,
    commitMetadataState: 'omitted-for-deterministic-proof-artifact',
    proofMode: 'app-install-purchase-external-runtime-writer-delivery-blocker-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    checklistState: 'deferred-current-e-c-docs-product-capability-checklist-lock',
    pendingChecklistDelta:
      'Add Install/purchase approval row note for external runtime writer delivery blocker proof: records missing external writer transport, platform adapter execution, provider/store execution, and child-device transport before delivery can be claimed.',
    evidence: {
      externalRuntimeWriterDeliveryBlockerContract:
        externalRuntimeHelpersContract,
      sourceExternalRuntimeWriterDeliveryBoundaryContract:
        externalRuntimeHelpersContract,
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-external-runtime-writer-delivery-blocker-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      platformExpectationDoc: 'docs/expectations/platforms.md',
      packageExport:
        '@ocentra-parent/schema-domain/app-install-purchase-external-runtime-writer-delivery-blocker-proof',
      packageReadme: 'packages/schema-domain/package.json',
      checklistRow: 'docs/product-capability-checklist.md row Install/purchase approval deferred by E-C lock',
      output: relative(repoRoot, proofPath),
    },
    externalRuntimeWriterDeliveryBlockerSummary: summary,
    externalRuntimeWriterDeliveryBlockerRows: parsedReadModel.externalRuntimeWriterDeliveryBlockerRows.map((row) => ({
      sourceDecisionAction: row.sourceDecisionAction,
      sourceExternalRuntimeWriterDeliveryBoundaryRowId: row.sourceExternalRuntimeWriterDeliveryBoundaryRowId,
      sourceExternalRuntimeWriterDeliveryBoundaryState: row.sourceExternalRuntimeWriterDeliveryBoundaryState,
      sourceExternalRuntimeWriterQueueRef: row.sourceExternalRuntimeWriterQueueRef,
      requiredExternalWriterTransportProofRefs: row.requiredExternalWriterTransportProofRefs,
      requiredPlatformAdapterProofRefs: row.requiredPlatformAdapterProofRefs,
      requiredProviderStoreProofRefs: row.requiredProviderStoreProofRefs,
      requiredChildDeviceDeliveryProofRefs: row.requiredChildDeviceDeliveryProofRefs,
      deliveryBlockerState: row.deliveryBlockerState,
      deliveryAttemptState: row.deliveryAttemptState,
      requiredRuntimeBlockers: row.requiredRuntimeBlockers,
      manualBlockerRefs: row.manualBlockerRefs,
      deliveryBlockerAuditEventRefs: row.deliveryBlockerAuditEventRefs,
      externalRuntimeWriterExecutionClaim: row.externalRuntimeWriterExecutionClaim,
      externalRuntimeWriterDeliveryClaim: row.externalRuntimeWriterDeliveryClaim,
      parentActionRuntimeDeliveryClaim: row.parentActionRuntimeDeliveryClaim,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      platformInterceptionClaim: row.platformInterceptionClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      childDeviceDeliveryClaim: row.childDeviceDeliveryClaim,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      appBlockingClaim: row.appBlockingClaim,
      childDataCustody: row.childDataCustody,
      ocentraHostedFamilyDataCustodyClaim: row.ocentraHostedFamilyDataCustodyClaim,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(
    `app-install-purchase-external-runtime-writer-delivery-blocker-proof-ok:${relative(repoRoot, proofPath)}`
  );
}

async function loadExternalRuntimeWriterDeliveryBlockerProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-external-runtime-writer-delivery-blocker-proof.js'
  );
  return import(pathToFileURL(modulePath).href);
}

async function runCommand(command, args) {
  const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit' });
  const exitCode = await new Promise((resolve) => {
    child.on('close', resolve);
  });
  commands.push({ command: `${command} ${args.join(' ')}`, exitCode });
  if (exitCode !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${exitCode}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
