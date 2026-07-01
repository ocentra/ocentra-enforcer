import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'support-incident-workflow-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];

await main();

async function main() {
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
      'tests/unit/support-incident-workflow.test.ts',
    ])
  );

  const commit = await gitHead();
  const readModel = await parseReadModel();
  assertReadModel(readModel);
  await assertPackageExport();

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit,
    proofMode: 'support-incident-workflow-proof',
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/support-incident-workflow.ts',
      guards: 'packages/schema-domain/src/support-incident-workflow-guards.ts',
      readModel: 'packages/schema-domain/src/support-incident-workflow-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/support-incident-workflow.test.ts',
      output: relative(repoRoot, proofPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      expectations: ['docs/expectations/static-analysis-security.md', 'docs/expectations/data-custody.md'],
    },
    claimsProved: [
      'Production support incident workflow rows cover parent consent, privacy/legal disclosure, redaction/audit review, backend upload manual-required, billing escalation manual-required, and account lookup manual-required states.',
      'Privacy and legal disclosure refs must be present before support incident export or handoff can proceed past the consent gate.',
      'Redaction and custody audit refs are required for every support workflow row.',
      'Backend upload, billing provider escalation, and account lookup stay manual-required without executing provider/backend contact.',
      'Support incident workflow output excludes tokens, child activity, raw URLs, screenshots, journals, SQLite snapshots, private paths, command lines, keystrokes, clipboard data, message contents, provider secrets, remote sessions, SLA claims, and Ocentra-hosted child activity custody.',
    ],
    claimsNotProved: [
      'support backend upload',
      'account lookup execution',
      'billing provider contact',
      'remote support session',
      'production SLA',
      'Ocentra-hosted child activity custody',
      'public privacy policy publication',
    ],
    readModel,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`support-incident-workflow-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function parseReadModel() {
  const modulePath = join(repoRoot, 'packages', 'logging-domain', 'dist', 'support-incident-workflow.js');
  const readModelPath = join(repoRoot, 'packages', 'logging-domain', 'dist', 'support-incident-workflow-read-model.js');
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.SupportIncidentWorkflowReadModelSchema.parse(readModelModule.SupportIncidentWorkflowReadModel);
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, 'support-incident-workflow-proof');
  assert.equal(readModel.entries.length, 6);
  const disclosure = entryFor(readModel, 'support-workflow-privacy-legal-disclosure');
  const redaction = entryFor(readModel, 'support-workflow-redaction-audit-review');
  const upload = entryFor(readModel, 'support-workflow-backend-upload-manual-required');
  const billing = entryFor(readModel, 'support-workflow-billing-escalation-manual-required');
  const account = entryFor(readModel, 'support-workflow-account-lookup-manual-required');

  assert.equal(disclosure.privacyDisclosureState, 'disclosed-before-export');
  assert.equal(disclosure.legalDisclosureState, 'disclosed-before-export');
  assert.deepEqual(redaction.auditRefs, ['support-incident-audit-event-ref', 'custody-boundary-audit-ref']);
  assert.equal(upload.backendUploadState, 'manual-required');
  assert.equal(upload.backendUploadExecuted, false);
  assert.equal(billing.billingEscalationState, 'manual-required');
  assert.equal(billing.billingProviderContacted, false);
  assert.equal(account.accountLookupState, 'manual-required');
  assert.equal(account.accountLookupExecuted, false);
}

async function assertPackageExport() {
  const exported = await import('@ocentra-parent/schema-domain/support-incident-workflow');
  assert.equal(typeof exported.SupportIncidentWorkflowReadModelSchema.parse, 'function');
}

function entryFor(readModel, incidentId) {
  const entry = readModel.entries.find((candidate) => candidate.incidentId === incidentId);
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
