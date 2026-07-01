import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '26-process-app-correlation-model');
const testRoot = join('test-results', 'network-process-app-correlation-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-process-app-correlation.json'),
  `${JSON.stringify(
    {
      pidSnapshot: {
        state: 'ProcessAttributed',
        basis: 'PidSnapshot',
        uncertainty: 'ConfirmedByReplay',
      },
      appInventory: {
        state: 'ProcessAndAppAttributed',
        basis: 'ProcessPathAppInventory',
        uncertainty: 'AppInventoryMatched',
      },
      processNameOnly: {
        state: 'ProcessCandidate',
        uncertainty: 'CandidateNeedsConfirmation',
      },
      unavailableOrUnknown: ['AdapterUnavailable', 'ProcessUnknown'],
      networkOnlyMustNotClaim: ['exact-url', 'browser-url', 'page-content', 'decrypted-payload'],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-process-app-correlation-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'process_correlation'],
    log: join(proofRoot, 'process-correlation-tests.log'),
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
  proof: 'network-process-app-correlation',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedProcessAppCorrelation: join(proofRoot, 'expected-process-app-correlation.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['26 Process/app correlation model'],
  notClaimed: [
    'browser URL, active tab, page content, or decrypted payload visibility from network evidence',
    'host adapter enforcement, blocking, or termination',
    'AI, policy, broker, family-hub, or portal runtime integration',
    'foreground session or managed browser correlation beyond replayed process/app evidence',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-process-app-correlation-proof-ok:process-tests,clippy,source-shape');
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
