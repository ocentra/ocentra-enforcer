import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-platform-artifact-proof');
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
      'tests/unit/app-install-purchase-platform-artifact-proof.test.ts',
    ])
  );

  const proofModule = await loadPlatformArtifactProofModule();
  const parsedReadModel = proofModule.AppInstallPurchasePlatformArtifactProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchasePlatformArtifactProof(parsedReadModel);

  assert.deepEqual(summary, {
    platformArtifactRows: 5,
    reportRuntimeEvidenceRows: 4,
    attachedPlatformArtifacts: 5,
    unavailableStoreMetadataRows: 1,
  });
  assert.deepEqual(
    parsedReadModel.platformStoreArtifacts.map((row) => `${row.platform}:${row.storeSurface}:${row.artifactKind}`),
    [
      'windows:microsoft-store:platform-store-metadata-artifact',
      'macos:mac-app-store:platform-store-metadata-artifact',
      'linux:linux-package-manager:platform-limitation-report-artifact',
      'android:google-play:platform-store-metadata-artifact',
      'ios:apple-app-store:platform-store-metadata-artifact',
    ]
  );
  assert.deepEqual(
    parsedReadModel.reportRuntimeEvidence.map((row) => `${row.reportSurface}:${row.runtimeReportDeliveryClaim}`),
    [
      'request-audit-history:not-delivered',
      'parent-decision-audit-history:not-delivered',
      'child-facing-state-report:not-delivered',
      'platform-limitation-report:not-delivered',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-provider-api'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('not-generic-app-blocking'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-platform-artifact-proof',
    commands,
    evidence: {
      platformArtifactContract: 'packages/schema-domain/src/app-install-purchase-platform-artifact-proof.ts',
      sourceRuntimeContract: 'packages/schema-domain/src/app-install-purchase-runtime-proof.ts',
      sourceApprovalContract: 'packages/schema-domain/src/app-install-purchase-approval.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-platform-artifact-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklist: 'docs/product-capability-checklist.md',
      output: relative(repoRoot, proofPath),
    },
    platformArtifactSummary: summary,
    platformStoreArtifacts: parsedReadModel.platformStoreArtifacts.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      platformSourceRowId: row.platformSourceRowId,
      packageSourceArtifactRowId: row.packageSourceArtifactRowId,
      artifactRef: row.artifactRef,
      artifactKind: row.artifactKind,
      artifactSourceState: row.artifactSourceState,
      sourceStoreMetadataArtifactState: row.sourceStoreMetadataArtifactState,
      sourcePackageArtifactState: row.sourcePackageArtifactState,
      storeIntegrationClaim: row.storeIntegrationClaim,
      providerApiClaim: row.providerApiClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      childDeliveryClaim: row.childDeliveryClaim,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      appBlockingClaim: row.appBlockingClaim,
      requiredProofRefs: row.requiredProofRefs,
      reportRefs: row.reportRefs,
      claimBoundary: row.claimBoundary,
    })),
    reportRuntimeEvidence: parsedReadModel.reportRuntimeEvidence.map((row) => ({
      reportSurface: row.reportSurface,
      artifactRef: row.artifactRef,
      artifactSourceState: row.artifactSourceState,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      providerApiClaim: row.providerApiClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      auditEventRefs: row.auditEventRefs,
      reportRefs: row.reportRefs,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-platform-artifact-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadPlatformArtifactProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-platform-artifact-proof.js'
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
