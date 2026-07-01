import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '29-app-game-session-correlation');
const testRoot = join('test-results', 'network-app-game-session-correlation-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-app-game-session-correlation.json'),
  `${JSON.stringify(
    {
      foregroundSession: {
        state: 'ForegroundSessionConfirmed',
        basis: 'StoredForegroundEvidence',
        evidenceGrade: 'C',
      },
      runningSession: {
        state: 'RunningSessionConfirmed',
        basis: 'StoredSessionSummary',
      },
      launcherOnly: {
        state: 'LauncherOnlyGuarded',
        basis: 'LauncherOnlyEvidence',
        launcherOnlyGuarded: true,
      },
      candidate: {
        state: 'CandidateNeedsReview',
        basis: 'CandidateStoredEvidence',
      },
      appGameSessionMustNotClaim: [
        'exact-url',
        'screen-content',
        'ai-device-scanner',
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
    name: 'network-app-game-session-correlation-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', '--test', 'unit'],
    log: join(proofRoot, 'app-game-session-correlation-tests.log'),
  },
  {
    name: 'network-evidence-clippy',
    command: 'cargo',
    args: ['clippy', '-p', 'ocentra-network-evidence', '--all-targets', '--', '-D', 'warnings'],
    log: join(proofRoot, 'clippy.log'),
  },
];
const commandResults = commands.map(runCommand);

const proof = {
  proof: 'network-app-game-session-correlation',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedAppGameSessionCorrelation: join(proofRoot, 'expected-app-game-session-correlation.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['29 App/game foreground/session correlation'],
  notClaimed: [
    'app/game adapter implementation, live process/window capture, or launcher crawling',
    'exact URL, screen content, screenshots, keystrokes, game telemetry, or decrypted payload',
    'AI scanning the device or inventing session duration',
    'policy authority, adapter authority, enforcement-command publication, process termination, or time-limit execution',
    'portal, broker, family-hub, or live service integration',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-app-game-session-correlation-proof-ok:session-tests,clippy');
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
