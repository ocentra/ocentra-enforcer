import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '21-domain-normalization-public-suffix');
const testRoot = join('test-results', 'network-domain-normalization-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-domain-normalization.json'),
  `${JSON.stringify(
    {
      normalizedDomain: 'video.example.test',
      publicSuffix: 'test',
      registrableDomain: 'example.test',
      longestSuffixExample: {
        input: 'media.child.example.co.uk',
        publicSuffix: 'co.uk',
        registrableDomain: 'example.co.uk',
      },
      rejectedMalformedExamples: ['bad..example.test', 'bad_label.example.test'],
      networkOnlyMustNotClaim: ['exact-url', 'page-content', 'decrypted-payload'],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-domain-normalization-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'domain_'],
    log: join(proofRoot, 'domain-normalization-tests.log'),
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
  proof: 'network-domain-normalization-public-suffix',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedDomainNormalization: join(proofRoot, 'expected-domain-normalization.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['21 Domain normalization and public suffix model'],
  notClaimed: [
    'live capture or process/app/browser correlation',
    'full Mozilla Public Suffix List freshness',
    'visited URL, page content, message, search query, or decrypted payload visibility',
    'category database, AI, policy, adapter, broker, family-hub, or portal runtime integration',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-domain-normalization-proof-ok:domain-tests,clippy,source-shape');
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
