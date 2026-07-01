import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'app-install-purchase-approval');
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
      'tests/unit/app-install-purchase-approval.test.ts',
    ])
  );

  const approvalModule = await loadModule('app-install-purchase-approval.js');
  const proofModule = await loadModule('app-install-purchase-approval-proof.js');
  const parsedProof = approvalModule.AppInstallPurchaseApprovalContractProofSchema.parse(
    proofModule.AppInstallPurchaseApprovalContractProofReadModel
  );

  assert.deepEqual(requestKindCounts(parsedProof), {
    install: 1,
    purchase: 1,
    subscription: 1,
  });
  assert.deepEqual(platformStateCounts(parsedProof), {
    supported: 5,
    'manual-required': 24,
    unavailable: 6,
  });
  assert.deepEqual(metadataFreshnessCounts(parsedProof), {
    fresh: 1,
    stale: 1,
    'manual-required': 1,
  });
  assert.deepEqual(packageSourceArtifactStatusCounts(parsedProof), {
    'manual-required': 2,
    unavailable: 1,
    'device-proof-required': 2,
  });
  assert.deepEqual(decisionActionCounts(parsedProof), {
    approve: 1,
    deny: 1,
    'time-box': 1,
    'review-needed': 1,
  });
  assert.deepEqual(childVisibleStatusCounts(parsedProof), {
    'pending-parent-review-visible': 1,
    'approved-visible': 1,
    'denied-visible': 1,
    'time-box-visible': 1,
    'review-needed-visible': 1,
  });

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'app-install-purchase-approval',
    commands,
    evidence: {
      approvalContract: 'packages/schema-domain/src/app-install-purchase-approval.ts',
      approvalProofReadModel: 'packages/schema-domain/src/app-install-purchase-approval-proof.ts',
      platformSourceContract: 'packages/schema-domain/src/app-install-purchase-approval-platform-sources.ts',
      packageSourceContract: 'packages/schema-domain/src/app-install-purchase-approval-package-sources.ts',
      contractTest: 'packages/schema-domain/tests/unit/app-install-purchase-approval.test.ts',
      featureDoc: 'docs/features/app-install-purchase-approval.md',
      expectationDoc: 'docs/expectations/app-install-purchase-approval.md',
      checklistDoc: 'docs/product-capability-checklist.md',
      packageExport: '@ocentra-parent/schema-domain/app-install-purchase-approval',
      proofExport: '@ocentra-parent/schema-domain/app-install-purchase-approval-proof',
      output: relative(repoRoot, proofPath),
    },
    requestKindCounts: requestKindCounts(parsedProof),
    platformStateCounts: platformStateCounts(parsedProof),
    metadataFreshnessCounts: metadataFreshnessCounts(parsedProof),
    packageSourceArtifactStatusCounts: packageSourceArtifactStatusCounts(parsedProof),
    decisionActionCounts: decisionActionCounts(parsedProof),
    childVisibleStatusCounts: childVisibleStatusCounts(parsedProof),
    nonClaims: parsedProof.nonClaims,
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`app-install-purchase-approval-ok:${relative(repoRoot, proofPath)}`);
}

async function loadModule(file) {
  const modulePath = join(repoRoot, 'packages', 'schema-domain', 'dist', file);
  return import(pathToFileURL(modulePath).href);
}

function requestKindCounts(proof) {
  return countBy([
    proof.installRequest.requestKind,
    proof.purchaseRequest.requestKind,
    proof.subscriptionRequest.requestKind,
  ]);
}

function platformStateCounts(proof) {
  return countBy(
    proof.platformSupportMatrix.flatMap((row) => [
      row.contractRequestState,
      row.storeMetadataState,
      row.installInterceptionState,
      row.purchaseInterceptionState,
      row.subscriptionInterceptionState,
      row.childPendingState,
      row.approvalDeliveryState,
    ])
  );
}

function metadataFreshnessCounts(proof) {
  return countBy([
    proof.installRequest.storeMetadata.freshness,
    proof.purchaseRequest.storeMetadata.freshness,
    proof.subscriptionRequest.storeMetadata.freshness,
  ]);
}

function packageSourceArtifactStatusCounts(proof) {
  return countBy(proof.packageSourceArtifacts.map((row) => row.artifactStatus));
}

function decisionActionCounts(proof) {
  return countBy(proof.approvalDecisions.map((decision) => decision.decisionAction));
}

function childVisibleStatusCounts(proof) {
  return countBy(proof.childFacingStates.map((state) => state.childVisibleStatus));
}

function countBy(values) {
  return values.reduce((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
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

