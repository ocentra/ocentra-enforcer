import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '20-flow-aggregation-sessionization');
const testRoot = join('test-results', 'network-flow-sessionization-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-flow-summary.json'),
  `${JSON.stringify(
    {
      fiveTuple: {
        initiatorIp: '192.168.1.25',
        initiatorPort: 53001,
        responderIp: '203.0.113.7',
        responderPort: 443,
        protocol: 'Tcp',
      },
      reverseDirectionMerged: true,
      idleTimeoutSplitsSession: true,
      counters: {
        initiatorToResponderPackets: 1,
        responderToInitiatorPackets: 1,
        initiatorToResponderBytes: 74,
        responderToInitiatorBytes: 98,
      },
      networkOnlyMustNotClaim: ['exact-url', 'page-content', 'decrypted-payload'],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-flow-aggregation-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'flow_aggregation'],
    log: join(proofRoot, 'flow-aggregation-tests.log'),
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
  proof: 'network-flow-sessionization',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedFlowSummary: join(proofRoot, 'expected-flow-summary.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['20 Flow aggregation/sessionization'],
  notClaimed: [
    'live Npcap/libpcap capture',
    'process/app/browser correlation',
    'exact URL, page content, message, search query, or decrypted payload visibility',
    'AI, policy, adapter, broker, family-hub, or portal runtime integration',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-flow-sessionization-proof-ok:flow-tests,clippy,source-shape');
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
