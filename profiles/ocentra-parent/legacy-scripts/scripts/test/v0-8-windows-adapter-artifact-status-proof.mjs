import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-windows-adapter-artifact-status-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runCommand('node', ['scripts/test/v0-8-windows-adapter-capability-proof.mjs']);
  await runCommand('node', ['scripts/test/v0-8-windows-adapter-artifact-gate.mjs']);
  await runCommand('node', ['scripts/test/v0-8-windows-adapter-artifact-ingestion-proof.mjs']);
  await runCommand('node', ['scripts/test/v0-8-supported-adapter-runtime-proof.mjs']);
  await assertProtocolHarness();

  const { V08SupportedAdapterRuntimeProofReadModel } =
    await import('@ocentra-parent/schema-domain/v0-8-supported-adapter-runtime-proof');
  const artifactEntries = artifactStatusEntries(V08SupportedAdapterRuntimeProofReadModel);

  assertArtifactStatusReadModel(V08SupportedAdapterRuntimeProofReadModel, artifactEntries);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-windows-adapter-artifact-status-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      tsRuntimeContract: 'packages/schema-domain/src/v0-8-supported-adapter-runtime-proof.ts',
      rustProtocol: 'crates/agent-protocol/src/enforcement_supported_adapter_runtime_proof.rs',
      rustProtocolTest:
        'crates/agent-protocol/tests/contract/enforcement_supported_adapter_runtime_proof_tests.rs',
      rustServiceReadModel:
        'crates/agent-service/src/enforcement_api/enforcement_supported_adapter_runtime_proof_read_model.rs',
      rustServiceReadModelTest:
        'crates/agent-service/tests/unit/enforcement_supported_adapter_runtime_proof_read_model_tests.rs',
      capabilityProof: 'test-results/v0-8-windows-adapter-capability-proof/proof.json',
      artifactGateProof: 'test-results/v0-8-windows-adapter-artifact-gate/proof.json',
      artifactIngestionProof: 'test-results/v0-8-windows-adapter-artifact-ingestion-proof/proof.json',
      supportedAdapterRuntimeProof: 'test-results/v0-8-supported-adapter-runtime-proof/proof.json',
    },
    counts: {
      totalRuntimeEntries: V08SupportedAdapterRuntimeProofReadModel.entries.length,
      artifactStatusEntries: artifactEntries.length,
      claimUpgradeFlagsRaised: claimUpgradeCount(artifactEntries),
    },
    artifactStatusRows: artifactEntries.map((entry) => ({
      proofEntryId: entry.proofEntryId,
      adapterCapability: entry.adapterCapability,
      runtimeState: entry.runtimeState,
      adapterResult: entry.adapterResult,
      linkedProofArtifacts: entry.linkedProofArtifacts,
      manualProofRequirements: entry.manualProofRequirements,
    })),
    productTruth: {
      appArtifacts:
        'Windows app artifacts are surfaced as manual-review-only status and do not prove broad installed-app blocking.',
      networkArtifacts:
        'Windows network/domain artifacts are surfaced as manual-review-only status and do not prove DNS, VPN, packet, or domain blocking.',
      managedBrowserArtifacts:
        'Windows managed-browser artifacts are surfaced as manual-review-only status and do not prove active-tab or exact URL enforcement.',
      noClaims:
        'The status rows preserve false broad app, network/domain, exact active-tab, notification, tamper, mobile, and unsupported-platform claim flags.',
    },
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-windows-adapter-artifact-status-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function artifactStatusEntries(readModel) {
  const ids = new Set([
    'windows-broad-installed-app-artifact-status',
    'windows-host-network-domain-artifact-status',
    'windows-managed-browser-artifact-status',
  ]);
  return readModel.entries.filter((entry) => ids.has(entry.proofEntryId));
}

function assertArtifactStatusReadModel(readModel, artifactEntries) {
  assertEqual(readModel.readModelId, 'v0-8-supported-adapter-runtime-proof', 'read model id');
  assertEqual(readModel.entries.length, 13, 'supported adapter runtime entry count');
  assertEqual(artifactEntries.length, 3, 'artifact status entry count');
  assertSetHas(
    new Set(readModel.sourceReadModelIds),
    'v0-8-windows-adapter-artifact-ingestion-proof',
    'source read model ids'
  );

  for (const entry of artifactEntries) {
    assertEqual(entry.runtimeState, 'manual-required', `${entry.proofEntryId} runtime state`);
    assertEqual(entry.adapterResult, 'manual-proof-required', `${entry.proofEntryId} adapter result`);
    assertSetHas(
      new Set(entry.linkedProofCommands),
      'node scripts/test/v0-8-windows-adapter-artifact-ingestion-proof.mjs',
      `${entry.proofEntryId} linked command`
    );
    assertSetHas(
      new Set(entry.linkedProofArtifacts),
      'test-results/v0-8-windows-adapter-artifact-ingestion-proof/proof.json',
      `${entry.proofEntryId} linked artifact`
    );
  }

  assertEqual(claimUpgradeCount(artifactEntries), 0, 'artifact status claim upgrade count');
  assertSetHas(
    new Set(artifactEntries.map((entry) => entry.adapterCapability)),
    'broad-installed-app-artifact-status',
    'artifact status capabilities'
  );
  assertSetHas(
    new Set(artifactEntries.map((entry) => entry.adapterCapability)),
    'host-network-domain-artifact-status',
    'artifact status capabilities'
  );
  assertSetHas(
    new Set(artifactEntries.map((entry) => entry.adapterCapability)),
    'managed-browser-artifact-status',
    'artifact status capabilities'
  );

  proofLabels.push('v0.8.windows-adapter-artifact-status.runtime-visible');
  proofLabels.push('v0.8.windows-adapter-artifact-status.manual-review-only');
  proofLabels.push('v0.8.windows-adapter-artifact-status.no-claim-upgrade');
}

function claimUpgradeCount(entries) {
  return entries.filter(
    (entry) =>
      entry.broadInstalledAppBlockingClaimed ||
      entry.networkDomainBlockingClaimed ||
      entry.exactActiveTabEnforcementClaimed ||
      entry.notificationDeliveryClaimed ||
      entry.tamperHardeningClaimed ||
      entry.mobileControlClaimed ||
      entry.unsupportedPlatformBehaviorClaimed
  ).length;
}

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/contract.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "contract/enforcement_supported_adapter_runtime_proof_tests.rs"]',
    'supported adapter runtime contract harness registration exists'
  );
}

async function runCommand(command, args) {
  const commandLine = [command, ...args].join(' ');
  commands.push(commandLine);
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${commandLine} exited with ${code}`))));
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
