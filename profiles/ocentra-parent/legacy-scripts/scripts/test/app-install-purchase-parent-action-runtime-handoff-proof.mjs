import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-parent-action-runtime-handoff-proof');
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
      'tests/unit/app-install-purchase-parent-action-runtime-handoff-proof.test.ts',
    ])
  );

  const proofModule = await loadRuntimeHandoffProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseParentActionRuntimeHandoffProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseParentActionRuntimeHandoffProof(parsedReadModel);
  assert.deepEqual(summary, {
    runtimeHandoffRows: 4,
    queuedRuntimeWriterRows: 3,
    manualReviewRequiredRows: 1,
    platformBoundaryLinkedRows: 4,
    runtimeDeliveredRows: 0,
    childDeliveredRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.runtimeHandoffRows.map((row) => `${row.sourceDecisionAction}:${row.runtimeHandoffStatus}`),
    [
      'approve:queued-for-runtime-writer',
      'deny:queued-for-runtime-writer',
      'time-box:queued-for-runtime-writer',
      'review-needed:manual-review-required',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-runtime-action-writer-implementation'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-parent-action-runtime-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-ocentra-hosted-family-data-custody'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-parent-action-runtime-handoff-proof',
    commands,
    evidence: {
      parentActionRuntimeHandoffContract:
        'packages/schema-domain/src/app-install-purchase-parent-action-runtime-handoff-proof.ts',
      sourceParentReviewActionContract: 'packages/schema-domain/src/app-install-purchase-parent-review-action-proof.ts',
      sourcePlatformAdapterBoundaryContract:
        'packages/schema-domain/src/app-install-purchase-platform-adapter-boundary-proof.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-parent-action-runtime-handoff-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistDelta: 'DOC_DELTA: docs/product-capability-checklist.md row Install/purchase approval',
      output: relative(repoRoot, proofPath),
    },
    runtimeHandoffSummary: summary,
    runtimeHandoffRows: parsedReadModel.runtimeHandoffRows.map((row) => ({
      sourceParentReviewActionRowId: row.sourceParentReviewActionRowId,
      sourceDecisionAction: row.sourceDecisionAction,
      sourceRequestKind: row.sourceRequestKind,
      resultingApprovalState: row.resultingApprovalState,
      parentActionReferenceId: row.parentActionReferenceId,
      runtimeHandoffStatus: row.runtimeHandoffStatus,
      runtimeActionWriterClaim: row.runtimeActionWriterClaim,
      parentActionRuntimeDeliveryClaim: row.parentActionRuntimeDeliveryClaim,
      platformAdapterBoundaryRefs: row.platformAdapterBoundaryRefs,
      auditEventRefs: row.auditEventRefs,
      reportRuntimeRefs: row.reportRuntimeRefs,
      portalApprovalUiClaim: row.portalApprovalUiClaim,
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
  console.log(`app-install-purchase-parent-action-runtime-handoff-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadRuntimeHandoffProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-parent-action-runtime-handoff-proof.js'
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
