import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '13-live-capture-proof-gate');
const testRoot = join('test-results', 'network-live-capture-proof-gate');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-live-capture-boundary.json'),
  `${JSON.stringify(
    {
      reportRef: 'network-live-capture-row13',
      supportedProofStates: ['proof-ready', 'manual-required', 'unavailable', 'degraded'],
      requiredArtifacts: [
        'driver proof',
        'interface enumeration',
        'permission proof',
        'bounded capture proof',
        'clean stop proof',
        'quota rotation proof',
        'retention delete export proof',
        'custody proof',
        'private family traffic exclusion proof',
      ],
      unsupportedClaimsRejected: [
        'live capture driver invocation by proof harness',
        'unbounded capture',
        'raw PCAP without custody',
        'exact URL',
        'page content',
        'private message',
        'search query',
        'decrypted payload',
        'policy authority',
        'adapter authority',
        'enforcement command',
      ],
      authorityBoundary:
        'Row13 records whether a live capture path is proof-ready or manual-required; it does not invoke Npcap/libpcap, capture packets, or publish adapter commands.',
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-live-capture-proof-gate-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'live_capture_gate'],
    log: join(proofRoot, 'live-capture-proof-gate-tests.log'),
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
  proof: 'network-live-capture-proof-gate',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedLiveCaptureBoundary: join(proofRoot, 'expected-live-capture-boundary.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['13 Live pcap/Npcap/libpcap capture adapter'],
  provenBoundaries: [
    'driver, interface, permission, bounded capture, clean stop, quota, retention, custody, and private-traffic proof refs',
    'manual-required, unavailable, and degraded states',
    'unsupported live execution and content claim rejection',
  ],
  notClaimed: [
    'live Npcap/libpcap invocation',
    'actual packet capture or raw artifact creation',
    'unbounded capture',
    'raw PCAP without custody',
    'exact URL, page content, private message, search query, or decrypted payload availability',
    'policy or adapter authority',
    'enforcement command publication',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-live-capture-proof-gate-ok:live-capture-tests,clippy,source-shape');
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
