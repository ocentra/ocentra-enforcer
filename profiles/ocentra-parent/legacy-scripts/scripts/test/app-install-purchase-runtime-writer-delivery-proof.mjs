import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-runtime-writer-delivery-proof');
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
      'tests/unit/app-install-purchase-runtime-writer-delivery-proof.test.ts',
    ])
  );

  const proofModule = await loadRuntimeWriterDeliveryProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseRuntimeWriterDeliveryProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseRuntimeWriterDeliveryProof(parsedReadModel);
  assert.deepEqual(summary, {
    runtimeWriterDeliveryRows: 4,
    writerEnvelopeReadyRows: 3,
    manualReviewRequiredRows: 1,
    storeStatusLinkedRows: 4,
    writerImplementedRows: 0,
    runtimeDeliveredRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.runtimeWriterDeliveryRows.map(
      (row) => `${row.sourceDecisionAction}:${row.sourceRuntimeHandoffStatus}:${row.runtimeWriterDeliveryState}`
    ),
    [
      'approve:queued-for-runtime-writer:writer-envelope-ready',
      'deny:queued-for-runtime-writer:writer-envelope-ready',
      'time-box:queued-for-runtime-writer:writer-envelope-ready',
      'review-needed:manual-review-required:manual-review-required',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-runtime-writer-implementation'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-runtime-writer-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-parent-action-runtime-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-provider-api-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-ocentra-hosted-family-data-custody'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-runtime-writer-delivery-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    evidence: {
      runtimeWriterDeliveryContract: 'packages/schema-domain/src/app-install-purchase-runtime-writer-delivery-proof.ts',
      sourceParentActionRuntimeHandoffContract:
        'packages/schema-domain/src/app-install-purchase-parent-action-runtime-handoff-proof.ts',
      sourceStoreStatusHandoffContract: 'packages/schema-domain/src/app-install-purchase-store-status-handoff-proof.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-runtime-writer-delivery-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistRow: 'docs/product-capability-checklist.md row Install/purchase approval',
      packageReadme: 'packages/schema-domain/package.json',
      output: relative(repoRoot, proofPath),
    },
    runtimeWriterDeliverySummary: summary,
    runtimeWriterDeliveryRows: parsedReadModel.runtimeWriterDeliveryRows.map((row) => ({
      sourceDecisionAction: row.sourceDecisionAction,
      sourceRuntimeHandoffStatus: row.sourceRuntimeHandoffStatus,
      runtimeWriterDeliveryState: row.runtimeWriterDeliveryState,
      runtimeWriterQueueState: row.runtimeWriterQueueState,
      sourceStoreStatusHandoffRefs: row.sourceStoreStatusHandoffRefs,
      sourceStoreStatusHandoffStates: row.sourceStoreStatusHandoffStates,
      storeStatusHandoffEvidenceRefs: row.storeStatusHandoffEvidenceRefs,
      auditEventRefs: row.auditEventRefs,
      reportRuntimeRefs: row.reportRuntimeRefs,
      runtimeWriterImplementationClaim: row.runtimeWriterImplementationClaim,
      runtimeWriterDeliveryClaim: row.runtimeWriterDeliveryClaim,
      parentActionRuntimeDeliveryClaim: row.parentActionRuntimeDeliveryClaim,
      storeStatusHandoffDeliveryClaim: row.storeStatusHandoffDeliveryClaim,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      childDeliveryClaim: row.childDeliveryClaim,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      interceptionClaim: row.interceptionClaim,
      appBlockingClaim: row.appBlockingClaim,
      childDataCustody: row.childDataCustody,
      ocentraHostedFamilyDataCustodyClaim: row.ocentraHostedFamilyDataCustodyClaim,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-runtime-writer-delivery-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadRuntimeWriterDeliveryProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-runtime-writer-delivery-proof.js'
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
