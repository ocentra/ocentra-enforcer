import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-report-status-read-model-handoff-proof');
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
      'tests/unit/app-install-purchase-report-status-read-model-handoff-proof.test.ts',
    ])
  );

  const proofModule = await loadReportStatusReadModelHandoffModule();
  const parsedReadModel = proofModule.AppInstallPurchaseReportStatusReadModelHandoffProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseReportStatusReadModelHandoffProof(parsedReadModel);

  assert.deepEqual(summary, {
    reportStatusReadModelRows: 4,
    readyRows: 3,
    manualRequiredRows: 1,
    portalReportUiRows: 0,
    externallyDeliveredRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.reportStatusReadModelRows.map(
      (row) =>
        `${row.sourceDecisionAction}:${row.sourceApprovalReportDomainState}:${row.sourceRuntimeReportWriterDeliveryState}:${row.parentVisibleReportStatusState}`
    ),
    [
      'approve:approval-report-ready:report-delivery-ready:parent-report-status-ready',
      'deny:approval-report-ready:report-delivery-ready:parent-report-status-ready',
      'time-box:approval-report-ready:report-delivery-ready:parent-report-status-ready',
      'review-needed:approval-report-manual-review:manual-required:manual-required',
    ]
  );
  for (const nonClaim of [
    'no-portal-report-ui',
    'no-external-runtime-report-delivery',
    'no-provider-api-execution',
    'no-store-integration',
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
    proofMode: 'app-install-purchase-report-status-read-model-handoff-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    docsState: 'feature-expectation-docs-updated-checklist-readme-sequenced-behind-current-locks',
    evidence: {
      reportStatusReadModelHandoffContract:
        'packages/schema-domain/src/app-install-purchase-report-status-read-model-handoff-proof.ts',
      sourceRuntimeReportWriterDeliveryContract:
        'packages/schema-domain/src/app-install-purchase-runtime-report-writer-delivery-proof.ts',
      sourceApprovalReportDomainContract:
        'packages/schema-domain/src/app-install-purchase-approval-report-domain-proof.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-report-status-read-model-handoff-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md updated for report status read-model handoff proof.',
      expectationDoc:
        'docs/expectations/app-install-purchase-approval.md updated for report status read-model handoff proof.',
      checklistRowDeferred:
        'docs/product-capability-checklist.md Install/purchase approval row update sequenced behind current E-C checklist lock.',
      packageExportDeferred:
        'packages/schema-domain/package.json is the current generated/thin package surface for this proof.',
      readmeDeferred: 'packages/schema-domain/package.json is the current package surface for this proof.',
      output: relative(repoRoot, proofPath),
    },
    reportStatusReadModelSummary: summary,
    reportStatusReadModelRows: parsedReadModel.reportStatusReadModelRows.map((row) => ({
      sourceDecisionAction: row.sourceDecisionAction,
      sourceApprovalReportDomainRowId: row.sourceApprovalReportDomainRowId,
      sourceRuntimeReportWriterDeliveryRowId: row.sourceRuntimeReportWriterDeliveryRowId,
      sourceApprovalReportDomainState: row.sourceApprovalReportDomainState,
      sourceRuntimeReportWriterDeliveryState: row.sourceRuntimeReportWriterDeliveryState,
      sourceRuntimeReportWriterReceiptState: row.sourceRuntimeReportWriterReceiptState,
      parentVisibleReportStatusState: row.parentVisibleReportStatusState,
      parentVisibleReportStatusRef: row.parentVisibleReportStatusRef,
      parentVisibleReportReceiptRef: row.parentVisibleReportReceiptRef,
      reportAuditEventRefs: row.reportAuditEventRefs,
      portalReportUiClaim: row.portalReportUiClaim,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
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
  console.log(`app-install-purchase-report-status-read-model-handoff-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadReportStatusReadModelHandoffModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-report-status-read-model-handoff-proof.js'
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
