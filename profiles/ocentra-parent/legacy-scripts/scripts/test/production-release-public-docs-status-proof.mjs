import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-release-public-docs-status-proof';
const resultDir = join(repoRoot, 'test-results', proofMode);
const outputDir = join(repoRoot, 'output', proofMode);
const proofPath = join(resultDir, 'proof.json');
const summaryPath = join(outputDir, 'proof-summary.json');
const deterministicCheckedAt = 'deterministic-proof-artifact';
const deterministicCommit = 'branch-head-validated-by-harness';
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
      'tests/read-model/production-release-public-docs-status.test.ts',
    ])
  );

  const contract = await assertBuiltContract();
  const packageExports = await assertPublicPackageExports();
  const documentation = await assertDocumentationProof();
  const proof = {
    schemaVersion: 1,
    checkedAt: deterministicCheckedAt,
    commit: deterministicCommit,
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/production-release-public-docs-status.ts',
      values: 'packages/schema-domain/src/production-release-public-docs-status-values.ts',
      readModel: 'packages/schema-domain/src/production-release-public-docs-status-read-model.ts',
      contractTest: 'packages/schema-domain/tests/read-model/production-release-public-docs-status.test.ts',
      packageExports,
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
    commit: proof.commit,
    proofMode,
    packageExports,
    rowCount: proof.rows.length,
    rows: proof.rows.map((row) => row.document),
    output: relativePath(proofPath),
    knownGaps: proof.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertBuiltContract() {
  const contractModulePath = pathToFileURL(
    join(repoRoot, 'packages', 'schema-domain', 'dist', 'production-release-public-docs-status.js')
  );
  const readModelPath = pathToFileURL(
    join(repoRoot, 'packages', 'schema-domain', 'dist', 'production-release-public-docs-status-read-model.js')
  );
  const contractModule = await import(contractModulePath.href);
  const readModelModule = await import(readModelPath.href);
  const proof = contractModule.ProductionReleasePublicDocsStatusProofSchema.parse(
    readModelModule.ProductionReleasePublicDocsStatusReadModel
  );

  assert.deepEqual(contractModule.summarizeProductionReleasePublicDocsStatusRows(proof.rows), {
    'privacy-policy': 1,
    'retention-policy': 1,
    'export-delete-process': 1,
    'support-runbook': 1,
    'incident-status-disclosure': 1,
    'legal-disclosure': 1,
  });
  assert.equal(proof.publicWebsitePublicationClaim, 'manual-required');
  assert.equal(proof.supportBackendUploadClaim, 'manual-required');
  assert.equal(proof.accountLookupExecutionClaim, 'manual-required');
  assert.equal(proof.billingProviderContactClaim, 'manual-required');
  assert.equal(proof.remoteSupportSessionClaim, 'not-implemented');
  assert.equal(proof.productionSlaClaim, 'not-implemented');
  assert.equal(proof.childActivityCustodyClaim, 'not-implemented');

  return {
    rows: proof.rows.map((row) => ({
      document: row.document,
      sourceDocumentState: row.sourceDocumentState,
      publicPublicationState: row.publicPublicationState,
      publicRouteState: row.publicRouteState,
      sourceProof: row.sourceProof,
      disclosureAudience: row.disclosureAudience,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionReleasePublicDocsStatusKnownGaps,
  };
}

async function assertPublicPackageExports() {
  const contractModule = await import('@ocentra-parent/schema-domain/production-release-public-docs-status');
  const readModelModule =
    await import('@ocentra-parent/schema-domain/production-release-public-docs-status-read-model');
  const valuesModule = await import('@ocentra-parent/schema-domain/production-release-public-docs-status-values');

  assert.equal(typeof contractModule.decodeProductionReleasePublicDocsStatusProof, 'function');
  assert.ok(contractModule.ProductionReleasePublicDocsStatusProofSchema);
  assert.ok(readModelModule.ProductionReleasePublicDocsStatusReadModel);
  assert.deepEqual(valuesModule.RequiredPublicDocsStatusDocuments, [
    'privacy-policy',
    'retention-policy',
    'export-delete-process',
    'support-runbook',
    'incident-status-disclosure',
    'legal-disclosure',
  ]);

  return [
    '@ocentra-parent/schema-domain/production-release-public-docs-status',
    '@ocentra-parent/schema-domain/production-release-public-docs-status-read-model',
    '@ocentra-parent/schema-domain/production-release-public-docs-status-values',
  ];
}

async function assertDocumentationProof() {
  const docs = [
    'docs/features/production-distribution-support.md',
    'docs/expectations/documentation.md',
    'docs/expectations/data-custody.md',
    'docs/expectations/release-installer.md',
  ];
  for (const path of docs) {
    assertIncludes(await readRepoFile(path), proofMode, `${path} proof note`);
  }
  return docs;
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
