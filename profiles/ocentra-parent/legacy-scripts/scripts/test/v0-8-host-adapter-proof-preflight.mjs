import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-host-adapter-proof-preflight');
const proofPath = join(outputDir, 'proof.json');
const proofCommand = 'node scripts/test/v0-8-host-adapter-proof-preflight.mjs';
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const { EnforcementBroadAdapterCapability, V08BroadOsAdapterReadinessMatrix } =
    await import('../../packages/schema-domain/dist/enforcement-readiness.js');
  const { V08HostAdapterProofPreflightMatrix } =
    await import('../../packages/schema-domain/dist/enforcement-host-adapter-preflight.js');
  const proofMatrix = JSON.parse(await readFile(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json')));

  assertPreflightMatrix(V08HostAdapterProofPreflightMatrix, V08BroadOsAdapterReadinessMatrix);
  assertProofMatrix(proofMatrix);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-host-adapter-proof-preflight',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      preflightContract: 'packages/schema-domain/src/enforcement-host-adapter-preflight.ts',
      readinessContract: 'packages/schema-domain/src/enforcement-readiness.ts',
      proofMatrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/v0-8-host-adapter-proof-preflight-2026-05-29.md',
    },
    counts: preflightCounts(V08HostAdapterProofPreflightMatrix),
    preflightMatrix: V08HostAdapterProofPreflightMatrix,
    productTruth: {
      broadAppBlocking:
        'manual-required until OS-approved package identity, block apply, rollback, and audit artifacts exist',
      networkDomainBlocking:
        'manual-required until a host network filter or DNS/VPN adapter proves metadata-only apply and rollback',
      managedBrowser:
        'manual-required unless managed browser boundary ties active document evidence to command enforcement',
      unmanagedBrowser:
        'not-claimed for exact URL, tab, title, download, page, HTTPS content, or intent without explicit integration',
      rollbackAntiTamper:
        'manual-required until admin hardening, anti-tamper, rollback, and bypass-resistance artifacts exist',
    },
    claimUpgradeRefusal: claimUpgradeRefusal(EnforcementBroadAdapterCapability),
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-host-adapter-proof-preflight-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function assertPreflightMatrix(preflightMatrix, readinessMatrix) {
  assertEqual(preflightMatrix.entries.length, 6, 'preflight entry count');
  const counts = preflightCounts(preflightMatrix);
  assertEqual(counts.byProductClaimState['manual-required'], 5, 'manual-required preflight count');
  assertEqual(counts.byProductClaimState['not-claimed'], 1, 'not-claimed preflight count');
  assertEqual(counts.byPreflightStatus['blocked-by-missing-artifact'], 5, 'blocked preflight count');
  assertEqual(counts.byPreflightStatus['not-claimable-from-current-proof'], 1, 'not claimable count');

  const readinessById = new Map(readinessMatrix.entries.map((entry) => [entry.readinessId, entry]));
  for (const entry of preflightMatrix.entries) {
    const readiness = readinessById.get(entry.readinessId);
    if (readiness === undefined) {
      throw new Error(`Missing linked readiness entry: ${entry.readinessId}`);
    }
    assertEqual(entry.capability, readiness.capability, `${entry.preflightId} capability link`);
    assertAtLeast(entry.requiredEvidenceArtifacts.length, 3, `${entry.preflightId} artifacts`);
    assertAtLeast(entry.manualProofSteps.length, 2, `${entry.preflightId} manual steps`);
    assertAtLeast(entry.unsafeUpgradeExamples.length, 2, `${entry.preflightId} unsafe examples`);
  }

  proofLabels.push('v0.8.host-adapter-preflight.contract-counts');
  proofLabels.push('v0.8.host-adapter-preflight.readiness-links');
  proofLabels.push('v0.8.host-adapter-preflight.claim-upgrade-refusal');
}

function assertProofMatrix(matrix) {
  assertArrayIncludes(matrix.requiredCompletedClaimIds, 'v0-8-host-adapter-proof-preflight', 'required claim id');

  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === 'v0-8-host-adapter-proof-preflight');
  if (scenario === undefined) {
    throw new Error('Proof matrix is missing V0.8 host adapter proof preflight scenario.');
  }
  assertArrayIncludes(scenario.ciCommands, proofCommand, 'preflight scenario command');

  const claim = matrix.claims.find((candidate) => candidate.id === 'v0-8-host-adapter-proof-preflight');
  if (claim === undefined) {
    throw new Error('Proof matrix is missing V0.8 host adapter proof preflight claim.');
  }
  assertEqual(claim.runtimeSurfaceCoverage.processPackageIdentity.state, 'manual-required', 'identity state');
  assertEqual(claim.runtimeSurfaceCoverage.unmanagedBrowserExactEvidence.state, 'not-claimed', 'unmanaged state');
  proofLabels.push('proof-matrix.v0-8-host-adapter-proof-preflight');
}

function preflightCounts(matrix) {
  return {
    entries: matrix.entries.length,
    byProductClaimState: countBy(matrix.entries.map((entry) => entry.productClaimState)),
    byPreflightStatus: countBy(matrix.entries.map((entry) => entry.preflightStatus)),
    byPreflightGate: countBy(matrix.entries.map((entry) => entry.preflightGate)),
  };
}

function claimUpgradeRefusal(capability) {
  return {
    decision: 'rejected',
    requestedUpgrade: 'product-ready-broad-host-adapter-enforcement',
    acceptedCapabilities: {
      [capability.BroadAppBlocking]: 'manual-required',
      [capability.NetworkDomainBlocking]: 'manual-required',
      [capability.ManagedBrowserServiceCommand]: 'manual-required',
      [capability.ManagedBrowserExactUrlControl]: 'manual-required',
      [capability.UnmanagedBrowserExactEvidence]: 'not-claimed',
      [capability.AdminAntiTamperRollback]: 'manual-required',
    },
    missingArtifactGroups: [
      'process-package identity and installed app inventory',
      'network filter or DNS/VPN adapter apply and rollback proof',
      'managed browser active document and exact URL enforcement proof',
      'explicit browser integration proof for unmanaged exact URL evidence',
      'admin hardening, anti-tamper, rollback, and bypass-resistance proof',
    ],
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
