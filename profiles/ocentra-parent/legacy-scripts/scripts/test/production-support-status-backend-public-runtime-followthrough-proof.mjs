import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-status-backend-public-runtime-followthrough-proof';
const resultDir = join(repoRoot, 'test-results', proofMode);
const outputDir = join(repoRoot, 'output', proofMode);
const proofPath = join(resultDir, 'proof.json');
const summaryPath = join(outputDir, 'proof-summary.json');
const commands = [];
const requiredPackageExports = [
  '@ocentra-parent/schema-domain/production-support-status-backend-public-runtime-followthrough-proof',
  '@ocentra-parent/schema-domain/production-support-status-backend-public-runtime-followthrough-read-model',
  '@ocentra-parent/schema-domain/production-support-status-backend-public-runtime-followthrough-values',
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
      contract: 'packages/schema-domain/src/production-support-status-backend-public-runtime-followthrough-proof.ts',
      values: 'packages/schema-domain/src/production-support-status-backend-public-runtime-followthrough-values.ts',
      readModel:
        'packages/schema-domain/src/production-support-status-backend-public-runtime-followthrough-read-model.ts',
      proofHarness: 'scripts/test/production-support-status-backend-public-runtime-followthrough-proof.mjs',
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
    followthroughStates: contract.followthroughStates,
    output: relativePath(proofPath),
    knownGaps: proof.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertBuiltContract() {
  const contractModule = await importBuiltModule(
    'production-support-status-backend-public-runtime-followthrough-proof.js'
  );
  const readModelModule = await importBuiltModule(
    'production-support-status-backend-public-runtime-followthrough-read-model.js'
  );
  const valuesModule = await importBuiltModule(
    'production-support-status-backend-public-runtime-followthrough-values.js'
  );
  const proof = contractModule.ProductionSupportStatusBackendPublicRuntimeFollowthroughProofSchema.parse(
    readModelModule.ProductionSupportStatusBackendPublicRuntimeFollowthroughReadModel
  );
  const summary = contractModule.summarizeProductionSupportStatusBackendPublicRuntimeFollowthroughRows(proof.rows);

  assert.equal(typeof contractModule.decodeProductionSupportStatusBackendPublicRuntimeFollowthroughProof, 'function');
  assert.deepEqual(valuesModule.RequiredStatusBackendPublicRuntimeFollowthroughStates, [
    'requested',
    'queued',
    'running',
    'succeeded',
    'failed',
    'manual-required',
  ]);
  for (const target of valuesModule.RequiredStatusBackendPublicRuntimeFollowthroughTargets) {
    assert.deepEqual(summary[target], {
      requested: 1,
      queued: 1,
      running: 1,
      succeeded: 1,
      failed: 1,
      'manual-required': 1,
    });
  }
  assert.equal(proof.publicRuntimeExecutionClaim, 'not-implemented');
  assert.equal(proof.statusBackendExecutionClaim, 'manual-required');
  assert.equal(proof.supportBackendUploadExecutionClaim, 'manual-required');
  assert.equal(proof.accountLookupExecutionClaim, 'manual-required');
  assert.equal(proof.billingProviderContactClaim, 'manual-required');
  assert.equal(proof.productionSlaClaim, 'not-implemented');
  assert.equal(proof.legalDisclosureExecutionClaim, 'manual-required');
  assert.equal(proof.childActivityCustodyClaim, 'not-implemented');
  assertFollowthroughRowsRemainManual(proof.rows);

  return {
    targets: valuesModule.RequiredStatusBackendPublicRuntimeFollowthroughTargets,
    followthroughStates: valuesModule.RequiredStatusBackendPublicRuntimeFollowthroughStates,
    rows: proof.rows.map((row) => ({
      target: row.target,
      followthroughState: row.followthroughState,
      sourceProof: row.sourceProof,
      publicRuntimeFollowthroughState: row.publicRuntimeFollowthroughState,
      statusBackendFollowthroughState: row.statusBackendFollowthroughState,
      supportBackendUploadState: row.supportBackendUploadState,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionSupportStatusBackendPublicRuntimeFollowthroughKnownGaps,
  };
}

function assertFollowthroughRowsRemainManual(rows) {
  for (const row of rows) {
    assert.notEqual(row.publicRuntimeFollowthroughState, 'implemented', `${row.target} must not claim public runtime`);
    assert.notEqual(
      row.publicRuntimeFollowthroughState,
      'executed',
      `${row.target} must not claim public runtime execution`
    );
    assert.notEqual(row.statusBackendFollowthroughState, 'implemented', `${row.target} must not claim status backend`);
    assert.notEqual(
      row.statusBackendFollowthroughState,
      'executed',
      `${row.target} must not claim status backend execution`
    );
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
  assert.equal(typeof contract.ProductionSupportStatusBackendPublicRuntimeFollowthroughProofSchema.parse, 'function');
  assert.equal(typeof readModel.ProductionSupportStatusBackendPublicRuntimeFollowthroughReadModel, 'object');
  assert(Array.isArray(values.RequiredStatusBackendPublicRuntimeFollowthroughTargets));
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
