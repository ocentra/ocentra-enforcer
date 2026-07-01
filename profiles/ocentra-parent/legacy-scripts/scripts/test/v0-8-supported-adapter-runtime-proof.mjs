import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'v0-8-supported-adapter-runtime-proof');
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
    'enforcement_supported_adapter_runtime_proof',
  ]);
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_supported_adapter_runtime_proof',
  ]);
  await assertProtocolHarness();

  const { V08SupportedAdapterRuntimeProofReadModel } =
    await import('@ocentra-parent/schema-domain/v0-8-supported-adapter-runtime-proof');
  const summary = summarizeReadModel(V08SupportedAdapterRuntimeProofReadModel);

  assertReadModel(V08SupportedAdapterRuntimeProofReadModel, summary);

  const proof = {
    schemaVersion: 1,
    proofMode: 'v0-8-supported-adapter-runtime-proof',
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    commands,
    proofLabels,
    evidence: {
      generatedRuntimeContract: 'packages/schema-domain/src/v0-8-supported-adapter-runtime-proof.ts',
      rustProtocol: 'crates/agent-protocol/src/enforcement_supported_adapter_runtime_proof.rs',
      rustProtocolTest:
        'crates/agent-protocol/tests/contract/enforcement_supported_adapter_runtime_proof_tests.rs',
      rustServiceReadModel:
        'crates/agent-service/src/enforcement_api/enforcement_supported_adapter_runtime_proof_read_model.rs',
      rustServiceTest:
        'crates/agent-service/tests/unit/enforcement_supported_adapter_runtime_proof_read_model_tests.rs',
      rustServiceCommand: 'agent.enforcement.supported-adapter-runtime-proof.get',
      rustServiceEvent: 'agent.enforcement.supported-adapter-runtime-proof.reported',
      proofHarness: 'scripts/test/v0-8-supported-adapter-runtime-proof.mjs',
    },
    counts: summary,
    claimsProved: [
      'Supported adapter runtime proof is contract-backed across generated schema-domain output and Rust protocol structs',
      'Service WebSocket command returns a 13-entry supported adapter proof read model',
      'Windows app/game owned-process time-limit and Windows network observe-only policy handoff are implemented-boundary',
      'Windows app, network/domain, and managed-browser artifact status rows link capability, gate, and ingestion proof without claim upgrades',
      'Broad installed-app blocking, host network/domain blocking, mobile control, and exact active-tab enforcement remain gated',
      'Linux support is unavailable, macOS support is unsupported, and permission/dependency loss is degraded',
      'No broad app, network/domain, exact active-tab, notification, tamper, mobile, or unsupported-platform claim flag is upgraded',
    ],
    claimsNotProved: [
      'global installed-app blocking',
      'host network or domain blocking',
      'managed active-tab exact URL enforcement',
      'notification delivery',
      'tamper hardening or uninstall resistance',
      'Linux, macOS, Android, or iOS enforcement support',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-8-supported-adapter-runtime-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${relative(repoRoot, proofPath)}`);
}

function summarizeReadModel(readModel) {
  return {
    entries: readModel.entries.length,
    byPlatform: countBy(readModel.entries.map((entry) => entry.platform)),
    byRuntimeState: countBy(readModel.entries.map((entry) => entry.runtimeState)),
    byAdapterResult: countBy(readModel.entries.map((entry) => entry.adapterResult)),
    broadInstalledAppBlockingClaimed: readModel.entries.filter((entry) => entry.broadInstalledAppBlockingClaimed)
      .length,
    networkDomainBlockingClaimed: readModel.entries.filter((entry) => entry.networkDomainBlockingClaimed).length,
    exactActiveTabEnforcementClaimed: readModel.entries.filter((entry) => entry.exactActiveTabEnforcementClaimed)
      .length,
    notificationDeliveryClaimed: readModel.entries.filter((entry) => entry.notificationDeliveryClaimed).length,
    tamperHardeningClaimed: readModel.entries.filter((entry) => entry.tamperHardeningClaimed).length,
    mobileControlClaimed: readModel.entries.filter((entry) => entry.mobileControlClaimed).length,
    unsupportedPlatformBehaviorClaimed: readModel.entries.filter((entry) => entry.unsupportedPlatformBehaviorClaimed)
      .length,
  };
}

function assertReadModel(readModel, summary) {
  assertEqual(readModel.readModelId, 'v0-8-supported-adapter-runtime-proof', 'read model id');
  assertEqual(summary.entries, 13, 'entry count');
  assertEqual(summary.byPlatform.windows, 9, 'Windows entry count');
  assertEqual(summary.byPlatform.linux, 1, 'Linux entry count');
  assertEqual(summary.byPlatform.macos, 1, 'macOS entry count');
  assertEqual(summary.byPlatform.android, 1, 'Android entry count');
  assertEqual(summary.byPlatform.ios, 1, 'iOS entry count');
  assertEqual(summary.byRuntimeState['implemented-boundary'], 2, 'implemented-boundary count');
  assertEqual(summary.byRuntimeState['manual-required'], 7, 'manual-required count');
  assertEqual(summary.byRuntimeState['not-claimed'], 1, 'not-claimed count');
  assertEqual(summary.byRuntimeState.degraded, 1, 'degraded count');
  assertEqual(summary.byRuntimeState.unavailable, 1, 'unavailable count');
  assertEqual(summary.byRuntimeState.unsupported, 1, 'unsupported count');
  assertEqual(summary.broadInstalledAppBlockingClaimed, 0, 'broad app claim count');
  assertEqual(summary.networkDomainBlockingClaimed, 0, 'network/domain claim count');
  assertEqual(summary.exactActiveTabEnforcementClaimed, 0, 'exact active-tab claim count');
  assertEqual(summary.notificationDeliveryClaimed, 0, 'notification claim count');
  assertEqual(summary.tamperHardeningClaimed, 0, 'tamper hardening claim count');
  assertEqual(summary.mobileControlClaimed, 0, 'mobile control claim count');
  assertEqual(summary.unsupportedPlatformBehaviorClaimed, 0, 'unsupported platform behavior claim count');

  for (const sourceReadModelId of [
    'v0-8-broad-os-adapter-runtime-proof',
    'v0-8-enforcement-policy-dispatch-proof',
    'v0-8-enforcement-product-control-spine',
    'network-flow-read-model',
    'v0-8-windows-adapter-capability-proof',
    'v0-8-windows-adapter-artifact-gate',
    'v0-8-windows-adapter-artifact-ingestion-proof',
  ]) {
    assertSetHas(new Set(readModel.sourceReadModelIds), sourceReadModelId, 'source read model ids');
  }

  assertEntry(readModel, 'windows-app-game-owned-process-time-limit', {
    runtimeState: 'implemented-boundary',
    adapterResult: 'supported-boundary-proved',
  });
  assertEntry(readModel, 'windows-network-flow-observe-policy-handoff', {
    runtimeState: 'implemented-boundary',
    adapterResult: 'supported-boundary-proved',
  });
  assertEntry(readModel, 'windows-broad-installed-app-blocking-manual-gate', {
    runtimeState: 'manual-required',
    adapterResult: 'manual-proof-required',
  });
  assertEntry(readModel, 'windows-host-network-domain-blocking-manual-gate', {
    runtimeState: 'manual-required',
    adapterResult: 'manual-proof-required',
  });
  assertEntry(readModel, 'windows-broad-installed-app-artifact-status', {
    runtimeState: 'manual-required',
    adapterResult: 'manual-proof-required',
  });
  assertEntry(readModel, 'windows-host-network-domain-artifact-status', {
    runtimeState: 'manual-required',
    adapterResult: 'manual-proof-required',
  });
  assertEntry(readModel, 'windows-managed-browser-artifact-status', {
    runtimeState: 'manual-required',
    adapterResult: 'manual-proof-required',
  });
  assertEntry(readModel, 'windows-managed-exact-active-tab-not-claimed', {
    runtimeState: 'not-claimed',
    adapterResult: 'not-claimed',
  });
  assertEntry(readModel, 'linux-host-adapter-unavailable', {
    runtimeState: 'unavailable',
    adapterResult: 'target-unavailable',
  });
  assertEntry(readModel, 'macos-host-adapter-unsupported', {
    runtimeState: 'unsupported',
    adapterResult: 'unsupported-platform',
  });

  proofLabels.push('v0.8.supported-adapter-runtime-proof.service-command');
  proofLabels.push('v0.8.supported-adapter-runtime-proof.runtime-read-model');
  proofLabels.push('v0.8.supported-adapter-runtime-proof.windows-artifact-status');
  proofLabels.push('v0.8.supported-adapter-runtime-proof.supported-boundaries');
  proofLabels.push('v0.8.supported-adapter-runtime-proof.no-claim-upgrade');
  proofLabels.push('v0.8.supported-adapter-runtime-proof.platform-boundaries');
}

function assertEntry(readModel, proofEntryId, expected) {
  const entry = readModel.entries.find((candidate) => candidate.proofEntryId === proofEntryId);
  if (entry === undefined) {
    throw new Error(`missing supported adapter proof entry ${proofEntryId}`);
  }
  assertEqual(entry.runtimeState, expected.runtimeState, `${proofEntryId} runtime state`);
  assertEqual(entry.adapterResult, expected.adapterResult, `${proofEntryId} adapter result`);
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

async function assertProtocolHarness() {
  const harness = await readFile(join(repoRoot, 'crates/agent-protocol/tests/contract.rs'), 'utf8');
  assertIncludes(
    harness,
    '#[path = "contract/enforcement_supported_adapter_runtime_proof_tests.rs"]',
    'supported adapter runtime contract harness registration exists'
  );
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
