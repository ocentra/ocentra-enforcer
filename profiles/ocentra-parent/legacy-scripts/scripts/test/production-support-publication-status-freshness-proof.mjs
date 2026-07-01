import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-publication-status-freshness-proof';
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
      'tests/read-model/production-support-publication-status-freshness-proof.test.ts',
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
      contract: 'packages/schema-domain/src/production-support-publication-status-freshness-proof.ts',
      values: 'packages/schema-domain/src/production-support-publication-status-freshness-values.ts',
      readModel: 'packages/schema-domain/src/production-support-publication-status-freshness-read-model.ts',
      contractTest:
        'packages/schema-domain/tests/read-model/production-support-publication-status-freshness-proof.test.ts',
      documentation,
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
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
    rows: proof.rows.map((row) => row.surface),
    output: relativePath(proofPath),
    knownGaps: proof.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertBuiltContract() {
  const contractModule = await importBuiltSchemaDomainModule('production-support-publication-status-freshness-proof');
  const readModelModule = await importBuiltSchemaDomainModule(
    'production-support-publication-status-freshness-read-model'
  );
  const valuesModule = await importBuiltSchemaDomainModule('production-support-publication-status-freshness-values');
  const proof = contractModule.ProductionSupportPublicationStatusFreshnessProofSchema.parse(
    readModelModule.ProductionSupportPublicationStatusFreshnessReadModel
  );

  assert.equal(typeof contractModule.decodeProductionSupportPublicationStatusFreshnessProof, 'function');
  assert.deepEqual(valuesModule.RequiredPublicationStatusFreshnessSurfaces, [
    'support-runbook-publication-freshness',
    'incident-status-publication-freshness',
    'public-support-contact-publication-freshness',
    'support-backend-upload-publication-freshness',
    'privacy-legal-publication-freshness',
    'account-billing-support-publication-freshness',
  ]);
  assert.deepEqual(contractModule.summarizeProductionSupportPublicationStatusFreshnessRows(proof.rows), {
    'support-runbook-publication-freshness': 1,
    'incident-status-publication-freshness': 1,
    'public-support-contact-publication-freshness': 1,
    'support-backend-upload-publication-freshness': 1,
    'privacy-legal-publication-freshness': 1,
    'account-billing-support-publication-freshness': 1,
  });
  assert.equal(proof.publicRuntimeClaim, 'not-implemented');
  assert.equal(proof.supportPublicationExecutionClaim, 'manual-required');
  assert.equal(proof.supportBackendUploadExecutionClaim, 'manual-required');
  assert.equal(proof.accountLookupExecutionClaim, 'manual-required');
  assert.equal(proof.billingProviderContactClaim, 'manual-required');
  assert.equal(proof.productionSlaClaim, 'not-implemented');
  assert.equal(proof.legalDisclosureExecutionClaim, 'manual-required');
  assert.equal(proof.childActivityCustodyClaim, 'not-implemented');

  return {
    rows: proof.rows.map((row) => ({
      surface: row.surface,
      sourceProof: row.sourceProof,
      sourceContractState: row.sourceContractState,
      freshnessPolicyState: row.freshnessPolicyState,
      publicPublicationState: row.publicPublicationState,
      publicRuntimeState: row.publicRuntimeState,
      supportBackendUploadState: row.supportBackendUploadState,
      legalExecutionState: row.legalExecutionState,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionSupportPublicationStatusFreshnessKnownGaps,
  };
}

async function assertDocumentationProof() {
  const docs = [
    'docs/features/production-distribution-support.md',
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
