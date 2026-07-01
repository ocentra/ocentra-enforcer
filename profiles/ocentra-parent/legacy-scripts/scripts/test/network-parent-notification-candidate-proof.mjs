import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '35-parent-notification-candidate-mapping');
const testRoot = join('test-results', 'network-parent-notification-candidate-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-parent-notification-candidate.json'),
  `${JSON.stringify(
    {
      requiredRefs: ['notification-candidate-ref', 'policy-decision-ref', 'parent-rule-ref', 'evidence-refs'],
      severityMapping: {
        dryRunBlockOrLimit: 'urgent candidate only',
        parentReview: 'review candidate only',
        observeOnly: 'info candidate only',
      },
      deliveryState: 'candidate-only; no provider delivery claim',
      payloadBoundary: 'no sensitive payload, raw packet payload, page content, message content, or decrypted payload',
      authorityBoundary: 'no adapter action or enforcement command authorization',
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-parent-notification-candidate-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'notification_candidate'],
    log: join(proofRoot, 'notification-candidate-tests.log'),
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
  proof: 'network-parent-notification-candidate',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedParentNotificationCandidate: join(proofRoot, 'expected-parent-notification-candidate.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['35 Parent notification candidate mapping'],
  notClaimed: [
    'notification provider delivery',
    'push/email/SMS send attempt',
    'sensitive payload transport',
    'adapter execution or adapter authorization',
    'enforcement command publication',
    'portal UI rendering',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-parent-notification-candidate-proof-ok:notification-tests,clippy,source-shape');
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
