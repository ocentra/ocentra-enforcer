import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-runtime-writer-execution-delivery-proof');
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
      'tests/unit/app-install-purchase-runtime-writer-execution-delivery-proof.test.ts',
    ])
  );

  const proofModule = await loadRuntimeWriterExecutionDeliveryProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseRuntimeWriterExecutionDeliveryProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseRuntimeWriterExecutionDeliveryProof(parsedReadModel);

  assert.deepEqual(summary, {
    runtimeWriterExecutionDeliveryRows: 4,
    parentOwnedEnvelopeRows: 3,
    deliveryResultReceiptRows: 3,
    manualRequiredRows: 1,
    providerExecutedRows: 0,
    childDeliveredRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.runtimeWriterExecutionDeliveryRows.map(
      (row) =>
        `${row.sourceDecisionAction}:${row.runtimeWriterEnvelopeState}:${row.runtimeWriterExecutionDeliveryState}`
    ),
    [
      'approve:parent-owned-envelope-written:delivery-result-recorded',
      'deny:parent-owned-envelope-written:delivery-result-recorded',
      'time-box:parent-owned-envelope-written:delivery-result-recorded',
      'review-needed:manual-required:manual-required',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-provider-api-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-store-integration'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-platform-interception'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-app-blocking'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-ocentra-hosted-family-data-custody'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-runtime-writer-execution-delivery-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    evidence: {
      runtimeWriterExecutionDeliveryContract:
        'packages/schema-domain/src/app-install-purchase-runtime-writer-execution-delivery-proof.ts',
      sourceRuntimeWriterDeliveryContract:
        'packages/schema-domain/src/app-install-purchase-runtime-writer-delivery-proof.ts',
      sourceParentActionDeliveryReadinessContract:
        'packages/schema-domain/src/app-install-purchase-parent-action-delivery-readiness-proof.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-runtime-writer-execution-delivery-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistRow: 'docs/product-capability-checklist.md row Install/purchase approval',
      packageExport: '@ocentra-parent/schema-domain/app-install-purchase-runtime-writer-execution-delivery-proof',
      packageReadme: 'packages/schema-domain/package.json',
      output: relative(repoRoot, proofPath),
    },
    runtimeWriterExecutionDeliverySummary: summary,
    runtimeWriterExecutionDeliveryRows: parsedReadModel.runtimeWriterExecutionDeliveryRows.map((row) => ({
      sourceDecisionAction: row.sourceDecisionAction,
      sourceRuntimeWriterDeliveryRowId: row.sourceRuntimeWriterDeliveryRowId,
      sourceParentActionDeliveryReadinessRowId: row.sourceParentActionDeliveryReadinessRowId,
      runtimeWriterEnvelopeState: row.runtimeWriterEnvelopeState,
      runtimeWriterEnvelopeRef: row.runtimeWriterEnvelopeRef,
      runtimeWriterExecutionDeliveryState: row.runtimeWriterExecutionDeliveryState,
      deliveryResultReceiptRef: row.deliveryResultReceiptRef,
      deliveryResultAuditEventRefs: row.deliveryResultAuditEventRefs,
      parentActionAuditEventRefs: row.parentActionAuditEventRefs,
      reportRuntimeRefs: row.reportRuntimeRefs,
      runtimeWriterExecutionClaim: row.runtimeWriterExecutionClaim,
      runtimeWriterDeliveryClaim: row.runtimeWriterDeliveryClaim,
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
  console.log(`app-install-purchase-runtime-writer-execution-delivery-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadRuntimeWriterExecutionDeliveryProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-runtime-writer-execution-delivery-proof.js'
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
