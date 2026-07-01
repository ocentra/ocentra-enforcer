import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const resultDir = join(repoRoot, 'test-results', 'production-support-backend-upload-status-proof');
const outputDir = join(repoRoot, 'output', 'production-support-backend-upload-status-proof');
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
      'tests/unit/support-backend-upload-status.test.ts',
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
    proofMode: 'production-support-backend-upload-status-proof',
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/support-backend-upload-status.ts',
      guards: 'packages/schema-domain/src/support-backend-upload-status-guards.ts',
      readModel: 'packages/schema-domain/src/support-backend-upload-status-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/support-backend-upload-status.test.ts',
      proofOutput: relative(repoRoot, proofPath),
      summaryOutput: relative(repoRoot, summaryPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      expectations: [
        'docs/expectations/data-custody.md',
        'docs/expectations/release-installer.md',
        'docs/expectations/documentation.md',
      ],
    },
    claimsProved: [
      'Support backend upload status rows are parent-initiated and parent-consented before any queued, running, succeeded, failed, manual-required, backend-unavailable, or provider-unavailable state is accepted.',
      'Each status row is redaction-backed and audit-backed with support-safe status, support-bundle, retry, abandon, failure, manual-proof, and release package/runtime references only.',
      'Failed status rows prove retry exhaustion and parent/operator abandon references; backend/provider unavailable rows prove retry-queued behavior; manual-required rows require support backend implementation and operator runbook proof.',
      'Package exports expose the support backend upload status contract and read-model to consumers through @ocentra-parent/schema-domain.',
      'Rows reject tokens, raw child activity, raw URLs, screenshots, journals, SQLite snapshots, private paths, command lines, keystrokes, clipboard data, message contents, provider secrets, remote support transcripts, real backend execution, account lookup, billing provider execution, and default Ocentra-hosted family data.',
    ],
    claimsNotProved: [
      'raw child activity custody',
      'provider secrets',
      'remote support transcripts',
      'real support backend upload execution',
      'account lookup execution',
      'billing provider execution',
      'default Ocentra-hosted family data',
      'production SLA',
    ],
    readModel,
  };

  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit,
    proofMode: proof.proofMode,
    statesCovered: readModel.entries.map((entry) => entry.uploadStatus),
    output: relative(repoRoot, proofPath),
    claimsNotProved: proof.claimsNotProved,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`production-support-backend-upload-status-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function parseReadModel() {
  const modulePath = join(repoRoot, 'packages', 'logging-domain', 'dist', 'support-backend-upload-status.js');
  const readModelPath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'support-backend-upload-status-read-model.js'
  );
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.SupportBackendUploadStatusReadModelSchema.parse(readModelModule.SupportBackendUploadStatusReadModel);
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, 'support-backend-upload-status-proof');
  assert.equal(readModel.entries.length, 7);
  const states = new Set(readModel.entries.map((entry) => entry.uploadStatus));
  for (const state of [
    'upload-queued',
    'upload-running',
    'upload-succeeded',
    'upload-failed',
    'upload-manual-required',
    'backend-unavailable',
    'provider-unavailable',
  ]) {
    assert.equal(states.has(state), true);
  }

  const failed = entryFor(readModel, 'support-upload-status-failed-abandoned');
  const manual = entryFor(readModel, 'support-upload-status-manual-required');
  const backendUnavailable = entryFor(readModel, 'support-upload-status-backend-unavailable');
  const providerUnavailable = entryFor(readModel, 'support-upload-status-provider-unavailable');
  assert.equal(failed.retryState, 'retry-exhausted');
  assert.equal(failed.abandonState, 'abandoned');
  assert.equal(manual.retryState, 'manual-required');
  assert.equal(backendUnavailable.backendAvailabilityState, 'unavailable');
  assert.equal(providerUnavailable.providerAvailabilityState, 'unavailable');
}

async function assertPackageExports() {
  const contract = await import('@ocentra-parent/schema-domain/support-backend-upload-status');
  const readModel = await import('@ocentra-parent/schema-domain/support-backend-upload-status-read-model');
  assert.equal(typeof contract.SupportBackendUploadStatusReadModelSchema.parse, 'function');
  assert.equal(readModel.SupportBackendUploadStatusReadModel.entries.length, 7);
}

function entryFor(readModel, uploadId) {
  const entry = readModel.entries.find((candidate) => candidate.uploadId === uploadId);
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

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
