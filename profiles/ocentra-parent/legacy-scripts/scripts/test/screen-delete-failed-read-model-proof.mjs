import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofDir = join(repoRoot, 'output', 'screen-plan-proof', 'delete-failed-read-model');
const testDir = join(repoRoot, 'test-results', 'screen-delete-failed-read-model-proof');

rmSync(proofDir, { recursive: true, force: true });
rmSync(testDir, { recursive: true, force: true });
mkdirSync(proofDir, { recursive: true });
mkdirSync(testDir, { recursive: true });

const commands = [
  runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-protocol', 'screen_evidence']),
  runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'activity_store_screen_evidence']),
];

const protocolSource = readFileSync(join(repoRoot, 'crates', 'agent-protocol', 'src', 'screen_evidence.rs'), 'utf8');
const coreSource = readFileSync(
  join(repoRoot, 'crates', 'agent-core', 'src', 'activity_store_screen_evidence.rs'),
  'utf8'
);
const coreTests = readFileSync(
  join(repoRoot, 'crates', 'agent-core', 'src', 'activity_store_screen_evidence_tests.rs'),
  'utf8'
);

assertIncludes(protocolSource, 'SCREEN_DELETION_DELETE_FAILED');
assertIncludes(protocolSource, 'SCREEN_QUEUE_STATUS_FAILED');
assertIncludes(coreSource, 'delete_failed_count: deletion_state_count');
assertIncludes(coreSource, 'SCREEN_QUEUE_STATUS_FAILED');
assertIncludes(coreTests, 'activity_store_surfaces_screen_delete_failed_queue_health');

const proof = {
  proof: 'screen-delete-failed-read-model-proof',
  proofTier: 'P2_CONTRACT_READ_MODEL_PROOF',
  branch: currentBranch(),
  commands,
  artifacts: {
    proofSummary: relative(repoRoot, join(proofDir, 'proof-summary.json')),
  },
  assertions: {
    rustProtocolDefinesDeleteFailedDeletionState: true,
    rustProtocolDefinesFailedQueueStatus: true,
    activityStoreCountsDeleteFailedRows: true,
    activityStoreMapsDeleteFailedLatestStatusToFailed: true,
    rawImageRetainedByDefault: false,
  },
  nonClaims: [
    'This proves Rust protocol/read-model surfacing for delete-failed screen custody rows.',
    'This does not simulate an operating-system filesystem deletion failure.',
    'This does not enable raw screenshot retention or weaken delete-after-success/delete-after-expiry defaults.',
  ],
};

writeFileSync(join(proofDir, 'proof-summary.json'), `${JSON.stringify(proof, null, 2)}\n`);
console.log('screen-delete-failed-read-model-proof-ok');
console.log(`proof=${relative(repoRoot, join(proofDir, 'proof-summary.json'))}`);

function runCommand(command, args) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  const name = [command, ...args].join(' ');
  writeFileSync(join(testDir, `${safeName(name)}.stdout.log`), result.stdout ?? '');
  writeFileSync(join(testDir, `${safeName(name)}.stderr.log`), result.stderr ?? '');
  if (result.status !== 0) {
    throw new Error(`${name} failed with status ${result.status ?? 'unknown'}`);
  }
  return {
    command: name,
    status: result.status,
    stdout: relative(repoRoot, join(testDir, `${safeName(name)}.stdout.log`)),
    stderr: relative(repoRoot, join(testDir, `${safeName(name)}.stderr.log`)),
  };
}

function currentBranch() {
  const result = spawnSync('git', ['branch', '--show-current'], {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  return result.stdout.trim();
}

function assertIncludes(source, expected) {
  if (!source.includes(expected)) {
    throw new Error(`Expected source to include ${expected}`);
  }
}

function safeName(value) {
  return value
    .replace(/[^a-z0-9]+/gi, '-')
    .replace(/^-|-$/g, '')
    .toLowerCase();
}
