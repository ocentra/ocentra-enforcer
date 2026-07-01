import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = 'output/network-plan-proof/adapter-capability-status';
const testRoot = 'test-results/network-adapter-capability-status-proof';
const proofPath = `${testRoot}/proof.json`;
const planProofPath = `${proofRoot}/proof-summary.json`;
const sourceRefs = [
  'crates/agent-protocol/src/windows_adapter_capability.rs',
  'crates/agent-protocol/tests/contract/windows_adapter_capability_tests.rs',
  'crates/agent-service/src/windows_adapter_capability_read_model.rs',
  'crates/agent-service/tests/unit/windows_adapter_capability_read_model_tests.rs',
  'packages/agent-protocol-domain/tests/unit/network-windows-firewall-lab-status.test.ts',
  'packages/agent-protocol-domain/tests/unit/network-windows-wfp-gate-status.test.ts',
  'packages/agent-protocol-domain/tests/unit/network-android-vpnservice-gate-status.test.ts',
  'packages/agent-protocol-domain/tests/unit/network-apple-network-extension-gate-status.test.ts',
  'packages/agent-protocol-domain/tests/unit/network-linux-nftables-lab-status.test.ts',
  'scripts/test/network-adapter-capability-status-proof.mjs',
];
const proofLabels = [
  'rust-contract-windows-adapter-capability-tests',
  'rust-service-windows-adapter-capability-read-model-tests',
  'agent-protocol-domain-network-gate-status-tests',
];

mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const expectedAdapterCapabilityStatus = {
  sourceOfTruth: 'Row52 platform claim manifest proof entries plus locked target-to-status projection',
  targetStatusMapping: {
    WindowsFirewall: 'supported',
    WindowsWfp: 'lab-ready',
    AndroidVpnService: 'physical-device-ready',
    AppleNetworkExtensionMacOs: 'apple-device-ready',
    AppleNetworkExtensionIos: 'apple-device-ready',
    LinuxNftables: 'distro-ready',
    LinuxEbpf: 'distro-ready',
    LinuxTun: 'distro-ready',
  },
  reportableNonReadyStates: ['dry-run', 'research-only', 'manual-required', 'unavailable'],
  authorizationInvariant: 'adapter_authorized_by_proof is accepted only for Row52 ready claim rows',
  requiredRefs: [
    'platform manifest ref',
    'adapter capability refs or missing-artifact follow-ups',
    'OS/device refs',
    'permission or entitlement refs when available',
    'audit refs',
    'service-backed status proof ref',
  ],
  notClaimed: [
    'generic platform support',
    'live adapter execution',
    'enforcement command publication',
    'UI policy authority',
    'broader platform capability UX',
  ],
};

writeFileSync(
  `${proofRoot}/expected-adapter-capability-status.json`,
  `${JSON.stringify(expectedAdapterCapabilityStatus, null, 2)}\n`
);

const [agentProtocolDomainCommand, agentProtocolDomainArgs] = npmCommand([
  'run',
  'test',
  '--workspace',
  '@ocentra-parent/agent-protocol-domain',
  '--',
  'tests/unit/network-windows-firewall-lab-status.test.ts',
  'tests/unit/network-windows-wfp-gate-status.test.ts',
  'tests/unit/network-android-vpnservice-gate-status.test.ts',
  'tests/unit/network-apple-network-extension-gate-status.test.ts',
  'tests/unit/network-linux-nftables-lab-status.test.ts',
]);

const commands = [
  {
    name: 'agent-protocol-windows-adapter-capability-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-protocol', 'windows_adapter_capability'],
  },
  {
    name: 'agent-service-windows-adapter-capability-read-model-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', 'windows_adapter_capability_read_model'],
  },
  {
    name: 'agent-protocol-domain-network-gate-status-tests',
    command: agentProtocolDomainCommand,
    args: agentProtocolDomainArgs,
  },
];
const commandResults = commands.map((entry) => runCommand(entry));
writeFileSync(`${proofRoot}/validation-commands.log`, validationCommandsLog(commandResults));

