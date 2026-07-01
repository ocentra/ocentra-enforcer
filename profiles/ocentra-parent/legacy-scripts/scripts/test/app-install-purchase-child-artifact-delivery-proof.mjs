import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-child-artifact-delivery-proof');
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
      'tests/unit/app-install-purchase-child-artifact-delivery-proof.test.ts',
    ])
  );

  const proofModule = await loadChildArtifactProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseChildArtifactDeliveryProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseChildArtifactDeliveryProof(parsedReadModel);

  assert.deepEqual(summary, {
    childArtifactRows: 5,
    childDeliveryRows: 5,
    attachedChildArtifactRefs: 4,
    unavailableChildArtifactRows: 1,
    notDeliveredRows: 5,
  });
  assert.deepEqual(
    parsedReadModel.childPackageArtifacts.map(
      (row) => `${row.platform}:${row.storeSurface}:${row.childArtifactSourceState}:${row.childDeliveryClaim}`
    ),
    [
      'windows:microsoft-store:child-package-artifact-ref-attached:not-delivered',
      'macos:mac-app-store:child-package-artifact-ref-attached:not-delivered',
      'linux:linux-package-manager:platform-unavailable:not-delivered',
      'android:google-play:child-package-artifact-ref-attached:not-delivered',
      'ios:apple-app-store:child-package-artifact-ref-attached:not-delivered',
    ]
  );
  assert.deepEqual(
    parsedReadModel.childDeliveryBoundaries.map((row) => `${row.childVisibleStatus}:${row.childDeliveryClaim}`),
    [
      'pending-parent-review-visible:not-delivered',
      'approved-visible:not-delivered',
      'denied-visible:not-delivered',
      'time-box-visible:not-delivered',
      'review-needed-visible:not-delivered',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-runtime-capture'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-activity-data'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-child-artifact-delivery-proof',
    commands,
    evidence: {
      childArtifactDeliveryContract: 'packages/schema-domain/src/app-install-purchase-child-artifact-delivery-proof.ts',
      sourcePlatformArtifactContract: 'packages/schema-domain/src/app-install-purchase-platform-artifact-proof.ts',
      sourceRuntimeContract: 'packages/schema-domain/src/app-install-purchase-runtime-proof.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-child-artifact-delivery-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistDelta: 'DOC_DELTA: docs/product-capability-checklist.md row Install/purchase approval',
      parentDomainReadmeDelta: 'DOC_DELTA: packages/schema-domain/package.json app install/purchase bullet and gap',
      output: relative(repoRoot, proofPath),
    },
    childArtifactSummary: summary,
    childPackageArtifacts: parsedReadModel.childPackageArtifacts.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      platformArtifactRowId: row.platformArtifactRowId,
      packageSourceArtifactRowId: row.packageSourceArtifactRowId,
      platformArtifactRef: row.platformArtifactRef,
      childPackageArtifactRef: row.childPackageArtifactRef,
      packageSourceArtifactState: row.packageSourceArtifactState,
      childArtifactSourceState: row.childArtifactSourceState,
      childArtifactCaptureClaim: row.childArtifactCaptureClaim,
      deliveryState: row.deliveryState,
      childDeliveryClaim: row.childDeliveryClaim,
      providerApiClaim: row.providerApiClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      interceptionClaim: row.interceptionClaim,
      childDataCustody: row.childDataCustody,
      reportRefs: row.reportRefs,
      requiredProofRefs: row.requiredProofRefs,
      claimBoundary: row.claimBoundary,
    })),
    childDeliveryBoundaries: parsedReadModel.childDeliveryBoundaries.map((row) => ({
      sourceChildStateId: row.sourceChildStateId,
      requestId: row.requestId,
      platform: row.platform,
      childVisibleStatus: row.childVisibleStatus,
      deliveryState: row.deliveryState,
      childArtifactRef: row.childArtifactRef,
      childDeliveryClaim: row.childDeliveryClaim,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      providerApiClaim: row.providerApiClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      appBlockingClaim: row.appBlockingClaim,
      auditEventRefs: row.auditEventRefs,
      reportRefs: row.reportRefs,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-child-artifact-delivery-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadChildArtifactProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-child-artifact-delivery-proof.js'
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
