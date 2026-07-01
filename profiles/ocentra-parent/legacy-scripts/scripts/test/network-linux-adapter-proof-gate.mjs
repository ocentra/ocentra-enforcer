import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '42-linux-adapter-proof-gate');
const testRoot = join('test-results', 'network-linux-adapter-proof-gate');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-linux-adapter-proof-gate.json'),
  `${JSON.stringify(
    {
      requiredRefs: [
        'policy-decision-ref',
        'parent-rule-ref',
        'evidence-refs',
        'distro-ref',
        'kernel-ref',
        'distro-kernel-proof-ref',
        'permission-proof-ref',
        'adapter-api-capability-proof-ref',
        'adapter-plan-proof-ref',
        'service-manager-scope-proof-ref',
        'rollback-plan-ref',
        'lab-result-artifact-ref',
        'audit-event-ref',
      ],
      supportedAdapterKinds: ['nftables', 'eBPF', 'TUN'],
      distroProofReadyState:
        'grade A block policy plus distro/kernel, permission, adapter API, rollback, and audit proof artifacts',
      researchOnlyState: 'non-executable and allowed without Linux artifacts',
      manualRequiredState: 'weak evidence, non-block policy, manual capability, or missing artifacts',
      unavailableState: 'non-executable Linux adapter capability-unavailable state',
      unsupportedClaimsRejected: [
        'exact URL',
        'decrypted payload',
        'page content',
        'generic Linux support',
        'live adapter install',
        'packet filtering',
        'kernel hook load',
        'TUN interface mutation',
        'service-manager install',
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
    name: 'network-linux-adapter-gate-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'linux_adapter_gate'],
    log: join(proofRoot, 'linux-adapter-gate-tests.log'),
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'clippy.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs'],
    log: join(proofRoot, 'source-shape.log'),
  },
];
const commandResults = commands.map(runCommand);

const proof = {
  proof: 'network-linux-adapter-proof-gate',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedLinuxAdapterProofGate: join(proofRoot, 'expected-linux-adapter-proof-gate.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['42 Linux nftables/eBPF/TUN adapter/proof gate'],
  notClaimed: [
    'generic Linux support',
    'live nftables/eBPF/TUN install',
    'packet filtering',
    'kernel hook loading',
    'TUN interface mutation',
    'service-manager install',
    'adapter action authorization',
    'enforcement command publication',
    'decrypted payload or page content inspection',
    'exact URL claim from network-only evidence',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-linux-adapter-proof-gate-ok:linux-adapter-gate-tests,clippy,source-shape');
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
