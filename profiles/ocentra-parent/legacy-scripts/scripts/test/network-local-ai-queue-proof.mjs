import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '33-network-triggered-local-ai-queue');
const testRoot = join('test-results', 'network-local-ai-queue-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-local-ai-queue-boundary.json'),
  `${JSON.stringify(
    {
      acceptedInputs: ['trigger-ref', 'evidence-refs', 'summary-refs', 'model-runtime-ref', 'queue-ref'],
      queuedOnlyWhen: ['local-ai-review-recommended', 'parent-enabled', 'model-runtime-available', 'queue-available'],
      skippedStates: ['not-recommended', 'disabled-by-parent', 'model-unavailable', 'queue-unavailable'],
      aiInputBoundary:
        'AI receives evidence refs and summary refs only; no raw packet, page content, decrypted payload, policy command, or adapter command',
      exactUrlBoundary:
        'Exact URL visibility can be represented only by managed-browser evidence refs, not network-derived URL strings',
      policyState: 'evidence input only; no policy decision or adapter authority claim',
      adapterState: 'not-authorized',
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-local-ai-queue-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'local_ai_queue'],
    log: join(proofRoot, 'local-ai-queue-tests.log'),
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
  proof: 'network-local-ai-queue',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedLocalAiQueueBoundary: join(proofRoot, 'expected-local-ai-queue-boundary.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['33 Network-triggered local AI queue'],
  notClaimed: [
    'local AI model execution',
    'local AI worker runtime',
    'remote AI provider or family-hub queue delivery',
    'raw packet payload, page content, message content, search terms, or decrypted payload in AI input',
    'network-only exact URL visibility',
    'policy decision authority',
    'adapter action, blocking, or termination authorization',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-local-ai-queue-proof-ok:queue-tests,clippy,source-shape');
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
