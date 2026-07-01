import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-os-adapter-manual-artifact-gates');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));

  const { V08OsAdapterManualArtifactGateReadModel } =
    await import('@ocentra-parent/schema-domain/v0-8-os-adapter-manual-artifact-gates');
  const proofMatrix = JSON.parse(await readFile(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json')));
  const summary = summarizeReadModel(V08OsAdapterManualArtifactGateReadModel);

  assertReadModel(V08OsAdapterManualArtifactGateReadModel, summary);
  assertProofMatrix(proofMatrix);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-os-adapter-manual-artifact-gates',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      tsContract: 'packages/schema-domain/src/v0-8-os-adapter-manual-artifact-gates.ts',
      proofHarness: 'scripts/test/v0-8-os-adapter-manual-artifact-gates.mjs',
      proofMatrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/v0-8-os-adapter-manual-artifact-gates-2026-05-30.md',
    },
    counts: summary,
    claimsProved: [
      'Windows broad app, process/package identity, owned-process terminate, parent cancel, restart recovery, audit, service permission, package lifecycle, network/domain, and managed-browser exact URL gates require manual artifacts before claims upgrade.',
      'Unmanaged browser exact URL, title, page, download, HTTPS content, and intent evidence remain not-claimed without explicit browser integration.',
      'Android UsageStats, accessibility, VPN/DNS, device-owner, managed-profile, and package lifecycle gates remain mobile-artifact-required.',
      'iOS Family Controls, DeviceActivity, Screen Time, Network Extension, background execution, signing, and TestFlight gates remain mobile-artifact-required.',
      'Linux adapter artifact gates are unavailable in this proof and cannot inherit Windows artifacts.',
      'Windows, Linux, and Android manual gate rows carry opaque host capability probe refs without exposing raw paths, device serials, distro names, or private diagnostics.',
    ],
    claimsNotProved: [
      'product-ready broad app blocking',
      'host network/domain blocking',
      'managed browser exact active-tab URL enforcement',
      'unmanaged browser exact URL/title/page/download evidence',
      'Linux or macOS host adapter support',
      'Android privileged device-owner, managed-profile, VPN/DNS, accessibility, UsageStats, or package lifecycle support',
      'iOS Family Controls, DeviceActivity, Screen Time, Network Extension, background, signing, or TestFlight support',
    ],
    manualProofRequirements: [
      'Windows same-identity app/package, apply, rollback, audit, service permission, and package lifecycle artifacts.',
      'Host network filter or DNS/VPN apply, rollback, and audit custody artifacts.',
      'Managed browser active-tab URL, exact URL apply, rollback, and audit custody artifacts.',
      'Explicit browser integration artifacts for unmanaged URL, title, page, download, HTTPS content, and intent evidence.',
      'Linux and macOS package, service/permission, apply, rollback, and audit artifacts.',
      'Android emulator or physical-device artifacts for UsageStats, accessibility, VPN/DNS, device-owner, managed-profile, package lifecycle, signing, and foreground service behavior.',
      'iOS entitlement, authorization, apply/rollback, signing, TestFlight, install, and physical-device artifacts.',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-os-adapter-manual-artifact-gates-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeReadModel(readModel) {
  return {
    entries: readModel.entries.length,
    byPlatform: countBy(readModel.entries.map((entry) => entry.platform)),
    byGateOutcome: countBy(readModel.entries.map((entry) => entry.gateOutcome)),
    byGateDecision: countBy(readModel.entries.map((entry) => entry.gateDecision)),
    productReadyBlockingClaimed: readModel.entries.filter((entry) => entry.productReadyBlockingClaimed).length,
    broadInstalledAppBlockingClaimed: readModel.entries.filter((entry) => entry.broadInstalledAppBlockingClaimed)
      .length,
    networkDomainBlockingClaimed: readModel.entries.filter((entry) => entry.networkDomainBlockingClaimed).length,
    managedBrowserExactUrlClaimed: readModel.entries.filter((entry) => entry.managedBrowserExactUrlClaimed).length,
    unmanagedBrowserExactEvidenceClaimed: readModel.entries.filter(
      (entry) => entry.unmanagedBrowserExactEvidenceClaimed
    ).length,
    unsupportedPlatformClaimed: readModel.entries.filter((entry) => entry.unsupportedPlatformClaimed).length,
    mobilePrivilegeClaimed: readModel.entries.filter((entry) => entry.mobilePrivilegeClaimed).length,
    hostCapabilityProbeRefRows: readModel.entries.filter((entry) => entry.hostCapabilityProbeRefs.length > 0).length,
  };
}

function assertReadModel(readModel, summary) {
  assertEqual(readModel.readModelId, 'v0-8-os-adapter-manual-artifact-gates', 'read model id');
  assertEqual(summary.entries, 25, 'entry count');
  assertEqual(summary.byPlatform.windows, 11, 'Windows entry count');
  assertEqual(summary.byPlatform.linux, 1, 'Linux entry count');
  assertEqual(summary.byPlatform.macos, 1, 'macOS entry count');
  assertEqual(summary.byPlatform.android, 6, 'Android entry count');
  assertEqual(summary.byPlatform.ios, 6, 'iOS entry count');
  assertEqual(summary.byGateOutcome['manual-required'], 23, 'manual-required outcome count');
  assertEqual(summary.byGateOutcome['not-claimed'], 1, 'not-claimed outcome count');
  assertEqual(summary.byGateOutcome.unavailable, 1, 'unavailable outcome count');
  assertEqual(summary.byGateDecision['requires-host-artifacts'], 11, 'host artifact gate count');
  assertEqual(summary.byGateDecision['requires-mobile-artifacts'], 12, 'mobile artifact gate count');
  assertEqual(summary.byGateDecision['unsupported-surface'], 1, 'unsupported surface gate count');
  assertEqual(summary.byGateDecision['adapter-unavailable'], 1, 'adapter unavailable gate count');
  assertEqual(summary.productReadyBlockingClaimed, 0, 'product-ready claim count');
  assertEqual(summary.broadInstalledAppBlockingClaimed, 0, 'broad app claim count');
  assertEqual(summary.networkDomainBlockingClaimed, 0, 'network/domain claim count');
  assertEqual(summary.managedBrowserExactUrlClaimed, 0, 'managed exact URL claim count');
  assertEqual(summary.unmanagedBrowserExactEvidenceClaimed, 0, 'unmanaged exact evidence claim count');
  assertEqual(summary.unsupportedPlatformClaimed, 0, 'unsupported platform claim count');
  assertEqual(summary.mobilePrivilegeClaimed, 0, 'mobile privilege claim count');
  assertEqual(summary.hostCapabilityProbeRefRows, 18, 'host capability probe ref row count');

  const surfaces = new Set(readModel.entries.map((entry) => entry.surface));
  for (const expectedSurface of [
    'windows-broad-installed-app-identity',
    'windows-network-domain-filter-apply-rollback',
    'windows-managed-browser-exact-url',
    'windows-unmanaged-exact-title-page-download',
    'android-usage-stats',
    'android-accessibility-service',
    'android-vpn-dns',
    'android-device-owner',
    'android-managed-profile',
    'android-package-lifecycle',
    'ios-family-controls',
    'ios-device-activity',
    'ios-screen-time',
    'ios-network-extension',
    'ios-background-execution-signing',
    'ios-testflight-device-install',
  ]) {
    assertSetHas(surfaces, expectedSurface, 'manual artifact gate surface coverage');
  }

  proofLabels.push('v0.8.os-adapter-manual-artifact-gates.read-model');
  proofLabels.push('v0.8.os-adapter-manual-artifact-gates.no-product-ready-upgrade');
  proofLabels.push('v0.8.os-adapter-manual-artifact-gates.mobile-privilege-gates');
  proofLabels.push('v0.8.os-adapter-manual-artifact-gates.host-capability-probe-refs');
}

function assertProofMatrix(matrix) {
  const claim = matrix.claims.find((candidate) => candidate.id === 'v0-8-os-adapter-manual-artifact-gates');
  if (claim === undefined) {
    throw new Error('Proof matrix is missing V0.8 OS adapter manual artifact gates claim.');
  }

  const scenario = matrix.checkpointScenarios.find(
    (candidate) => candidate.id === 'v0-8-os-adapter-manual-artifact-gates'
  );
  if (scenario === undefined) {
    throw new Error('Proof matrix is missing V0.8 OS adapter manual artifact gates checkpoint scenario.');
  }

  assertSetHas(
    new Set(matrix.requiredCompletedClaimIds),
    'v0-8-os-adapter-manual-artifact-gates',
    'OS adapter manual artifact gates proof claim is required'
  );
  assertSetHas(
    new Set(claim.ciProof.commands),
    'node scripts/test/v0-8-os-adapter-manual-artifact-gates.mjs',
    'OS adapter manual artifact gates proof command is matrix-listed'
  );
  assertEqual(claim.runtimeSurfaceCoverage.productReadyBlocking.state, 'not-claimed', 'product-ready blocking state');
  assertEqual(
    claim.runtimeSurfaceCoverage.windowsHostArtifacts.state,
    'manual-required',
    'Windows host artifact state'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.androidPrivilegedArtifacts.state,
    'manual-required',
    'Android privileged artifact state'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.iosEntitlementArtifacts.state,
    'manual-required',
    'iOS entitlement artifact state'
  );
  proofLabels.push('proof-matrix.v0-8-os-adapter-manual-artifact-gates');
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
