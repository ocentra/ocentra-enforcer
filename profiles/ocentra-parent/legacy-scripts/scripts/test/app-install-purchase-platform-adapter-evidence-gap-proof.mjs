import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-platform-adapter-evidence-gap-proof');
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
      'tests/unit/app-install-purchase-platform-adapter-evidence-gap-proof.test.ts',
    ])
  );

  const proofModule = await loadProofModule();
  const parsedReadModel = proofModule.AppInstallPurchasePlatformAdapterEvidenceGapProofReadModel;
  const summary = proofModule.summarizeAppInstallPurchasePlatformAdapterEvidenceGapProof(parsedReadModel);

  assert.deepEqual(summary, {
    platformAdapterEvidenceGapRows: 5,
    adapterEvidenceGapRows: 1,
    manualAdapterEvidenceRequiredRows: 1,
    platformUnavailableRows: 1,
    blockedBeforeClaimRows: 2,
    realAdapterEvidenceRows: 0,
    adapterImplementedRows: 0,
    productClaimApprovedRows: 0,
  });

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-platform-adapter-evidence-gap-proof',
    baseMainState: 'after-pr485-provider-store-api-execution-proof-merged',
    commands,
    packageExportState: 'canonical schema-domain public subpath export confirmed; proof imports the built dist module directly.',
    checklistState: 'updated-docs-product-capability-checklist-app-install-row',
    evidence: {
      platformAdapterEvidenceGapContract:
        'packages/schema-domain/src/app-install-purchase-platform-adapter-evidence-gap-proof.ts',
      sourceProviderStoreApiExecutionContract:
        'packages/schema-domain/src/app-install-purchase-provider-store-api-execution-proof.ts',
      sourcePlatformProofReadinessContract:
        'packages/schema-domain/src/app-install-purchase-platform-proof-readiness.ts',
      contractTest:
        'packages/schema-domain/tests/unit/app-install-purchase-platform-adapter-evidence-gap-proof.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      platformExpectationDoc: 'docs/expectations/platforms.md',
      checklistDoc: 'docs/product-capability-checklist.md',
      packageReadme: 'packages/schema-domain/package.json',
      packageExport:
        'packages/schema-domain/package.json publishes this proof as a public subpath export; the script imports the built dist module directly.',
      output: relative(repoRoot, proofPath),
    },
    platformAdapterEvidenceGapSummary: summary,
    platformAdapterEvidenceGapRows: parsedReadModel.platformAdapterEvidenceGapRows,
    nonClaims: parsedReadModel.nonClaims,
    knownGaps: parsedReadModel.knownGaps,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-platform-adapter-evidence-gap-proof-ok:${relative(repoRoot, proofPath)}`);
}

async function loadProofModule() {
  const modulePath = join(
    repoRoot,
    'packages',
    'schema-domain',
    'dist',
    'app-install-purchase-platform-adapter-evidence-gap-proof.js'
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
