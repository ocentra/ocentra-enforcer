import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(
  repoRoot,
  'test-results',
  'app-install-purchase-external-runtime-transport-dispatch-preflight-proof'
);
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
      'tests/unit/app-install-purchase-external-runtime-transport-dispatch-preflight-proof.test.ts',
    ])
  );

  const proofModule = await loadExternalRuntimeTransportDispatchPreflightProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseExternalRuntimeTransportDispatchPreflightProofReadModel;
  const summary =
    proofModule.summarizeAppInstallPurchaseExternalRuntimeTransportDispatchPreflightProof(parsedReadModel);

  assert.deepEqual(summary, {
    externalRuntimeTransportDispatchPreflightRows: 4,
    blockedPreflightRows: 3,
    manualRequiredRows: 1,
    withheldDispatchPackets: 3,
    readyDispatchRows: 0,
    externalRuntimeWriterDeliveredRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.externalRuntimeTransportDispatchPreflightRows.map(
      (row) => `${row.sourceDecisionAction}:${row.dispatchPreflightState}:${row.dispatchPacketState}`
    ),
    [
      'approve:blocked-waiting-runtime-artifacts:withheld',
      'deny:blocked-waiting-runtime-artifacts:withheld',
      'time-box:blocked-waiting-runtime-artifacts:withheld',
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
    proofMode: 'app-install-purchase-external-runtime-transport-dispatch-preflight-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    checklistState: 'deferred-current-docs-product-capability-checklist-lock',
    pendingChecklistDelta:
      'Add Install/purchase approval row note for external runtime transport dispatch preflight proof: records parent-owned withheld dispatch packets that keep delivery blocked until external writer transport handler, provider/store execution handler, platform adapter execution handler, and child-device transport receipt proof refs are real.',
    evidence: {
      externalRuntimeTransportDispatchPreflightContract:
        externalRuntimeHelpersContract,
      sourceExternalRuntimeTransportQueueContract:
        externalRuntimeHelpersContract,
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-external-runtime-transport-dispatch-preflight-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      platformExpectationDoc: 'docs/expectations/platforms.md',
      packageExport:
        '@ocentra-parent/schema-domain/app-install-purchase-external-runtime-transport-dispatch-preflight-proof',
      packageReadme: 'packages/schema-domain/package.json',
      checklistRow: 'docs/product-capability-checklist.md row Install/purchase approval deferred by current lock',
      output: relative(repoRoot, proofPath),
    },
    externalRuntimeTransportDispatchPreflightSummary: summary,
    externalRuntimeTransportDispatchPreflightRows: parsedReadModel.externalRuntimeTransportDispatchPreflightRows.map(
      (row) => ({
        sourceDecisionAction: row.sourceDecisionAction,
        sourceExternalRuntimeTransportQueueRowId: row.sourceExternalRuntimeTransportQueueRowId,
        sourceTransportQueueState: row.sourceTransportQueueState,
        sourceTransportDispatchState: row.sourceTransportDispatchState,
        parentOwnedTransportQueueRef: row.parentOwnedTransportQueueRef,
        parentOwnedDispatchPreflightRef: row.parentOwnedDispatchPreflightRef,
        parentOwnedDispatchPacketRef: row.parentOwnedDispatchPacketRef,
        dispatchPreflightState: row.dispatchPreflightState,
        dispatchPacketState: row.dispatchPacketState,
        dispatchReadinessState: row.dispatchReadinessState,
        requiredDispatchArtifactBlockers: row.requiredDispatchArtifactBlockers,
        externalWriterTransportHandlerProofRefs: row.externalWriterTransportHandlerProofRefs,
        providerStoreExecutionHandlerProofRefs: row.providerStoreExecutionHandlerProofRefs,
        platformAdapterExecutionHandlerProofRefs: row.platformAdapterExecutionHandlerProofRefs,
        childDeviceTransportReceiptProofRefs: row.childDeviceTransportReceiptProofRefs,
        dispatchBlockedReasonRefs: row.dispatchBlockedReasonRefs,
        dispatchPreflightAuditEventRefs: row.dispatchPreflightAuditEventRefs,
        externalRuntimeWriterExecutionClaim: row.externalRuntimeWriterExecutionClaim,
        externalRuntimeWriterDeliveryClaim: row.externalRuntimeWriterDeliveryClaim,
        providerApiExecutionClaim: row.providerApiExecutionClaim,
        platformAdapterClaim: row.platformAdapterClaim,
        childDeviceDeliveryClaim: row.childDeviceDeliveryClaim,
        appBlockingClaim: row.appBlockingClaim,
        claimBoundary: row.claimBoundary,
      })
    ),
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(
    `app-install-purchase-external-runtime-transport-dispatch-preflight-proof-ok:${relative(repoRoot, proofPath)}`
  );
}

async function loadExternalRuntimeTransportDispatchPreflightProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-external-runtime-transport-dispatch-preflight-proof.js'
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
