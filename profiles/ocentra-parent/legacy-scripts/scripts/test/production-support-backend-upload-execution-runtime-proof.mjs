import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const resultDir = join(repoRoot, 'test-results', 'production-support-backend-upload-execution-runtime-proof');
const outputDir = join(repoRoot, 'output', 'production-support-backend-upload-execution-runtime-proof');
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
      'tests/unit/support-backend-upload-execution-runtime.test.ts',
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
    proofMode: 'production-support-backend-upload-execution-runtime-proof',
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/support-backend-upload-execution-runtime.ts',
      guards: 'packages/schema-domain/src/support-backend-upload-execution-runtime-guards.ts',
      readModel: 'packages/schema-domain/src/support-backend-upload-execution-runtime-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/support-backend-upload-execution-runtime.test.ts',
      proofOutput: relative(repoRoot, proofPath),
      summaryOutput: relative(repoRoot, summaryPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      expectations: [
        'docs/expectations/data-custody.md',
        'docs/expectations/release-installer.md',
        'docs/expectations/static-analysis-security.md',
      ],
    },
    claimsProved: [
      'Support backend upload execution runtime rows are parent-initiated and parent-consented before request, redaction preflight, manual dispatch, unavailable, retry, or abandon states are accepted.',
      'Execution runtime rows link to the prior support backend upload status proof through status refs while keeping payloads to redacted runtime refs, audit refs, redaction preflight refs, support-bundle manifest refs, retry refs, abandon refs, and manual proof refs.',
      'Manual dispatch rows require support backend upload adapter implementation plus operator runbook and retention/delete proof before execution can be claimed.',
      'Backend/provider unavailable rows prove retry-scheduled behavior, and operator-abandoned rows prove retry-exhausted and parent/operator abandon refs.',
      'Package exports expose the execution runtime contract and read-model to consumers through @ocentra-parent/schema-domain.',
      'Rows reject tokens, raw child activity, raw URLs, screenshots, journals, SQLite snapshots, private paths, command lines, keystrokes, clipboard data, message contents, provider secrets, remote support transcripts, real backend execution, account lookup, billing provider contact, remote support sessions, production SLA, and default Ocentra-hosted family data.',
    ],
    claimsNotProved: [
      'raw child activity custody',
      'provider secrets',
      'remote support transcripts',
      'real support backend upload execution',
      'account lookup execution',
      'billing provider contact execution',
      'remote support session execution',
      'production SLA',
      'default Ocentra-hosted family data',
    ],
    readModel,
  };

  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit,
    proofMode: proof.proofMode,
    statesCovered: readModel.entries.map((entry) => entry.runtimeState),
    output: relative(repoRoot, proofPath),
    claimsNotProved: proof.claimsNotProved,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`production-support-backend-upload-execution-runtime-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function parseReadModel() {
  const modulePath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'support-backend-upload-execution-runtime.js'
  );
  const readModelPath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'support-backend-upload-execution-runtime-read-model.js'
  );
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.SupportBackendUploadExecutionRuntimeReadModelSchema.parse(
    readModelModule.SupportBackendUploadExecutionRuntimeReadModel
  );
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, 'support-backend-upload-execution-runtime-proof');
  assert.equal(readModel.entries.length, 7);
  const states = new Set(readModel.entries.map((entry) => entry.runtimeState));
  for (const state of [
    'execution-request-recorded',
    'redaction-preflight-ready',
    'dispatch-manual-required',
    'backend-unavailable',
    'provider-unavailable',
    'retry-scheduled',
    'operator-abandoned',
  ]) {
    assert.equal(states.has(state), true);
  }

  const manual = entryFor(readModel, 'support-upload-dispatch-manual-required');
  const abandoned = entryFor(readModel, 'support-upload-execution-operator-abandoned');
  assert.equal(manual.retryState, 'manual-required');
  assert.equal(manual.realSupportBackendUploadExecuted, false);
  assert.equal(abandoned.retryState, 'retry-exhausted');
  assert.equal(abandoned.abandonState, 'abandoned');
}

async function assertPackageExports() {
  const contract = await import('@ocentra-parent/schema-domain/support-backend-upload-execution-runtime');
  const readModel = await import('@ocentra-parent/schema-domain/support-backend-upload-execution-runtime-read-model');
  assert.equal(typeof contract.SupportBackendUploadExecutionRuntimeReadModelSchema.parse, 'function');
  assert.equal(readModel.SupportBackendUploadExecutionRuntimeReadModel.entries.length, 7);
}

function entryFor(readModel, runtimeId) {
  const entry = readModel.entries.find((candidate) => candidate.runtimeId === runtimeId);
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
