import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-package-source-capture-status-proof');
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
      'tests/unit/app-install-purchase-package-source-capture-status-proof.test.ts',
    ])
  );

  const proofModule = await loadPackageSourceCaptureStatusProofModule();
  const parsedReadModel = proofModule.AppInstallPurchasePackageSourceCaptureStatusProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchasePackageSourceCaptureStatusProof(parsedReadModel);

  assert.deepEqual(summary, {
    packageSourceCaptureRows: 5,
    capturedRows: 1,
    blockedRows: 2,
    manualRequiredRows: 1,
    unavailableRows: 1,
    artifactLinkedRows: 5,
    auditLinkedRows: 5,
    reportLinkedRows: 5,
    deliveredRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.packageSourceCaptureRows.map(
      (row) =>
        `${row.platform}:${row.storeSurface}:${row.captureRequestState}:${row.packageSourceCaptureStatus}:${row.platformLimitationState}`
    ),
    [
      'windows:microsoft-store:accepted-for-local-package-source-proof:captured:local-package-source-readable',
      'macos:mac-app-store:manual-host-proof-required:manual-required:requires-manual-host-proof',
      'linux:linux-package-manager:platform-unavailable:unavailable:platform-unavailable',
      'android:google-play:blocked-by-device-management-policy:blocked:requires-device-owner-or-managed-profile',
      'ios:apple-app-store:blocked-by-apple-entitlement:blocked:requires-apple-entitlement',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-provider-api-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-portal-approval-ui'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-ocentra-hosted-family-data-custody'), true);
  const productCapabilityChecklist = await readFile(join(repoRoot, 'docs', 'product-capability-checklist.md'), 'utf8');
  assert.match(
    productCapabilityChecklist,
    /Install\/purchase approval.*package-source capture\/status.*child-device package-source capture requests/s
  );

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-package-source-capture-status-proof',
    commands,
    evidence: {
      packageSourceCaptureStatusContract:
        'packages/schema-domain/src/app-install-purchase-package-source-capture-status-proof.ts',
      sourceChildArtifactDeliveryContract:
        'packages/schema-domain/src/app-install-purchase-child-artifact-delivery-proof.ts',
      sourceStoreStatusHandoffContract: 'packages/schema-domain/src/app-install-purchase-store-status-handoff-proof.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-package-source-capture-status-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      platformExpectationDoc: 'docs/expectations/platforms.md',
      checklistRow:
        'COMPLETED: docs/product-capability-checklist.md Install/purchase approval row includes package-source capture/status proof with captured/blocked/manual-required/unavailable rows.',
      packageExport:
        'packages/schema-domain/package.json no longer publishes this proof as a public subpath export; the script imports the built dist module directly.',
      output: relative(repoRoot, proofPath),
    },
    packageSourceCaptureStatusSummary: summary,
    packageSourceCaptureRows: parsedReadModel.packageSourceCaptureRows.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      sourceChildPackageArtifactRowId: row.sourceChildPackageArtifactRowId,
      sourceChildPackageArtifactRef: row.sourceChildPackageArtifactRef,
      sourceStoreStatusHandoffRowId: row.sourceStoreStatusHandoffRowId,
      sourceStoreStatusHandoffState: row.sourceStoreStatusHandoffState,
      packageSourceArtifactState: row.packageSourceArtifactState,
      childArtifactSourceState: row.childArtifactSourceState,
      captureRequestState: row.captureRequestState,
      packageSourceCaptureStatus: row.packageSourceCaptureStatus,
      platformLimitationState: row.platformLimitationState,
      packageSourceCaptureArtifactRefs: row.packageSourceCaptureArtifactRefs,
      sourceStoreStatusEvidenceRefs: row.sourceStoreStatusEvidenceRefs,
      auditEventRefs: row.auditEventRefs,
      reportRefs: row.reportRefs,
      requiredProofRefs: row.requiredProofRefs,
      packageSourceCaptureClaim: row.packageSourceCaptureClaim,
      packageSourceCaptureExecutionClaim: row.packageSourceCaptureExecutionClaim,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      portalApprovalUiClaim: row.portalApprovalUiClaim,
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
  console.log(`app-install-purchase-package-source-capture-status-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadPackageSourceCaptureStatusProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-package-source-capture-status-proof.js'
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
