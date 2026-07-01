import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'output', 'data-custody-storage-plan-proof', '01-custody-source-of-truth');
const proofPath = join(outputDir, 'data-custody-source-of-truth-proof.json');
const commands = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand(
    ...npmCommand([
      'run',
      'test',
      '--workspace',
      '@ocentra-parent/schema-domain',
      '--',
      'tests/contract/data-custody-source-of-truth.test.ts',
    ])
  );

  const proofModule = await loadContractProofModule();
  await assertPackageExport(proofModule);

  const proof = proofModule.DataCustodySourceOfTruthProofReadModel;
  const authorityCounts = proofModule.summarizeDataCustodyAuthorities(proof.rows);
  const hostingModeCounts = proofModule.summarizeDataCustodyOcentraHostingModes(proof.rows);

  assert.equal(proof.rows.length, proofModule.RequiredDataCustodyClassIds.length);
  assert.deepEqual(proof.rows.map((row) => row.classId), proofModule.RequiredDataCustodyClassIds);
  assert.deepEqual(proof.allowedOcentraHostedMetadata, proofModule.HostedOcentraMetadataClassIds);
  assert.deepEqual(proof.mustNeverBeHostedByDefault, proofModule.MustNeverBeHostedByDefaultClassIds);
  assert.deepEqual(proof.nonClaims, proofModule.RequiredDataCustodyNonClaims);
  assert.equal(proof.accountControlPlaneSeparated, true);
  assert.equal(proof.providerOwnedBillingIdentitySeparated, true);
  assert.equal(proof.ocentraIsDefaultChildDataStore, false);
  assert.equal(proof.providerAutoApplyClaimed, false);
  assert.equal(proof.supportDecryptByDefaultClaimed, false);
  assert.equal(proof.sqliteAsTruthLayerClaimed, false);
  assert.equal(proof.rawChildActivityHostedByDefaultClaimed, false);

  const proofJson = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'data-custody-source-of-truth-proof',
    commands,
    evidence: {
      rustContract: 'crates/schema/src/data_custody_source_of_truth.rs',
      rustTsEmitter: 'crates/schema/src/data_custody_source_of_truth_ts.rs',
      rustTsExportBin: 'crates/schema/src/bin/export_data_custody_source_of_truth_contract_types.rs',
      rustContractTest: 'crates/schema/tests/contract/data_custody_source_of_truth.rs',
      tsBoundaryAdapter: 'packages/schema-domain/src/custody-boundary.ts',
      tsMatrixAdapter: 'packages/schema-domain/src/data-custody-matrix.ts',
      generatedTs: 'packages/schema-domain/src/generated/data-custody-source-of-truth-contracts.ts',
      contractTest: 'packages/schema-domain/tests/contract/data-custody-source-of-truth.test.ts',
      builtModule: 'packages/schema-domain/dist/data-custody-matrix.js',
      packageExport: '@ocentra-parent/schema-domain/data-custody-matrix',
      output: relative(repoRoot, proofPath),
    },
    classIds: proof.rows.map((row) => row.classId),
    authorityCounts,
    hostingModeCounts,
    allowedOcentraHostedMetadata: proof.allowedOcentraHostedMetadata,
    mustNeverBeHostedByDefault: proof.mustNeverBeHostedByDefault,
    derivedRows: proof.rows
      .filter((row) => row.sourceOfTruth.kind === 'derived-from-data-classes')
      .map((row) => ({
        classId: row.classId,
        sourceClassIds: row.sourceOfTruth.sourceClassIds,
      })),
    rawChildEvidenceRows: proof.rows
      .filter((row) => row.rawChildEvidenceAllowed)
      .map((row) => ({
        classId: row.classId,
        reportExposure: row.reportExposure,
        notificationExposure: row.notificationExposure,
      })),
    nonClaims: proof.nonClaims,
    claimSafeLanguage: proof.claimSafeLanguage,
    separationFlags: {
      accountControlPlaneSeparated: proof.accountControlPlaneSeparated,
      providerOwnedBillingIdentitySeparated: proof.providerOwnedBillingIdentitySeparated,
    },
    noClaimFlags: {
      ocentraIsDefaultChildDataStore: proof.ocentraIsDefaultChildDataStore,
      providerAutoApplyClaimed: proof.providerAutoApplyClaimed,
      supportDecryptByDefaultClaimed: proof.supportDecryptByDefaultClaimed,
      sqliteAsTruthLayerClaimed: proof.sqliteAsTruthLayerClaimed,
      rawChildActivityHostedByDefaultClaimed: proof.rawChildActivityHostedByDefaultClaimed,
    },
    knownGaps: proofModule.DataCustodyKnownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proofJson, null, 2)}\n`);
  console.log(`data-custody-source-of-truth-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadContractProofModule() {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'data-custody-matrix.js');
  return import(pathToFileURL(modulePath).href);
}

async function assertPackageExport(proofModule) {
  const exportedModule = await import('@ocentra-parent/schema-domain/data-custody-matrix');
  assert.equal(
    exportedModule.DataCustodySourceOfTruthProofReadModel.matrixId,
    proofModule.DataCustodySourceOfTruthProofReadModel.matrixId
  );
}

async function gitHead() {
  const output = await commandOutput('git', ['rev-parse', 'HEAD']);
  return output.trim();
}

async function commandOutput(command, args) {
  const chunks = [];
  const child = spawn(command, args, { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
  child.stdout.on('data', (chunk) => chunks.push(chunk));
  child.stderr.on('data', (chunk) => chunks.push(chunk));
  const exitCode = await new Promise((resolve) => {
    child.on('close', resolve);
  });
  const output = Buffer.concat(chunks).toString('utf8');
  if (exitCode !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${exitCode}\n${output}`);
  }
  return output;
}

async function runCommand(command, args) {
  const startedAt = new Date().toISOString();
  const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit' });
  const exitCode = await new Promise((resolve) => {
    child.on('close', resolve);
  });
  commands.push({ command: `${command} ${args.join(' ')}`, startedAt, exitCode });
  if (exitCode !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${exitCode}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
