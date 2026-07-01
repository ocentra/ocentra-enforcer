import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '40-android-vpnservice-proof-gate');
const testRoot = join('test-results', 'network-android-vpnservice-proof-gate');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-android-vpnservice-proof-gate.json'),
  `${JSON.stringify(
    {
      requiredRefs: [
        'policy-decision-ref',
        'parent-rule-ref',
        'evidence-refs',
        'package-ref',
        'vpn-service-ref',
        'vpn-service-declaration-ref',
        'user-consent-proof-ref',
        'physical-device-proof-ref',
        'package-identity-proof-ref',
        'virtual-interface-proof-ref',
        'traffic-observation-proof-ref',
        'rollback-plan-ref',
        'audit-event-ref',
      ],
      optionalDeviceOwnerRef: 'device-owner-proof-ref is required only when Device Owner authority is claimed',
      physicalDeviceProofReadyState:
        'grade A block policy plus physical-device VpnService consent/interface proof artifacts',
      researchOnlyState: 'non-executable and allowed without device artifacts',
      manualRequiredState:
        'weak evidence, non-block policy, manual capability, missing artifacts, or missing Device Owner proof when required',
      unavailableState: 'non-executable Android VpnService capability-unavailable state',
      unsupportedClaimsRejected: [
        'exact URL',
        'decrypted payload',
        'page content',
        'emulator-only product support',
        'live VPN tunnel',
        'packet block',
        'app/package correlation',
      ],
      adapterApplyAuthorized: false,
      enforcementCommandPublished: false,
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-android-vpnservice-gate-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', '--test', 'unit'],
    log: join(proofRoot, 'android-vpnservice-gate-tests.log'),
  },
  {
    name: 'agent-service-network-bridge-runtime-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-service', '--test', 'network_bridge_runtime'],
    log: join(proofRoot, 'agent-service-network-bridge-runtime-tests.log'),
  },
  {
    name: 'agent-protocol-root-contract-shape-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-parent-agent-protocol', '--test', 'contract'],
    log: join(proofRoot, 'agent-protocol-root-contract-shape-tests.log'),
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'clippy.log'),
  },
];
const commandResults = commands.map(runCommand);

const proof = {
  proof: 'network-android-vpnservice-proof-gate',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedAndroidVpnServiceProofGate: join(proofRoot, 'expected-android-vpnservice-proof-gate.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['40 Android VpnService adapter/proof gate'],
  notClaimed: [
    'emulator-only product support',
    'live Android VpnService tunnel',
    'packet blocking',
    'app/package correlation',
    'Device Owner authority without proof',
    'adapter action authorization',
    'enforcement command publication',
    'decrypted payload or page content inspection',
    'exact URL claim from network-only evidence',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-android-vpnservice-proof-gate-ok:vpnservice-gate-tests,clippy');
console.log(`proof=${join(proofRoot, 'proof-summary.json')}`);

function runCommand(entry) {
  const result = spawnSync(entry.command, entry.args, { encoding: 'utf8', shell: false });
  writeFileSync(entry.log, `${result.stdout ?? ''}${result.stderr ?? ''}`);
  if (result.status !== 0) {
    throw new Error(`${entry.name} failed with exit ${result.status}`);
  }
  return {
    name: entry.name,
    command: [entry.command, ...entry.args].join(' '),
    status: result.status,
    log: entry.log,
  };
}

function runText(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', shell: false });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with exit ${result.status}`);
  }
  return `${result.stdout ?? ''}${result.stderr ?? ''}`;
}
