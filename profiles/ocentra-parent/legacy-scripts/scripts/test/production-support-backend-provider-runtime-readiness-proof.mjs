import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'production-support-backend-provider-runtime-readiness-proof';
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
      'tests/unit/support-backend-provider-runtime-readiness.test.ts',
    ])
  );

  const readModel = await parseReadModel();
  assertReadModel(readModel);
  await assertPackageExports();
  await assertLinkedProofExports();

  const proof = {
    schemaVersion: 1,
    checkedAt: deterministicCheckedAt,
    commit: deterministicCommit,
    proofMode,
    commands,
    evidence: {
      contract: 'packages/schema-domain/src/support-backend-provider-runtime-readiness.ts',
      readModel: 'packages/schema-domain/src/support-backend-provider-runtime-readiness-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/support-backend-provider-runtime-readiness.test.ts',
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      checklist: 'docs/product-capability-checklist.md',
      expectations: [
        'docs/expectations/data-custody.md',
        'docs/expectations/release-installer.md',
        'docs/expectations/static-analysis-security.md',
      ],
      packageExports: [
        '@ocentra-parent/schema-domain/support-backend-provider-runtime-readiness',
        '@ocentra-parent/schema-domain/support-backend-provider-runtime-readiness-read-model',
      ],
    },
    claimsProved: [
      'Support backend provider runtime readiness rows compose existing upload execution, custody audit, provider-secret readiness, account/SLA, privacy/legal, and case status proof refs.',
      'Rows cover upload runtime linked, provider-secret preflight linked, billing provider manual-required, account lookup manual-required, legal disclosure manual-required, remote support manual-required, SLA manual-required, and support-safe audit export states.',
      'Rows disclose only support-safe status refs and manual proof refs while preserving no Ocentra-hosted family data as the custody state.',
      'Package exports expose the support backend provider runtime readiness contract and read model through @ocentra-parent/schema-domain.',
      'Rows reject provider secrets, payment provider tokens, raw child activity, raw support bundle payloads, account lookup results, billing provider contact records, remote support transcripts, support backend upload execution, provider-secret delivery, account lookup execution, billing provider contact execution, legal disclosure execution, remote support sessions, production SLA, and default Ocentra-hosted family data.',
    ],
    claimsNotProved: [
      'real support backend upload execution',
      'provider-secret delivery',
      'account lookup execution',
      'billing provider contact execution',
      'legal disclosure execution',
      'remote support session execution',
      'production SLA',
      'provider-secret custody execution',
      'default Ocentra-hosted family data',
      'raw child activity custody',
    ],
    readModel,
  };

  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    proofMode: proof.proofMode,
    statesCovered: readModel.entries.map((entry) => entry.readinessState),
    output: relativePath(proofPath),
    claimsNotProved: proof.claimsNotProved,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)}:${relativePath(summaryPath)}`);
}

async function parseReadModel() {
  const modulePath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'support-backend-provider-runtime-readiness.js'
  );
  const readModelPath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'support-backend-provider-runtime-readiness-read-model.js'
  );
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.SupportBackendProviderRuntimeReadinessReadModelSchema.parse(
    readModelModule.SupportBackendProviderRuntimeReadinessReadModel
  );
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, proofMode);
  assert.equal(readModel.entries.length, 8);
  const states = new Set(readModel.entries.map((entry) => entry.readinessState));
  for (const state of [
    'upload-runtime-linked',
    'provider-secret-preflight-linked',
    'billing-provider-manual-required',
    'account-lookup-manual-required',
    'legal-disclosure-manual-required',
    'remote-support-manual-required',
    'sla-manual-required',
    'audit-export-ready',
  ]) {
    assert.equal(states.has(state), true);
  }

  const upload = entryFor(readModel, 'support-backend-provider-upload-runtime-linked');
  assert.equal(upload.uploadRuntimeState, 'readiness-only');
  assert.equal(upload.supportBackendUploadExecuted, false);

  const audit = entryFor(readModel, 'support-backend-provider-audit-export-ready');
  assert.equal(audit.productionSlaClaimed, false);
  assert.equal(audit.ocentraHostedFamilyDataDefault, false);
}

async function assertPackageExports() {
  const contract = await import('@ocentra-parent/schema-domain/support-backend-provider-runtime-readiness');
  const readModel = await import('@ocentra-parent/schema-domain/support-backend-provider-runtime-readiness-read-model');
  assert.equal(typeof contract.SupportBackendProviderRuntimeReadinessReadModelSchema.parse, 'function');
  assert.equal(readModel.SupportBackendProviderRuntimeReadinessReadModel.entries.length, 8);
}

async function assertLinkedProofExports() {
  const upload = await import('@ocentra-parent/schema-domain/support-backend-upload-execution-runtime-read-model');
  const custody = await import('@ocentra-parent/schema-domain/support-backend-upload-custody-audit-read-model');
  const provider = await import('@ocentra-parent/schema-domain/provider-secret-execution-readiness-read-model');
  assert.equal(upload.SupportBackendUploadExecutionRuntimeReadModel.entries.length, 7);
  assert.equal(custody.SupportBackendUploadCustodyAuditReadModel.entries.length, 5);
  assert.equal(provider.ProviderSecretExecutionReadinessReadModel.entries.length, 7);
}

function entryFor(readModel, statusId) {
  const entry = readModel.entries.find((candidate) => candidate.statusId === statusId);
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
