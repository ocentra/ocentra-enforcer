import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '23-social-video-game-cloud-gaming-classifier');
const testRoot = join('test-results', 'network-social-video-game-classifier-proof');
const sourceStatusExcludes = [
  proofRoot,
  testRoot,
  join('output', 'network-plan-proof', '22-domain-category-intelligence'),
  join('test-results', 'network-category-intelligence-proof'),
  join('output', 'network-plan-proof', '48-risk-budget-threshold-proof'),
  join('test-results', 'network-risk-budget-threshold-proof'),
];
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-social-video-game-classifier.json'),
  `${JSON.stringify(
    {
      domainCategory: {
        domain: 'watch.video.example.test',
        category: 'Video',
        basis: 'DomainCategory',
        confirmationRequired: false,
      },
      cdnCandidate: {
        category: 'CloudGaming',
        basis: 'CdnCandidateNeedsConfirmation',
        confirmationRequired: true,
      },
      browserConfirmedCdn: {
        category: 'CloudGaming',
        basis: 'BrowserConfirmedCdn',
        confirmationRequired: false,
      },
      processCandidate: {
        category: 'Game',
        basis: 'ProcessCandidateNeedsConfirmation',
        confirmationRequired: true,
      },
      networkOnlyMustNotClaim: ['exact-url', 'page-content', 'decrypted-payload'],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-social-video-game-classifier-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'classifier_'],
    log: join(proofRoot, 'classifier-tests.log'),
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
  proof: 'network-social-video-game-cloud-gaming-classifier',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  sourceCommit: runText('git', ['rev-parse', 'HEAD']).trim(),
  artifactCommit: 'see the enclosing git commit for generated proof artifacts',
  originMain: runText('git', ['rev-parse', 'origin/main']).trim(),
  mergeBase: runText('git', ['merge-base', 'HEAD', 'origin/main']).trim(),
  sourceStatusShort: runText('git', [
    'status',
    '--short',
    '--',
    '.',
    ...sourceStatusExcludes.map((path) => `:(exclude)${path.replaceAll('\\', '/')}`),
  ]),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedClassifier: join(proofRoot, 'expected-social-video-game-classifier.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['23 Social/video/game/cloud-gaming classifier'],
  notClaimed: [
    'production CDN intelligence corpus',
    'process/app/browser correlation beyond explicit fixture hints',
    'visited URL, page content, message, search query, or decrypted payload visibility',
    'VPN/proxy classifier, AI, policy, adapter, broker, family-hub, or portal runtime integration',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-social-video-game-classifier-proof-ok:classifier-tests,clippy,source-shape');
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
