import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-broad-os-adapter-runtime-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand(...npmCommand(['run', 'build', '--workspace', '@ocentra-parent/schema-domain']));
  await runCommand('node', ['scripts/test/v0-8-broad-os-adapter-proof.mjs']);
  await runCommand('node', ['scripts/test/v0-8-browser-domain-adapter-proof.mjs']);
  await runCommand('node', ['scripts/test/v0-8-os-adapter-manual-artifact-gates.mjs']);

  const { V08BroadOsAdapterRuntimeProofReadModel } =
    await import('@ocentra-parent/schema-domain/v0-8-broad-os-adapter-runtime-proof');
  const { V08BroadOsAdapterProofReadModel } = await import('@ocentra-parent/schema-domain/v0-8-broad-os-adapter-proof');
  const { V08BrowserDomainAdapterProofReadModel } =
    await import('@ocentra-parent/schema-domain/v0-8-browser-domain-adapter-proof');
  const { V08OsAdapterManualArtifactGateReadModel } =
    await import('@ocentra-parent/schema-domain/v0-8-os-adapter-manual-artifact-gates');
  const proofMatrix = JSON.parse(await readFile(join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json')));
  const summary = summarizeReadModel(V08BroadOsAdapterRuntimeProofReadModel);

  assertReadModel(V08BroadOsAdapterRuntimeProofReadModel, summary);
  assertSourceReadModels(
    V08BroadOsAdapterRuntimeProofReadModel,
    V08BroadOsAdapterProofReadModel,
    V08BrowserDomainAdapterProofReadModel,
    V08OsAdapterManualArtifactGateReadModel
  );
  assertProofMatrix(proofMatrix);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-broad-os-adapter-runtime-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      tsContract: 'packages/schema-domain/src/v0-8-broad-os-adapter-runtime-proof.ts',
      proofHarness: 'scripts/test/v0-8-broad-os-adapter-runtime-proof.mjs',
      sourceBroadProof: 'test-results/v0-8-broad-os-adapter-proof/proof.json',
      sourceBrowserDomainProof: 'test-results/v0-8-browser-domain-adapter-proof/proof.json',
      sourceManualArtifactGates: 'test-results/v0-8-os-adapter-manual-artifact-gates/proof.json',
      proofMatrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/v0-8-broad-os-adapter-runtime-proof-2026-05-30.md',
    },
    counts: summary,
    claimsProved: [
      'Windows owned-process pid/name guardrails and app timer lifecycle are final-pass runtime boundaries only.',
      'Windows managed-browser session intervention is a runtime boundary only and does not prove exact URL enforcement.',
      'Broad installed-app blocking, network/domain blocking, managed exact URL control, macOS, Android, and iOS remain manual-required.',
      'Linux host runtime support remains unavailable in this final pass.',
      'Unmanaged browser exact URL, active tab, title, page, download, HTTPS content, and intent evidence remains not-claimed.',
    ],
    claimsNotProved: [
      'global installed-app blocking',
      'host network or domain blocking',
      'managed browser exact active-tab URL enforcement',
      'unmanaged browser exact evidence',
      'Linux or macOS host enforcement support',
      'Android device-owner, managed-profile, VPN/DNS, accessibility, UsageStats, or package lifecycle support',
      'iOS Family Controls, DeviceActivity, Screen Time, Network Extension, signing, TestFlight, or device support',
    ],
    manualProofRequirements: [
      'Windows app identity, apply, rollback, and audit custody artifacts before broad app claims upgrade.',
      'Host DNS/filter/VPN apply, rollback, and audit custody artifacts before network/domain claims upgrade.',
      'Managed browser active-tab, exact URL apply, rollback, and audit artifacts before exact URL claims upgrade.',
      'Explicit unmanaged browser integration artifacts for URL, title, page, download, HTTPS content, and intent evidence.',
      'Linux and macOS package, permission, service, apply, rollback, and audit artifacts.',
      'Android device-owner or managed-profile, UsageStats, accessibility or VPN/DNS, and package lifecycle artifacts.',
      'iOS Family Controls, DeviceActivity, Screen Time, Network Extension, entitlement, signing, and TestFlight/device artifacts.',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-broad-os-adapter-runtime-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeReadModel(readModel) {
  return {
    entries: readModel.entries.length,
    byPlatform: countBy(readModel.entries.map((entry) => entry.platform)),
    byProductClaimState: countBy(readModel.entries.map((entry) => entry.productClaimState)),
    byEvidenceState: countBy(readModel.entries.map((entry) => entry.evidenceState)),
    broadInstalledAppBlockingClaimed: readModel.entries.filter((entry) => entry.broadInstalledAppBlockingClaimed)
      .length,
    networkDomainBlockingClaimed: readModel.entries.filter((entry) => entry.networkDomainBlockingClaimed).length,
    managedBrowserExactUrlClaimed: readModel.entries.filter((entry) => entry.managedBrowserExactUrlClaimed).length,
    unmanagedBrowserExactEvidenceClaimed: readModel.entries.filter(
      (entry) => entry.unmanagedBrowserExactEvidenceClaimed
    ).length,
    unsupportedPlatformClaimed: readModel.entries.filter((entry) => entry.unsupportedPlatformClaimed).length,
    mobilePrivilegeClaimed: readModel.entries.filter((entry) => entry.mobilePrivilegeClaimed).length,
  };
}

function assertReadModel(readModel, summary) {
  assertEqual(readModel.readModelId, 'v0-8-broad-os-adapter-runtime-proof', 'read model id');
  assertEqual(summary.entries, 10, 'entry count');
  assertEqual(summary.byPlatform.windows, 6, 'Windows entry count');
  assertEqual(summary.byPlatform.linux, 1, 'Linux entry count');
  assertEqual(summary.byPlatform.macos, 1, 'macOS entry count');
  assertEqual(summary.byPlatform.android, 1, 'Android entry count');
  assertEqual(summary.byPlatform.ios, 1, 'iOS entry count');
  assertEqual(summary.byProductClaimState['implemented-boundary'], 2, 'implemented-boundary count');
  assertEqual(summary.byProductClaimState['manual-required'], 6, 'manual-required count');
  assertEqual(summary.byProductClaimState.unavailable, 1, 'unavailable count');
  assertEqual(summary.byProductClaimState['not-claimed'], 1, 'not-claimed count');
  assertEqual(summary.byEvidenceState['composite-runtime-proof'], 2, 'composite runtime proof count');
  assertEqual(summary.byEvidenceState['manual-artifact-required'], 6, 'manual artifact required count');
  assertEqual(summary.byEvidenceState['target-unavailable'], 1, 'target unavailable count');
  assertEqual(summary.byEvidenceState['not-implemented'], 1, 'not implemented count');
  assertEqual(summary.broadInstalledAppBlockingClaimed, 0, 'broad app claim count');
  assertEqual(summary.networkDomainBlockingClaimed, 0, 'network/domain claim count');
  assertEqual(summary.managedBrowserExactUrlClaimed, 0, 'managed exact URL claim count');
  assertEqual(summary.unmanagedBrowserExactEvidenceClaimed, 0, 'unmanaged exact evidence claim count');
  assertEqual(summary.unsupportedPlatformClaimed, 0, 'unsupported platform claim count');
  assertEqual(summary.mobilePrivilegeClaimed, 0, 'mobile privilege claim count');
  proofLabels.push('v0.8.broad-os-adapter-runtime-proof.read-model');
  proofLabels.push('v0.8.broad-os-adapter-runtime-proof.no-claim-upgrade');
}

function assertSourceReadModels(runtimeProof, broadProof, browserDomainProof, manualArtifactGates) {
  const sourceIds = new Set(runtimeProof.sourceReadModelIds);
  for (const expectedSource of [
    broadProof.readModelId,
    browserDomainProof.readModelId,
    manualArtifactGates.readModelId,
    'v0-8-os-adapter-product-proof',
  ]) {
    assertSetHas(sourceIds, expectedSource, 'runtime proof source read model coverage');
  }
  proofLabels.push('v0.8.broad-os-adapter-runtime-proof.source-proof-coverage');
}

function assertProofMatrix(matrix) {
  const claim = matrix.claims.find((candidate) => candidate.id === 'v0-8-broad-os-adapter-runtime-proof');
  if (claim === undefined) {
    throw new Error('Proof matrix is missing V0.8 broad OS adapter runtime proof claim.');
  }

  const scenario = matrix.checkpointScenarios.find(
    (candidate) => candidate.id === 'v0-8-broad-os-adapter-runtime-proof'
  );
  if (scenario === undefined) {
    throw new Error('Proof matrix is missing V0.8 broad OS adapter runtime proof checkpoint scenario.');
  }

  assertSetHas(
    new Set(matrix.requiredCompletedClaimIds),
    'v0-8-broad-os-adapter-runtime-proof',
    'broad OS adapter runtime proof claim is required'
  );
  assertSetHas(
    new Set(claim.ciProof.commands),
    'node scripts/test/v0-8-broad-os-adapter-runtime-proof.mjs',
    'broad OS adapter runtime proof command is matrix-listed'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.broadInstalledAppRuntime.state,
    'manual-required',
    'broad installed-app runtime state'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.networkDomainRuntime.state,
    'manual-required',
    'network/domain runtime state'
  );
  assertEqual(
    claim.runtimeSurfaceCoverage.unmanagedBrowserExactEvidence.state,
    'not-claimed',
    'unmanaged browser exact evidence state'
  );
  assertEqual(claim.runtimeSurfaceCoverage.linuxHostRuntime.state, 'unavailable', 'Linux runtime state');
  proofLabels.push('proof-matrix.v0-8-broad-os-adapter-runtime-proof');
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
