import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-release-public-surface-publication-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const outputProofDir = join(repoRoot, 'output', proofMode);
const proofPath = join(outputDir, 'proof.json');
const summaryPath = join(outputProofDir, 'proof-summary.json');
const publicHost = 'family.ocentra.ca';
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(outputProofDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/schema-domain',
      '--',
      'tests/proof/production-release-public-status-proof.test.ts',
    ])
  );
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

  const status = await assertPublicStatusProof();
  const runtime = await assertRuntimeHandoffProof();
  const docs = await assertPublicDocsProof();
  const documentation = await assertDocumentationProof();
  const commit = await gitHead();
  const publicationRows = publicationReadinessRows(status, runtime, docs);
  const knownGaps = [
    `${publicHost} public website runtime is not implemented.`,
    'Download, release, update, account, subscription, support, and public-doc publication rows require runtime or manual publication proof.',
    'Account backend, billing provider runtime, support backend upload, production publishing, signing/store proof, updater execution, legal execution, and production SLA remain unclaimed.',
    'No child activity custody, raw support bundle payloads, provider secrets, account lookup results, billing provider contact records, remote support transcripts, or parent rules are included.',
  ];
  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit,
    proofMode,
    publicHost,
    commands,
    evidence: {
      statusContract: 'packages/schema-domain/src/production-release-public-status-proof.ts',
      runtimeHandoffContract: 'packages/schema-domain/src/production-release-public-runtime-handoff.ts',
      docsStatusContract: 'packages/schema-domain/src/production-release-public-docs-status.ts',
      documentation,
      output: relativePath(proofPath),
      summary: relativePath(summaryPath),
    },
    publicationRows,
    nonClaims: unique([...status.nonClaims, ...runtime.nonClaims, ...docs.nonClaims]),
    knownGaps,
  };
  const summary = {
    proofMode,
    publicHost,
    commit,
    statusSurfaceCount: status.surfaces.length,
    runtimeHandoffCount: runtime.handoffRows.length,
    runtimeAdapterCount: runtime.adapterRows.length,
    publicDocumentCount: docs.rows.length,
    knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertPublicStatusProof() {
  const proofModulePath = pathToFileURL(
    join(repoRoot, 'packages', 'schema-domain', 'dist', 'production-release-public-status-proof.js')
  );
  const proofModule = await import(proofModulePath.href);
  const proof = proofModule.ProductionReleasePublicStatusProofSchema.parse(
    proofModule.ProductionReleasePublicStatusProofReadModel
  );

  assert.deepEqual(proofModule.summarizeProductionReleasePublicStatusSurfaces(proof.surfaces), {
    'public-download': 1,
    'release-status': 1,
    'update-status': 1,
    'account-status': 1,
    'subscription-status': 1,
    'support-status': 1,
  });
  assert.equal(proof.publicHostState, 'not-implemented');
  assert.equal(proof.productionPublishingState, 'production-promotion-required');
  assert.equal(proof.publicSupportRuntimeClaim, 'not-implemented');
  assert.equal(proof.childActivityCustodyClaim, 'not-implemented');
  assertPublicStatusRowsRemainUnpublished(proof.surfaces);

  return proof;
}

async function assertRuntimeHandoffProof() {
  const contractModule = await import('@ocentra-parent/schema-domain/production-release-public-runtime-handoff');
  const readModelModule =
    await import('@ocentra-parent/schema-domain/production-release-public-runtime-handoff-read-model');
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
  assertRuntimeRowsRemainUnexecuted(proof.handoffRows, proof.adapterRows);

  return proof;
}

async function assertPublicDocsProof() {
  const contractModule = await import('@ocentra-parent/schema-domain/production-release-public-docs-status');
  const readModelModule =
    await import('@ocentra-parent/schema-domain/production-release-public-docs-status-read-model');
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
  assertPublicDocsRowsRemainUnpublished(proof.rows);

  return proof;
}

function assertPublicStatusRowsRemainUnpublished(rows) {
  for (const row of rows) {
    assert.notEqual(row.backendRuntimeState, 'implemented', `${row.surface} backend must not be implemented`);
    assert.notEqual(row.parentVisibleState, 'implemented', `${row.surface} visible state must not be implemented`);
    assertForbiddenDataExcluded(row.allowedDataClasses, row.forbiddenDataClasses, row.surface);
  }
}

function assertRuntimeRowsRemainUnexecuted(handoffRows, adapterRows) {
  for (const row of handoffRows) {
    assert.notEqual(row.routeState, 'implemented', `${row.surface} route must not be implemented`);
    assert.notEqual(row.runtimeAdapterState, 'implemented', `${row.surface} runtime must not be implemented`);
    assert.notEqual(row.backendAdapterState, 'implemented', `${row.surface} backend must not be implemented`);
    assertForbiddenDataExcluded(row.supportSafeDataClasses, row.forbiddenDataClasses, row.surface);
  }
  for (const row of adapterRows) {
    assert.notEqual(row.adapterState, 'implemented', `${row.adapter} adapter must not be implemented`);
    assert.notEqual(row.executionClaim, 'executed', `${row.adapter} must not be executed`);
    assert.equal(row.providerSecretCustody, 'not-present');
    assert.equal(row.childActivityCustody, 'not-included');
  }
}

function assertPublicDocsRowsRemainUnpublished(rows) {
  for (const row of rows) {
    assert.equal(row.sourceDocumentState, 'source-contract-ready');
    assert.equal(row.publicPublicationState, 'manual-required');
    assert.equal(row.publicRouteState, 'not-implemented');
    assertForbiddenDataExcluded(row.supportSafeDataClasses, row.forbiddenDataClasses, row.document);
  }
}

function assertForbiddenDataExcluded(allowedDataClasses, forbiddenDataClasses, label) {
  for (const dataClass of forbiddenDataClasses) {
    assert(!allowedDataClasses.includes(dataClass), `${label} unexpectedly allows ${dataClass}`);
  }
}

function publicationReadinessRows(status, runtime, docs) {
  return [
    {
      surface: 'family-public-site',
      host: publicHost,
      publicationState: status.publicHostState,
      runtimeState: runtime.publicWebsiteRuntimeClaim,
      sourceProofs: [
        'production-release-public-status-proof',
        'production-release-public-runtime-handoff-proof',
        'production-release-public-docs-status-proof',
      ],
    },
    ...runtime.handoffRows.map((row) => ({
      surface: row.surface,
      host: publicHost,
      publicationState: row.routeState,
      runtimeState: row.runtimeAdapterState,
      backendState: row.backendAdapterState,
      sourceProof: row.sourceProof,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
    ...docs.rows.map((row) => ({
      surface: row.document,
      host: publicHost,
      publicationState: row.publicPublicationState,
      runtimeState: row.publicRouteState,
      sourceProof: row.sourceProof,
      supportSafeDataClasses: row.supportSafeDataClasses,
    })),
  ];
}

async function assertDocumentationProof() {
  const docs = [
    'docs/features/production-distribution-support.md',
    'docs/expectations/release-installer.md',
    'docs/expectations/documentation.md',
    'docs/expectations/data-custody.md',
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

function unique(values) {
  return [...new Set(values)];
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
