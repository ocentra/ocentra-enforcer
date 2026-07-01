import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-product-claim-store-handoff-proof');
const proofPath = join(outputDir, 'proof.json');
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
      'tests/unit/app-install-purchase-product-claim-store-handoff-proof.test.ts',
    ])
  );

  const proofModule = await loadProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseProductClaimStoreHandoffProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseProductClaimStoreHandoffProof(parsedReadModel);

  assert.deepEqual(summary, {
    productClaimStoreHandoffRows: 5,
    reviewReadyRows: 1,
    manualRequiredRows: 1,
    unavailableRows: 3,
    productClaimApprovedRows: 0,
    providerExecutedRows: 0,
  });
  assert.deepEqual(
    parsedReadModel.productClaimStoreHandoffRows.map(
      (row) =>
        `${row.platform}:${row.storeSurface}:${row.sourceSafeParentWorkflowState}:${row.sourceManualEvidencePacketState}:${row.storeHandoffState}`
    ),
    [
      'windows:microsoft-store:safe-parent-review-ready:manual-evidence-packet-ready:store-handoff-review-ready',
      'macos:mac-app-store:manual-parent-review-required:manual-review-required:store-handoff-manual-required',
      'linux:linux-package-manager:unsupported-store-workflow-blocked:provider-unavailable:store-handoff-unavailable',
      'android:google-play:unsupported-store-workflow-blocked:manual-review-required:store-handoff-unavailable',
      'ios:apple-app-store:unsupported-store-workflow-blocked:manual-review-required:store-handoff-unavailable',
    ]
  );

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-product-claim-store-handoff-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    productDocState: 'updated-feature-expectation-checklist-for-store-handoff-proof',
    evidence: {
      productClaimStoreHandoffContract:
        'packages/schema-domain/src/app-install-purchase-product-claim-store-handoff-proof.ts',
      sourceSafeParentWorkflowContract:
        'packages/schema-domain/src/app-install-purchase-product-claim-safe-parent-workflow-proof.ts',
      sourceManualEvidencePacketContract:
        'packages/schema-domain/src/app-install-purchase-provider-store-manual-evidence-packet-proof.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-product-claim-store-handoff-proof.test.ts',
      packageExport: '@ocentra-parent/schema-domain/app-install-purchase-product-claim-store-handoff-proof',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistDoc: 'docs/product-capability-checklist.md',
      output: relative(repoRoot, proofPath),
    },
    storeHandoffSummary: summary,
    productClaimStoreHandoffRows: parsedReadModel.productClaimStoreHandoffRows,
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-product-claim-store-handoff-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-product-claim-store-handoff-proof.js'
  );
  return import(pathToFileURL(modulePath).href);
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
