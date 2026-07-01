import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(
  repoRoot,
  'output',
  'data-custody-storage-plan-proof',
  '08-parent-storage-settings-apply-flow'
);
const proofPath = join(outputDir, 'parent-storage-settings-apply-flow-proof.json');
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
      'tests/contract/parent-storage-settings-apply-flow.test.ts',
    ])
  );

  const proofModule = await loadContractProofModule();
  await assertPackageExport(proofModule);
  const proof = proofModule.ParentStorageSettingsApplyFlowContractProofReadModel;

  assert.deepEqual(proof.noClaims, proofModule.RequiredParentStorageNoClaims);
  assert.deepEqual(
    proof.deleteActions.map((row) => row.actionKind),
    proofModule.RequiredParentStorageDeleteActionKinds
  );
  assert.equal(proof.modeCard.currentModeLabel, 'manual-required');
  assert.equal(proof.modeCard.manualRequiredVisible, true);
  assert.equal(proof.restorePreview.confirmationRequired, true);
  assert.equal(proof.restorePreview.tombstonesPreserved, true);
  assert.equal(proof.applyDecision.applyState, 'applyRequiresConfirmation');
  assert.equal(proof.disconnectAction.existingFilesMayRemain, true);
  assert.equal(proof.disconnectAction.providerDeleteRequestedSeparately, true);
  assert.equal(
    proof.deleteActions.find((row) => row.actionKind === 'delete-provider-backup-copy')?.state,
    'manual-required'
  );

  const proofJson = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'parent-storage-settings-apply-flow-proof',
    commands,
    evidence: {
      rustContract: 'crates/schema/src/parent_storage_settings_apply_flow.rs',
      rustCustodyCore: 'crates/storage-custody-core/src/parent_storage_settings_apply_flow.rs',
      tsAdapter: 'packages/schema-domain/src/parent-storage-settings-apply-flow.ts',
      generatedTs: 'packages/schema-domain/src/generated/parent-storage-settings-apply-flow-contracts.ts',
      contractTest: 'packages/schema-domain/tests/contract/parent-storage-settings-apply-flow.test.ts',
      builtModule: 'packages/schema-domain/dist/parent-storage-settings-apply-flow.js',
      packageExport: '@ocentra-parent/schema-domain/parent-storage-settings-apply-flow',
      output: relative(repoRoot, proofPath),
    },
    modeCard: {
      currentModeLabel: proof.modeCard.currentModeLabel,
      uiState: proof.modeCard.uiState,
      providerMode: proof.modeCard.providerMode,
      providerStatus: proof.modeCard.providerStatus,
      syncState: proof.modeCard.syncState,
      manualRequiredVisible: proof.modeCard.manualRequiredVisible,
    },
    restorePreview: {
      previewState: proof.restorePreview.previewState,
      confirmationRequired: proof.restorePreview.confirmationRequired,
      partialRestore: proof.restorePreview.partialRestore,
      rejectedSections: proof.restorePreview.rejectedSections,
      tombstonesPreserved: proof.restorePreview.tombstonesPreserved,
    },
    applyDecision: {
      applyState: proof.applyDecision.applyState,
      confirmationRequired: proof.applyDecision.confirmationRequired,
      willChange: proof.applyDecision.willChange,
      willNotChange: proof.applyDecision.willNotChange,
      preservedTombstones: proof.applyDecision.preservedTombstones,
    },
    deleteActions: proof.deleteActions.map((row) => ({
      actionKind: row.actionKind,
      state: row.state,
      separateFromDisconnect: row.separateFromDisconnect,
      proofRequired: row.proofRequired,
    })),
    disconnectAction: proof.disconnectAction,
    claimSafeCopyKeys: proof.claimSafeCopy.map((row) => row.copyKey),
    noClaims: proof.noClaims,
    knownGaps: proofModule.ParentStorageSettingsApplyFlowKnownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proofJson, null, 2)}\n`);
  console.log(`parent-storage-settings-apply-flow-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadContractProofModule() {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', 'parent-storage-settings-apply-flow.js');
  return import(pathToFileURL(modulePath).href);
}

async function assertPackageExport(proofModule) {
  const exportedModule = await import('@ocentra-parent/schema-domain/parent-storage-settings-apply-flow');
  assert.equal(
    exportedModule.ParentStorageSettingsApplyFlowContractProofReadModel.modeCard.rowId,
    proofModule.ParentStorageSettingsApplyFlowContractProofReadModel.modeCard.rowId
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
