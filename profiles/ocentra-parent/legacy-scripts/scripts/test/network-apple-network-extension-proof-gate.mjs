import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '41-apple-network-extension-proof-gate');
const testRoot = join('test-results', 'network-apple-network-extension-proof-gate');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-apple-network-extension-proof-gate.json'),
  `${JSON.stringify(
    {
      requiredRefs: [
        'policy-decision-ref',
        'parent-rule-ref',
        'evidence-refs',
        'bundle-ref',
        'network-extension-ref',
        'developer-team-proof-ref',
        'entitlement-approval-proof-ref',
        'provisioning-profile-proof-ref',
        'signing-proof-ref',
        'device-or-testflight-proof-ref',
        'network-extension-declaration-ref',
        'extension-configuration-proof-ref',
        'rollback-plan-ref',
        'audit-event-ref',
      ],
      optionalSupervisionRef:
        'supervision-or-mdm-proof-ref is required only when supervision or MDM authority is claimed',
      appleEntitlementProofReadyState:
        'grade A block policy plus Apple entitlement, signing, device/TestFlight, and Network Extension proof artifacts',
      researchOnlyState: 'non-executable and allowed without Apple artifacts',
      manualRequiredState:
        'weak evidence, non-block policy, manual capability, missing artifacts, or missing supervision proof when required',
      unavailableState: 'non-executable Apple Network Extension capability-unavailable state',
      unsupportedClaimsRejected: [
        'exact URL',
        'decrypted payload',
        'page content',
        'simulator-only product support',
        'live Network Extension behavior',
        'packet block',
        'app-level control',
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
    name: 'network-apple-network-extension-gate-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', '--test', 'unit'],
    log: join(proofRoot, 'apple-network-extension-gate-tests.log'),
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
  proof: 'network-apple-network-extension-proof-gate',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedAppleNetworkExtensionProofGate: join(proofRoot, 'expected-apple-network-extension-proof-gate.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['41 Apple Network Extension adapter/proof gate'],
  notClaimed: [
    'simulator-only product support',
    'live Apple Network Extension tunnel or content filter',
    'packet blocking',
    'app-level control',
    'Apple entitlement approval without proof',
    'supervision or MDM authority without proof',
    'adapter action authorization',
    'enforcement command publication',
    'decrypted payload or page content inspection',
    'exact URL claim from network-only evidence',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-apple-network-extension-proof-gate-ok:network-extension-gate-tests,clippy');
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
