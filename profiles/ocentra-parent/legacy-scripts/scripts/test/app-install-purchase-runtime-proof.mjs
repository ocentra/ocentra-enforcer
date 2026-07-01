import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-runtime-proof');
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
      'tests/unit/app-install-purchase-runtime-proof.test.ts',
    ])
  );

  const proofModule = await loadRuntimeProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseRuntimeProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseRuntimeProof(parsedReadModel);

  assert.deepEqual(summary, {
    platformRows: 5,
    childDeliveryRows: 5,
    reportIntegrationRows: 4,
    statusReadinessRows: 5,
    boundaryOnlyRows: 5,
    unavailablePlatformRows: 1,
    statusReadinessOnlyRows: 5,
    statusReaderImplementedRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.platformRuntimeArtifacts.map(
      (row) => `${row.platform}:${row.storeSurface}:${row.runtimeClaimState}`
    ),
    [
      'windows:microsoft-store:boundary-only',
      'macos:mac-app-store:boundary-only',
      'linux:linux-package-manager:boundary-only',
      'android:google-play:boundary-only',
      'ios:apple-app-store:boundary-only',
    ]
  );
  assert.deepEqual(
    parsedReadModel.childDeliveryBoundaries.map((row) => row.runtimeDeliveryClaim),
    ['not-delivered', 'not-delivered', 'not-delivered', 'not-delivered', 'not-delivered']
  );
  assert.deepEqual(
    parsedReadModel.reportIntegrationBoundaries.map((row) => row.runtimeReportClaim),
    ['not-delivered', 'not-delivered', 'not-delivered', 'not-delivered']
  );
  assert.deepEqual(
    parsedReadModel.statusReadinessBoundaries.map(
      (row) => `${row.childVisibleStatus}:${row.statusReadinessClaim}:${row.runtimeStatusReaderClaim}`
    ),
    [
      'pending-parent-review-visible:runtime-status-readiness-only:not-implemented',
      'approved-visible:runtime-status-readiness-only:not-implemented',
      'denied-visible:runtime-status-readiness-only:not-implemented',
      'time-box-visible:runtime-status-readiness-only:not-implemented',
      'review-needed-visible:runtime-status-readiness-only:not-implemented',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-runtime-report-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-runtime-status-reader-implementation'), true);
  assert.equal(parsedReadModel.nonClaims.includes('not-generic-app-blocking'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-runtime-proof',
    commands,
    evidence: {
      runtimeProofContract: 'packages/schema-domain/src/app-install-purchase-runtime-proof.ts',
      runtimeProofRules: 'packages/schema-domain/src/app-install-purchase-runtime-proof-rules.ts',
      sourceContract: 'packages/schema-domain/src/app-install-purchase-approval.ts',
      sourceProofReadModel: 'packages/schema-domain/src/app-install-purchase-approval-proof.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-runtime-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      output: relative(repoRoot, proofPath),
    },
    runtimeSummary: summary,
    platformRuntimeArtifacts: parsedReadModel.platformRuntimeArtifacts.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      platformSourceRowId: row.platformSourceRowId,
      packageSourceArtifactRowId: row.packageSourceArtifactRowId,
      storeMetadataArtifactState: row.storeMetadataArtifactState,
      packageSourceArtifactState: row.packageSourceArtifactState,
      childPendingDeliveryState: row.childPendingDeliveryState,
      childResultDeliveryState: row.childResultDeliveryState,
      reportIntegrationState: row.reportIntegrationState,
      runtimeClaimState: row.runtimeClaimState,
      requiredProofRefs: row.requiredProofRefs,
      reportRefs: row.reportRefs,
      claimBoundary: row.claimBoundary,
    })),
    childDeliveryBoundaries: parsedReadModel.childDeliveryBoundaries.map((row) => ({
      childVisibleStatus: row.childVisibleStatus,
      deliveryState: row.deliveryState,
      runtimeDeliveryClaim: row.runtimeDeliveryClaim,
      auditEventRefs: row.auditEventRefs,
      reportRefs: row.reportRefs,
      claimBoundary: row.claimBoundary,
    })),
    statusReadinessBoundaries: parsedReadModel.statusReadinessBoundaries.map((row) => ({
      childVisibleStatus: row.childVisibleStatus,
      sourceChildStateId: row.sourceChildStateId,
      sourceRequestId: row.sourceRequestId,
      requestKind: row.requestKind,
      platform: row.platform,
      sourceApprovalState: row.sourceApprovalState,
      sourceDeliveryState: row.sourceDeliveryState,
      sourceRuntimeDeliveryClaim: row.sourceRuntimeDeliveryClaim,
      statusReadinessClaim: row.statusReadinessClaim,
      runtimeStatusReaderClaim: row.runtimeStatusReaderClaim,
      childDeliveryClaim: row.childDeliveryClaim,
      reportRuntimeDeliveryClaim: row.reportRuntimeDeliveryClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      appBlockingClaim: row.appBlockingClaim,
      auditEventRefs: row.auditEventRefs,
      reportRefs: row.reportRefs,
      claimBoundary: row.claimBoundary,
    })),
    reportIntegrationBoundaries: parsedReadModel.reportIntegrationBoundaries.map((row) => ({
      surface: row.surface,
      integrationState: row.integrationState,
      runtimeReportClaim: row.runtimeReportClaim,
      auditEventRefs: row.auditEventRefs,
      reportRefs: row.reportRefs,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-runtime-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadRuntimeProofModule() {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'app-install-purchase-runtime-proof.js');
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
