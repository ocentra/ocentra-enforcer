import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-provider-secret-custody-status-proof';
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
      'tests/unit/provider-secret-custody-status.test.ts',
    ])
  );

  const commit = await gitHead();
  const readModel = await parseReadModel();
  assertReadModel(readModel);
  await assertPackageExports();

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit,
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/provider-secret-custody-status.ts',
      readModel: 'packages/schema-domain/src/provider-secret-custody-status-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/provider-secret-custody-status.test.ts',
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      expectations: [
        'docs/expectations/release-installer.md',
        'docs/expectations/data-custody.md',
        'docs/expectations/billing.md',
      ],
      packageExports: [
        '@ocentra-parent/schema-domain/provider-secret-custody-status',
        '@ocentra-parent/schema-domain/provider-secret-custody-status-read-model',
      ],
    },
    claimsProved: [
      'Provider-secret custody status rows cover boundary recorded, provider-secret absent, backend secret store manual-required, rotation manual-required, revocation manual-required, and support-safe audit export states.',
      'Rows link to legal/provider readiness, billing support status, redaction, audit, and data-custody refs while disclosing only support-safe status metadata.',
      'Provider-secret custody, backend secret store, rotation, and revocation remain not implemented or manual-required until real provider secret custody proof exists.',
      'Package exports expose the provider-secret custody status contract and read model through @ocentra-parent/schema-domain.',
      'Rows reject provider secrets, payment provider tokens, raw child activity, raw support bundle payloads, account lookup results, billing provider contact records, remote support transcripts, provider custody execution, backend secret store implementation, rotation/revocation execution, support backend upload execution, account lookup execution, billing provider contact execution, remote support sessions, production SLA, and default Ocentra-hosted family data.',
    ],
    claimsNotProved: [
      'provider secret custody execution',
      'backend secret store implementation',
      'provider secret rotation execution',
      'provider secret revocation execution',
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
    statesCovered: readModel.entries.map((entry) => entry.custodyStatus),
    output: relativePath(proofPath),
    claimsNotProved: proof.claimsNotProved,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)}`);
}

async function parseReadModel() {
  const modulePath = join(repoRoot, 'packages', 'logging-domain', 'dist', 'provider-secret-custody-status.js');
  const readModelPath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'provider-secret-custody-status-read-model.js'
  );
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.ProviderSecretCustodyStatusReadModelSchema.parse(readModelModule.ProviderSecretCustodyStatusReadModel);
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, proofMode);
  assert.equal(readModel.entries.length, 6);
  const states = new Set(readModel.entries.map((entry) => entry.custodyStatus));
  for (const state of [
    'custody-boundary-recorded',
    'provider-secret-absent',
    'backend-secret-store-manual-required',
    'rotation-manual-required',
    'revocation-manual-required',
    'audit-export-ready',
  ]) {
    assert.equal(states.has(state), true);
  }

  const absent = entryFor(readModel, 'provider-secret-absent-from-support-status');
  assert.equal(absent.containsProviderSecrets, false);
  assert.equal(absent.providerSecretCustodyState, 'not-implemented');
  assert.equal(absent.backendSecretStoreState, 'not-applicable');

  const auditExport = entryFor(readModel, 'provider-secret-custody-audit-export-ready');
  assert.equal(auditExport.rotationExecuted, false);
  assert.equal(auditExport.revocationExecuted, false);
  assert.equal(auditExport.supportBackendUploadExecuted, false);
}

async function assertPackageExports() {
  const contract = await import('@ocentra-parent/schema-domain/provider-secret-custody-status');
  const readModel = await import('@ocentra-parent/schema-domain/provider-secret-custody-status-read-model');
  assert.equal(typeof contract.ProviderSecretCustodyStatusReadModelSchema.parse, 'function');
  assert.equal(readModel.ProviderSecretCustodyStatusReadModel.entries.length, 6);
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

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
