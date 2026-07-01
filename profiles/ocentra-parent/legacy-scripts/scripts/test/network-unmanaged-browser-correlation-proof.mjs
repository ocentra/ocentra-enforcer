import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '28-unmanaged-browser-correlation');
const testRoot = join('test-results', 'network-unmanaged-browser-correlation-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-unmanaged-browser-correlation.json'),
  `${JSON.stringify(
    {
      knownBrowserProcess: {
        state: 'ProcessOnlyBypassEvidence',
        basis: 'KnownBrowserProcess',
        possibleBypass: true,
        exactUrlAvailable: false,
        activeTabAvailable: false,
      },
      portableBrowserProcess: {
        state: 'ProcessOnlyBypassEvidence',
        basis: 'PortableBrowserProcess',
        possibleBypass: true,
      },
      browserLikeProcess: {
        state: 'ProcessOnlyBypassCandidate',
        basis: 'BrowserLikeProcessName',
        evidenceGrade: 'D',
      },
      managedBrowserBoundary: {
        state: 'ManagedBrowserBoundary',
        possibleBypass: false,
      },
      unmanagedBrowserMustNotClaim: [
        'exact-url',
        'active-tab',
        'page-title',
        'page-content',
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
    name: 'network-unmanaged-browser-correlation-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'unmanaged_browser_correlation'],
    log: join(proofRoot, 'unmanaged-browser-correlation-tests.log'),
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
  proof: 'network-unmanaged-browser-correlation',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedUnmanagedBrowserCorrelation: join(proofRoot, 'expected-unmanaged-browser-correlation.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['28 Unmanaged browser correlation'],
  notClaimed: [
    'exact unmanaged browser URL, active tab, page title, page content, browser history, cookies, form data, search terms, or decrypted payload',
    'managed-browser exact URL bridge implementation',
    'browser adapter implementation, process termination, AppLocker/App Control, policy authority, adapter authority, or enforcement-command publication',
    'portal, AI, broker, family-hub, or live service integration',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-unmanaged-browser-correlation-proof-ok:browser-tests,clippy,source-shape');
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
