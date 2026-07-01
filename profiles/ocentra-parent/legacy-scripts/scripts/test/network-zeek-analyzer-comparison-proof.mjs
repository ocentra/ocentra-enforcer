import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '43-zeek-structured-log-analyzer-comparison');
const testRoot = join('test-results', 'network-zeek-analyzer-comparison-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'approved-zeek-comparison-fixture.json'),
  `${JSON.stringify(
    {
      sourceFixtureRef: 'pcap-fixture-network-43',
      analyzerRunRef: 'zeek-analyzer-run-43',
      supportedLogKinds: ['conn', 'dns', 'http', 'tls', 'ssl'],
      expectedRows: {
        conn: 1,
        dns: 1,
        http: 1,
        tls: 1,
        ssl: 1,
      },
      preservedStates: ['unknown', 'missing', 'ambiguous', 'encrypted'],
      comparisonArtifacts: [
        'approved-Conn-comparison-artifact-43',
        'approved-Dns-comparison-artifact-43',
        'approved-Http-comparison-artifact-43',
        'approved-Tls-comparison-artifact-43',
        'approved-Ssl-comparison-artifact-43',
      ],
      networkOnlyMustNotClaim: [
        'exact-url',
        'page-content',
        'decrypted-payload',
        'signature-alert-ingestion',
        'policy-authority',
        'adapter-authority',
        'enforcement-command',
      ],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-zeek-generator-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'zeek_generator'],
    log: join(proofRoot, 'zeek-generator-tests.log'),
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
  proof: 'network-zeek-analyzer-comparison',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    approvedComparisonFixture: join(proofRoot, 'approved-zeek-comparison-fixture.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['43 Zeek-style structured log generator and analyzer comparison proof'],
  provenLogKinds: ['conn', 'dns', 'http', 'tls', 'ssl'],
  notClaimed: [
    'live Zeek, TShark, or Wireshark process invocation',
    'live Npcap/libpcap capture',
    'Suricata or Snort-compatible signature alert ingestion',
    'exact URL, page content, message, search query, or decrypted payload visibility',
    'policy, adapter, or enforcement command authority',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-zeek-analyzer-comparison-proof-ok:zeek-tests,clippy,source-shape');
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
