import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '27-managed-browser-correlation-bridge');
const testRoot = join('test-results', 'network-managed-browser-correlation-proof');
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

writeFileSync(
  join(proofRoot, 'expected-managed-browser-correlation.json'),
  `${JSON.stringify(
    {
      matchingManagedBrowserEvidence: {
        state: 'ExactUrlConfirmed',
        basis: 'ManagedBrowserUrlEvidence',
        exactUrlFromManagedBrowser: true,
        exactUrlFromNetwork: false,
      },
      networkDomainOnly: {
        state: 'NetworkDomainOnly',
        basis: 'NetworkDomainEvidenceOnly',
        exactUrl: null,
      },
      mismatch: {
        state: 'BrowserDomainMismatch',
        exactUrl: null,
      },
      networkOnlyMustNotClaim: ['exact-url', 'browser-url', 'page-content', 'decrypted-payload'],
    },
    null,
    2
  )}\n`
);

const commands = [
  {
    name: 'network-managed-browser-correlation-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'managed_browser_correlation'],
    log: join(proofRoot, 'managed-browser-correlation-tests.log'),
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
  proof: 'network-managed-browser-correlation',
  checkedAt: new Date().toISOString(),
  branch: runText('git', ['branch', '--show-current']).trim(),
  commit: runText('git', ['rev-parse', 'HEAD']).trim(),
  statusShort: runText('git', ['status', '--short']),
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    expectedManagedBrowserCorrelation: join(proofRoot, 'expected-managed-browser-correlation.json'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['27 Managed browser correlation bridge'],
  notClaimed: [
    'exact URL visibility from network metadata alone',
    'unmanaged browser URL visibility',
    'page content, message content, search terms, or decrypted payload visibility',
    'AI, policy, adapter, broker, family-hub, or portal runtime integration',
  ],
};
writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-managed-browser-correlation-proof-ok:browser-tests,clippy,source-shape');
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
