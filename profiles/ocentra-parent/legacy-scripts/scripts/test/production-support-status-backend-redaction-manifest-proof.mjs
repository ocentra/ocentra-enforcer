import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-status-backend-redaction-manifest-proof';
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
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/logging-domain']));
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/logging-domain',
      '--',
      'tests/unit/status-backend-redaction-manifest.test.ts',
    ])
  );

  const readModel = await parseReadModel();
  assertReadModel(readModel);
  await assertPackageExports();

  const proof = {
    schemaVersion: 1,
    checkedAt: deterministicCheckedAt,
    commit: deterministicCommit,
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/status-backend-redaction-manifest.ts',
      guards: 'packages/schema-domain/src/status-backend-redaction-manifest-guards.ts',
      readModel: 'packages/schema-domain/src/status-backend-redaction-manifest-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/status-backend-redaction-manifest.test.ts',
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      expectations: ['docs/expectations/data-custody.md', 'docs/expectations/release-installer.md'],
    },
    claimsProved: [
      'Status backend redaction manifest rows are parent-consented and support-safe before redaction-ready, manual-required, review, failure, or backend-unavailable states are accepted.',
      'Rows link to status backend target, execution queue, queue/audit persistence, redaction manifest, redaction summary, redaction review, failure, and manual proof references while keeping payloads to redacted status refs only.',
      'Redaction review queued, running, failed, and backend-unavailable rows remain manual-required until real status backend execution and support-safe manifest review proof exists.',
      'Package exports expose the status backend redaction manifest contract and read model through @ocentra-parent/schema-domain.',
      'Rows reject tokens, raw child activity, raw support bundles, provider secrets, account lookup results, billing contact records, backend upload payloads, status backend payloads, public runtime payloads, remote support transcripts, status backend execution, status backend payload custody, durable payload storage, payload deletion, retry worker execution, audit persistence execution, public runtime execution, support upload execution, provider execution, account lookup execution, billing provider contact, remote support sessions, production SLA, and default Ocentra-hosted family data.',
    ],
    claimsNotProved: [
      'real status backend execution',
      'status backend payload custody',
      'durable status backend payload storage',
      'status backend payload deletion execution',
      'retry worker execution',
      'audit persistence execution',
      'public runtime execution',
      'support backend upload execution',
      'provider execution',
      'account lookup execution',
      'billing provider contact execution',
      'remote support session execution',
      'production SLA',
      'default Ocentra-hosted family data',
      'child activity custody',
    ],
    readModel,
  };

  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    proofMode: proof.proofMode,
    statesCovered: readModel.entries.map((entry) => entry.manifestState),
    output: relativePath(proofPath),
    claimsNotProved: proof.claimsNotProved,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)}:${relativePath(summaryPath)}`);
}

async function parseReadModel() {
  const modulePath = join(repoRoot, 'packages', 'logging-domain', 'dist', 'status-backend-redaction-manifest.js');
  const readModelPath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'status-backend-redaction-manifest-read-model.js'
  );
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.StatusBackendRedactionManifestReadModelSchema.parse(
    readModelModule.StatusBackendRedactionManifestReadModel
  );
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, proofMode);
  assert.equal(readModel.entries.length, 6);
  const states = new Set(readModel.entries.map((entry) => entry.manifestState));
  for (const state of [
    'redaction-manifest-ready',
    'redaction-manifest-manual-required',
    'redaction-review-queued',
    'redaction-review-running',
    'redaction-review-failed',
    'backend-unavailable',
  ]) {
    assert.equal(states.has(state), true);
  }

  const unavailable = entryFor(readModel, 'status-backend-redaction-backend-unavailable');
  assert.equal(unavailable.redactionManifestState, 'manual-required');
  assert.equal(unavailable.realStatusBackendExecution, false);
  assert.equal(unavailable.statusBackendPayloadCustodyClaimed, false);
  assert.equal(unavailable.auditPersistenceExecuted, false);
}

async function assertPackageExports() {
  const contract = await import('@ocentra-parent/schema-domain/status-backend-redaction-manifest');
  const readModel = await import('@ocentra-parent/schema-domain/status-backend-redaction-manifest-read-model');
  assert.equal(typeof contract.StatusBackendRedactionManifestReadModelSchema.parse, 'function');
  assert.equal(readModel.StatusBackendRedactionManifestReadModel.entries.length, 6);
}

function entryFor(readModel, manifestId) {
  const entry = readModel.entries.find((candidate) => candidate.manifestId === manifestId);
  assert.notEqual(entry, undefined);
  return entry;
}

function relativePath(targetPath) {
  return relative(repoRoot, targetPath).replaceAll('\\', '/');
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

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
