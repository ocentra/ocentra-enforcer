import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-status-backend-execution-queue-proof';
const resultDir = join(repoRoot, 'test-results', proofMode);
const outputDir = join(repoRoot, 'output', proofMode);
const proofPath = join(resultDir, 'proof.json');
const summaryPath = join(outputDir, 'proof-summary.json');
const commands = [];
const requiredPackageExports = [
  '@ocentra-parent/schema-domain/production-support-status-backend-execution-queue-proof',
  '@ocentra-parent/schema-domain/production-support-status-backend-execution-queue-read-model',
  '@ocentra-parent/schema-domain/production-support-status-backend-execution-queue-values',
];

await main();

async function main() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const contract = await assertBuiltContract();
  const documentation = await assertDocumentationProof();
  const exportedPackagePaths = await assertPackageExports();
  const commit = await gitHead();
  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit,
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/production-support-status-backend-execution-queue-proof.ts',
      values: 'packages/schema-domain/src/production-support-status-backend-execution-queue-values.ts',
      readModel: 'packages/schema-domain/src/production-support-status-backend-execution-queue-read-model.ts',
      proofHarness: 'scripts/test/production-support-status-backend-execution-queue-proof.mjs',
      documentation,
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      packageExports: exportedPackagePaths,
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
    queueStates: contract.queueStates,
    output: relativePath(proofPath),
    knownGaps: proof.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertBuiltContract() {
  const contractModule = await importBuiltModule('production-support-status-backend-execution-queue-proof.js');
  const readModelModule = await importBuiltModule('production-support-status-backend-execution-queue-read-model.js');
  const valuesModule = await importBuiltModule('production-support-status-backend-execution-queue-values.js');
  const proof = contractModule.ProductionSupportStatusBackendExecutionQueueProofSchema.parse(
    readModelModule.ProductionSupportStatusBackendExecutionQueueReadModel
  );
  const summary = contractModule.summarizeProductionSupportStatusBackendExecutionQueueRows(proof.rows);

  assert.equal(typeof contractModule.decodeProductionSupportStatusBackendExecutionQueueProof, 'function');
  assert.deepEqual(valuesModule.RequiredStatusBackendExecutionQueueStates, [
    'requested',
    'authorized',
    'queued',
    'running',
    'succeeded',
    'failed',
    'manual-required',
    'backend-unavailable',
  ]);
  for (const target of valuesModule.RequiredStatusBackendExecutionQueueTargets) {
    assert.deepEqual(summary[target], {
      requested: 1,
      authorized: 1,
      queued: 1,
      running: 1,
      succeeded: 1,
      failed: 1,
      'manual-required': 1,
      'backend-unavailable': 1,
    });
  }
  assert.equal(proof.statusBackendExecutionClaim, 'manual-required');
  assert.equal(proof.publicRuntimeExecutionClaim, 'not-implemented');
  assert.equal(proof.providerExecutionClaim, 'not-implemented');
  assert.equal(proof.supportBackendUploadExecutionClaim, 'manual-required');
  assert.equal(proof.accountLookupExecutionClaim, 'manual-required');
  assert.equal(proof.billingProviderContactClaim, 'manual-required');
  assert.equal(proof.productionSlaClaim, 'not-implemented');
  assert.equal(proof.legalDisclosureExecutionClaim, 'manual-required');
  assert.equal(proof.childActivityCustodyClaim, 'not-implemented');
  assertQueueRowsRemainManual(proof.rows);

  return {
    targets: valuesModule.RequiredStatusBackendExecutionQueueTargets,
    queueStates: valuesModule.RequiredStatusBackendExecutionQueueStates,
    rows: proof.rows.map((row) => ({
      target: row.target,
      queueState: row.queueState,
      sourceProof: row.sourceProof,
      authorizationState: row.authorizationState,
      queueAdapterState: row.queueAdapterState,
      backendExecutionState: row.backendExecutionState,
      retryState: row.retryState,
      auditState: row.auditState,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionSupportStatusBackendExecutionQueueKnownGaps,
  };
}

function assertQueueRowsRemainManual(rows) {
  for (const row of rows) {
    assert.notEqual(row.backendExecutionState, 'implemented', `${row.target} must not claim backend implementation`);
    assert.notEqual(row.backendExecutionState, 'executed', `${row.target} must not claim backend execution`);
    assert.notEqual(row.publicRuntimeExecutionState, 'implemented', `${row.target} must not claim public runtime`);
    assert.notEqual(
      row.publicRuntimeExecutionState,
      'executed',
      `${row.target} must not claim public runtime execution`
    );
    assert.notEqual(row.providerExecutionState, 'implemented', `${row.target} must not claim provider implementation`);
    assert.notEqual(row.providerExecutionState, 'executed', `${row.target} must not claim provider execution`);
    assert.notEqual(row.supportBackendUploadState, 'executed', `${row.target} must not claim support upload execution`);
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
  assert.equal(typeof contract.ProductionSupportStatusBackendExecutionQueueProofSchema.parse, 'function');
  assert.equal(typeof readModel.ProductionSupportStatusBackendExecutionQueueReadModel, 'object');
  assert(Array.isArray(values.RequiredStatusBackendExecutionQueueTargets));
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

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
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
