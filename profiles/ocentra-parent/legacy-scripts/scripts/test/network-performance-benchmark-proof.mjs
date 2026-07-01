import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '49-performance-benchmark-proof');
const testRoot = join('test-results', 'network-performance-benchmark-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-performance-boundary.json'),
  `${JSON.stringify(
    {
      reportRef: 'network-performance-row49',
      acceptedInputs: [
        'fixture rows',
        'packet counts',
        'flow counts',
        'event counts',
        'latency metrics',
        'CPU metrics',
        'memory metrics',
        'disk metrics',
        'queue metrics',
        'path states',
      ],
      requiredMetrics: [
        'packet-to-summary latency',
        'packet-to-detection latency',
        'detection-to-cascade latency',
        'event throughput',
        'queue depth',
        'dropped events',
        'CPU milliseconds',
        'memory peak KiB',
        'disk bytes',
        'high-concurrency flow count',
      ],
      pathStates: ['dry-run', 'manual-required', 'unsupported', 'unavailable', 'degraded'],
      unsupportedClaimsRejected: [
        'real-time response SLO',
        'production SLO',
        'raw PCAP',
        'exact URL',
        'page content',
        'decrypted payload',
        'adapter action',
        'host filtering',
        'enforcement command',
      ],
      authorityBoundary:
        'Benchmark proof records fixture metrics only; it does not prove production performance, adapter execution, host filtering, or real-time response behavior.',
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-performance-benchmark-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'performance_benchmark'],
    log: join(proofRoot, 'performance-benchmark-tests.log'),
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
  proof: 'network-performance-benchmark-proof',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedPerformanceBoundary: join(proofRoot, 'expected-performance-boundary.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['49 Performance, latency, resource, and high-concurrency benchmark proof'],
  provenBoundaries: [
    'packet-to-detection latency metric aggregation',
    'event throughput metric aggregation',
    'CPU, memory, disk, queue, and dropped-event metrics',
    'high-concurrency fixture flow count',
    'manual/unavailable/degraded path-state preservation',
    'unsupported real-time and adapter claim rejection',
  ],
  notClaimed: [
    'production SLO or hardware-specific performance',
    'real-time response guarantee',
    'live packet capture or raw PCAP access',
    'adapter execution, host filtering, or enforcement command publication',
    'exact URL, page content, or decrypted payload availability',
    'portal performance UI rendering',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-performance-benchmark-proof-ok:performance-tests,clippy,source-shape');
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
