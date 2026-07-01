import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-process-package-identity-proof-bridge');
const proofPath = join(outputDir, 'proof.json');
const proofCommand = 'node scripts/test/v0-8-process-package-identity-proof-bridge.mjs';
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const { V08HostAdapterProofPreflightMatrix } =
    await import('../../packages/schema-domain/dist/enforcement-host-adapter-preflight.js');
  const { V08ProcessPackageIdentityProofBridgeMatrix } =
    await import('../../packages/schema-domain/dist/enforcement-process-package-identity.js');
  const proofMatrix = JSON.parse(await readFile(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json')));

  assertBridgeMatrix(V08ProcessPackageIdentityProofBridgeMatrix, V08HostAdapterProofPreflightMatrix);
  assertProofMatrix(proofMatrix);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-process-package-identity-proof-bridge',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      bridgeContract: 'packages/schema-domain/src/enforcement-process-package-identity.ts',
      preflightContract: 'packages/schema-domain/src/enforcement-host-adapter-preflight.ts',
      proofMatrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/v0-8-process-package-identity-proof-bridge-2026-05-29.md',
    },
    counts: bridgeCounts(V08ProcessPackageIdentityProofBridgeMatrix),
    bridgeMatrix: V08ProcessPackageIdentityProofBridgeMatrix,
    productTruth: {
      installedAppInventory:
        'manual-required until a real Windows inventory source and evidence refs prove package or executable identity',
      processLineage:
        'manual-required until pid, parent pid where available, executable path, start time, adapter id, and freshness are recorded',
      packageIdentity:
        'unknown apps stay unknown; package, unpackaged executable, launcher, and display names are distinct evidence states',
      publisherSignature:
        'publisher/signature evidence can be valid, invalid, unsigned, unavailable, or manual-required but is never invented',
      rollbackAudit:
        'broad app rollback enforcement is not claimed; audit custody must tie identity refs, policy decision, fallback, and adapter outcome',
    },
    claimUpgradeRefusal: {
      decision: 'rejected',
      requestedUpgrade: 'product-ready-broad-app-blocking-from-process-package-bridge',
      acceptedState: 'manual-required',
      missingArtifactGroups: [
        'real Windows installed app inventory',
        'process lineage tied to executable identity',
        'package or unpackaged executable identity',
        'publisher/signature verification or typed unavailable state',
        'apply, rollback, and audit custody artifacts for the same identity',
      ],
    },
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-process-package-identity-proof-bridge-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function assertBridgeMatrix(bridgeMatrix, preflightMatrix) {
  assertEqual(bridgeMatrix.entries.length, 9, 'bridge entry count');
  const counts = bridgeCounts(bridgeMatrix);
  assertEqual(counts.byBridgeState['manual-required'], 7, 'manual bridge count');
  assertEqual(counts.byBridgeState.unavailable, 1, 'unavailable bridge count');
  assertEqual(counts.byBridgeState['not-claimed'], 1, 'not claimed bridge count');
  assertEqual(counts.byEvidenceClass.inventory, 2, 'inventory class count');
  assertEqual(counts.byEvidenceClass.package, 2, 'package class count');

  const preflightIds = new Set(preflightMatrix.entries.map((entry) => entry.preflightId));
  for (const entry of bridgeMatrix.entries) {
    if (!entry.preflightIds.every((preflightId) => preflightIds.has(preflightId))) {
      throw new Error(`${entry.bridgeId} references missing preflight id.`);
    }
    assertAtLeast(entry.requiredEvidenceArtifacts.length, 3, `${entry.bridgeId} artifacts`);
    assertAtLeast(entry.manualProofSteps.length, 2, `${entry.bridgeId} manual proof steps`);
    assertAtLeast(entry.acceptanceSignals.length, 2, `${entry.bridgeId} acceptance signals`);
    assertAtLeast(entry.unsafeUpgradeExamples.length, 2, `${entry.bridgeId} unsafe examples`);
  }

  const rollback = bridgeMatrix.entries.find((entry) => entry.proofPoint === 'rollback-readiness');
  assertEqual(rollback.bridgeState, 'not-claimed', 'rollback bridge state');
  assertEqual(rollback.proofLevel, 'not-proved', 'rollback proof level');
  proofLabels.push('v0.8.process-package-identity.bridge-counts');
  proofLabels.push('v0.8.process-package-identity.preflight-links');
  proofLabels.push('v0.8.process-package-identity.claim-upgrade-refusal');
}

function assertProofMatrix(matrix) {
  assertArrayIncludes(matrix.requiredCompletedClaimIds, 'v0-8-process-package-identity-proof-bridge', 'required id');

  const scenario = matrix.checkpointScenarios.find(
    (candidate) => candidate.id === 'v0-8-process-package-identity-proof-bridge'
  );
  if (scenario === undefined) {
    throw new Error('Proof matrix is missing V0.8 process package identity proof bridge scenario.');
  }
  assertArrayIncludes(scenario.ciCommands, proofCommand, 'bridge scenario command');

  const claim = matrix.claims.find((candidate) => candidate.id === 'v0-8-process-package-identity-proof-bridge');
  if (claim === undefined) {
    throw new Error('Proof matrix is missing V0.8 process package identity proof bridge claim.');
  }
  assertEqual(claim.runtimeSurfaceCoverage.installedAppInventory.state, 'manual-required', 'inventory state');
  assertEqual(claim.runtimeSurfaceCoverage.rollbackReadiness.state, 'not-claimed', 'rollback state');
  proofLabels.push('proof-matrix.v0-8-process-package-identity-proof-bridge');
}

function bridgeCounts(matrix) {
  return {
    entries: matrix.entries.length,
    byBridgeState: countBy(matrix.entries.map((entry) => entry.bridgeState)),
    byEvidenceClass: countBy(matrix.entries.map((entry) => entry.evidenceClass)),
    byProofPoint: countBy(matrix.entries.map((entry) => entry.proofPoint)),
  };
}

async function runCommand(command, args) {
  commands.push([command, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${command} exited with ${code}`))));
    child.once('error', reject);
  });
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
}

function countBy(values) {
  return values.reduce((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

function assertArrayIncludes(values, expected, label) {
  if (!Array.isArray(values) || !values.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
}

function assertAtLeast(actual, expected, label) {
  if (actual < expected) {
    throw new Error(`${label}: expected at least ${expected}, received ${actual}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
