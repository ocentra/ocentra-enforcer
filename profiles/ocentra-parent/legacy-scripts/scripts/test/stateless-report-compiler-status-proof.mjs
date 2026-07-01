import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'output', 'data-custody-storage-plan-proof', '06-report-query-custody');
const proofPath = join(outputDir, 'stateless-report-compiler-status-proof.json');
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
      'tests/contract/stateless-report-compiler-status.test.ts',
    ])
  );

  const proofModule = await loadContractProofModule();
  await assertPackageExport(proofModule);
  const readModel = proofModule.StatelessReportCompilerContractProofReadModel;
  const statusCounts = proofModule.summarizeStatelessReportCompilerStatuses(readModel.statuses);
  const requestedDataClassCounts = proofModule.summarizeStatelessReportCompilerRequestedDataClasses(readModel.request);

  assert.deepEqual(Object.keys(statusCounts), [
    'queued',
    'running',
    'succeeded',
    'failed',
    'expired',
    'manual-required',
  ]);
  assert.equal(
    Object.values(statusCounts).every((count) => count === 1),
    true
  );
  assert.equal(requestedDataClassCounts['generated-summary'], 1);
  assert.equal(requestedDataClassCounts['sqlite-query-row'], 1);
  assert.deepEqual(
    readModel.results.map((result) => result.status),
    ['succeeded', 'failed', 'expired', 'manual-required']
  );
  assert.equal(
    readModel.results.every((result) => result.tempArtifacts.deletionConfirmed),
    true
  );
  assert.equal(readModel.reportCompilerRuntimeClaimed, false);
  assert.equal(readModel.cloudWorkerClaimed, false);
  assert.equal(readModel.connectorOAuthProviderApiClaimed, false);
  assert.equal(readModel.ocentraHostedFamilyDataCustodyClaimed, false);
  assert.equal(readModel.childDeviceMutationClaimed, false);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'stateless-report-compiler-status-proof',
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/stateless-report-compiler-status.ts',
      values: 'packages/schema-domain/src/stateless-report-compiler-status-values.ts',
      contractTest: 'packages/schema-domain/tests/contract/stateless-report-compiler-status.test.ts',
      builtModule: 'packages/schema-domain/dist/stateless-report-compiler-status.js',
      packageExport: '@ocentra-parent/schema-domain/stateless-report-compiler-status',
      featureDoc: 'docs/features/reports-notifications-sync.md',
      expectationDocs: [
        'docs/expectations/sync-export.md',
        'docs/expectations/cloud.md',
        'docs/expectations/data-custody.md',
      ],
      output: relative(repoRoot, proofPath),
    },
    statusCounts,
    requestedDataClassCounts,
    resultStates: readModel.results.map((result) => result.status),
    sourceReferences: {
      connectorStatusRef: readModel.request.sourceConnectorStatusRef,
      sourceCursorRef: readModel.request.sourceCursorRef,
    },
    outputDestinationOwnership: readModel.request.outputDestinationOwnership,
    tempArtifactDeletionConfirmed: readModel.results.every((result) => result.tempArtifacts.deletionConfirmed),
    nonClaims: readModel.nonClaims,
    claimBoundaries: {
      reportCompilerRuntimeClaimed: readModel.reportCompilerRuntimeClaimed,
      cloudWorkerClaimed: readModel.cloudWorkerClaimed,
      connectorOAuthProviderApiClaimed: readModel.connectorOAuthProviderApiClaimed,
      portalUiClaimed: readModel.portalUiClaimed,
      ocentraHostedFamilyDataCustodyClaimed: readModel.ocentraHostedFamilyDataCustodyClaimed,
      uploadDownloadImplementationClaimed: readModel.uploadDownloadImplementationClaimed,
      childDeviceMutationClaimed: readModel.childDeviceMutationClaimed,
      retainedTempChildEvidenceClaimed: readModel.retainedTempChildEvidenceClaimed,
    },
    knownGaps: proofModule.StatelessReportCompilerKnownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`stateless-report-compiler-status-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadContractProofModule() {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'stateless-report-compiler-status.js');
  return import(pathToFileURL(modulePath).href);
}

async function assertPackageExport(proofModule) {
  const exportedModule = await import('@ocentra-parent/schema-domain/stateless-report-compiler-status');
  assert.equal(
    exportedModule.StatelessReportCompilerContractProofReadModel.request.requestId,
    proofModule.StatelessReportCompilerContractProofReadModel.request.requestId
  );
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
