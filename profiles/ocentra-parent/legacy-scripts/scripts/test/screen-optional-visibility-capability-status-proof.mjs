import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..', '..');
const proofDir = join(repoRoot, 'output', 'screen-plan-proof', 'optional-visibility-capability-status');
const proofPath = join(proofDir, 'proof-summary.json');
const retentionRuntimeProofPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  'screen-settings-service-command',
  'proof-summary.json'
);
const retentionDeletionProofPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  'screen-service-deletion-event-producer',
  'proof-summary.json'
);

run('npm', ['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);

const capabilityProofModule = await import('@ocentra-parent/schema-domain/screen-optional-visibility-capability-proof');

const generatedAt = new Date().toISOString();
const proof = capabilityProofModule.screenOptionalVisibilityCapabilityStatusProof(generatedAt);
const readinessStates = proof.rows.map((row) => row.readinessState);
const blockedLiveView = proof.rows.find((row) => row.capabilityKind === 'liveView' && row.readinessState === 'blocked');
const manualRetention = proof.rows.find(
  (row) => row.capabilityKind === 'rawScreenshotRetention' && row.readinessState === 'manualRequired'
);
const readyRetention = proof.rows.find(
  (row) => row.capabilityKind === 'rawScreenshotRetention' && row.readinessState === 'ready'
);

if (!blockedLiveView || !manualRetention || !readyRetention) {
  throw new Error('Expected blocked live-view, manual-required raw retention, and ready raw retention rows');
}
if (proof.rows.some((row) => row.rawFramesRetained || row.rawRemoteUploadAllowed || row.remoteInputAllowed)) {
  throw new Error(
    'Optional visibility capability status must not retain frames, upload raw screenshots, or allow remote input'
  );
}
if (blockedLiveView.liveViewPermissionGate?.permissionEvidenceKind !== 'screen-capture-only') {
  throw new Error('Expected live-view row to prove capture-only permission remains blocked');
}
if (!readyRetention.runtimeProofRef || !readyRetention.deletionProofRef) {
  throw new Error('Expected ready raw-retention row to carry runtime and deletion proof refs');
}
if (!existsSync(retentionRuntimeProofPath) || !existsSync(retentionDeletionProofPath)) {
  throw new Error('Expected raw-retention runtime and deletion proof artifacts to exist');
}

mkdirSync(proofDir, { recursive: true });
writeFileSync(
  proofPath,
  `${JSON.stringify(
    {
      ...proof,
      summary: {
        readinessStates,
        blockedLiveViewReason: blockedLiveView.reason,
        manualRetentionReason: manualRetention.reason,
        readyRetentionReason: readyRetention.reason,
        retentionRuntimeProofPresent: existsSync(retentionRuntimeProofPath),
        retentionDeletionProofPresent: existsSync(retentionDeletionProofPath),
      },
    },
    null,
    2
  )}\n`
);

console.log(`screen-optional-visibility-capability-status-proof-ok:${proofPath}`);

function run(command, args) {
  const runner = process.platform === 'win32' ? 'cmd' : command;
  const runnerArgs = process.platform === 'win32' ? ['/c', command, ...args] : args;
  execFileSync(runner, runnerArgs, { cwd: repoRoot, stdio: 'inherit' });
}
