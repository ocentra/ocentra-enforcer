import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'support-bundle-redaction-proof');
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
      'tests/unit/support-bundle-redaction.test.ts',
    ])
  );

  const commit = await gitHead();
  const readModel = await parseReadModel();
  assertReadModel(readModel);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit,
    proofMode: 'support-bundle-redaction-proof',
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/support-bundle-redaction.ts',
      contractTest: 'packages/logging-domain/tests/unit/support-bundle-redaction.test.ts',
      output: relative(repoRoot, proofPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      expectations: ['docs/expectations/release-installer.md', 'docs/expectations/billing.md'],
    },
    claimsProved: [
      'Support bundle redaction rows require parent consent or manual review before handoff.',
      'Support-safe bundle data classes are limited to release, commit, platform, package/runtime, service, route, capability, degraded, redaction, manual proof, incident, billing status, and account status references.',
      'Billing escalation and account lookup support paths remain manual-required and do not contact providers or execute account lookup.',
      'Backend upload, remote support, and production SLA remain not-implemented or manual-required states.',
      'Support output excludes tokens, child activity, raw URLs, screenshots, journals, SQLite snapshots, private paths, command lines, keystrokes, clipboard data, message contents, and provider secrets.',
    ],
    claimsNotProved: [
      'support backend upload',
      'billing provider escalation',
      'account backend lookup',
      'remote support session',
      'production SLA',
      'signed release publishing',
      'store distribution',
    ],
    readModel,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`support-bundle-redaction-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function parseReadModel() {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'support-bundle-redaction.js');
  const readModelPath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'support-bundle-redaction-read-model.js');
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.SupportBundleRedactionReadModelSchema.parse(readModelModule.SupportBundleRedactionReadModel);
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, 'support-bundle-redaction-proof');
  assert.equal(readModel.entries.length, 8);
  const ready = readModel.entries.find((entry) => entry.incidentId === 'support-incident-bundle-ready');
  const billing = readModel.entries.find(
    (entry) => entry.incidentId === 'support-incident-billing-escalation-manual-required'
  );
  const account = readModel.entries.find(
    (entry) => entry.incidentId === 'support-incident-account-lookup-manual-required'
  );

  assert.equal(ready.parentConsentState, 'parent-approved');
  assert.equal(ready.containsTokens, false);
  assert.equal(ready.containsChildActivity, false);
  assert.equal(ready.containsRawUrls, false);
  assert.equal(ready.containsScreenshots, false);
  assert.equal(ready.containsJournals, false);
  assert.equal(ready.containsSqliteSnapshots, false);
  assert.equal(ready.containsPrivatePaths, false);
  assert.equal(ready.containsCommandLines, false);
  assert.equal(ready.containsKeystrokes, false);
  assert.equal(ready.containsClipboardData, false);
  assert.equal(ready.containsMessageContents, false);
  assert.equal(ready.backendUploadExecuted, false);
  assert.equal(ready.remoteSupportSessionStarted, false);
  assert.equal(ready.productionSlaClaimed, false);
  assert.equal(billing.billingEscalationState, 'manual-required');
  assert.equal(billing.billingProviderContacted, false);
  assert.equal(account.accountLookupState, 'manual-required');
  assert.equal(account.accountLookupExecuted, false);
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
