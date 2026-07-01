import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(
  repoRoot,
  'test-results',
  'app-install-purchase-product-claim-platform-limitation-fallback-proof'
);
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
      'tests/unit/app-install-purchase-product-claim-platform-limitation-fallback-proof.test.ts',
    ])
  );

  const proofModule = await loadProofModule();
  const parsedReadModel = proofModule.AppInstallPurchaseProductClaimPlatformLimitationFallbackProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchaseProductClaimPlatformLimitationFallbackProof(parsedReadModel);

  assert.deepEqual(summary, {
    platformLimitationFallbackRows: 5,
    fallbackParentWorkflowReadyRows: 1,
    manualPlatformLimitationFallbackRequiredRows: 1,
    unsupportedPlatformLimitationFallbackBlockedRows: 3,
    productClaimApprovedRows: 0,
    providerExecutedRows: 0,
    platformAdapterImplementedRows: 0,
  });

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-product-claim-platform-limitation-fallback-proof',
    commands,
    packageExportState: 'not-claimed-new-public-export-deferred',
    checklistState: 'updated-docs-product-capability-checklist-app-install-row',
    evidence: {
      platformLimitationFallbackContract:
        'packages/schema-domain/src/app-install-purchase-product-claim-platform-limitation-fallback-proof.ts',
      sourcePlatformPreclaimContract:
        'packages/schema-domain/src/app-install-purchase-product-claim-platform-preclaim-proof.ts',
      sourceSafeParentWorkflowContract:
        'packages/schema-domain/src/app-install-purchase-product-claim-safe-parent-workflow-proof.ts',
      sourcePlatformLimitationActionContract:
        'packages/schema-domain/src/app-install-purchase-platform-limitation-action-proof.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-product-claim-platform-limitation-fallback-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      platformExpectationDoc: 'docs/expectations/platforms.md',
      checklistDoc: 'docs/product-capability-checklist.md',
      packageReadme: 'packages/schema-domain/package.json',
      packageExport:
        '@ocentra-parent/schema-domain/app-install-purchase-product-claim-platform-limitation-fallback-proof',
      output: relative(repoRoot, proofPath),
    },
    platformLimitationFallbackSummary: summary,
    platformLimitationFallbackRows: parsedReadModel.platformLimitationFallbackRows,
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(
    `app-install-purchase-product-claim-platform-limitation-fallback-proof-ok:${relative(repoRoot, proofPath)}`
  );
}

async function loadProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-product-claim-platform-limitation-fallback-proof.js'
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
