import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '32-cross-slice-evidence-bundle-builder');
const testRoot = join('test-results', 'network-evidence-bundle-builder-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-cross-slice-evidence-bundle.json'),
  `${JSON.stringify(
    {
      acceptedInputs: [
        'network-domain-category-ref',
        'managed-browser-exact-url-ref',
        'process-app-correlation-ref',
        'screen-summary-ref',
        'local-ai-suggestion-ref',
      ],
      bundleMustCarry: ['trigger-ref', 'all-evidence-refs', 'primary-source', 'next-checks', 'evidence-grade'],
      localAiQueueState: 'recommended-only; no queue enqueue or model execution claim',
      policyState: 'evidence input only; no policy decision or adapter authority claim',
      adapterState: 'not-authorized',
      networkOnlyMustNotClaim: ['exact-url', 'page-content', 'decrypted-payload'],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-evidence-bundle-builder-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'evidence_bundle'],
    log: join(proofRoot, 'bundle-builder-tests.log'),
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
  proof: 'network-evidence-bundle-builder',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedCrossSliceEvidenceBundle: join(proofRoot, 'expected-cross-slice-evidence-bundle.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['32 Cross-slice evidence bundle builder'],
  notClaimed: [
    'unmanaged browser collector implementation',
    'app/game foreground collector implementation',
    'screen summary collector implementation',
    'local AI queue enqueue or model execution',
    'policy decision authority',
    'adapter action, blocking, or termination authorization',
    'exact URL visibility from network metadata alone',
    'page content, message content, search terms, or decrypted payload visibility',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-evidence-bundle-builder-proof-ok:bundle-tests,clippy,source-shape');
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
