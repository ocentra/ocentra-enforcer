import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const resultDir = join(repoRoot, 'test-results', 'production-support-backend-upload-custody-audit-proof');
const outputDir = join(repoRoot, 'output', 'production-support-backend-upload-custody-audit-proof');
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
      'tests/unit/support-backend-upload-custody-audit.test.ts',
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
    proofMode: 'production-support-backend-upload-custody-audit-proof',
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/support-backend-upload-custody-audit.ts',
      guards: 'packages/schema-domain/src/support-backend-upload-custody-audit-guards.ts',
      readModel: 'packages/schema-domain/src/support-backend-upload-custody-audit-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/support-backend-upload-custody-audit.test.ts',
      proofOutput: relative(repoRoot, proofPath),
      summaryOutput: relative(repoRoot, summaryPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      expectations: ['docs/expectations/data-custody.md', 'docs/expectations/release-installer.md'],
    },
    claimsProved: [
      'Support backend upload custody audit rows are parent-initiated and parent-consented before custody, retention, delete, or audit export states are accepted.',
      'Rows link to the prior support backend upload status proof and execution/runtime proof while keeping payloads to redacted audit refs, custody refs, retention refs, delete refs, and manual proof refs.',
      'Retention and deletion remain manual-required until published retention, deletion, and support-safe audit export proof exists.',
      'Support-safe custody audit export rows are source-contract proof only and do not claim backend payload retention or deletion execution.',
      'Package exports expose the custody audit contract and read model through @ocentra-parent/schema-domain.',
      'Rows reject tokens, raw child activity, raw URLs, screenshots, journals, SQLite snapshots, private paths, command lines, keystrokes, clipboard data, message contents, provider secrets, remote support transcripts, real backend execution, backend payload retention, backend payload deletion, account lookup, billing provider contact, remote support sessions, production SLA, and default Ocentra-hosted family data.',
    ],
    claimsNotProved: [
      'real support backend upload execution',
      'support backend payload retention',
      'support backend payload deletion',
      'raw child activity custody',
      'provider secrets',
      'remote support transcripts',
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
    statesCovered: readModel.entries.map((entry) => entry.auditState),
    output: relative(repoRoot, proofPath),
    claimsNotProved: proof.claimsNotProved,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`production-support-backend-upload-custody-audit-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function parseReadModel() {
  const modulePath = join(repoRoot, 'packages', 'logging-domain', 'dist', 'support-backend-upload-custody-audit.js');
  const readModelPath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'support-backend-upload-custody-audit-read-model.js'
  );
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.SupportBackendUploadCustodyAuditReadModelSchema.parse(
    readModelModule.SupportBackendUploadCustodyAuditReadModel
  );
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, 'production-support-backend-upload-custody-audit-proof');
  assert.equal(readModel.entries.length, 5);
  const states = new Set(readModel.entries.map((entry) => entry.auditState));
  for (const state of [
    'custody-boundary-recorded',
    'retention-manual-required',
    'delete-request-recorded',
    'deletion-manual-required',
    'audit-export-ready',
  ]) {
    assert.equal(states.has(state), true);
  }

  const auditExport = entryFor(readModel, 'support-upload-custody-audit-export-ready');
  assert.equal(auditExport.auditExportState, 'support-safe-export-ready');
  assert.equal(auditExport.realSupportBackendUploadExecuted, false);
  assert.equal(auditExport.supportBackendRetainedPayload, false);
  assert.equal(auditExport.supportBackendDeletedPayload, false);
}

async function assertPackageExports() {
  const contract = await import('@ocentra-parent/schema-domain/support-backend-upload-custody-audit');
  const readModel = await import('@ocentra-parent/schema-domain/support-backend-upload-custody-audit-read-model');
  assert.equal(typeof contract.SupportBackendUploadCustodyAuditReadModelSchema.parse, 'function');
  assert.equal(readModel.SupportBackendUploadCustodyAuditReadModel.entries.length, 5);
}

function entryFor(readModel, auditId) {
  const entry = readModel.entries.find((candidate) => candidate.auditId === auditId);
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
