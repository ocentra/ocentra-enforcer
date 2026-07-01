import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-release-public-runtime-handoff-proof';
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
      'tests/proof/production-release-public-runtime-handoff.test.ts',
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
      contract: 'packages/schema-domain/src/production-release-public-runtime-handoff.ts',
      values: 'packages/schema-domain/src/production-release-public-runtime-handoff-values.ts',
      readModel: 'packages/schema-domain/src/production-release-public-runtime-handoff-read-model.ts',
      contractTest: 'packages/schema-domain/tests/proof/production-release-public-runtime-handoff.test.ts',
      packageExports,
      documentation,
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
    },
    handoffRows: contract.handoffRows,
    adapterRows: contract.adapterRows,
    nonClaims: contract.nonClaims,
    knownGaps: contract.knownGaps,
  };
  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    proofMode,
    packageExports,
    handoffRowCount: proof.handoffRows.length,
    adapterRowCount: proof.adapterRows.length,
    surfaces: proof.handoffRows.map((row) => row.surface),
    adapters: proof.adapterRows.map((row) => row.adapter),
    output: relativePath(proofPath),
    knownGaps: proof.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertBuiltContract() {
  const contractModulePath = pathToFileURL(
    join(repoRoot, 'packages', 'schema-domain', 'dist', 'production-release-public-runtime-handoff.js')
  );
  const readModelPath = pathToFileURL(
    join(repoRoot, 'packages', 'schema-domain', 'dist', 'production-release-public-runtime-handoff-read-model.js')
  );
  const contractModule = await import(contractModulePath.href);
  const readModelModule = await import(readModelPath.href);
  const proof = contractModule.ProductionReleasePublicRuntimeHandoffProofSchema.parse(
    readModelModule.ProductionReleasePublicRuntimeHandoffReadModel
  );

  assert.deepEqual(contractModule.summarizeProductionReleasePublicRuntimeHandoffs(proof.handoffRows), {
    'public-download': 1,
    'release-status': 1,
    'update-status': 1,
    'account-status': 1,
    'subscription-status': 1,
    'support-status': 1,
  });
  assert.deepEqual(contractModule.summarizeProductionReleasePublicRuntimeAdapters(proof.adapterRows), {
    'public-website-runtime': 1,
    'download-status-backend': 1,
    'release-publishing-pipeline': 1,
    'updater-status-runtime': 1,
    'account-backend': 1,
    'billing-provider-runtime': 1,
    'support-backend-upload': 1,
  });
  assert.equal(proof.publicWebsiteRuntimeClaim, 'not-implemented');
  assert.equal(proof.accountBackendRuntimeClaim, 'backend-required');
  assert.equal(proof.billingProviderRuntimeClaim, 'not-implemented');
  assert.equal(proof.supportBackendUploadClaim, 'manual-required');
  assert.equal(proof.productionPublishingState, 'production-promotion-required');
  assert.equal(proof.signingStoreProofState, 'manual-required');
  assert.equal(proof.updaterExecutionState, 'manual-required');
  assert.equal(proof.childActivityCustodyClaim, 'not-implemented');

  return {
    handoffRows: proof.handoffRows.map((row) => ({
      surface: row.surface,
      handoffTarget: row.handoffTarget,
      routeState: row.routeState,
      runtimeAdapterState: row.runtimeAdapterState,
      backendAdapterState: row.backendAdapterState,
      sourceProof: row.sourceProof,
    })),
    adapterRows: proof.adapterRows.map((row) => ({
      adapter: row.adapter,
      adapterState: row.adapterState,
      executionClaim: row.executionClaim,
    })),
    nonClaims: proof.nonClaims,
    knownGaps: readModelModule.ProductionReleasePublicRuntimeHandoffKnownGaps,
  };
}

async function assertPublicPackageExports() {
  const contractModule = await import('@ocentra-parent/schema-domain/production-release-public-runtime-handoff');
  const readModelModule =
    await import('@ocentra-parent/schema-domain/production-release-public-runtime-handoff-read-model');
  const valuesModule = await import('@ocentra-parent/schema-domain/production-release-public-runtime-handoff-values');

  assert.equal(typeof contractModule.decodeProductionReleasePublicRuntimeHandoffProof, 'function');
  assert.ok(contractModule.ProductionReleasePublicRuntimeHandoffProofSchema);
  assert.ok(readModelModule.ProductionReleasePublicRuntimeHandoffReadModel);
  assert.deepEqual(valuesModule.RequiredPublicRuntimeSurfaces, [
    'public-download',
    'release-status',
    'update-status',
    'account-status',
    'subscription-status',
    'support-status',
  ]);

  return [
    '@ocentra-parent/schema-domain/production-release-public-runtime-handoff',
    '@ocentra-parent/schema-domain/production-release-public-runtime-handoff-read-model',
    '@ocentra-parent/schema-domain/production-release-public-runtime-handoff-values',
  ];
}

async function assertDocumentationProof() {
  const docs = [
    'docs/features/production-distribution-support.md',
    'docs/expectations/release-installer.md',
    'docs/expectations/platform-deliverables.md',
    'docs/expectations/cloud.md',
    'docs/expectations/billing.md',
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
