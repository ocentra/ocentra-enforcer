import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-status-backend-durable-queue-runtime-proof';
const resultDir = join(repoRoot, 'test-results', proofMode);
const outputDir = join(repoRoot, 'output', proofMode);
const proofPath = join(resultDir, 'proof.json');
const summaryPath = join(outputDir, 'proof-summary.json');
const commands = [];
const requiredPackageExports = [
  '@ocentra-parent/schema-domain/production-support-status-backend-durable-queue-runtime-proof',
  '@ocentra-parent/schema-domain/production-support-status-backend-durable-queue-runtime-read-model',
  '@ocentra-parent/schema-domain/production-support-status-backend-durable-queue-runtime-values',
];

await main();

async function main() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

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
      contract: 'packages/schema-domain/src/production-support-status-backend-durable-queue-runtime-proof.ts',
      values: 'packages/schema-domain/src/production-support-status-backend-durable-queue-runtime-values.ts',
      readModel: 'packages/schema-domain/src/production-support-status-backend-durable-queue-runtime-read-model.ts',
      proofHarness: 'scripts/test/production-support-status-backend-durable-queue-runtime-proof.mjs',
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
    runtimeBoundaryStates: contract.runtimeBoundaryStates,
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
    'production-support-status-backend-durable-queue-runtime-proof.js'
  );
  const readModelModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-durable-queue-runtime-read-model.js'
  );
  const valuesModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-durable-queue-runtime-values.js'
  );
  const proof = contractModule.ProductionSupportStatusBackendDurableQueueRuntimeProofSchema.parse(
    readModelModule.ProductionSupportStatusBackendDurableQueueRuntimeReadModel
  );
  const summary = contractModule.summarizeProductionSupportStatusBackendDurableQueueRuntimeRows(proof.rows);

  assert.equal(typeof contractModule.decodeProductionSupportStatusBackendDurableQueueRuntimeProof, 'function');
  assert.deepEqual(valuesModule.RequiredDurableQueueRuntimeStates, [
    'queue-storage-boundary-ready',
    'retry-worker-boundary-ready',
    'audit-persistence-boundary-ready',
    'dead-letter-runtime-boundary-ready',
    'runtime-boundary-manual-required',
    'backend-unavailable',
  ]);
  for (const target of valuesModule.RequiredDurableQueueRuntimeTargets) {
    assert.deepEqual(summary[target], {
      'queue-storage-boundary-ready': 1,
      'retry-worker-boundary-ready': 1,
      'audit-persistence-boundary-ready': 1,
      'dead-letter-runtime-boundary-ready': 1,
      'runtime-boundary-manual-required': 1,
      'backend-unavailable': 1,
    });
  }
  assert.deepEqual(proof.sourceContractRefs, valuesModule.RequiredDurableQueueRuntimeSourceProofs);
  assert.equal(proof.statusBackendExecutionClaim, 'manual-required');
  assert.equal(proof.durableQueueStorageClaim, 'manual-required');
  assert.equal(proof.retryWorkerExecutionClaim, 'manual-required');
  assert.equal(proof.auditPersistenceClaim, 'manual-required');
  assert.equal(proof.deadLetterPayloadCustodyClaim, 'manual-required');
  assert.equal(proof.publicRuntimeExecutionClaim, 'not-implemented');
  assert.equal(proof.providerExecutionClaim, 'not-implemented');
  assert.equal(proof.supportBackendUploadExecutionClaim, 'manual-required');
  assert.equal(proof.accountLookupExecutionClaim, 'manual-required');
  assert.equal(proof.billingProviderContactClaim, 'manual-required');
  assert.equal(proof.legalDisclosureExecutionClaim, 'manual-required');
  assert.equal(proof.remoteSupportSessionClaim, 'not-implemented');
  assert.equal(proof.productionSlaClaim, 'not-implemented');
  assert.equal(proof.providerSecretCustodyClaim, 'not-implemented');
  assert.equal(proof.childActivityCustodyClaim, 'not-implemented');
  assertDurableQueueRuntimeRowsRemainManual(proof.rows);

  return {
    targets: valuesModule.RequiredDurableQueueRuntimeTargets,
    runtimeBoundaryStates: valuesModule.RequiredDurableQueueRuntimeStates,
    sourceContractRefs: proof.sourceContractRefs,
    rows: proof.rows.map((row) => ({
      target: row.target,
      runtimeBoundaryState: row.runtimeBoundaryState,
      sourceProofRefs: row.sourceProofRefs,
      durableQueueStorageState: row.durableQueueStorageState,
      retryWorkerState: row.retryWorkerState,
      auditPersistenceState: row.auditPersistenceState,
      deadLetterPayloadCustodyState: row.deadLetterPayloadCustodyState,
      statusBackendExecutionState: row.statusBackendExecutionState,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionSupportStatusBackendDurableQueueRuntimeKnownGaps,
  };
}

async function assertLinkedProofs() {
  const queueReadModelModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-queue-audit-persistence-read-model.js'
  );
  const deadLetterReadModelModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-dead-letter-read-model.js'
  );
  const runtimeClosureReadModelModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-runtime-closure-read-model.js'
  );

  const queue = queueReadModelModule.ProductionSupportStatusBackendQueueAuditPersistenceReadModel;
  const deadLetter = deadLetterReadModelModule.ProductionSupportStatusBackendDeadLetterReadModel;
  const runtimeClosure = runtimeClosureReadModelModule.ProductionSupportStatusBackendRuntimeClosureReadModel;

  assert.equal(queue.schemaVersion, 'production-support-status-backend-queue-audit-persistence-proof');
  assert.equal(queue.rows.length, 48);
  assert.equal(deadLetter.schemaVersion, 'production-support-status-backend-dead-letter-proof');
  assert.equal(deadLetter.rows.length, 48);
  assert.equal(runtimeClosure.schemaVersion, 'production-support-status-backend-runtime-closure-proof');
  assert.equal(runtimeClosure.rows.length, 36);

  return {
    queueAuditRows: queue.rows.length,
    deadLetterRows: deadLetter.rows.length,
    runtimeClosureRows: runtimeClosure.rows.length,
  };
}

function assertDurableQueueRuntimeRowsRemainManual(rows) {
  for (const row of rows) {
    assert.notEqual(row.durableQueueStorageState, 'implemented', `${row.target} must not claim durable storage`);
    assert.notEqual(row.durableQueueStorageState, 'executed', `${row.target} must not claim durable storage execution`);
    assert.notEqual(row.durableQueueStorageState, 'persisted', `${row.target} must not claim durable persistence`);
    assert.notEqual(row.retryWorkerState, 'executed', `${row.target} must not claim retry worker execution`);
    assert.notEqual(row.auditPersistenceState, 'persisted', `${row.target} must not claim audit persistence`);
    assert.notEqual(
      row.deadLetterPayloadCustodyState,
      'persisted',
      `${row.target} must not claim dead-letter payload custody`
    );
    assert.notEqual(row.publicRuntimeExecutionState, 'executed', `${row.target} must not claim public runtime`);
    assert.notEqual(row.providerExecutionState, 'executed', `${row.target} must not claim provider execution`);
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
  assert.equal(typeof contract.ProductionSupportStatusBackendDurableQueueRuntimeProofSchema.parse, 'function');
  assert.equal(typeof readModel.ProductionSupportStatusBackendDurableQueueRuntimeReadModel, 'object');
  assert(Array.isArray(values.RequiredDurableQueueRuntimeTargets));
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
