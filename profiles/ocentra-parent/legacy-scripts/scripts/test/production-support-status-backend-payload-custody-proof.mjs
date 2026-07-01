import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-status-backend-payload-custody-proof';
const resultDir = join(repoRoot, 'test-results', 'production-support-status-backend-payload-custody-proof');
const outputDir = join(repoRoot, 'output', 'production-support-status-backend-payload-custody-proof');
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
      'tests/unit/status-backend-payload-custody.test.ts',
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
      contract: 'packages/schema-domain/src/status-backend-payload-custody.ts',
      guards: 'packages/schema-domain/src/status-backend-payload-custody-guards.ts',
      readModel: 'packages/schema-domain/src/status-backend-payload-custody-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/status-backend-payload-custody.test.ts',
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      expectations: ['docs/expectations/data-custody.md', 'docs/expectations/release-installer.md'],
    },
    claimsProved: [
      'Status backend payload custody rows are parent-consented and redaction-backed before custody, retention, delete, audit export, or backend-unavailable states are accepted.',
      'Rows link to status backend target, execution queue, queue audit, custody, retention, delete, redaction, and manual proof references while keeping payloads to support-safe status refs only.',
      'Retention and deletion remain manual-required until published retention, deletion, and support-safe audit export proof exists.',
      'Backend-unavailable rows prove the fallback status remains not-retained and manual-required rather than claiming durable status backend payload custody.',
      'Package exports expose the status backend payload custody contract and read model through @ocentra-parent/schema-domain.',
      'Rows reject tokens, raw child activity, raw support bundles, provider secrets, account lookup results, billing contact records, backend upload payloads, status backend payloads, public runtime payloads, remote support transcripts, status backend execution, durable payload storage, payload deletion execution, retry worker execution, audit persistence, public runtime execution, support upload execution, provider execution, account lookup execution, billing provider contact, remote support sessions, production SLA, and default Ocentra-hosted family data.',
    ],
    claimsNotProved: [
      'real status backend execution',
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
    statesCovered: readModel.entries.map((entry) => entry.custodyState),
    output: relativePath(proofPath),
    claimsNotProved: proof.claimsNotProved,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function parseReadModel() {
  const modulePath = join(repoRoot, 'packages', 'logging-domain', 'dist', 'status-backend-payload-custody.js');
  const readModelPath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'status-backend-payload-custody-read-model.js'
  );
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.StatusBackendPayloadCustodyReadModelSchema.parse(readModelModule.StatusBackendPayloadCustodyReadModel);
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, 'production-support-status-backend-payload-custody-proof');
  assert.equal(readModel.entries.length, 6);
  const states = new Set(readModel.entries.map((entry) => entry.custodyState));
  for (const state of [
    'custody-boundary-recorded',
    'retention-manual-required',
    'delete-request-recorded',
    'deletion-manual-required',
    'audit-export-ready',
    'backend-unavailable',
  ]) {
    assert.equal(states.has(state), true);
  }

  const unavailable = entryFor(readModel, 'status-backend-payload-backend-unavailable');
  assert.equal(unavailable.storageState, 'not-retained');
  assert.equal(unavailable.realStatusBackendExecution, false);
  assert.equal(unavailable.durableStatusBackendPayloadStorage, false);
  assert.equal(unavailable.statusBackendPayloadDeletionExecuted, false);
}

async function assertPackageExports() {
  const contract = await import('@ocentra-parent/schema-domain/status-backend-payload-custody');
  const readModel = await import('@ocentra-parent/schema-domain/status-backend-payload-custody-read-model');
  assert.equal(typeof contract.StatusBackendPayloadCustodyReadModelSchema.parse, 'function');
  assert.equal(readModel.StatusBackendPayloadCustodyReadModel.entries.length, 6);
}

function entryFor(readModel, custodyId) {
  const entry = readModel.entries.find((candidate) => candidate.custodyId === custodyId);
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

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
