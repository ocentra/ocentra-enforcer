import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-broad-os-adapter-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'enforcement_os_adapter_product_proof']);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_os_adapter_product_proof_read_model',
  ]);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_execute_reports_manual_required_service_states_for_unwired_adapters',
  ]);

  const { V08BroadOsAdapterProofReadModel } = await import('@ocentra-parent/schema-domain/v0-8-broad-os-adapter-proof');
  const { V08OsAdapterProductProofReadModel } =
    await import('@ocentra-parent/schema-domain/enforcement-os-adapter-product-proof');
  const { V08BroadOsAdapterReadinessMatrix } = await import('@ocentra-parent/schema-domain/enforcement-readiness');
  const { V08HostAdapterProofPreflightMatrix } =
    await import('@ocentra-parent/schema-domain/enforcement-host-adapter-preflight');

  const broadProofSummary = summarizeBroadProof(V08BroadOsAdapterProofReadModel);
  assertBroadProof(V08BroadOsAdapterProofReadModel, broadProofSummary);
  assertProductProof(V08OsAdapterProductProofReadModel);
  assertReadinessMatrix(V08BroadOsAdapterReadinessMatrix);
  assertPreflightMatrix(V08HostAdapterProofPreflightMatrix);
  const proofMatrix = JSON.parse(await readFile(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json')));
  assertProofMatrix(proofMatrix);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-broad-os-adapter-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      tsContract: 'packages/schema-domain/src/v0-8-broad-os-adapter-proof.ts',
      proofHarness: 'scripts/test/v0-8-broad-os-adapter-proof.mjs',
      proofMatrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/v0-8-broad-os-adapter-proof-2026-05-30.md',
      existingProductProofContract: 'packages/schema-domain/src/enforcement-os-adapter-product-proof.ts',
      existingHostPreflightContract: 'packages/schema-domain/src/enforcement-host-adapter-preflight.ts',
    },
    counts: broadProofSummary,
    claimsProved: [
      'Windows managed-session intervention is recorded as managed-browser-path proof only',
      'Windows owned-process pid/name guardrails and unmanaged browser process terminate/warn boundary stay process-scoped',
      'Windows app time-limit lifecycle proof remains timer/audit/restart/cancel scoped',
      'broad app blocking, network/domain blocking, managed exact URL control, admin hardening, non-Windows, Android, and iOS remain manual-required or unavailable',
      'unmanaged browser exact URL, active tab, title, page, download, HTTPS content, and intent evidence remains not-claimed',
    ],
    claimsNotProved: [
      'global installed-app blocking',
      'host network or domain blocking',
      'managed browser exact URL enforcement',
      'unmanaged browser exact evidence',
      'Linux, macOS, Android, or iOS child enforcement support',
      'admin hardening, anti-tamper, broad rollback, signing, stores, or entitlement-backed support',
    ],
    manualProofRequirements: [
      'OS-approved app/package identity, block apply, rollback, and audit custody artifacts',
      'host network filter or DNS/VPN adapter apply, rollback, and audit custody artifacts',
      'managed active-tab and exact URL apply/rollback/audit artifacts',
      'Linux and macOS host-specific adapter artifacts before platform support upgrades',
      'Android device-owner or managed-profile, UsageStats, accessibility or VPN/DNS, and package lifecycle artifacts',
      'iOS Family Controls, DeviceActivity, Network Extension, signing, entitlement, and TestFlight/device artifacts',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-broad-os-adapter-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeBroadProof(readModel) {
  return {
    entries: readModel.entries.length,
    byPlatform: countBy(readModel.entries.map((entry) => entry.platform)),
    byRuntimeProofState: countBy(readModel.entries.map((entry) => entry.runtimeProofState)),
    byReadinessState: countBy(readModel.entries.map((entry) => entry.readinessState)),
    byTargetSupport: countBy(readModel.entries.map((entry) => entry.targetSupport)),
    claimUpgradeAllowed: readModel.entries.filter((entry) => entry.claimUpgradeAllowed).length,
    broadOsBlockingClaimed: readModel.entries.filter((entry) => entry.broadOsBlockingClaimed).length,
    exactUrlClaimed: readModel.entries.filter((entry) => entry.exactUrlClaimed).length,
  };
}

function assertBroadProof(readModel, summary) {
  assertEqual(summary.entries, 13, 'broad proof entry count');
  assertEqual(summary.byPlatform.windows, 9, 'Windows entry count');
  assertEqual(summary.byPlatform.linux, 1, 'Linux entry count');
  assertEqual(summary.byPlatform.macos, 1, 'macOS entry count');
  assertEqual(summary.byPlatform.android, 1, 'Android entry count');
  assertEqual(summary.byPlatform.ios, 1, 'iOS entry count');
  assertEqual(summary.byRuntimeProofState['real-service-proof'], 4, 'real-service proof count');
  assertEqual(summary.byRuntimeProofState['manual-required'], 7, 'manual-required proof count');
  assertEqual(summary.byRuntimeProofState.unavailable, 1, 'unavailable proof count');
  assertEqual(summary.byRuntimeProofState['not-claimed'], 1, 'not-claimed proof count');
  assertEqual(summary.claimUpgradeAllowed, 0, 'claim upgrade count');
  assertEqual(summary.broadOsBlockingClaimed, 0, 'broad OS blocking claim count');
  assertEqual(summary.exactUrlClaimed, 0, 'exact URL claim count');

  const surfaces = new Set(readModel.entries.map((entry) => entry.surface));
  for (const expectedSurface of [
    'windows-managed-session-intervention',
    'windows-owned-process-guardrail',
    'windows-unmanaged-process-boundary',
    'windows-app-time-limit-lifecycle',
    'windows-broad-installed-app-blocking',
    'windows-network-domain-blocking',
    'windows-managed-browser-exact-url',
    'windows-unmanaged-exact-evidence',
    'windows-admin-rollback-hardening',
    'linux-broad-os-adapter',
    'macos-broad-os-adapter',
    'android-child-os-adapter',
    'ios-child-os-adapter',
  ]) {
    assertSetHas(surfaces, expectedSurface, 'broad proof surface coverage');
  }

  proofLabels.push('v0.8.broad-os-adapter-proof.read-model');
  proofLabels.push('v0.8.broad-os-adapter-proof.no-claim-upgrade');
  proofLabels.push('v0.8.broad-os-adapter-proof.target-platform-boundaries');
}

function assertProductProof(readModel) {
  const claimUpgradeCount = readModel.entries.filter((entry) => entry.claimUpgradeAllowed).length;
  const broadClaimCount = readModel.entries.filter((entry) => entry.broadBlockingClaimed).length;
  const exactUrlClaimCount = readModel.entries.filter((entry) => entry.exactUrlClaimed).length;
  assertEqual(readModel.entries.length, 12, 'product proof entry count');
  assertEqual(claimUpgradeCount, 0, 'product proof claim upgrade count');
  assertEqual(broadClaimCount, 0, 'product proof broad blocking claim count');
  assertEqual(exactUrlClaimCount, 0, 'product proof exact URL claim count');
  proofLabels.push('v0.8.existing-product-proof.claim-boundaries');
}

function assertReadinessMatrix(matrix) {
  const readinessCounts = countBy(matrix.entries.map((entry) => entry.readinessState));
  assertEqual(matrix.entries.length, 9, 'readiness entry count');
  assertEqual(readinessCounts.implemented, 3, 'readiness implemented count');
  assertEqual(readinessCounts['manual-required'], 5, 'readiness manual-required count');
  assertEqual(readinessCounts['not-claimed'], 1, 'readiness not-claimed count');
  proofLabels.push('v0.8.existing-readiness.boundaries');
}

function assertPreflightMatrix(matrix) {
  const states = new Set(matrix.entries.map((entry) => entry.productClaimState));
  assertSetHas(states, 'manual-required', 'preflight manual-required state');
  assertSetHas(states, 'not-claimed', 'preflight not-claimed state');
  proofLabels.push('v0.8.host-adapter-preflight.boundaries');
}

function assertProofMatrix(matrix) {
  const claim = matrix.claims.find((candidate) => candidate.id === 'v0-8-broad-os-adapter-proof');
  if (claim === undefined) {
    throw new Error('Proof matrix is missing V0.8 broad OS adapter proof claim.');
  }

  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === 'v0-8-broad-os-adapter-proof');
  if (scenario === undefined) {
    throw new Error('Proof matrix is missing V0.8 broad OS adapter proof checkpoint scenario.');
  }

  assertSetHas(
    new Set(matrix.requiredCompletedClaimIds),
    'v0-8-broad-os-adapter-proof',
    'broad OS proof claim is required'
  );
  assertSetHas(
    new Set(claim.ciProof.commands),
    'node scripts/test/v0-8-broad-os-adapter-proof.mjs',
    'broad OS proof command is matrix-listed'
  );
  assertEqual(claim.runtimeSurfaceCoverage.broadInstalledAppBlocking.state, 'manual-required', 'broad app state');
  assertEqual(claim.runtimeSurfaceCoverage.linuxBroadAdapter.state, 'unavailable', 'Linux broad adapter state');
  assertEqual(claim.runtimeSurfaceCoverage.unmanagedBrowserExactEvidence.state, 'not-claimed', 'browser exact state');
  proofLabels.push('proof-matrix.v0-8-broad-os-adapter-proof');
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
