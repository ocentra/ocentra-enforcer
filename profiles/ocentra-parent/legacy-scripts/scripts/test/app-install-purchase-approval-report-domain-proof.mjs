import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-approval-report-domain-proof');
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
      'tests/unit/app-install-purchase-approval-report-domain-proof.test.ts',
    ])
  );

  const proofModule = await loadApprovalReportDomainProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseApprovalReportDomainProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseApprovalReportDomainProof(parsedReadModel);

  assert.deepEqual(summary, {
    approvalReportDomainRows: 4,
    readyRows: 3,
    manualReviewRows: 1,
    unavailableRows: 0,
    reportLinkedRows: 4,
    portalApprovalUiRows: 0,
    portalReportUiRows: 0,
  });
  assert.equal(parsedReadModel.nonClaims.includes('no-portal-approval-ui'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-portal-report-ui'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-runtime-report-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-provider-api-execution'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-child-device-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-ocentra-hosted-family-data-custody'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-approval-report-domain-proof',
    commands,
    packageExportState: 'canonical-schema-domain-public-subpath-export-confirmed; proof imports the built dist module directly.',
    checklistState: 'validated-product-capability-checklist-row',
    evidence: {
      approvalReportDomainContract: 'packages/schema-domain/src/app-install-purchase-approval-report-domain-proof.ts',
      sourceParentReviewActionContract: 'packages/schema-domain/src/app-install-purchase-parent-review-action-proof.ts',
      sourceReportRuntimeContract: 'packages/schema-domain/src/app-install-purchase-report-runtime-proof.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-approval-report-domain-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      packageExport:
        '@ocentra-parent/schema-domain/app-install-purchase-approval-report-domain-proof',
      checklistRow:
        'COMPLETED: docs/product-capability-checklist.md Install/purchase approval row includes approval/report domain proof.',
      output: relative(repoRoot, proofPath),
    },
    approvalReportDomainSummary: summary,
    approvalReportDomainRows: parsedReadModel.approvalReportDomainRows.map((row) => ({
      sourceDecisionAction: row.sourceDecisionAction,
      sourceParentReviewActionState: row.sourceParentReviewActionState,
      approvalReportDomainState: row.approvalReportDomainState,
      parentActionRecorded: row.parentActionRecorded,
      sourceReportRuntimeRefs: row.sourceReportRuntimeRefs,
      sourceReportSurfaces: row.sourceReportSurfaces,
      sourceAuditEventRefs: row.sourceAuditEventRefs,
      domainReadModelClaim: row.domainReadModelClaim,
      portalApprovalUiClaim: row.portalApprovalUiClaim,
      portalReportUiClaim: row.portalReportUiClaim,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      providerApiExecutionClaim: row.providerApiExecutionClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      childDeviceDeliveryClaim: row.childDeviceDeliveryClaim,
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
  console.log(`app-install-purchase-approval-report-domain-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadApprovalReportDomainProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-approval-report-domain-proof.js'
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
