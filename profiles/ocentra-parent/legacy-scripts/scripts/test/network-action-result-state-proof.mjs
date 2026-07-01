import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '53-action-result-state-proof');
const testRoot = join('test-results', 'network-action-result-state-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-action-result-state-proof.json'),
  `${JSON.stringify(
    {
      requiredRefs: [
        'action-result-ref',
        'policy-decision-ref',
        'parent-rule-ref',
        'evidence-refs',
        'adapter-proof-ref',
        'apply-artifact-ref',
        'result-artifact-ref',
        'audit-event-ref',
      ],
      blockedState: 'grade A block policy plus apply-ready adapter proof and all adapter result refs',
      terminatedState:
        'process or app target plus grade A block policy, apply-ready adapter proof, and all adapter result refs',
      dryRunState: 'non-result and allowed without adapter result artifacts',
      manualRequiredState:
        'weak evidence, parent-review policy, manual capability, manual adapter proof, missing artifact refs, or invalid terminate target',
      unavailableState: 'non-result capability or adapter-proof unavailable state',
      unsupportedClaimsRejected: [
        'exact URL',
        'decrypted payload',
        'page content',
        'host mutation',
        'published enforcement command',
      ],
      hostMutationClaimed: false,
      enforcementCommandPublished: false,
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-action-result-state-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', '--test', 'unit'],
    log: join(proofRoot, 'action-result-state-tests.log'),
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
  proof: 'network-action-result-state-proof',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedActionResultStateProof: join(proofRoot, 'expected-action-result-state-proof.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['real block/terminate/unavailable result state'],
  claimsProved: [
    'blocked result state requires grade A block policy and apply-ready adapter proof refs',
    'terminated result state is restricted to process or app targets',
    'dry-run, manual-required, and unavailable states do not accept adapter results',
    'weak evidence and parent-review policy cannot produce blocked or terminated states',
    'proof rejects exact URL, decrypted payload, page content, host mutation, and enforcement command claims',
  ],
  notClaimed: [
    'live host DNS mutation',
    'live Windows Firewall mutation',
    'process termination execution',
    'adapter command invocation',
    'published enforcement command',
    'exact URL claim from network-only evidence',
    'decrypted payload or page content inspection',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-action-result-state-proof-ok:action-result-tests,clippy');
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
