import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-status-backend-execution-continuation-proof';
const resultDir = join(repoRoot, 'test-results', proofMode);
const outputDir = join(repoRoot, 'output', proofMode);
const proofPath = join(resultDir, 'proof.json');
const summaryPath = join(outputDir, 'proof-summary.json');
const commands = [];
const requiredPackageExports = [
  '@ocentra-parent/schema-domain/production-support-status-backend-execution-continuation-proof',
  '@ocentra-parent/schema-domain/production-support-status-backend-execution-continuation-read-model',
  '@ocentra-parent/schema-domain/production-support-status-backend-execution-continuation-values',
];

await main();

async function main() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/logging-domain']));

  const contract = await assertBuiltContract();
  const linkedProofs = await assertLinkedProofs();
  const documentation = await assertDocumentationProof();
  const packageExport = await assertPackageExports();
  const commit = 'branch-head-validated-by-harness';
  const proof = {
    schemaVersion: 1,
    checkedAt: 'deterministic-proof-artifact',
    commit,
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/production-support-status-backend-execution-continuation-proof.ts',
      values: 'packages/schema-domain/src/production-support-status-backend-execution-continuation-values.ts',
      readModel: 'packages/schema-domain/src/production-support-status-backend-execution-continuation-read-model.ts',
      proofHarness: 'scripts/test/production-support-status-backend-execution-continuation-proof.mjs',
      linkedProofs,
      documentation,
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      packageExport,
    },
    rowCount: contract.rows.length,
    rows: contract.rows,
    sourceContractRefs: contract.sourceContractRefs,
    nonClaims: contract.nonClaims,
    knownGaps: contract.knownGaps,
  };
  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit,
    proofMode,
    rowCount: proof.rowCount,
    targets: contract.targets,
    continuationStates: contract.continuationStates,
    sourceContractRefs: contract.sourceContractRefs,
    output: relativePath(proofPath),
    knownGaps: proof.knownGaps,
    packageExport,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertBuiltContract() {
  const contractModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-execution-continuation-proof.js'
  );
  const readModelModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-execution-continuation-read-model.js'
  );
  const valuesModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-execution-continuation-values.js'
  );
  const proof = contractModule.ProductionSupportStatusBackendExecutionContinuationProofSchema.parse(
    readModelModule.ProductionSupportStatusBackendExecutionContinuationReadModel
  );
  const summary = contractModule.summarizeProductionSupportStatusBackendExecutionContinuationRows(proof.rows);

  assert.equal(typeof contractModule.decodeProductionSupportStatusBackendExecutionContinuationProof, 'function');
  assert.deepEqual(valuesModule.RequiredExecutionContinuationStates, [
    'execution-preflight-ready',
    'runtime-worker-required',
    'durable-storage-required',
    'payload-custody-required',
    'redaction-manifest-required',
    'manual-required',
    'backend-unavailable',
  ]);
  for (const target of valuesModule.RequiredExecutionContinuationTargets) {
    assert.deepEqual(summary[target], {
      'execution-preflight-ready': 1,
      'runtime-worker-required': 1,
      'durable-storage-required': 1,
      'payload-custody-required': 1,
      'redaction-manifest-required': 1,
      'manual-required': 1,
      'backend-unavailable': 1,
    });
  }
  assert.deepEqual(proof.sourceContractRefs, valuesModule.RequiredExecutionContinuationSourceProofs);
  assert.equal(proof.statusBackendExecutionClaim, 'manual-required');
  assert.equal(proof.durableQueueStorageClaim, 'manual-required');
  assert.equal(proof.retryWorkerExecutionClaim, 'manual-required');
  assert.equal(proof.auditPersistenceClaim, 'manual-required');
  assert.equal(proof.deadLetterPayloadCustodyClaim, 'manual-required');
  assert.equal(proof.statusBackendPayloadCustodyClaim, 'manual-required');
  assert.equal(proof.redactionManifestExecutionClaim, 'manual-required');
  assert.equal(proof.publicRuntimeExecutionClaim, 'not-implemented');
  assert.equal(proof.providerExecutionClaim, 'not-implemented');
  assert.equal(proof.defaultHostedFamilyDataClaim, 'not-implemented');
  assert.equal(proof.childActivityCustodyClaim, 'not-implemented');
  assertExecutionContinuationRowsRemainManual(proof.rows);

  return {
    targets: valuesModule.RequiredExecutionContinuationTargets,
    continuationStates: valuesModule.RequiredExecutionContinuationStates,
    sourceContractRefs: proof.sourceContractRefs,
    rows: proof.rows.map((row) => ({
      target: row.target,
      continuationState: row.continuationState,
      sourceProofRefs: row.sourceProofRefs,
      statusBackendExecutionState: row.statusBackendExecutionState,
      durableQueueStorageState: row.durableQueueStorageState,
      retryWorkerExecutionState: row.retryWorkerExecutionState,
      auditPersistenceState: row.auditPersistenceState,
      deadLetterPayloadCustodyState: row.deadLetterPayloadCustodyState,
      statusBackendPayloadCustodyState: row.statusBackendPayloadCustodyState,
      redactionManifestExecutionState: row.redactionManifestExecutionState,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionSupportStatusBackendExecutionContinuationKnownGaps,
  };
}

async function assertLinkedProofs() {
  const durableRuntime = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-durable-queue-runtime-read-model.js'
  );
  const runtimeClosure = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-runtime-closure-read-model.js'
  );
  const payloadCustody = await importBuiltModule('logging-domain', 'status-backend-payload-custody-read-model.js');
  const redactionManifest = await importBuiltModule(
    'logging-domain',
    'status-backend-redaction-manifest-read-model.js'
  );

  assert.equal(
    durableRuntime.ProductionSupportStatusBackendDurableQueueRuntimeReadModel.schemaVersion,
    'production-support-status-backend-durable-queue-runtime-proof'
  );
  assert.equal(
    runtimeClosure.ProductionSupportStatusBackendRuntimeClosureReadModel.schemaVersion,
    'production-support-status-backend-runtime-closure-proof'
  );
  assert.equal(
    payloadCustody.StatusBackendPayloadCustodyReadModel.readModelId,
    'production-support-status-backend-payload-custody-proof'
  );
  assert.equal(
    redactionManifest.StatusBackendRedactionManifestReadModel.readModelId,
    'production-support-status-backend-redaction-manifest-proof'
  );

  return {
    durableRuntimeRows: durableRuntime.ProductionSupportStatusBackendDurableQueueRuntimeReadModel.rows.length,
    runtimeClosureRows: runtimeClosure.ProductionSupportStatusBackendRuntimeClosureReadModel.rows.length,
    payloadCustodyRows: payloadCustody.StatusBackendPayloadCustodyReadModel.entries.length,
    redactionManifestRows: redactionManifest.StatusBackendRedactionManifestReadModel.entries.length,
  };
}

