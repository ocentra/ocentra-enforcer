import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-provider-secret-rotation-revocation-status-proof';
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
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/logging-domain']));
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/logging-domain',
      '--',
      'tests/unit/provider-secret-rotation-revocation-status.test.ts',
    ])
  );

  const readModel = await parseReadModel();
  assertReadModel(readModel);
  await assertPackageExports();
  const checkedAt = readModel.generatedAt;
  const commit = await proofInputDigest(readModel);

  const proof = {
    schemaVersion: 1,
    checkedAt,
    commit,
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/provider-secret-rotation-revocation-status.ts',
      readModel: 'packages/schema-domain/src/provider-secret-rotation-revocation-status-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/provider-secret-rotation-revocation-status.test.ts',
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      checklist: 'docs/product-capability-checklist.md',
      expectations: ['docs/expectations/data-custody.md', 'docs/expectations/static-analysis-security.md'],
      packageExports: [
        '@ocentra-parent/schema-domain/provider-secret-rotation-revocation-status',
        '@ocentra-parent/schema-domain/provider-secret-rotation-revocation-status-read-model',
      ],
    },
    claimsProved: [
      'Provider-secret rotation/revocation status rows cover rotation requested, rotation preflight-ready, rotation manual-required, revocation requested, revocation preflight-ready, revocation manual-required, and audit-export-ready states.',
      'Rows link to provider-secret custody status, provider-secret execution readiness, backend secret-store preflight, operator approval, manual proof, and audit refs while disclosing only support-safe status metadata.',
      'Backend secret-store, rotation, revocation, and provider-secret delivery remain not implemented or manual-required until real provider custody and execution proof exists.',
      'Package exports expose the provider-secret rotation/revocation status contract and read model through @ocentra-parent/schema-domain.',
      'Rows reject provider secrets, payment provider tokens, raw child activity, raw support bundle payloads, account lookup results, billing provider contact records, remote support transcripts, backend secret-store execution, rotation execution, revocation execution, provider-secret delivery, support backend upload execution, account lookup execution, billing provider contact execution, remote support sessions, production SLA, and default Ocentra-hosted family data.',
    ],
    claimsNotProved: [
      'backend secret store execution',
      'provider secret rotation execution',
      'provider secret revocation execution',
      'provider secret delivery',
      'support backend upload execution',
      'account lookup execution',
      'billing provider contact execution',
      'remote support session execution',
      'production SLA',
      'default Ocentra-hosted family data',
      'raw child activity custody',
    ],
    readModel,
  };

  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit,
    proofMode,
    statesCovered: readModel.entries.map((entry) => entry.rotationRevocationStatus),
    output: relativePath(proofPath),
    claimsNotProved: proof.claimsNotProved,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)}`);
}

async function parseReadModel() {
  const modulePath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'provider-secret-rotation-revocation-status.js'
  );
  const readModelPath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'provider-secret-rotation-revocation-status-read-model.js'
  );
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.ProviderSecretRotationRevocationStatusReadModelSchema.parse(
    readModelModule.ProviderSecretRotationRevocationStatusReadModel
  );
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, proofMode);
  assert.equal(readModel.entries.length, 7);
  const states = new Set(readModel.entries.map((entry) => entry.rotationRevocationStatus));
  for (const state of [
    'rotation-requested',
    'rotation-preflight-ready',
    'rotation-manual-required',
    'revocation-requested',
    'revocation-preflight-ready',
    'revocation-manual-required',
    'audit-export-ready',
  ]) {
    assert.equal(states.has(state), true);
  }

  const rotation = entryFor(readModel, 'provider-secret-rotation-manual-required');
  assert.equal(rotation.rotationExecuted, false);
  assert.equal(rotation.operatorApprovalState, 'manual-required');

  const revocation = entryFor(readModel, 'provider-secret-revocation-manual-required');
  assert.equal(revocation.revocationExecuted, false);
  assert.equal(revocation.providerSecretDelivered, false);
}

async function assertPackageExports() {
  const contract = await import('@ocentra-parent/schema-domain/provider-secret-rotation-revocation-status');
  const readModel = await import('@ocentra-parent/schema-domain/provider-secret-rotation-revocation-status-read-model');
  assert.equal(typeof contract.ProviderSecretRotationRevocationStatusReadModelSchema.parse, 'function');
  assert.equal(readModel.ProviderSecretRotationRevocationStatusReadModel.entries.length, 7);
}

function entryFor(readModel, statusId) {
  const entry = readModel.entries.find((candidate) => candidate.statusId === statusId);
  assert.notEqual(entry, undefined);
  return entry;
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

async function proofInputDigest(readModel) {
  const hash = createHash('sha256');
  for (const path of [
    'packages/schema-domain/src/provider-secret-rotation-revocation-status.ts',
    'packages/schema-domain/src/provider-secret-rotation-revocation-status-guards.ts',
    'packages/schema-domain/src/provider-secret-rotation-revocation-status-read-model.ts',
    'packages/logging-domain/tests/unit/provider-secret-rotation-revocation-status.test.ts',
    'packages/logging-domain/package.json',
    'scripts/test/provider-secret-rotation-revocation-status-proof.mjs',
    'docs/features/production-distribution-support.md',
    'docs/expectations/data-custody.md',
    'docs/product-capability-checklist.md',
  ]) {
    hash.update(path);
    hash.update('\0');
    hash.update(await readFile(join(repoRoot, path)));
    hash.update('\0');
  }
  hash.update(JSON.stringify(readModel));
  return `proof-input-sha256:${hash.digest('hex')}`;
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
