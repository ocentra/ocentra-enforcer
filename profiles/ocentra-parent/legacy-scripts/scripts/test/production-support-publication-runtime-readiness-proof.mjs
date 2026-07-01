import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofMode = 'production-support-publication-runtime-readiness-proof';
const resultDir = join(repoRoot, 'test-results', proofMode);
const outputDir = join(repoRoot, 'output', proofMode);
const proofPath = join(resultDir, 'proof.json');
const summaryPath = join(outputDir, 'proof-summary.json');
const commands = [];

await main();

async function main() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/schema-domain',
      '--',
      'tests/proof/production-support-publication-runtime-readiness-proof.test.ts',
    ])
  );

  const contract = await assertBuiltContract();
  const documentation = await assertDocumentationProof();
  const commit = await gitHead();
  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit,
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/production-support-publication-runtime-readiness-proof.ts',
      values: 'packages/schema-domain/src/production-support-publication-runtime-readiness-values.ts',
      readModel: 'packages/schema-domain/src/production-support-publication-runtime-readiness-read-model.ts',
      contractTest:
        'packages/schema-domain/tests/proof/production-support-publication-runtime-readiness-proof.test.ts',
      documentation,
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      packageExport: 'schema-domain-owner; production-domain-local-export-retired',
    },
    rows: contract.rows,
    nonClaims: contract.nonClaims,
    knownGaps: contract.knownGaps,
  };
  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit,
    proofMode,
    rowCount: proof.rows.length,
    rows: proof.rows.map((row) => row.item),
    output: relativePath(proofPath),
    knownGaps: proof.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertBuiltContract() {
  const contractModule = await importBuiltSchemaDomainModule('production-support-publication-runtime-readiness-proof');
  const readModelModule = await importBuiltSchemaDomainModule(
    'production-support-publication-runtime-readiness-read-model'
  );
  const valuesModule = await importBuiltSchemaDomainModule('production-support-publication-runtime-readiness-values');
  const proof = contractModule.ProductionSupportPublicationRuntimeReadinessProofSchema.parse(
    readModelModule.ProductionSupportPublicationRuntimeReadinessReadModel
  );

  assert.equal(typeof contractModule.decodeProductionSupportPublicationRuntimeReadinessProof, 'function');
  assert.deepEqual(valuesModule.RequiredPublicationRuntimeReadinessItems, [
    'public-runtime-publication-adapter-readiness',
    'support-runbook-publication-runner-readiness',
    'incident-status-publication-runner-readiness',
    'support-upload-publication-runtime-readiness',
    'privacy-legal-publication-runtime-readiness',
    'public-support-contact-runtime-readiness',
  ]);
  assert.deepEqual(contractModule.summarizeProductionSupportPublicationRuntimeReadinessRows(proof.rows), {
    'public-runtime-publication-adapter-readiness': 1,
    'support-runbook-publication-runner-readiness': 1,
    'incident-status-publication-runner-readiness': 1,
    'support-upload-publication-runtime-readiness': 1,
    'privacy-legal-publication-runtime-readiness': 1,
    'public-support-contact-runtime-readiness': 1,
  });
  assert.equal(proof.publicRuntimeExecutionClaim, 'not-implemented');
  assert.equal(proof.publicationRunnerExecutionClaim, 'manual-required');
  assert.equal(proof.supportBackendUploadExecutionClaim, 'manual-required');
  assert.equal(proof.accountLookupExecutionClaim, 'manual-required');
  assert.equal(proof.billingProviderContactClaim, 'manual-required');
  assert.equal(proof.productionSlaClaim, 'not-implemented');
  assert.equal(proof.legalDisclosureExecutionClaim, 'manual-required');
  assert.equal(proof.childActivityCustodyClaim, 'not-implemented');
  assertPublicationRuntimeRowsRemainManual(proof.rows);

  return {
    rows: proof.rows.map((row) => ({
      item: row.item,
      sourceProof: row.sourceProof,
      sourceContractState: row.sourceContractState,
      runtimeAdapterState: row.runtimeAdapterState,
      publicationRunnerState: row.publicationRunnerState,
      supportBackendUploadState: row.supportBackendUploadState,
      publicRuntimeState: row.publicRuntimeState,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionSupportPublicationRuntimeReadinessKnownGaps,
  };
}

function assertPublicationRuntimeRowsRemainManual(rows) {
  for (const row of rows) {
    assert.notEqual(row.publicRuntimeState, 'implemented', `${row.item} must not claim public runtime`);
    assert.notEqual(row.publicRuntimeState, 'executed', `${row.item} must not claim public runtime execution`);
    assert.notEqual(row.publicationRunnerState, 'implemented', `${row.item} must not claim publication runner`);
    assert.notEqual(row.publicationRunnerState, 'executed', `${row.item} must not claim publication runner execution`);
    assert.notEqual(row.supportBackendUploadState, 'executed', `${row.item} must not claim support upload execution`);
    for (const dataClass of row.forbiddenDataClasses) {
      assert(!row.supportSafeDataClasses.includes(dataClass), `${row.item} unexpectedly allows ${dataClass}`);
    }
  }
}

async function assertDocumentationProof() {
  const docs = [
    'docs/features/production-distribution-support.md',
    'docs/expectations/release-installer.md',
    'docs/expectations/documentation.md',
    'docs/product-capability-checklist.md',
  ];
  for (const path of docs) {
    assertIncludes(await readRepoFile(path), proofMode, `${path} proof note`);
  }
  return docs;
}

async function importBuiltSchemaDomainModule(moduleName) {
  return import(pathToFileURL(join(repoRoot, 'packages', 'schema-domain', 'dist', `${moduleName}.js`)).href);
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
