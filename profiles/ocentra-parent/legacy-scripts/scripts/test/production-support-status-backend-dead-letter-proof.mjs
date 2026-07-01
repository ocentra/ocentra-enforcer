import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-status-backend-dead-letter-proof';
const resultDir = join(repoRoot, 'test-results', proofMode);
const outputDir = join(repoRoot, 'output', proofMode);
const proofPath = join(resultDir, 'proof.json');
const summaryPath = join(outputDir, 'proof-summary.json');
const deterministicCheckedAt = 'deterministic-proof-artifact';
const deterministicCommit = 'branch-head-validated-by-harness';
const commands = [];
const requiredPackageExports = [
  '@ocentra-parent/schema-domain/production-support-status-backend-dead-letter-proof',
  '@ocentra-parent/schema-domain/production-support-status-backend-dead-letter-read-model',
  '@ocentra-parent/schema-domain/production-support-status-backend-dead-letter-values',
];

await main();

async function main() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const contract = await assertBuiltContract();
  const documentation = await assertDocumentationProof();
  const packageExports = await assertPackageExports();
  const proof = {
    schemaVersion: 1,
    checkedAt: deterministicCheckedAt,
    commit: deterministicCommit,
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/production-support-status-backend-dead-letter-proof.ts',
      values: 'packages/schema-domain/src/production-support-status-backend-dead-letter-values.ts',
      readModel: 'packages/schema-domain/src/production-support-status-backend-dead-letter-read-model.ts',
      proofHarness: 'scripts/test/production-support-status-backend-dead-letter-proof.mjs',
      documentation,
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      packageExports,
    },
    rowCount: contract.rows.length,
    rows: contract.rows,
    nonClaims: contract.nonClaims,
    knownGaps: contract.knownGaps,
  };
  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    proofMode,
    rowCount: proof.rowCount,
    targets: contract.targets,
    deadLetterStates: contract.deadLetterStates,
    output: relativePath(proofPath),
    packageExports,
    knownGaps: proof.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertBuiltContract() {
  const contractModule = await importBuiltModule('production-support-status-backend-dead-letter-proof.js');
  const readModelModule = await importBuiltModule('production-support-status-backend-dead-letter-read-model.js');
  const valuesModule = await importBuiltModule('production-support-status-backend-dead-letter-values.js');
  const proof = contractModule.ProductionSupportStatusBackendDeadLetterProofSchema.parse(
    readModelModule.ProductionSupportStatusBackendDeadLetterReadModel
  );
  const summary = contractModule.summarizeProductionSupportStatusBackendDeadLetterRows(proof.rows);

  assert.equal(typeof contractModule.decodeProductionSupportStatusBackendDeadLetterProof, 'function');
  assert.deepEqual(valuesModule.RequiredStatusBackendDeadLetterStates, [
    'requested',
    'authorized',
    'dead-lettered',
    'triage-ready',
    'retry-blocked',
    'failed',
    'manual-required',
    'backend-unavailable',
  ]);
  for (const target of valuesModule.RequiredStatusBackendDeadLetterTargets) {
    assert.deepEqual(summary[target], {
      requested: 1,
      authorized: 1,
      'dead-lettered': 1,
      'triage-ready': 1,
      'retry-blocked': 1,
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
  assertDeadLetterRowsRemainManual(proof.rows);

  return {
    targets: valuesModule.RequiredStatusBackendDeadLetterTargets,
    deadLetterStates: valuesModule.RequiredStatusBackendDeadLetterStates,
    rows: proof.rows.map((row) => ({
      target: row.target,
      deadLetterState: row.deadLetterState,
      sourceProof: row.sourceProof,
      durableQueueStorageState: row.durableQueueStorageState,
      retryWorkerState: row.retryWorkerState,
      auditPersistenceState: row.auditPersistenceState,
      backendExecutionState: row.backendExecutionState,
      deadLetterPayloadCustodyState: row.deadLetterPayloadCustodyState,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionSupportStatusBackendDeadLetterKnownGaps,
  };
}

function assertDeadLetterRowsRemainManual(rows) {
  for (const row of rows) {
    assert.notEqual(row.durableQueueStorageState, 'implemented', `${row.target} must not claim durable storage`);
    assert.notEqual(row.durableQueueStorageState, 'executed', `${row.target} must not claim durable storage execution`);
    assert.notEqual(row.durableQueueStorageState, 'persisted', `${row.target} must not claim durable persistence`);
    assert.notEqual(row.retryWorkerState, 'implemented', `${row.target} must not claim retry worker implementation`);
    assert.notEqual(row.retryWorkerState, 'executed', `${row.target} must not claim retry worker execution`);
    assert.notEqual(row.auditPersistenceState, 'implemented', `${row.target} must not claim audit implementation`);
    assert.notEqual(row.auditPersistenceState, 'executed', `${row.target} must not claim audit execution`);
    assert.notEqual(row.auditPersistenceState, 'persisted', `${row.target} must not claim audit persistence`);
    assert.notEqual(row.deadLetterPayloadCustodyState, 'persisted', `${row.target} must not claim payload custody`);
    assert.notEqual(row.backendExecutionState, 'executed', `${row.target} must not claim backend execution`);
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
  assert.equal(typeof contract.ProductionSupportStatusBackendDeadLetterProofSchema.parse, 'function');
  assert.equal(typeof readModel.ProductionSupportStatusBackendDeadLetterReadModel, 'object');
  assert(Array.isArray(values.RequiredStatusBackendDeadLetterTargets));
  return requiredPackageExports;
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
