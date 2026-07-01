import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-platform-limitation-action-proof');
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
      'tests/unit/app-install-purchase-platform-limitation-action-proof.test.ts',
    ])
  );

  const proofModule = await loadPlatformLimitationActionProofModule();
  const parsedReadModel = proofModule.AppInstallPurchasePlatformLimitationActionProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchasePlatformLimitationActionProof(parsedReadModel);

  assert.deepEqual(summary, {
    platformLimitationActionRows: 5,
    readyRows: 1,
    manualRequiredRows: 3,
    unavailableRows: 1,
    reportStatusLinkedRows: 5,
    providerExecutedRows: 0,
    portalRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.platformLimitationActionRows.map(
      (row) =>
        `${row.platform}:${row.storeSurface}:${row.sourceProviderStoreReportStatusState}:${row.platformLimitationActionState}`
    ),
    [
      'windows:microsoft-store:provider-store-report-status-ready:parent-action-ready',
      'macos:mac-app-store:manual-required:manual-required',
      'linux:linux-package-manager:unavailable:unavailable',
      'android:google-play:manual-required:manual-required',
      'ios:apple-app-store:manual-required:manual-required',
    ]
  );
  for (const nonClaim of [
    'no-portal-approval-ui',
    'no-portal-report-ui',
    'no-external-runtime-report-delivery',
    'no-provider-api-execution',
    'no-store-integration',
    'no-billing-provider-contact',
    'no-platform-adapter-implementation',
    'no-child-device-delivery',
    'no-app-blocking',
    'no-child-activity-data',
    'no-ocentra-hosted-family-data-custody',
  ]) {
    assert.equal(parsedReadModel.nonClaims.includes(nonClaim), true);
  }

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-platform-limitation-action-proof',
    commands,
    packageExportState: 'not-required-private-proof-script-imports-built-module-directly',
    docsState: 'feature-expectation-docs-updated-checklist-readme-not-touched',
    evidence: {
      platformLimitationActionContract:
        'packages/schema-domain/src/app-install-purchase-platform-limitation-action-proof.ts',
      sourceProviderStoreReportStatusContract:
        'packages/schema-domain/src/app-install-purchase-provider-store-report-status-proof.ts',
      sourceReportStatusReadModelContract:
        'packages/schema-domain/src/app-install-purchase-report-status-read-model-handoff-proof.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-platform-limitation-action-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md updated for platform limitation action proof.',
      expectationDoc:
        'docs/expectations/app-install-purchase-approval.md updated for platform limitation action proof.',
      checklistRowDeferred:
        'docs/product-capability-checklist.md Install/purchase approval row update sequenced by primary.',
      output: relative(repoRoot, proofPath),
    },
    platformLimitationActionSummary: summary,
    platformLimitationActionRows: parsedReadModel.platformLimitationActionRows.map((row) => ({
      platform: row.platform,
      storeSurface: row.storeSurface,
      sourceProviderStoreReportStatusRowId: row.sourceProviderStoreReportStatusRowId,
      sourceProviderStoreReportStatusState: row.sourceProviderStoreReportStatusState,
      sourceReportStatusReadModelRowIds: row.sourceReportStatusReadModelRowIds,
      sourceReportStatusReadModelStates: row.sourceReportStatusReadModelStates,
      parentVisibleReportStatusRefs: row.parentVisibleReportStatusRefs,
      auditEventRefs: row.auditEventRefs,
      platformLimitationActionState: row.platformLimitationActionState,
      parentLimitationActionRef: row.parentLimitationActionRef,
      portalApprovalUiClaim: row.portalApprovalUiClaim,
      portalReportUiClaim: row.portalReportUiClaim,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      billingProviderContactClaim: row.billingProviderContactClaim,
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
  console.log(`app-install-purchase-platform-limitation-action-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadPlatformLimitationActionProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-platform-limitation-action-proof.js'
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
