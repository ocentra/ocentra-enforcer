import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-broad-os-adapter-proof-readiness');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];
let broadAdapterCapability;

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'enforcement_readiness']);
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'enforcement_readiness']);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_execute_reports_manual_required_service_states_for_unwired_adapters',
  ]);

  const { EnforcementBroadAdapterCapability, V08BroadOsAdapterReadinessMatrix } =
    await import('../../packages/schema-domain/dist/enforcement-readiness.js');
  broadAdapterCapability = EnforcementBroadAdapterCapability;
  const readinessMatrix = V08BroadOsAdapterReadinessMatrix;
  assertReadinessMatrix(readinessMatrix);
  const proofMatrix = JSON.parse(await readFile(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json')));
  assertProofMatrix(proofMatrix);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-broad-os-adapter-proof-readiness',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      readinessContract: 'packages/schema-domain/src/enforcement-readiness.ts',
      rustProtocol: 'crates/agent-protocol/src/enforcement_readiness.rs',
      rustCore: 'crates/agent-core/src/enforcement_readiness.rs',
      serviceBoundaryTest: 'crates/agent-service/tests/unit/enforcement_tests.rs',
      proofMatrix: 'docs/expectations/pre-ai-proof-matrix.json',
    },
    counts: readinessCounts(readinessMatrix),
    readinessMatrix,
    productTruth: {
      ownedProcessAndTimeLimit:
        'implemented only for owned-process pid/name guardrails and app time-limit lifecycle where the host supports the adapter',
      broadBlocking:
        'broad app blocking, network/domain blocking, managed-browser service-command exact URL enforcement, admin hardening, anti-tamper, and rollback stay manual-required or unavailable',
      browserEvidence:
        'unmanaged browser proof remains process-only; exact URL, active tab, title, download source, page text, HTTPS content, and intent are not claimed without managed browser or explicit browser integration proof',
    },
    manualProofRequiredBeforeClaimUpgrade: [
      'OS-approved app/package identity, block, rollback, and bypass-resistance artifacts',
      'host network filter and domain block apply/rollback artifacts',
      'managed browser active tab and exact URL enforcement artifacts',
      'admin hardening and anti-tamper artifacts',
      'Android device-owner and iOS Family Controls entitlement/device artifacts before mobile child claims change',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-broad-os-adapter-proof-readiness-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function assertReadinessMatrix(matrix) {
  assertEqual(matrix.entries.length, 9, 'readiness entry count');
  const counts = readinessCounts(matrix);
  assertEqual(counts.byReadinessState.implemented, 3, 'implemented readiness count');
  assertEqual(counts.byReadinessState['manual-required'], 5, 'manual-required readiness count');
  assertEqual(counts.byReadinessState['not-claimed'], 1, 'not-claimed readiness count');
  assertEntry(matrix, broadAdapterCapability.BroadAppBlocking, 'manual-required', 'manual-proof-required');
  assertEntry(matrix, broadAdapterCapability.NetworkDomainBlocking, 'manual-required', 'manual-proof-required');
  assertEntry(matrix, broadAdapterCapability.ManagedBrowserServiceCommand, 'manual-required', 'manual-proof-required');
  assertEntry(matrix, broadAdapterCapability.UnmanagedBrowserExactEvidence, 'not-claimed', 'not-proved');
  proofLabels.push('v0.8.broad-os-adapter-readiness.contract-counts');
  proofLabels.push('v0.8.broad-os-adapter-readiness.claim-boundaries');
}

function assertEntry(matrix, capability, readinessState, proofLevel) {
  const entry = matrix.entries.find((candidate) => candidate.capability === capability);
  if (entry === undefined) {
    throw new Error(`Missing readiness entry: ${capability}`);
  }
  assertEqual(entry.readinessState, readinessState, `${capability} readiness state`);
  assertEqual(entry.proofLevel, proofLevel, `${capability} proof level`);
  if (entry.requiredArtifacts.length === 0 && readinessState !== 'implemented') {
    throw new Error(`${capability} must list required artifacts before claim upgrade.`);
  }
}

function readinessCounts(matrix) {
  return {
    entries: matrix.entries.length,
    byReadinessState: countBy(matrix.entries.map((entry) => entry.readinessState)),
    byRuntimeOwner: countBy(matrix.entries.map((entry) => entry.runtimeOwner)),
    byAdapterKind: countBy(matrix.entries.map((entry) => entry.adapterKind)),
  };
}

function assertProofMatrix(matrix) {
  const scenario = matrix.checkpointScenarios.find(
    (candidate) => candidate.id === 'v0-8-broad-os-adapter-proof-readiness'
  );
  if (scenario === undefined) {
    throw new Error('Proof matrix is missing V0.8 broad OS adapter readiness checkpoint scenario.');
  }
  assertSetHas(
    new Set(scenario.ciCommands),
    'node scripts/test/v0-8-broad-os-adapter-proof-readiness.mjs',
    'readiness command is matrix-listed'
  );
  assertSetHas(
    new Set(matrix.requiredCompletedClaimIds),
    'v0-8-broad-os-adapter-proof-readiness',
    'readiness claim is required'
  );
  const claim = matrix.claims.find((candidate) => candidate.id === 'v0-8-broad-os-adapter-proof-readiness');
  if (claim === undefined) {
    throw new Error('Proof matrix is missing V0.8 broad OS adapter readiness claim.');
  }
  assertEqual(claim.runtimeSurfaceCoverage.broadAppDomainBrowserBlocking.state, 'manual-required', 'broad state');
  assertEqual(claim.runtimeSurfaceCoverage.unmanagedBrowserExactEvidence.state, 'not-claimed', 'browser state');
  proofLabels.push('proof-matrix.v0-8-broad-os-adapter-proof-readiness');
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

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
}

function assertSetHas(set, value, label) {
  if (!set.has(value)) {
    throw new Error(`${label}: missing ${value}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
