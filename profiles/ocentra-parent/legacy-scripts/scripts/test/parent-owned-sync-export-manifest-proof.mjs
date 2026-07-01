import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'output', 'data-custody-storage-plan-proof', '03-parent-owned-cloud-sync');
const proofPath = join(outputDir, 'parent-owned-sync-export-manifest-proof.json');
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
      'tests/contract/parent-owned-sync-export.test.ts',
    ])
  );

  const proofModule = await loadContractProofModule();
  await assertPackageExport(proofModule);
  const proof = proofModule.ParentOwnedSyncExportContractProofReadModel;

  const dataClassCounts = proofModule.summarizeParentOwnedSyncExportDataClasses(proof.manifest.items);
  const providerModeCounts = proofModule.summarizeParentOwnedSyncExportProviderModes(proof.providerStatuses);
  const providerStatusCounts = proofModule.summarizeParentOwnedSyncExportProviderStatuses(proof.providerStatuses);
  const syncStateCounts = proofModule.summarizeParentOwnedSyncExportSyncStates(proof.syncStates);
  const tombstoneStateCounts = proofModule.summarizeParentOwnedSyncExportTombstoneStates(proof.tombstones);

  assert.equal(Object.values(dataClassCounts).every((count) => count === 1), true);
  assert.equal(providerModeCounts['google-drive-appdata'], 1);
  assert.equal(providerModeCounts['disabled'], 1);
  assert.equal(providerStatusCounts['manual-required'], 1);
  assert.equal(providerStatusCounts.revoked, 1);
  assert.equal(providerStatusCounts['wrong-account'], 1);
  assert.equal(providerStatusCounts['folder-unavailable'], 1);
  assert.equal(providerStatusCounts['partial-upload'], 1);
  assert.equal(providerStatusCounts.disconnected, 1);
  assert.equal(syncStateCounts.synced, 1);
  assert.equal(syncStateCounts['manual-required'], 1);
  assert.equal(tombstoneStateCounts.propagated, 1);
  assert.equal(tombstoneStateCounts.blocked, 1);
  assert.equal(tombstoneStateCounts['manual-required'], 1);
  assert.equal(proof.transferRuntimeClaimed, false);
  assert.equal(proof.connectorOAuthClaimed, false);
  assert.equal(proof.uploadRuntimeClaimed, false);
  assert.equal(proof.deleteRuntimeClaimed, false);
  assert.equal(proof.ocentraHostedChildEvidenceStored, false);

  const proofJson = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'parent-owned-sync-export-manifest-proof',
    commands,
    evidence: {
      rustContract: 'crates/schema/src/parent_owned_sync_export.rs',
      rustCustodyCore: 'crates/storage-custody-core/src/parent_owned_sync_export.rs',
      tsAdapter: 'packages/schema-domain/src/parent-owned-sync-export.ts',
      generatedTs: 'packages/schema-domain/src/generated/parent-owned-sync-export-contracts.ts',
      contractTest: 'packages/schema-domain/tests/contract/parent-owned-sync-export.test.ts',
      builtModule: 'packages/schema-domain/dist/parent-owned-sync-export.js',
      packageExport: '@ocentra-parent/schema-domain/parent-owned-sync-export',
      expectationDoc: 'docs/expectations/sync-export.md',
      output: relative(repoRoot, proofPath),
    },
    providerModes: proof.providerStatuses.map((row) => row.providerMode),
    providerStatuses: proof.providerStatuses.map((row) => ({
      mode: row.providerMode,
      status: row.providerStatus,
      disconnectVisibilityState: row.disconnectVisibilityState,
      deleteVisibilityState: row.deleteVisibilityState,
    })),
    dataClassCounts,
    providerModeCounts,
    providerStatusCounts,
    syncStates: proof.syncStates.map((row) => ({
      syncState: row.syncState,
      manifestIntegrityState: row.manifestIntegrityState,
      providerStatusRef: row.providerStatusRef,
      parentActionRequired: row.parentActionRequired,
    })),
    syncStateCounts,
    tombstones: proof.tombstones.map((row) => ({
      dataClass: row.dataClass,
      propagationState: row.propagationState,
      providerStatusRef: row.providerStatusRef,
    })),
    tombstoneStateCounts,
    nonClaims: proof.nonClaims,
    claimBoundaries: {
      transferRuntimeClaimed: proof.transferRuntimeClaimed,
      connectorOAuthClaimed: proof.connectorOAuthClaimed,
      uploadRuntimeClaimed: proof.uploadRuntimeClaimed,
      deleteRuntimeClaimed: proof.deleteRuntimeClaimed,
      ocentraHostedChildEvidenceStored: proof.ocentraHostedChildEvidenceStored,
    },
    knownGaps: proofModule.ParentOwnedSyncExportKnownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proofJson, null, 2)}\n`);
  console.log(`parent-owned-sync-export-manifest-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadContractProofModule() {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'parent-owned-sync-export.js');
  return import(pathToFileURL(modulePath).href);
}

async function assertPackageExport(proofModule) {
  const exportedModule = await import('@ocentra-parent/schema-domain/parent-owned-sync-export');
  assert.equal(
    exportedModule.ParentOwnedSyncExportContractProofReadModel.manifest.manifestId,
    proofModule.ParentOwnedSyncExportContractProofReadModel.manifest.manifestId
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
