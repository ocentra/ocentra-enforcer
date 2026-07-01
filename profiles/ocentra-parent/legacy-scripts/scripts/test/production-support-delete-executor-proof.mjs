import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofMode = 'production-support-delete-executor-proof';
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
      'tests/unit/delete-executor-proof.test.ts',
    ])
  );

  const contract = await assertPackageExports();
  const documentation = await assertDocumentationProof();
  const knownGaps = [
    'Real data export/delete runtime execution remains unimplemented.',
    'Durable queues, payload deletion execution, provider execution, public runtime, legal execution, and production SLA remain unclaimed.',
    'Child activity custody, raw support bundle payloads, provider secrets, remote support transcripts, and default Ocentra-hosted family data remain excluded.',
    'docs/product-capability-checklist.md update is proposed in hub report because E-B owns the shared checklist lock.',
  ];
  const proof = {
    schemaVersion: 1,
    checkedAt: deterministicCheckedAt,
    commit: deterministicCommit,
    proofMode,
    commands,
    evidence: {
      loggingContract: 'packages/schema-domain/src/delete-executor-proof.ts',
      loggingReadModel: 'packages/schema-domain/src/delete-executor-read-model.ts',
      loggingTest: 'packages/logging-domain/tests/unit/delete-executor-proof.test.ts',
      packageExports: [
        '@ocentra-parent/schema-domain/delete-executor-proof',
        '@ocentra-parent/schema-domain/delete-executor-read-model',
      ],
      documentation,
      proofOutput: relativePath(proofPath),
      summaryOutput: relativePath(summaryPath),
    },
    targetSummary: contract.targetSummary,
    statusSummary: contract.statusSummary,
    rows: contract.rows,
    knownGaps,
  };
  const summary = {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    proofMode,
    rowCount: proof.rows.length,
    targetSummary: proof.targetSummary,
    statusSummary: proof.statusSummary,
    output: relativePath(proofPath),
    knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`${proofMode}-ok:${relativePath(proofPath)} ${relativePath(summaryPath)}`);
}

async function assertPackageExports() {
  const contractModule = await import('@ocentra-parent/schema-domain/delete-executor-proof');
  const readModelModule = await import('@ocentra-parent/schema-domain/delete-executor-read-model');
  const readModel = contractModule.DeleteExecutorReadModelSchema.parse(readModelModule.DeleteExecutorReadModel);
  const targetSummary = contractModule.summarizeDeleteExecutorTargets(readModel.rows);
  const statusSummary = contractModule.summarizeDeleteExecutorStatuses(readModel.rows);

  assert.deepEqual(targetSummary, {
    'local-export-output': 2,
    'support-backend-payload': 1,
    'status-backend-payload': 1,
    'public-runtime-payload': 1,
    'legal-disclosure-payload': 1,
  });
  assert.deepEqual(statusSummary, {
    'source-contract-ready': 0,
    'delete-request-recorded': 1,
    'executor-manual-required': 2,
    'executor-unavailable': 1,
    'blocked-before-runtime': 2,
  });
  assert.equal(typeof contractModule.decodeDeleteExecutorReadModel, 'function');

  return {
    targetSummary,
    statusSummary,
    rows: readModel.rows.map((row) => ({
      rowId: row.rowId,
      target: row.target,
      status: row.status,
      custodyBoundary: row.custodyBoundary,
      sourceProofRefs: row.sourceProofRefs,
      manualProofRequirements: row.manualProofRequirements,
    })),
  };
}

async function assertDocumentationProof() {
  const docs = [
    'docs/features/production-distribution-support.md',
    'docs/expectations/data-custody.md',
    'packages/logging-domain/README.md',
  ];
  for (const path of docs) {
    assertIncludes(await readRepoFile(path), proofMode, `${path} proof note`);
  }
  return docs;
}

async function readRepoFile(path) {
  return readFile(join(repoRoot, path), 'utf8');
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

function assertIncludes(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
