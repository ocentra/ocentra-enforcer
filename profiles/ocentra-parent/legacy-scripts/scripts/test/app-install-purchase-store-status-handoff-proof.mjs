import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-store-status-handoff-proof');
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
      'tests/unit/app-install-purchase-store-status-handoff-proof.test.ts',
    ])
  );

  const proofModule = await loadStoreStatusHandoffProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseStoreStatusHandoffProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseStoreStatusHandoffProof(parsedReadModel);
  assert.deepEqual(summary, {
    storeStatusHandoffRows: 5,
    approvedApiRequiredRows: 1,
    entitlementRequiredRows: 2,
    manualRequiredRows: 1,
    unavailableRows: 1,
    parentActionRuntimeLinkedRows: 5,
    deliveredRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.storeStatusHandoffRows.map(
      (row) => `${row.platform}:${row.storeSurface}:${row.storeStatusHandoffState}:${row.storeStatusRuntimeState}`
    ),
    [
      'windows:microsoft-store:approved-api-status-proof-required:not-implemented',
      'macos:mac-app-store:manual-platform-status-review-required:manual-required',
      'linux:linux-package-manager:platform-store-status-unavailable:unavailable',
      'android:google-play:store-entitlement-status-proof-required:not-implemented',
      'ios:apple-app-store:store-entitlement-status-proof-required:not-implemented',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-provider-api-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-parent-action-runtime-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-ocentra-hosted-family-data-custody'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-store-status-handoff-proof',
    commands,
    evidence: {
      storeStatusHandoffContract: 'packages/schema-domain/src/app-install-purchase-store-status-handoff-proof.ts',
      sourcePlatformAdapterBoundaryContract:
        'packages/schema-domain/src/app-install-purchase-platform-adapter-boundary-proof.ts',
      sourceParentActionRuntimeContract:
        'packages/schema-domain/src/app-install-purchase-parent-action-runtime-handoff-proof.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-store-status-handoff-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistRow: 'docs/product-capability-checklist.md row Install/purchase approval',
      output: relative(repoRoot, proofPath),
    },
    storeStatusHandoffSummary: summary,
    storeStatusHandoffRows: parsedReadModel.storeStatusHandoffRows.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      sourcePlatformAdapterBoundaryRowId: row.sourcePlatformAdapterBoundaryRowId,
      sourceParentActionRuntimeHandoffRefs: row.sourceParentActionRuntimeHandoffRefs,
      sourceParentActionRuntimeStatuses: row.sourceParentActionRuntimeStatuses,
      sourceAdapterEvidenceState: row.sourceAdapterEvidenceState,
      sourceAdapterRuntimeState: row.sourceAdapterRuntimeState,
      storeStatusHandoffState: row.storeStatusHandoffState,
      storeStatusRuntimeState: row.storeStatusRuntimeState,
      storeStatusHandoffEvidenceRefs: row.storeStatusHandoffEvidenceRefs,
      sourceReportRuntimeRefs: row.sourceReportRuntimeRefs,
      storeStatusHandoffClaim: row.storeStatusHandoffClaim,
      statusHandoffDeliveryClaim: row.statusHandoffDeliveryClaim,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      parentActionRuntimeDeliveryClaim: row.parentActionRuntimeDeliveryClaim,
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
  console.log(`app-install-purchase-store-status-handoff-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadStoreStatusHandoffProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-store-status-handoff-proof.js'
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
