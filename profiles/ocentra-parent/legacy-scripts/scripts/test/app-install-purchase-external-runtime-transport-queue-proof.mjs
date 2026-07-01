import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-external-runtime-transport-queue-proof');
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
      'tests/unit/app-install-purchase-external-runtime-transport-queue-proof.test.ts',
    ])
  );

  const proofModule = await loadExternalRuntimeTransportQueueProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseExternalRuntimeTransportQueueProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseExternalRuntimeTransportQueueProof(parsedReadModel);

  assert.deepEqual(summary, {
    externalRuntimeTransportQueueRows: 4,
    queuedBlockedRows: 3,
    manualRequiredRows: 1,
    dispatchBlockedRows: 3,
    retryScheduledRows: 1,
    externalRuntimeWriterDeliveredRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.externalRuntimeTransportQueueRows.map(
      (row) =>
        `${row.sourceDecisionAction}:${row.externalRuntimeTransportQueueState}:${row.externalRuntimeTransportDispatchState}`
    ),
    [
      'approve:queued-blocked:dispatch-blocked',
      'deny:queued-blocked:dispatch-blocked',
      'time-box:queued-blocked:dispatch-blocked',
      'review-needed:manual-required:manual-required',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-external-runtime-writer-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-external-runtime-writer-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-provider-api-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-platform-adapter-implementation'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-app-blocking'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: parsedReadModel.updatedAt,
    commitMetadataState: 'omitted-for-deterministic-proof-artifact',
    proofMode: 'app-install-purchase-external-runtime-transport-queue-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    checklistState: 'deferred-current-e-c-docs-product-capability-checklist-lock',
    pendingChecklistDelta:
      'Add Install/purchase approval row note for external runtime transport queue proof: records parent-owned queue/dispatch guard rows that keep delivery blocked until external writer transport, provider/store execution, platform adapter execution, and child-device transport proof refs are real.',
    evidence: {
      externalRuntimeTransportQueueContract:
        externalRuntimeHelpersContract,
      sourceExternalRuntimeWriterDeliveryBlockerContract:
        externalRuntimeHelpersContract,
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-external-runtime-transport-queue-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      platformExpectationDoc: 'docs/expectations/platforms.md',
      packageExport: '@ocentra-parent/schema-domain/app-install-purchase-external-runtime-transport-queue-proof',
      packageReadme: 'packages/schema-domain/package.json',
      checklistRow: 'docs/product-capability-checklist.md row Install/purchase approval deferred by E-C lock',
      output: relative(repoRoot, proofPath),
    },
    externalRuntimeTransportQueueSummary: summary,
    externalRuntimeTransportQueueRows: parsedReadModel.externalRuntimeTransportQueueRows.map((row) => ({
      sourceDecisionAction: row.sourceDecisionAction,
      sourceExternalRuntimeWriterDeliveryBlockerRowId: row.sourceExternalRuntimeWriterDeliveryBlockerRowId,
      sourceDeliveryBlockerState: row.sourceDeliveryBlockerState,
      sourceDeliveryAttemptState: row.sourceDeliveryAttemptState,
      parentOwnedTransportQueueRef: row.parentOwnedTransportQueueRef,
      externalRuntimeTransportQueueState: row.externalRuntimeTransportQueueState,
      externalRuntimeTransportDispatchState: row.externalRuntimeTransportDispatchState,
      externalRuntimeTransportRetryState: row.externalRuntimeTransportRetryState,
      requiredRuntimeBlockers: row.requiredRuntimeBlockers,
      requiredExternalWriterTransportProofRefs: row.requiredExternalWriterTransportProofRefs,
      requiredChildDeviceTransportProofRefs: row.requiredChildDeviceTransportProofRefs,
      requiredProviderStoreProofRefs: row.requiredProviderStoreProofRefs,
      requiredPlatformAdapterProofRefs: row.requiredPlatformAdapterProofRefs,
      blockedDispatchReasonRefs: row.blockedDispatchReasonRefs,
      queueGuardAuditEventRefs: row.queueGuardAuditEventRefs,
      externalRuntimeWriterExecutionClaim: row.externalRuntimeWriterExecutionClaim,
      externalRuntimeWriterDeliveryClaim: row.externalRuntimeWriterDeliveryClaim,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      childDeviceDeliveryClaim: row.childDeviceDeliveryClaim,
      appBlockingClaim: row.appBlockingClaim,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-external-runtime-transport-queue-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadExternalRuntimeTransportQueueProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-external-runtime-transport-queue-proof.js'
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
