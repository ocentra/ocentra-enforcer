import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'network-plan-proof', '03a-live-capture-storage-proof');
const testRoot = join('test-results', 'network-live-capture-storage-proof');
const proofBranch = runText('git', ['branch', '--show-current']).trim();
const proofCommit = runText('git', ['rev-parse', 'HEAD']).trim();
const proofStatusShort = runText('git', ['status', '--short']);
mkdirSync(proofRoot, { recursive: true });
mkdirSync(testRoot, { recursive: true });

const boundary = {
  reportRef: 'network-live-capture-storage-row03a',
  requiredBeforeStorageAuthorization: [
    'proof-ready live capture proof ref',
    'raw artifact manifest ref',
    'local encrypted storage location ref',
    'encryption-at-rest verification ref',
    'quota rotation ref',
    'retention policy ref',
    'delete/export ref',
    'custody chain ref',
    'private family traffic exclusion ref',
  ],
  supportedStorageStates: ['custody-ready', 'manual-required', 'unavailable', 'degraded'],
  unsupportedClaimsRejected: [
    'live capture execution by proof harness',
    'remote upload',
    'raw PCAP without custody',
    'exact URL',
    'page content',
    'private message',
    'search query',
    'decrypted payload',
    'policy authority',
    'adapter authority',
    'enforcement command',
  ],
  authorityBoundary:
    'Row03a proves raw capture artifact custody and retention/delete/export refs before local storage authorization. It does not invoke capture drivers, create raw artifacts, upload artifacts, inspect content, or authorize policy/adapter/enforcement actions.',
};

writeFileSync(join(proofRoot, '03a-live-capture-storage-proof.json'), `${JSON.stringify(boundary, null, 2)}\n`);

const commands = [
  {
    name: 'network-raw-capture-storage-tests',
    command: 'cargo',
    args: ['test', '-p', 'ocentra-network-evidence', 'raw_capture_storage'],
    log: join(proofRoot, 'raw-capture-storage-tests.log'),
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
writeFileSync(
  join(proofRoot, '12-validation-commands.log'),
  `${commandResults.map((entry) => `${entry.command}\nlog=${entry.log}`).join('\n\n')}\n`
);

const proof = {
  proof: 'network-live-capture-storage',
  checkedAt: new Date().toISOString(),
  branch: proofBranch,
  commit: proofCommit,
  statusShort: proofStatusShort,
  proofRoot,
  testRoot,
  commands: commandResults,
  artifacts: {
    storageBoundary: join(proofRoot, '03a-live-capture-storage-proof.json'),
    commandLog: join(proofRoot, '12-validation-commands.log'),
    proofSummary: join(proofRoot, 'proof-summary.json'),
    testProof: join(testRoot, 'proof.json'),
  },
  provenRows: ['03a Live capture storage custody proof'],
  provenBoundaries: [
    'raw capture artifact storage requires proof-ready live capture refs plus local custody refs',
    'encrypted local storage, quota, retention, delete/export, custody chain, and private traffic exclusion refs are preserved',
    'manual-required, unavailable, and degraded states remain visible without storage authorization',
    'raw PCAP without custody, remote upload, content, policy, adapter, and enforcement claims are rejected',
  ],
  notClaimed: [
    'live capture execution',
    'raw artifact creation',
    'remote upload',
    'raw PCAP without custody',
    'exact URL, page content, private message, search query, or decrypted payload',
    'policy or adapter authority',
    'enforcement command publication',
  ],
};

writeFileSync(join(proofRoot, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(join(testRoot, 'proof.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('network-live-capture-storage-proof-ok:storage-tests,clippy,source-shape');
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
