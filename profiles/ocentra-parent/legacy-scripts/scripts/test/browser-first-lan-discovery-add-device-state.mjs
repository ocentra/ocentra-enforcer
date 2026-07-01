import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofId = 'browser-first-lan-discovery-add-device-state';
const outputDir = join(repoRoot, 'test-results', proofId);
const proofPath = join(outputDir, 'proof.json');
const checks = [
  {
    label: 'agent-protocol-domain browser add-device contract',
    command: 'npm',
    args: [
      '--workspace',
      '@ocentra-parent/agent-protocol-domain',
      'run',
      'test',
      '--',
      '--run',
      'tests/unit/lan-pairing-browser-add-device-state.test.ts',
    ],
  },
  {
    label: 'Rust protocol browser add-device read model',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-protocol', 'lan_pairing_browser_add_device_state', '--quiet'],
  },
  {
    label: 'Rust service LAN status read model',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', 'lan_pairing_browser_add_device_state', '--quiet'],
  },
];

const commands = [];
const proofLabels = [];

for (const check of checks) {
  console.log(`[browser-first-lan] ${check.label}`);
  commands.push([check.command, ...check.args].join(' '));
  const result = spawnSync(check.command, check.args, {
    cwd: repoRoot,
    shell: true,
    stdio: 'inherit',
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
  proofLabels.push(check.label);
}

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(proof(), null, 2)}\n`);
console.log(`[browser-first-lan] proof harness passed evidence=${proofPath}`);

function proof() {
  return {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: gitHead(),
    proofMode: proofId,
    commands,
    proofLabels,
    evidence: {
      canonicalContract: 'packages/schema-domain/src/agent-lan-add-device.ts',
      agentProtocolContract: 'packages/schema-domain/src/agent-command-event-contracts.ts',
      agentProtocolTest: 'packages/agent-protocol-domain/tests/unit/lan-pairing-browser-add-device-state.test.ts',
      rustProtocolContract: 'crates/agent-protocol/src/lan_pairing_browser_add_device_state.rs',
      rustServiceAdapter: 'crates/agent-service/src/lan_pairing_browser_add_device_state.rs',
      rustServiceTest: 'crates/agent-service/tests/unit/lan_pairing_browser_add_device_state.rs',
      output: relativePath(proofPath),
    },
    readModelBoundary: {
      discoverySource: 'local-service',
      localServiceDiscoveryState: 'service-backed',
      physicalHouseholdLanState: 'manual-required',
      cloudRelayState: 'unavailable',
      trustedDeviceRegistry: 'full-registry-entries-plus-id-summaries',
      selectedDeviceReadiness: 'trust-reachability-ready-for-control',
      routeAuditChecks: ['allowed-origin', 'target-device-match', 'replayed', 'revoked', 'stale', 'offline'],
    },
    claimsProved: [
      'browser-first add-device state has centralized schema-domain contracts consumed by the agent protocol boundary',
      'service-backed LAN status reports local-service, physical manual-required, and cloud unavailable states',
      'trusted-device registry entries are emitted in the read model, not only id summaries',
      'selected-device readiness exposes trust, reachability, stale/offline, and ready-for-control fields',
    ],
    claimsNotProved: [
      'remote desktop or remote control',
      'physical household LAN scan or two-device router/firewall artifact readiness',
      'cloud relay routing, authentication, storage, or remote access',
      'visible portal UI rendering',
    ],
  };
}

function gitHead() {
  const result = spawnSync('git', ['rev-parse', 'HEAD'], {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: true,
  });
  if (result.status !== 0) {
    throw new Error('git rev-parse HEAD failed');
  }
  return result.stdout.trim();
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}
