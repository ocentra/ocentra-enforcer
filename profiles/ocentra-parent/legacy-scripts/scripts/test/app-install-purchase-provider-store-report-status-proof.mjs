import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-provider-store-report-status-proof');
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
      'tests/unit/app-install-purchase-provider-store-report-status-proof.test.ts',
    ])
  );

  const proofModule = await loadProviderStoreReportStatusProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseProviderStoreReportStatusProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseProviderStoreReportStatusProof(parsedReadModel);

  assert.deepEqual(summary, {
    providerStoreReportStatusRows: 5,
    readyRows: 1,
    manualRequiredRows: 3,
    unavailableRows: 1,
    approvalReportLinkedRows: 5,
    providerExecutedRows: 0,
    portalRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.providerStoreReportStatusRows.map(
      (row) =>
        `${row.platform}:${row.storeSurface}:${row.sourceProviderStoreExecutionReadinessState}:${row.providerStoreReportStatusState}`
    ),
    [
      'windows:microsoft-store:provider-store-execution-ready:provider-store-report-status-ready',
      'macos:mac-app-store:manual-required:manual-required',
      'linux:linux-package-manager:unavailable:unavailable',
      'android:google-play:manual-required:manual-required',
      'ios:apple-app-store:manual-required:manual-required',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-provider-api-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-store-integration'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-billing-provider-contact'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-portal-approval-ui'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-portal-report-ui'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-runtime-report-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-ocentra-hosted-family-data-custody'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-provider-store-report-status-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    docsState: 'feature-expectation-docs-covered-checklist-readme-sequenced-behind-current-locks',
    evidence: {
      providerStoreReportStatusContract:
        'packages/schema-domain/src/app-install-purchase-provider-store-report-status-proof.ts',
      sourceProviderStoreExecutionReadinessContract:
        'packages/schema-domain/src/app-install-purchase-provider-store-execution-readiness-proof.ts',
      sourceApprovalReportDomainContract:
        'packages/schema-domain/src/app-install-purchase-approval-report-domain-proof.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-provider-store-report-status-proof.test.ts',
      featureDocDeferred:
        'docs/features/app-install-purchase-approval.md already records provider/store report status proof coverage.',
      expectationDocDeferred:
        'docs/expectations/app-install-purchase-approval.md already records provider/store report status proof coverage.',
      checklistRowDeferred:
        'docs/product-capability-checklist.md Install/purchase approval row update sequenced behind current E-C checklist lock.',
      packageExportDeferred:
        'packages/schema-domain/package.json is the current generated/thin package surface for this proof.',
      readmeDeferred: 'packages/schema-domain/package.json is the current package surface for this proof.',
      output: relative(repoRoot, proofPath),
    },
    providerStoreReportStatusSummary: summary,
    providerStoreReportStatusRows: parsedReadModel.providerStoreReportStatusRows.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      sourceProviderStoreExecutionReadinessRowId: row.sourceProviderStoreExecutionReadinessRowId,
      sourceProviderStoreExecutionReadinessState: row.sourceProviderStoreExecutionReadinessState,
      sourceApprovalReportDomainRowIds: row.sourceApprovalReportDomainRowIds,
      sourceApprovalReportDomainStates: row.sourceApprovalReportDomainStates,
      sourceReportRuntimeRefs: row.sourceReportRuntimeRefs,
      sourceAuditEventRefs: row.sourceAuditEventRefs,
      providerStoreReportStatusState: row.providerStoreReportStatusState,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      billingProviderContactClaim: row.billingProviderContactClaim,
      portalApprovalUiClaim: row.portalApprovalUiClaim,
      portalReportUiClaim: row.portalReportUiClaim,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      childDeviceDeliveryClaim: row.childDeviceDeliveryClaim,
      appBlockingClaim: row.appBlockingClaim,
      childDataCustody: row.childDataCustody,
      ocentraHostedFamilyDataCustodyClaim: row.ocentraHostedFamilyDataCustodyClaim,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-provider-store-report-status-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadProviderStoreReportStatusProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-provider-store-report-status-proof.js'
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
