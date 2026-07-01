import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-cross-platform-enforcement-capability-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-protocol',
    'enforcement_cross_platform_capability_proof',
  ]);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_cross_platform_capability_proof_read_model',
  ]);
  await assertProtocolHarness();

  const { V08CrossPlatformEnforcementCapabilityProofReadModel } =
    await import('@ocentra-parent/schema-domain/v0-8-cross-platform-enforcement-capability-proof');
  const proofMatrix = JSON.parse(await readFile(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json')));
  const summary = summarizeReadModel(V08CrossPlatformEnforcementCapabilityProofReadModel);

  assertReadModel(V08CrossPlatformEnforcementCapabilityProofReadModel, summary);
  assertProofMatrix(proofMatrix);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-cross-platform-enforcement-capability-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      tsContract: 'packages/schema-domain/src/v0-8-cross-platform-enforcement-capability-proof.ts',
      rustProtocol: 'crates/agent-protocol/src/enforcement_cross_platform_capability_proof.rs',
      rustProtocolTest: 'crates/agent-protocol/tests/contract/enforcement_cross_platform_capability_proof_tests.rs',
      rustServiceReadModel: 'crates/agent-service/src/enforcement_cross_platform_capability_proof_read_model.rs',
      rustServiceTest: 'crates/agent-service/tests/unit/enforcement_cross_platform_capability_proof_read_model_tests.rs',
      proofHarness: 'scripts/test/v0-8-cross-platform-enforcement-capability-proof.mjs',
      proofMatrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/v0-8-cross-platform-enforcement-capability-proof-2026-05-30.md',
    },
    counts: summary,
    claimsProved: [
      'Windows owned-process terminate, app time-limit lifecycle, managed-browser boundary, and unmanaged-browser process boundary stay implemented-boundary only',
      'Windows broad installed-app blocking and network/domain blocking remain manual-required',
      'Linux and macOS enforcement adapters remain scaffold-only until platform-specific apply/rollback proof exists',
      'Android device-owner, Android package lifecycle, iOS Family Controls, iOS signing, and iOS TestFlight remain manual-required',
      'Android store and iOS store distribution remain planned and are not privileged enforcement proof',
    ],
    claimsNotProved: [
      'global installed-app blocking',
      'host network or domain blocking',
      'managed browser exact URL enforcement',
      'unmanaged browser URL, active tab, title, page, HTTPS content, or intent certainty',
      'Linux, macOS, Android, or iOS child enforcement support',
      'device-owner policy, Family Controls entitlement, signing, TestFlight, Google Play, or App Store production readiness',
    ],
    manualProofRequirements: [
      'OS-approved installed-app identity, apply, rollback, and audit custody artifacts',
      'host network filter or DNS/VPN adapter apply, rollback, and custody evidence',
      'Linux and macOS service-manager, permissions, package, and adapter apply/rollback artifacts',
      'Android device-owner or managed-profile enrollment, policy apply, package lifecycle, signing, release track, and policy review artifacts',
      'iOS Family Controls and DeviceActivity entitlement approval, Apple signing, device/TestFlight install, App Store Connect, and release artifacts',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-cross-platform-enforcement-capability-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeReadModel(readModel) {
  return {
    entries: readModel.entries.length,
    byPlatform: countBy(readModel.entries.map((entry) => entry.platform)),
    byCapabilityStatus: countBy(readModel.entries.map((entry) => entry.capabilityStatus)),
    byProductClaimState: countBy(readModel.entries.map((entry) => entry.productClaimState)),
    byAdapterExecutionState: countBy(readModel.entries.map((entry) => entry.adapterExecutionState)),
    broadBlockingClaimed: readModel.entries.filter((entry) => entry.broadBlockingClaimed).length,
    exactUrlClaimed: readModel.entries.filter((entry) => entry.exactUrlClaimed).length,
    privilegedMobileClaimed: readModel.entries.filter((entry) => entry.privilegedMobileClaimed).length,
    productionDistributionClaimed: readModel.entries.filter((entry) => entry.productionDistributionClaimed).length,
  };
}

function assertReadModel(readModel, summary) {
  assertEqual(readModel.readModelId, 'v0-8-cross-platform-enforcement-capability-proof', 'read model id');
  assertEqual(summary.entries, 15, 'entry count');
  assertEqual(summary.byPlatform.windows, 6, 'Windows entry count');
  assertEqual(summary.byPlatform.linux, 1, 'Linux entry count');
  assertEqual(summary.byPlatform.macos, 1, 'macOS entry count');
  assertEqual(summary.byPlatform.android, 3, 'Android entry count');
  assertEqual(summary.byPlatform.ios, 4, 'iOS entry count');
  assertEqual(summary.byProductClaimState['implemented-boundary'], 4, 'implemented-boundary count');
  assertEqual(summary.byProductClaimState['manual-required'], 7, 'manual-required count');
  assertEqual(summary.byProductClaimState.scaffold, 2, 'scaffold count');
  assertEqual(summary.byProductClaimState.planned, 2, 'planned count');
  assertEqual(summary.byAdapterExecutionState['executes-real-service'], 4, 'real-service execution count');
  assertEqual(summary.byAdapterExecutionState['returns-manual-required'], 7, 'manual execution count');
  assertEqual(summary.byAdapterExecutionState['scaffold-only'], 2, 'scaffold execution count');
  assertEqual(summary.byAdapterExecutionState['not-invoked'], 2, 'not-invoked execution count');
  assertEqual(summary.broadBlockingClaimed, 0, 'broad blocking claim count');
  assertEqual(summary.exactUrlClaimed, 0, 'exact URL claim count');
  assertEqual(summary.privilegedMobileClaimed, 0, 'privileged mobile claim count');
  assertEqual(summary.productionDistributionClaimed, 0, 'production distribution claim count');

  const surfaces = new Set(readModel.entries.map((entry) => entry.surface));
  for (const expectedSurface of [
    'windows-owned-process-terminate',
    'windows-app-time-limit-lifecycle',
    'windows-managed-browser-boundary',
    'windows-unmanaged-browser-process-boundary',
    'windows-broad-installed-app-blocking',
    'windows-network-domain-blocking',
    'linux-enforcement-adapter-scaffold',
    'macos-enforcement-adapter-scaffold',
    'android-device-owner-policy',
    'android-package-lifecycle',
    'android-store-distribution',
    'ios-family-controls',
    'ios-signing-entitlements',
    'ios-testflight-distribution',
    'ios-store-distribution',
  ]) {
    assertSetHas(surfaces, expectedSurface, 'cross-platform surface coverage');
  }

  proofLabels.push('v0.8.cross-platform-enforcement.read-model');
  proofLabels.push('v0.8.cross-platform-enforcement.no-claim-upgrade');
  proofLabels.push('v0.8.cross-platform-enforcement.manual-mobile-gates');
}

function assertProofMatrix(matrix) {
  const claim = matrix.claims.find((candidate) => candidate.id === 'v0-8-cross-platform-enforcement-capability-proof');
  if (claim === undefined) {
    throw new Error('Proof matrix is missing V0.8 cross-platform enforcement capability proof claim.');
  }

  const scenario = matrix.checkpointScenarios.find(
    (candidate) => candidate.id === 'v0-8-cross-platform-enforcement-capability-proof'
  );
  if (scenario === undefined) {
    throw new Error('Proof matrix is missing V0.8 cross-platform enforcement capability proof checkpoint scenario.');
  }

  assertSetHas(
    new Set(matrix.requiredCompletedClaimIds),
    'v0-8-cross-platform-enforcement-capability-proof',
    'cross-platform enforcement proof claim is required'
  );
  assertSetHas(
    new Set(claim.ciProof.commands),
    'node scripts/test/v0-8-cross-platform-enforcement-capability-proof.mjs',
    'cross-platform enforcement proof command is matrix-listed'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.windowsImplementedBoundaries.state,
    'implemented-boundary',
    'Windows implemented-boundary state'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.broadAppBlocking.state,
    'manual-required',
    'broad app manual-required state'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.androidPrivilegedMobile.state,
    'manual-required',
    'Android manual-required state'
  );
  assertEqual(claim.runtimeSurfaceCoverage.iosStores.state, 'planned', 'iOS store planned state');
  proofLabels.push('proof-matrix.v0-8-cross-platform-enforcement-capability-proof');
}

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/contract.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "contract/enforcement_cross_platform_capability_proof_tests.rs"]',
    'cross-platform enforcement protocol harness registration exists'
  );
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

function assertIncludes(text, expected, label) {
  if (!text.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
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
