import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '48-risk-budget-threshold-proof');
const testRoot = join('test-results', 'network-risk-budget-threshold-proof');
const sourceStatusExcludes = [
  proofRoot,
  testRoot,
  join('output', 'network-plan-proof', '22-domain-category-intelligence'),
  join('test-results', 'network-category-intelligence-proof'),
  join('output', 'network-plan-proof', '23-social-video-game-cloud-gaming-classifier'),
  join('test-results', 'network-social-video-game-classifier-proof'),
];
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-risk-budget-boundary.json'),
  `${JSON.stringify(
    {
      reportRef: 'network-risk-budget-row48',
      acceptedInputs: [
        'child profile ref',
        'household policy ref',
        'risk budget ref',
        'cascade ref',
        'AI audit report refs',
        'prior event refs',
        'adapter proof state',
      ],
      thresholds: ['monitor', 'ask-parent', 'warn-child', 'limit', 'block'],
      interventionStates: ['ignore', 'monitor', 'ask-parent', 'warn-child', 'limit', 'block', 'manual-required'],
      requiredProof: [
        'parent rule refs',
        'cited evidence refs',
        'safe behavior cap, expiry, audit reason, and UI explanation before risk pressure reduction',
        'adapter proof before limit or block mapping',
      ],
      unsupportedClaimsRejected: [
        'raw PCAP',
        'exact URL',
        'page content',
        'private message',
        'search query',
        'decrypted payload',
        'policy authority',
        'adapter authority',
        'enforcement command',
        'extra privilege grant',
        'allowance grant',
        'time grant',
      ],
      authorityBoundary:
        'Risk budget output maps action recommendations only; policy and adapter proof remain separate authority and no enforcement command is published.',
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-risk-budget-threshold-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'risk_budget'],
    log: join(proofRoot, 'risk-budget-threshold-tests.log'),
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
  proof: 'network-risk-budget-threshold-proof',
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
    expectedRiskBudgetBoundary: join(proofRoot, 'expected-risk-budget-boundary.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['48 Household risk budget and cascade threshold model'],
  provenBoundaries: [
    'age/profile/household threshold evaluation',
    'prior-event risk pressure',
    'safe-behavior credit policy proof',
    'adapter-proof-gated limit/block mapping',
    'unsupported-claim rejection',
  ],
  notClaimed: [
    'policy engine execution',
    'adapter execution or host filtering',
    'published enforcement command',
    'screen-time, allowance, app/game, or browser privilege grants',
    'raw PCAP, exact URL, page content, private message, search query, or decrypted payload use',
    'portal risk-budget UI rendering',
    'production risk scoring model',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-risk-budget-threshold-proof-ok:risk-budget-tests,clippy,source-shape');
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
