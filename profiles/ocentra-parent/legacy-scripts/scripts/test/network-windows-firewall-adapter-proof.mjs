import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '38-windows-firewall-adapter');
const testRoot = join('test-results', 'network-windows-firewall-adapter-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-windows-firewall-adapter-proof.json'),
  `${JSON.stringify(
    {
      requiredRefs: [
        'policy-decision-ref',
        'parent-rule-ref',
        'evidence-refs',
        'target-ref',
        'firewall-rule-ref',
        'adapter-authorization-ref',
        'adapter-capability-proof-ref',
        'apply-artifact-ref',
        'result-artifact-ref',
        'rollback-artifact-ref',
        'audit-event-ref',
      ],
      applyReadyState:
        'grade A block policy plus supported Windows Firewall capability and all apply/result/rollback/audit refs',
      dryRunState: 'non-executable and allowed without adapter artifacts',
      manualRequiredState:
        'weak evidence, parent-review policy, manual capability, missing artifact refs, or non-block policy action',
      unavailableState: 'non-executable capability-unavailable state',
      unsupportedClaimsRejected: ['exact URL', 'decrypted payload', 'page content', 'host firewall mutation'],
      rejectedCommandInvocations: ['netsh', 'PowerShell'],
      hostFirewallMutationClaim: false,
      enforcementCommandPublished: false,
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-windows-firewall-adapter-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'windows_firewall_adapter'],
    log: join(proofRoot, 'windows-firewall-adapter-tests.log'),
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
  proof: 'network-windows-firewall-adapter',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedWindowsFirewallAdapterProof: join(proofRoot, 'expected-windows-firewall-adapter-proof.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['38 Windows Firewall adapter'],
  notClaimed: [
    'live Windows Firewall mutation',
    'netsh or PowerShell command invocation',
    'OS permission elevation',
    'host packet filtering',
    'enforcement command publication',
    'decrypted payload or page content inspection',
    'exact URL claim from network-only evidence',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-windows-firewall-adapter-proof-ok:adapter-tests,clippy,source-shape');
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
