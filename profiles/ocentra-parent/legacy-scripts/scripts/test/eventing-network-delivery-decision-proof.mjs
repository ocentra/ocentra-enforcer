import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '45-eventing-delivery-decision-proof');
const eventingProofRoot = join('output', 'eventing-plan-proof', '63-delivery-decision-proof');
const testRoot = join('test-results', 'eventing-network-delivery-decision-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(eventingProofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'delivery-decision-requirements.json'),
  `${JSON.stringify(
    {
      namespace: 'network',
      localRoutes: ['local-in-process', 'local-service'],
      brokerRoutes: ['broker-backed', 'relay-hub'],
      localBackpressure: {
        boundedQueueCapacity: 32,
        ttlMillis: 30000,
        overflowDeadLetters: true,
        idempotencyRequired: true,
      },
      brokerRequiredArtifacts: [
        'custody proof',
        'publisher auth proof',
        'subscriber auth proof',
        'encryption proof',
        'retention policy',
        'replay plan',
        'deletion plan',
        'backpressure policy',
        'offset policy',
        'dedupe policy',
        'broker config',
      ],
      relayHubAdditionalArtifacts: ['relay hub identity', 'relay hub policy'],
      notImplementedByThisProof: [
        'broker delivery',
        'relay-hub delivery',
        'cross-process transport',
        'production retention/delete/export',
        'policy or adapter execution',
      ],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'eventing-delivery-decision-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-eventing', 'delivery_decision'],
    log: join(proofRoot, 'delivery-decision-tests.log'),
  },
  {
    name: 'eventing-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-eventing', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'clippy.log'),
  },
  {
    name: 'source-shape',
    command: 'node',
    args: ['scripts/check-source-shape.mjs', 'crates/ocentra-eventing'],
    log: join(proofRoot, 'source-shape.log'),
  },
];
const commandResults = commands.map(runCommand);

const proof = {
  proof: 'eventing-network-delivery-decision',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  eventingProofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    deliveryDecisionRequirements: join(proofRoot, 'delivery-decision-requirements.json'),
    networkProofSummary: join(proofRoot, 'proof-summary.json'),
    eventingProofSummary: join(eventingProofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: [
    'network row45 event topic namespace, publisher SDK, subscriber filtering, backpressure, retention, and broker/relay-hub decision proof',
  ],
  provenBehavior: [
    'local-first network routes remain ready with typed subscriber filtering and bounded backpressure metadata',
    'broker-backed route decisions enumerate custody, auth, encryption, retention, replay, deletion, offset, dedupe, and broker config artifacts',
    'relay-hub route decisions add relay identity and relay policy artifacts',
    'requirements-satisfied broker decisions still do not implement live broker delivery',
  ],
  notClaimed: [
    'live broker delivery',
    'relay-hub delivery',
    'cross-process transport implementation',
    'production retention/delete/export behavior',
    'policy, adapter, or enforcement authority',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(eventingProofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('eventing-network-delivery-decision-proof-ok:delivery-tests,clippy,source-shape');
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
