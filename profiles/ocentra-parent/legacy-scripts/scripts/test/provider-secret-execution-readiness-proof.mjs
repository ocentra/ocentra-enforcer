import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const proofMode = 'provider-secret-execution-readiness-proof';
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
      'tests/unit/provider-secret-execution-readiness.test.ts',
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
      contract: 'packages/schema-domain/src/provider-secret-execution-readiness.ts',
      readModel: 'packages/schema-domain/src/provider-secret-execution-readiness-read-model.ts',
      contractTest: 'packages/logging-domain/tests/unit/provider-secret-execution-readiness.test.ts',
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
      featureDoc: 'docs/features/production-distribution-support.md',
      expectations: ['docs/expectations/data-custody.md', 'docs/expectations/static-analysis-security.md'],
      packageExports: [
        '@ocentra-parent/schema-domain/provider-secret-execution-readiness',
        '@ocentra-parent/schema-domain/provider-secret-execution-readiness-read-model',
      ],
    },
    claimsProved: [
      'Provider-secret execution readiness rows cover execution boundary, backend secret-store preflight, rotation preflight, revocation preflight, operator approval, manual execution, and support-safe audit export states.',
      'Rows link to provider-secret custody status, backend secret-store preflight, rotation, revocation, operator approval, manual proof, and audit refs while disclosing only support-safe status metadata.',
      'Provider-secret backend store, rotation, revocation, and execution remain not implemented or manual-required until real provider-secret execution proof exists.',
      'Package exports expose the provider-secret execution readiness contract and read model through @ocentra-parent/schema-domain.',
      'Rows reject provider secrets, payment provider tokens, raw child activity, raw support bundle payloads, account lookup results, billing provider contact records, remote support transcripts, backend secret-store execution, rotation/revocation execution, provider-secret execution delivery, support backend upload execution, account lookup execution, billing provider contact execution, remote support sessions, production SLA, and default Ocentra-hosted family data.',
    ],
    claimsNotProved: [
      'backend secret store execution',
      'provider secret rotation execution',
      'provider secret revocation execution',
      'provider secret execution delivery',
      'provider secret custody execution',
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
    commit: proof.commit,
    proofMode: proof.proofMode,
    statesCovered: readModel.entries.map((entry) => entry.readinessStatus),
    output: relativePath(proofPath),
    claimsNotProved: proof.claimsNotProved,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)}:${relativePath(summaryPath)}`);
}

async function parseReadModel() {
  const modulePath = join(repoRoot, 'packages', 'logging-domain', 'dist', 'provider-secret-execution-readiness.js');
  const readModelPath = join(
    repoRoot,
    'packages',
    'logging-domain',
    'dist',
    'provider-secret-execution-readiness-read-model.js'
  );
  const module = await import(pathToFileURL(modulePath).href);
  const readModelModule = await import(pathToFileURL(readModelPath).href);
  return module.ProviderSecretExecutionReadinessReadModelSchema.parse(
    readModelModule.ProviderSecretExecutionReadinessReadModel
  );
}

function assertReadModel(readModel) {
  assert.equal(readModel.readModelId, proofMode);
  assert.equal(readModel.entries.length, 7);
  const states = new Set(readModel.entries.map((entry) => entry.readinessStatus));
  for (const state of [
    'execution-boundary-recorded',
    'backend-secret-store-preflight-required',
    'rotation-preflight-required',
    'revocation-preflight-required',
    'operator-approval-required',
    'execution-manual-required',
    'audit-export-ready',
  ]) {
    assert.equal(states.has(state), true);
  }

  const boundary = entryFor(readModel, 'provider-secret-execution-boundary-recorded');
  assert.equal(boundary.executionState, 'not-implemented');
  assert.equal(boundary.containsProviderSecrets, false);
  assert.equal(boundary.providerSecretExecutionDelivered, false);

  const auditExport = entryFor(readModel, 'provider-secret-execution-audit-export-ready');
  assert.equal(auditExport.backendSecretStoreExecuted, false);
  assert.equal(auditExport.providerSecretRotationExecuted, false);
  assert.equal(auditExport.providerSecretRevocationExecuted, false);
}

async function assertPackageExports() {
  const contract = await import('@ocentra-parent/schema-domain/provider-secret-execution-readiness');
  const readModel = await import('@ocentra-parent/schema-domain/provider-secret-execution-readiness-read-model');
  assert.equal(typeof contract.ProviderSecretExecutionReadinessReadModelSchema.parse, 'function');
  assert.equal(readModel.ProviderSecretExecutionReadinessReadModel.entries.length, 7);
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
