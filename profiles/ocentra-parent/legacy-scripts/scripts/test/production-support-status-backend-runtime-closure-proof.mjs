import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-status-backend-runtime-closure-proof';
const resultDir = join(repoRoot, 'test-results', proofMode);
const outputDir = join(repoRoot, 'output', proofMode);
const proofPath = join(resultDir, 'proof.json');
const summaryPath = join(outputDir, 'proof-summary.json');
const commands = [];
const requiredPackageExports = [
  '@ocentra-parent/schema-domain/production-support-status-backend-runtime-closure-proof',
  '@ocentra-parent/schema-domain/production-support-status-backend-runtime-closure-read-model',
  '@ocentra-parent/schema-domain/production-support-status-backend-runtime-closure-values',
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
      contract: 'packages/schema-domain/src/production-support-status-backend-runtime-closure-proof.ts',
      values: 'packages/schema-domain/src/production-support-status-backend-runtime-closure-values.ts',
      readModel: 'packages/schema-domain/src/production-support-status-backend-runtime-closure-read-model.ts',
      proofHarness: 'scripts/test/production-support-status-backend-runtime-closure-proof.mjs',
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
    closureStates: contract.closureStates,
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
    'production-support-status-backend-runtime-closure-proof.js'
  );
  const readModelModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-runtime-closure-read-model.js'
  );
  const valuesModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-runtime-closure-values.js'
  );
  const proof = contractModule.ProductionSupportStatusBackendRuntimeClosureProofSchema.parse(
    readModelModule.ProductionSupportStatusBackendRuntimeClosureReadModel
  );
  const summary = contractModule.summarizeProductionSupportStatusBackendRuntimeClosureRows(proof.rows);

  assert.equal(typeof contractModule.decodeProductionSupportStatusBackendRuntimeClosureProof, 'function');
  assert.deepEqual(valuesModule.RequiredRuntimeClosureStates, [
    'runtime-row-validated',
    'queue-audit-linked',
    'payload-custody-linked',
    'redaction-manifest-linked',
    'closure-manual-required',
    'backend-unavailable',
  ]);
  for (const target of valuesModule.RequiredRuntimeClosureTargets) {
    assert.deepEqual(summary[target], {
      'runtime-row-validated': 1,
      'queue-audit-linked': 1,
      'payload-custody-linked': 1,
      'redaction-manifest-linked': 1,
      'closure-manual-required': 1,
      'backend-unavailable': 1,
    });
  }
  assert.deepEqual(proof.sourceContractRefs, valuesModule.RequiredRuntimeClosureSourceProofs);
  assert.equal(proof.statusBackendExecutionClaim, 'manual-required');
  assert.equal(proof.durableQueueStorageClaim, 'manual-required');
  assert.equal(proof.retryWorkerExecutionClaim, 'manual-required');
  assert.equal(proof.auditPersistenceClaim, 'manual-required');
  assert.equal(proof.deadLetterPayloadCustodyClaim, 'manual-required');
  assert.equal(proof.statusBackendPayloadCustodyClaim, 'manual-required');
  assert.equal(proof.redactionManifestExecutionClaim, 'manual-required');
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
  assertRuntimeClosureRowsRemainManual(proof.rows);

  return {
    targets: valuesModule.RequiredRuntimeClosureTargets,
    closureStates: valuesModule.RequiredRuntimeClosureStates,
    sourceContractRefs: proof.sourceContractRefs,
    rows: proof.rows.map((row) => ({
      target: row.target,
      closureState: row.closureState,
      sourceProofRefs: row.sourceProofRefs,
      durableQueueStorageState: row.durableQueueStorageState,
      retryWorkerState: row.retryWorkerState,
      auditPersistenceState: row.auditPersistenceState,
      deadLetterPayloadCustodyState: row.deadLetterPayloadCustodyState,
      statusBackendPayloadCustodyState: row.statusBackendPayloadCustodyState,
      redactionManifestExecutionState: row.redactionManifestExecutionState,
      statusBackendExecutionState: row.statusBackendExecutionState,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionSupportStatusBackendRuntimeClosureKnownGaps,
  };
}

async function assertLinkedProofs() {
  const runtimeReadModelModule = await importBuiltModule(
    'schema-domain',
    'production-support-status-backend-runtime-execution-read-model.js'
  );
  const payloadReadModelModule = await importBuiltModule(
    'logging-domain',
    'status-backend-payload-custody-read-model.js'
  );
  const redactionReadModelModule = await importBuiltModule(
    'logging-domain',
    'status-backend-redaction-manifest-read-model.js'
  );

  const runtime = runtimeReadModelModule.ProductionSupportStatusBackendRuntimeExecutionReadModel;
  const payload = payloadReadModelModule.StatusBackendPayloadCustodyReadModel;
  const redaction = redactionReadModelModule.StatusBackendRedactionManifestReadModel;

  assert.equal(runtime.schemaVersion, 'production-support-status-backend-runtime-execution-proof');
  assert.equal(runtime.rows.length, 54);
  assert.equal(payload.readModelId, 'production-support-status-backend-payload-custody-proof');
  assert.equal(payload.entries.length, 6);
  assert.equal(redaction.readModelId, 'production-support-status-backend-redaction-manifest-proof');
  assert.equal(redaction.entries.length, 6);

  return {
    runtimeExecutionRows: runtime.rows.length,
    payloadCustodyEntries: payload.entries.length,
    redactionManifestEntries: redaction.entries.length,
  };
}

function assertRuntimeClosureRowsRemainManual(rows) {
  for (const row of rows) {
    assert.notEqual(row.durableQueueStorageState, 'implemented', `${row.target} must not claim durable storage`);
    assert.notEqual(row.durableQueueStorageState, 'executed', `${row.target} must not claim durable storage execution`);
    assert.notEqual(row.durableQueueStorageState, 'persisted', `${row.target} must not claim durable persistence`);
    assert.notEqual(row.auditPersistenceState, 'executed', `${row.target} must not claim audit execution`);
    assert.notEqual(row.auditPersistenceState, 'persisted', `${row.target} must not claim audit persistence`);
    assert.notEqual(
      row.statusBackendPayloadCustodyState,
      'persisted',
      `${row.target} must not claim status backend payload persistence`
    );
    assert.notEqual(
      row.redactionManifestExecutionState,
      'executed',
      `${row.target} must not claim redaction manifest execution`
    );
    assert.notEqual(row.publicRuntimeExecutionState, 'executed', `${row.target} must not claim public runtime`);
    assert.notEqual(row.providerExecutionState, 'executed', `${row.target} must not claim provider execution`);
    for (const dataClass of row.forbiddenDataClasses) {
      assert(!row.supportSafeDataClasses.includes(dataClass), `${row.target} unexpectedly allows ${dataClass}`);
    }
  }
}

async function assertDocumentationProof() {
  const docs = ['docs/features/production-distribution-support.md', 'docs/expectations/data-custody.md'];
  for (const path of docs) {
    assertIncludes(await readRepoFile(path), proofMode, `${path} proof note`);
  }
  return docs;
}

async function assertPackageExports() {
  const [contract, readModel, values] = await Promise.all(requiredPackageExports.map((specifier) => import(specifier)));
  assert.equal(typeof contract.ProductionSupportStatusBackendRuntimeClosureProofSchema.parse, 'function');
  assert.equal(typeof readModel.ProductionSupportStatusBackendRuntimeClosureReadModel, 'object');
  assert(Array.isArray(values.RequiredRuntimeClosureTargets));
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
