import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '30-screen-summary-trigger');
const testRoot = join('test-results', 'network-screen-summary-trigger-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-screen-summary-trigger.json'),
  `${JSON.stringify(
    {
      queued: {
        status: 'Queued',
        privacyMode: 'ActiveWindowScreenIfEnabled',
        requiresEncryptedTemporaryCustody: true,
        requiresDeleteAfterAnalysis: true,
        captureExecuted: false,
      },
      notRecommended: {
        status: 'NotRecommended',
        privacyMode: 'NetworkOnly',
      },
      disabledOrUnavailable: [
        'DisabledByParent',
        'QueueUnavailable',
        'CustodyManualRequired',
        'ProtectedSurfaceUnavailable',
        'Debounced',
      ],
      screenSummaryMustNotClaim: [
        'raw-image-retention',
        'remote-upload',
        'screen-content',
        'decrypted-payload',
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
    name: 'network-screen-summary-trigger-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'screen_summary_trigger'],
    log: join(proofRoot, 'screen-summary-trigger-tests.log'),
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
  proof: 'network-screen-summary-trigger',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedScreenSummaryTrigger: join(proofRoot, 'expected-screen-summary-trigger.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['30 Screen summary trigger integration'],
  notClaimed: [
    'screen capture adapter execution, local OCR or VLM execution, or screen analysis result creation',
    'raw image retention, remote upload, screen content visibility, screenshots, keystrokes, credential surfaces, or decrypted payload',
    'policy authority, adapter authority, enforcement-command publication, process termination, or time-limit execution',
    'portal, broker, family-hub, or live service integration',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-screen-summary-trigger-proof-ok:screen-tests,clippy,source-shape');
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
