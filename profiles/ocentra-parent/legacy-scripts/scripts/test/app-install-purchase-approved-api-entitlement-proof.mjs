import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-approved-api-entitlement-proof');
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
      'tests/unit/app-install-purchase-approved-api-entitlement-proof.test.ts',
    ])
  );

  const proofModule = await loadApprovedApiEntitlementProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseApprovedApiEntitlementProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseApprovedApiEntitlementProof(parsedReadModel);

  assert.deepEqual(summary, {
    evidenceRows: 5,
    approvedApiRequiredRows: 1,
    entitlementRequiredRows: 2,
    manualReviewRows: 1,
    unavailableRows: 1,
  });
  assert.deepEqual(
    parsedReadModel.evidenceRows.map(
      (row) =>
        `${row.platform}:${row.storeSurface}:${row.evidenceStatus}:${row.providerApiExecutionClaim}:${row.childDeliveryClaim}`
    ),
    [
      'windows:microsoft-store:approved-api-evidence-required:not-executed:not-delivered',
      'macos:mac-app-store:manual-platform-review-required:not-executed:not-delivered',
      'linux:linux-package-manager:platform-unavailable:not-executed:not-delivered',
      'android:google-play:store-entitlement-evidence-required:not-executed:not-delivered',
      'ios:apple-app-store:store-entitlement-evidence-required:not-executed:not-delivered',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-provider-api-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-runtime-report-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-activity-data'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-approved-api-entitlement-proof',
    commands,
    evidence: {
      approvedApiEntitlementContract:
        'packages/schema-domain/src/app-install-purchase-approved-api-entitlement-proof.ts',
      sourceChildArtifactContract: 'packages/schema-domain/src/app-install-purchase-child-artifact-delivery-proof.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-approved-api-entitlement-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistDelta: 'DOC_DELTA: docs/product-capability-checklist.md row Install/purchase approval',
      output: relative(repoRoot, proofPath),
    },
    approvedApiEntitlementSummary: summary,
    evidenceRows: parsedReadModel.evidenceRows.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      sourceChildArtifactRowId: row.sourceChildArtifactRowId,
      evidenceStatus: row.evidenceStatus,
      evidenceSource: row.evidenceSource,
      approvedApiEvidenceRef: row.approvedApiEvidenceRef,
      entitlementEvidenceRef: row.entitlementEvidenceRef,
      limitationReportRef: row.limitationReportRef,
      auditEventRefs: row.auditEventRefs,
      requiredProofRefs: row.requiredProofRefs,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      childDeliveryClaim: row.childDeliveryClaim,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      interceptionClaim: row.interceptionClaim,
      appBlockingClaim: row.appBlockingClaim,
      childDataCustody: row.childDataCustody,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-approved-api-entitlement-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadApprovedApiEntitlementProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-approved-api-entitlement-proof.js'
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
