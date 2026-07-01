import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-status-backend-runtime-execution-proof';
const resultDir = join(repoRoot, 'test-results', proofMode);
const outputDir = join(repoRoot, 'output', proofMode);
const proofPath = join(resultDir, 'proof.json');
const summaryPath = join(outputDir, 'proof-summary.json');
const commands = [];
const requiredPackageExports = [
  '@ocentra-parent/schema-domain/production-support-status-backend-runtime-execution-proof',
  '@ocentra-parent/schema-domain/production-support-status-backend-runtime-execution-read-model',
  '@ocentra-parent/schema-domain/production-support-status-backend-runtime-execution-values',
];

await main();

async function main() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const contract = await assertBuiltContract();
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
      contract: 'packages/schema-domain/src/production-support-status-backend-runtime-execution-proof.ts',
      values: 'packages/schema-domain/src/production-support-status-backend-runtime-execution-values.ts',
      readModel: 'packages/schema-domain/src/production-support-status-backend-runtime-execution-read-model.ts',
      proofHarness: 'scripts/test/production-support-status-backend-runtime-execution-proof.mjs',
      documentation,
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      packageExport,
    },
    rowCount: contract.rows.length,
    rows: contract.rows,
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
    readinessStates: contract.readinessStates,
    output: relativePath(proofPath),
    knownGaps: proof.knownGaps,
    packageExport,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertBuiltContract() {
  const contractModule = await importBuiltModule('production-support-status-backend-runtime-execution-proof.js');
  const readModelModule = await importBuiltModule('production-support-status-backend-runtime-execution-read-model.js');
  const valuesModule = await importBuiltModule('production-support-status-backend-runtime-execution-values.js');
  const proof = contractModule.ProductionSupportStatusBackendRuntimeExecutionProofSchema.parse(
    readModelModule.ProductionSupportStatusBackendRuntimeExecutionReadModel
  );
  const summary = contractModule.summarizeProductionSupportStatusBackendRuntimeExecutionRows(proof.rows);

  assert.equal(typeof contractModule.decodeProductionSupportStatusBackendRuntimeExecutionProof, 'function');
  assert.deepEqual(valuesModule.RequiredRuntimeExecutionStates, [
    'requested',
    'authorized',
    'queued',
    'running',
    'runtime-evidence-ready',
    'audit-ready',
    'failed',
    'manual-required',
    'backend-unavailable',
  ]);
  for (const target of valuesModule.RequiredRuntimeExecutionTargets) {
    assert.deepEqual(summary[target], {
      requested: 1,
      authorized: 1,
      queued: 1,
      running: 1,
      'runtime-evidence-ready': 1,
      'audit-ready': 1,
      failed: 1,
      'manual-required': 1,
      'backend-unavailable': 1,
    });
  }
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
  assert.equal(proof.productionSlaClaim, 'not-implemented');
  assert.equal(proof.legalDisclosureExecutionClaim, 'manual-required');
  assert.equal(proof.childActivityCustodyClaim, 'not-implemented');
  assertRuntimeExecutionRowsRemainManual(proof.rows);

  return {
    targets: valuesModule.RequiredRuntimeExecutionTargets,
    readinessStates: valuesModule.RequiredRuntimeExecutionStates,
    rows: proof.rows.map((row) => ({
      target: row.target,
      readinessState: row.readinessState,
      sourceProof: row.sourceProof,
      durableQueueStorageState: row.durableQueueStorageState,
      retryWorkerState: row.retryWorkerState,
      auditPersistenceState: row.auditPersistenceState,
      deadLetterState: row.deadLetterState,
      runtimeEvidenceState: row.runtimeEvidenceState,
      backendExecutionState: row.backendExecutionState,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionSupportStatusBackendRuntimeExecutionKnownGaps,
  };
}

function assertRuntimeExecutionRowsRemainManual(rows) {
  for (const row of rows) {
    assert.notEqual(row.durableQueueStorageState, 'implemented', `${row.target} must not claim durable storage`);
    assert.notEqual(row.durableQueueStorageState, 'executed', `${row.target} must not claim durable storage execution`);
    assert.notEqual(row.durableQueueStorageState, 'persisted', `${row.target} must not claim durable persistence`);
    assert.notEqual(row.retryWorkerState, 'implemented', `${row.target} must not claim retry worker implementation`);
    assert.notEqual(row.retryWorkerState, 'executed', `${row.target} must not claim retry worker execution`);
    assert.notEqual(row.auditPersistenceState, 'implemented', `${row.target} must not claim audit implementation`);
    assert.notEqual(row.auditPersistenceState, 'executed', `${row.target} must not claim audit execution`);
    assert.notEqual(row.auditPersistenceState, 'persisted', `${row.target} must not claim audit persistence`);
    assert.notEqual(row.deadLetterState, 'implemented', `${row.target} must not claim dead-letter implementation`);
    assert.notEqual(row.deadLetterState, 'executed', `${row.target} must not claim dead-letter execution`);
    assert.notEqual(row.backendExecutionState, 'executed', `${row.target} must not claim backend execution`);
    assert.notEqual(row.runtimeEvidenceState, 'executed', `${row.target} must not claim runtime evidence execution`);
    for (const dataClass of row.forbiddenDataClasses) {
      assert(!row.supportSafeDataClasses.includes(dataClass), `${row.target} unexpectedly allows ${dataClass}`);
    }
  }
}

async function assertDocumentationProof() {
  const docs = [
    'docs/features/production-distribution-support.md',
    'docs/expectations/release-installer.md',
    'docs/expectations/data-custody.md',
  ];
  for (const path of docs) {
    assertIncludes(await readRepoFile(path), proofMode, `${path} proof note`);
  }
  return docs;
}

async function assertPackageExports() {
  const [contract, readModel, values] = await Promise.all(requiredPackageExports.map((specifier) => import(specifier)));
  assert.equal(typeof contract.ProductionSupportStatusBackendRuntimeExecutionProofSchema.parse, 'function');
  assert.equal(typeof readModel.ProductionSupportStatusBackendRuntimeExecutionReadModel, 'object');
  assert(Array.isArray(values.RequiredRuntimeExecutionTargets));
  return {
    state: 'schema-domain-live-exports',
    exports: requiredPackageExports,
  };
}

async function importBuiltModule(fileName) {
  return import(pathToFileURL(join(repoRoot, 'packages', 'schema-domain', 'dist', fileName)).href);
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
