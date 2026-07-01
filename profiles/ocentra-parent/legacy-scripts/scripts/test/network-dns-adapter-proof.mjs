import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '37-dns-proxy-block-redirect-adapter');
const testRoot = join('test-results', 'network-dns-adapter-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-dns-adapter-proof.json'),
  `${JSON.stringify(
    {
      requiredRefs: [
        'policy-decision-ref',
        'parent-rule-ref',
        'evidence-refs',
        'adapter-authorization-ref',
        'adapter-capability-proof-ref',
        'apply-artifact-ref',
        'result-artifact-ref',
        'rollback-artifact-ref',
        'audit-event-ref',
      ],
      applyReadyState: 'grade A block policy plus supported DNS capability and all apply/result/rollback/audit refs',
      dryRunState: 'non-executable and allowed without adapter artifacts',
      manualRequiredState: 'weak evidence, parent-review policy, manual capability, or missing artifact refs',
      unavailableState: 'non-executable capability-unavailable state',
      unsupportedClaimsRejected: ['exact URL', 'decrypted payload', 'page content'],
      hostMutationClaim: false,
      enforcementCommandPublished: false,
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-dns-adapter-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', '--test', 'unit'],
    log: join(proofRoot, 'dns-adapter-tests.log'),
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
  proof: 'network-dns-adapter',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedDnsAdapterProof: join(proofRoot, 'expected-dns-adapter-proof.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['37 DNS proxy/block/redirect adapter'],
  notClaimed: [
    'live host DNS mutation',
    'system DNS proxy installation',
    'OS permission elevation',
    'enforcement command publication',
    'decrypted payload or page content inspection',
    'exact URL claim from network-only evidence',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-dns-adapter-proof-ok:dns-adapter-tests,clippy');
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