function assertExecutionContinuationRowsRemainManual(rows) {
  for (const row of rows) {
    assert.notEqual(row.statusBackendExecutionState, 'executed', `${row.target} must not execute status backend`);
    assert.notEqual(row.durableQueueStorageState, 'persisted', `${row.target} must not persist durable queue data`);
    assert.notEqual(row.retryWorkerExecutionState, 'executed', `${row.target} must not execute retry workers`);
    assert.notEqual(row.auditPersistenceState, 'persisted', `${row.target} must not persist audit rows`);
    assert.notEqual(
      row.statusBackendPayloadCustodyState,
      'persisted',
      `${row.target} must not claim status backend payload custody`
    );
    assert.notEqual(
      row.redactionManifestExecutionState,
      'executed',
      `${row.target} must not execute redaction manifests`
    );
    for (const dataClass of row.forbiddenDataClasses) {
      assert(!row.supportSafeDataClasses.includes(dataClass), `${row.target} unexpectedly allows ${dataClass}`);
    }
  }
}

async function assertDocumentationProof() {
  const docs = [
    'docs/features/production-distribution-support.md',
    'docs/expectations/data-custody.md',
    'docs/product-capability-checklist.md',
  ];
  for (const path of docs) {
    assertIncludes(await readRepoFile(path), proofMode, `${path} proof note`);
  }
  return docs;
}

async function assertPackageExports() {
  const [contract, readModel, values] = await Promise.all(requiredPackageExports.map((specifier) => import(specifier)));
  assert.equal(typeof contract.ProductionSupportStatusBackendExecutionContinuationProofSchema.parse, 'function');
  assert.equal(typeof readModel.ProductionSupportStatusBackendExecutionContinuationReadModel, 'object');
  assert(Array.isArray(values.RequiredExecutionContinuationTargets));
  return {
    state: 'schema-domain-live-exports',
    exports: requiredPackageExports,
  };
}

async function importBuiltModule(packageDirectory, fileName) {
  return import(pathToFileURL(join(repoRoot, 'packages', packageDirectory, 'dist', fileName)).href);
}

async function readRepoFile(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

async function runCommand(commandName, args) {
  commands.push([commandName, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${commandName} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
}

function assertIncludes(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