const proof = {
  schemaVersion: 1,
  proof: 'network-adapter-capability-status',
  sourceBranch: runText('git', ['branch', '--show-current']).trim(),
  proofRoot,
  testRoot,
  sourceRefs,
  sourceFingerprint: `source-tree:${sourceFingerprint(sourceRefs)}`,
  commands: commandResults,
  artifacts: {
    expectedAdapterCapabilityStatus: `${proofRoot}/expected-adapter-capability-status.json`,
    proofSummary: planProofPath,
    testProof: proofPath,
  },
  evidence: {
    rustContractTests: 'crates/agent-protocol/tests/contract/windows_adapter_capability_tests.rs',
    rustServiceReadModelTests: 'crates/agent-service/tests/unit/windows_adapter_capability_read_model_tests.rs',
    networkWindowsFirewallLabStatusTest: 'packages/agent-protocol-domain/tests/unit/network-windows-firewall-lab-status.test.ts',
    networkWindowsWfpGateStatusTest: 'packages/agent-protocol-domain/tests/unit/network-windows-wfp-gate-status.test.ts',
    networkAndroidVpnserviceGateStatusTest: 'packages/agent-protocol-domain/tests/unit/network-android-vpnservice-gate-status.test.ts',
    networkAppleNetworkExtensionGateStatusTest: 'packages/agent-protocol-domain/tests/unit/network-apple-network-extension-gate-status.test.ts',
    networkLinuxNftablesLabStatusTest: 'packages/agent-protocol-domain/tests/unit/network-linux-nftables-lab-status.test.ts',
    sourceRefs,
  },
  provenRows: ['Network feature checklist: Adapter capability status'],
  provenRootGates: [
    'adapter capability status projects existing Row52 platform manifest entries through a locked target-to-status mapping',
    'Windows Firewall ready maps to supported status',
    'Windows WFP ready maps to lab-ready status',
    'Android VpnService ready maps to physical-device-ready status',
    'Apple Network Extension macOS/iOS ready maps to Apple-device-ready status',
    'Linux nftables/eBPF/TUN ready maps to distro-ready status',
    'manual-required and unavailable rows preserve missing-artifact follow-ups',
    'Row52 platform manifest rejects non-ready adapter authorization before status projection',
    'adapter authorization is rejected on dry-run, research-only, manual-required, or unavailable status rows',
    'Rust contract and service tests keep adapter capability status in the current network topology without live host mutation, packet blocking, or policy authority claims',
    'agent-protocol-domain tests consume the generated network gate status contract surfaces for the current topology',
    'generic platform support, live adapter execution, broader platform capability UX, UI policy authority, and enforcement command publication claims are rejected',
  ],
  notClaimed: [
    'live host adapter mutation',
    'packet blocking or host filtering',
    'production platform support',
    'broader platform capability UX beyond the current network drawer',
    'exact URL, page content, private message, search query, or decrypted payload from network-only evidence',
    'policy engine execution',
    'enforcement command publication',
  ],
  proofLabels,
};

const serialized = `${JSON.stringify(proof, null, 2)}\n`;
writeFileSync(planProofPath, serialized);
writeFileSync(proofPath, serialized);
console.log(`network-adapter-capability-status-proof-ok:${proofLabels.join(',')}`);
console.log(`proof=${planProofPath}`);

function runCommand(entry) {
  const commandLine = [entry.command, ...entry.args].join(' ');
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  const log = `${proofRoot}/${safeName(entry.name)}.log`;
  writeFileSync(log, normalizeCommandOutput(`${result.stdout ?? ''}${result.stderr ?? ''}`));
  if (result.status !== 0) {
    throw new Error(`${commandLine} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: commandLine,
    status: result.status,
    log,
  };
}

function safeName(value) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/gu, '-')
    .replace(/^-|-$/gu, '')
    .slice(0, 80);
}

function normalizeCommandOutput(value) {
  const lines = value
    .replace(/\r\n/gu, '\n')
    .replace(/\\/gu, '/')
    .replace(/target\/debug\/deps\/[^\s)]+/gu, 'target/debug/deps/<test-binary>')
    .replace(/\b\d+\.\d+s\b/gu, '<duration>s')
    .replace(/\b\d+\.\d{2}ms\b/gu, '<duration>ms')
    .replace(/target\(s\) in [^\n]+/gu, 'target(s) in <duration>')
    .replace(/finished in [^\n]+/giu, 'finished in <duration>')
    .replace(/Duration [^\n]+/gu, 'Duration <duration>')
    .split('\n')
    .filter((line) => !/^\s+Compiling /u.test(line))
    .filter((line) => !/^\s+Blocking waiting for file lock on build directory$/u.test(line));
  return `${stableRustTestLines(lines).join('\n').trim()}\n`;
}

function stableRustTestLines(lines) {
  const sortedTestLines = lines.filter(isRustTestLine).sort();
  let nextTestLine = 0;
  return lines.map((line) => {
    if (!isRustTestLine(line)) {
      return line;
    }
    const sortedLine = sortedTestLines[nextTestLine];
    nextTestLine += 1;
    return sortedLine;
  });
}

function isRustTestLine(line) {
  return /^test .+ \.\.\. ok$/u.test(line);
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}

function validationCommandsLog(results) {
  return `${results.map((result) => `${result.name}: ${result.command} -> exit ${result.status}; log=${result.log}`).join('\n')}\n`;
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function sourceFingerprint(paths) {
  const hash = createHash('sha256');
  for (const path of paths) {
    hash.update(path);
    hash.update('\0');
    hash.update(readFileSync(path, 'utf8').replace(/\r\n/gu, '\n'));
    hash.update('\0');
  }
  return hash.digest('hex');
}
