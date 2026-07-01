import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-approval-contract-proof');
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
      'tests/unit/app-install-purchase-approval-proof.test.ts',
    ])
  );

  const proofModule = await loadContractProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseApprovalContractProofReadModel;
  const supportStateCounts = proofModule.summarizeAppInstallPurchaseApprovalSupportStates(
    parsedReadModel.platformSupportMatrix
  );
  const platformSourceMetadata = parsedReadModel.platformSourceMetadata;
  const packageSourceArtifacts = parsedReadModel.packageSourceArtifacts;
  assert.equal(supportStateCounts.supported > 0, true);
  assert.equal(supportStateCounts['manual-required'] > 0, true);
  assert.equal(supportStateCounts.unavailable > 0, true);
  assert.deepEqual(
    platformSourceMetadata.map((row) => `${row.platform}:${row.storeSurface}:${row.metadataState}`),
    [
      'windows:microsoft-store:manual-required',
      'macos:mac-app-store:manual-required',
      'linux:linux-package-manager:unavailable',
      'android:google-play:manual-required',
      'ios:apple-app-store:manual-required',
    ]
  );
  assert.deepEqual(
    platformSourceMetadata.map((row) => row.storeIntegrationClaim),
    ['not-claimed', 'not-claimed', 'not-claimed', 'not-claimed', 'not-claimed']
  );
  assert.deepEqual(
    platformSourceMetadata.map((row) => row.interceptionClaim),
    ['not-claimed', 'not-claimed', 'not-claimed', 'not-claimed', 'not-claimed']
  );
  assert.deepEqual(
    packageSourceArtifacts.map((row) => `${row.platform}:${row.storeSurface}:${row.artifactStatus}`),
    [
      'windows:microsoft-store:manual-required',
      'macos:mac-app-store:manual-required',
      'linux:linux-package-manager:unavailable',
      'android:google-play:device-proof-required',
      'ios:apple-app-store:device-proof-required',
    ]
  );
  assert.deepEqual(
    packageSourceArtifacts.map((row) => row.artifactEvidenceClaim),
    ['not-attached', 'not-attached', 'not-attached', 'not-attached', 'not-attached']
  );
  assert.deepEqual(
    packageSourceArtifacts.map((row) => row.childDataCustody),
    [
      'no-child-activity-data',
      'no-child-activity-data',
      'no-child-activity-data',
      'no-child-activity-data',
      'no-child-activity-data',
    ]
  );
  assert.deepEqual(
    parsedReadModel.approvalDecisions.map((decision) => decision.decisionAction),
    ['approve', 'deny', 'time-box', 'review-needed']
  );
  assert.deepEqual(
    parsedReadModel.childFacingStates.map((state) => state.childVisibleStatus),
    ['pending-parent-review-visible', 'approved-visible', 'denied-visible', 'time-box-visible', 'review-needed-visible']
  );
  assert.deepEqual(
    parsedReadModel.auditReportIntegration.map((row) => row.surface),
    [
      'request-audit-history',
      'parent-decision-audit-history',
      'child-facing-state-report',
      'platform-limitation-report',
    ]
  );

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-approval-contract-proof',
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/app-install-purchase-approval.ts',
      platformSourceContract: 'packages/schema-domain/src/app-install-purchase-approval-platform-sources.ts',
      packageSourceContract: 'packages/schema-domain/src/app-install-purchase-approval-package-sources.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-approval-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      output: relative(repoRoot, proofPath),
    },
    requestKinds: [
      parsedReadModel.installRequest.requestKind,
      parsedReadModel.purchaseRequest.requestKind,
      parsedReadModel.subscriptionRequest.requestKind,
    ],
    approvalDecisionActions: parsedReadModel.approvalDecisions.map((decision) => decision.decisionAction),
    supportStateCounts,
    platformSupportMatrix: parsedReadModel.platformSupportMatrix.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      contractRequestState: row.contractRequestState,
      storeMetadataState: row.storeMetadataState,
      installInterceptionState: row.installInterceptionState,
      purchaseInterceptionState: row.purchaseInterceptionState,
      subscriptionInterceptionState: row.subscriptionInterceptionState,
      childPendingState: row.childPendingState,
      approvalDeliveryState: row.approvalDeliveryState,
      proofRequirement: row.proofRequirement,
      claimBoundary: row.claimBoundary,
    })),
    platformSourceMetadata: platformSourceMetadata.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      sourceAuthority: row.sourceAuthority,
      metadataState: row.metadataState,
      sourceEvidenceState: row.sourceEvidenceState,
      fieldsAvailableFromContract: row.fieldsAvailableFromContract,
      fieldsRequiringPlatformProof: row.fieldsRequiringPlatformProof,
      requestKindCoverage: row.requestKindCoverage,
      requiredArtifacts: row.requiredArtifacts,
      limitationReason: row.limitationReason,
      limitationReportRef: row.limitationReportRef,
      storeIntegrationClaim: row.storeIntegrationClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      interceptionClaim: row.interceptionClaim,
      claimBoundary: row.claimBoundary,
    })),
    packageSourceArtifacts: packageSourceArtifacts.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      platformSourceRowId: row.platformSourceRowId,
      packageSourceKind: row.packageSourceKind,
      artifactStatus: row.artifactStatus,
      approvalPathState: row.approvalPathState,
      packageSourceFieldsRequired: row.packageSourceFieldsRequired,
      packageSourceFieldsAttached: row.packageSourceFieldsAttached,
      requestKindCoverage: row.requestKindCoverage,
      requiredArtifacts: row.requiredArtifacts,
      artifactEvidenceClaim: row.artifactEvidenceClaim,
      artifactEvidencePath: row.artifactEvidencePath,
      artifactCapturedAt: row.artifactCapturedAt,
      limitationReason: row.limitationReason,
      limitationReportRef: row.limitationReportRef,
      storeIntegrationClaim: row.storeIntegrationClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      interceptionClaim: row.interceptionClaim,
      childDataCustody: row.childDataCustody,
      claimBoundary: row.claimBoundary,
    })),
    childFacingStates: parsedReadModel.childFacingStates.map((state) => ({
      childVisibleStatus: state.childVisibleStatus,
      deliveryState: state.deliveryState,
      reportRefs: state.reportRefs,
      claimBoundary: state.claimBoundary,
    })),
    auditReportIntegration: parsedReadModel.auditReportIntegration.map((row) => ({
      surface: row.surface,
      integrationState: row.integrationState,
      reportRefs: row.reportRefs,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: parsedReadModel.nonClaims,
    claimBoundaries: {
      storeIntegrationClaim: parsedReadModel.storeIntegrationClaim,
      billingEntitlementClaim: parsedReadModel.billingEntitlementClaim,
      portalUiClaim: parsedReadModel.portalUiClaim,
      platformAdapterClaim: parsedReadModel.platformAdapterClaim,
      interceptionClaim: parsedReadModel.interceptionClaim,
      runtimeBlockingSeparation: parsedReadModel.runtimeBlockingSeparation,
    },
    knownGaps: proofModule.AppInstallPurchaseApprovalProofKnownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-approval-contract-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadContractProofModule() {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'app-install-purchase-approval-proof.js');
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
