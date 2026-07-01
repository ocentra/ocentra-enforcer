import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '44-signature-alert-ingestion-proof');
const testRoot = join('test-results', 'network-signature-alert-ingestion-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-signature-alerts.json'),
  `${JSON.stringify(
    {
      fixtureRef: 'signature-alert-fixture-44',
      ingestionRunRef: 'signature-alert-run-44',
      supportedSources: ['suricata', 'snort-compatible'],
      requiredFields: [
        'signature id',
        'rule source',
        'severity',
        'timestamp',
        'flow ref',
        'evidence ref',
        'custody ref',
      ],
      expectedRecords: {
        total: 2,
        suricata: 1,
        snortCompatible: 1,
        falsePositiveNonEnforcing: 1,
        adapterCallsAuthorized: 0,
        enforcementCommandsPublished: 0,
      },
      networkOnlyMustNotClaim: [
        'exact-url',
        'page-content',
        'decrypted-payload',
        'live-suricata-invocation',
        'live-snort-invocation',
        'ips-prevention',
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
    name: 'network-signature-alert-ingestion-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'signature_alert_ingestion'],
    log: join(proofRoot, 'signature-alert-ingestion-tests.log'),
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
  proof: 'network-signature-alert-ingestion',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedSignatureAlerts: join(proofRoot, 'expected-signature-alerts.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['44 Suricata/Snort-compatible signature alert ingestion proof'],
  provenSources: ['suricata', 'snort-compatible'],
  notClaimed: [
    'live Suricata or Snort process invocation',
    'live Npcap/libpcap capture',
    'IPS prevention or packet blocking',
    'exact URL, page content, message, search query, or decrypted payload visibility',
    'policy, adapter, or enforcement command authority',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-signature-alert-ingestion-proof-ok:signature-tests,clippy,source-shape');
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
