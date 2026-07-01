import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const resultDir = join(repoRoot, 'test-results', 'production-support-case-resolution-status-proof');
const outputDir = join(repoRoot, 'output', 'production-support-case-resolution-status-proof');
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
      'tests/unit/support-case-resolution-status.test.ts',
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
    proofMode: 'production-support-case-resolution-status-proof',
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/support-case-resolution-status.ts',
      guards: 'packages/schema-domain/src/support-case-resolution-status-guards.ts',
      readModel: 'packages/schema-domain/src/support-case-resolution-status-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/support-case-resolution-status.test.ts',
      proofOutput: relative(repoRoot, proofPath),
      summaryOutput: relative(repoRoot, summaryPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      expectations: ['docs/expectations/data-custody.md', 'docs/expectations/release-installer.md'],
    },
    claimsProved: [
      'Support case resolution rows cover opened, triage-ready, parent-update-ready, escalation-manual-required, response-manual-required, closure-ready, and SLA-manual-required states.',
      'Each row is parent-initiated and parent-consented with support-safe status refs, redaction refs, audit refs, publication refs, upload-status refs, and custody refs only.',
      'Escalation, operator response, and SLA rows remain manual-required until real operator workflow, provider contact, response execution, and published SLA proof exist.',
      'Package exports expose the support case resolution status contract and read-model to consumers through @ocentra-parent/schema-domain.',
      'Rows reject tokens, raw child activity, raw URLs, screenshots, journals, SQLite snapshots, private paths, command lines, keystrokes, clipboard data, message contents, provider secrets, remote support transcripts, backend upload execution, account lookup, billing provider contact, remote support session execution, production SLA claims, and default Ocentra-hosted family data.',
    ],
    claimsNotProved: [
      'real support backend upload execution',
      'provider contact execution',
      'account lookup execution',
      'billing provider contact execution',
      'remote support session execution',
      'production SLA',
      'raw child activity custody',
      'default Ocentra-hosted family data',
    ],
    readModel,
  };

  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit,
    proofMode: proof.proofMode,
    statesCovered: readModel.entries.map((entry) => entry.caseStatus),
    output: relative(repoRoot, proofPath),
    claimsNotProved: proof.claimsNotProved,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`production-support-case-resolution-status-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function parseReadModel() {
  const modulePath = join(repoRoot, 'packages', 'logging-domain', 'dist', 'support-case-resolution-status.js');
  const readModelPath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'support-case-resolution-status-read-model.js'
  );
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.SupportCaseResolutionStatusReadModelSchema.parse(readModelModule.SupportCaseResolutionStatusReadModel);
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, 'production-support-case-resolution-status-proof');
  assert.equal(readModel.entries.length, 7);
  const states = new Set(readModel.entries.map((entry) => entry.caseStatus));
  for (const state of [
    'case-opened',
    'triage-ready',
    'parent-update-ready',
    'escalation-manual-required',
    'response-manual-required',
    'closure-ready',
    'sla-manual-required',
  ]) {
    assert.equal(states.has(state), true);
  }

  const escalation = entryFor(readModel, 'support-case-escalation-manual-required');
  const response = entryFor(readModel, 'support-case-response-manual-required');
  const closure = entryFor(readModel, 'support-case-closure-ready');
  const sla = entryFor(readModel, 'support-case-sla-manual-required');
  assert.equal(escalation.escalationState, 'manual-required');
  assert.equal(response.operatorResponseState, 'manual-required');
  assert.equal(closure.closureRefs.length, 2);
  assert.equal(sla.slaState, 'manual-required');
}

async function assertPackageExports() {
  const contract = await import('@ocentra-parent/schema-domain/support-case-resolution-status');
  const readModel = await import('@ocentra-parent/schema-domain/support-case-resolution-status-read-model');
  assert.equal(typeof contract.SupportCaseResolutionStatusReadModelSchema.parse, 'function');
  assert.equal(readModel.SupportCaseResolutionStatusReadModel.entries.length, 7);
}

function entryFor(readModel, caseId) {
  const entry = readModel.entries.find((candidate) => candidate.caseId === caseId);
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
