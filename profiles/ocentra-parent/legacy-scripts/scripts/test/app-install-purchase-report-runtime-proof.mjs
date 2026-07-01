import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-report-runtime-proof');
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
      'tests/unit/app-install-purchase-report-runtime-proof.test.ts',
    ])
  );

  const proofModule = await loadReportRuntimeProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseReportRuntimeProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseReportRuntimeProof(parsedReadModel);
  assert.deepEqual(summary, {
    reportRuntimeRows: 4,
    compilerLinkedRows: 4,
    outputReportRefs: 4,
    portalDeliveredRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.reportRuntimeRows.map((row) => `${row.reportSurface}:${row.reportRuntimeStatusClaim}`),
    [
      'request-audit-history:compiler-status-linked',
      'parent-decision-audit-history:compiler-status-linked',
      'child-facing-state-report:compiler-status-linked',
      'platform-limitation-report:compiler-status-linked',
    ]
  );
  assert.equal(parsedReadModel.nonClaims.includes('no-portal-report-ui'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-runtime-report-delivery'), true);
  assert.equal(parsedReadModel.nonClaims.includes('no-ocentra-hosted-family-data-custody'), true);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-report-runtime-proof',
    commands,
    evidence: {
      reportRuntimeContract: 'packages/schema-domain/src/app-install-purchase-report-runtime-proof.ts',
      sourceChildArtifactContract: 'packages/schema-domain/src/app-install-purchase-child-artifact-delivery-proof.ts',
      sourcePlatformArtifactContract: 'packages/schema-domain/src/app-install-purchase-platform-artifact-proof.ts',
      sourceReportCompilerContract: 'packages/schema-domain/src/stateless-report-compiler-status.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-report-runtime-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistDelta: 'DOC_DELTA: docs/product-capability-checklist.md row Install/purchase approval',
      output: relative(repoRoot, proofPath),
    },
    reportRuntimeSummary: summary,
    reportRuntimeRows: parsedReadModel.reportRuntimeRows.map((row) => ({
      reportSurface: row.reportSurface,
      sourceReportRef: row.sourceReportRef,
      compilerRequestId: row.compilerRequestId,
      compilerStatuses: row.compilerStatuses,
      compilerFinalResultStatuses: row.compilerFinalResultStatuses,
      outputReportRef: row.outputReportRef,
      childArtifactRefs: row.childArtifactRefs,
      parentAuthorized: row.parentAuthorized,
      rawChildEvidenceRequested: row.rawChildEvidenceRequested,
      rawEvidenceExcludedFromOutput: row.rawEvidenceExcludedFromOutput,
      childDetailMinimized: row.childDetailMinimized,
      tempDeletionConfirmed: row.tempDeletionConfirmed,
      localEvidenceMutated: row.localEvidenceMutated,
      ocentraHostedReportRetained: row.ocentraHostedReportRetained,
      reportRuntimeStatusClaim: row.reportRuntimeStatusClaim,
      runtimeReportDeliveryClaim: row.runtimeReportDeliveryClaim,
      portalUiClaim: row.portalUiClaim,
      providerApiClaim: row.providerApiClaim,
      storeIntegrationClaim: row.storeIntegrationClaim,
      platformAdapterClaim: row.platformAdapterClaim,
      childDeliveryClaim: row.childDeliveryClaim,
      childDataCustody: row.childDataCustody,
      appBlockingClaim: row.appBlockingClaim,
      ocentraHostedFamilyDataCustodyClaim: row.ocentraHostedFamilyDataCustodyClaim,
      claimBoundary: row.claimBoundary,
    })),
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-report-runtime-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadReportRuntimeProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-report-runtime-proof.js'
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
