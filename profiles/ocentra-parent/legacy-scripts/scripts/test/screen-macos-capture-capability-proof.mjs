import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..', '..');
const outputDir = join(repoRoot, 'output', 'screen-plan-proof', 'macos');
const proofPath = join(outputDir, 'proof-summary.json');

run('npm', ['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);

const screenMacos = await import('@ocentra-parent/schema-domain/screen-macos-capture-capability-proof');

const generatedAt = new Date().toISOString();
const proof = screenMacos.screenMacosCaptureCapabilityProof(generatedAt);

const negativeChecks = [
  rejects('live readiness requires live session proof', () =>
    screenMacos.ScreenMacosCaptureCapabilityRowSchema.safeParse({
      ...proof.rows[0],
      captureState: 'ready',
      proofState: 'liveSessionVerified',
      permissionProofRef: 'screen-macos-screen-recording-permission-proof',
      deletionProofRef: 'screen-macos-deletion-proof',
      productMacosCaptureReady: true,
    })
  ),
  rejects('live readiness requires Screen Recording permission proof', () =>
    screenMacos.ScreenMacosCaptureCapabilityRowSchema.safeParse({
      ...proof.rows[0],
      captureState: 'ready',
      proofState: 'liveSessionVerified',
      liveSessionProofRef: 'screen-macos-live-display-proof',
      deletionProofRef: 'screen-macos-deletion-proof',
      productMacosCaptureReady: true,
    })
  ),
  rejects('ScreenCaptureKit rows require content filters', () =>
    screenMacos.ScreenMacosCaptureCapabilityRowSchema.safeParse({
      ...proof.rows[0],
      requiresScreenCaptureKitContentFilter: false,
    })
  ),
  rejects('silent background capture remains not claimed', () =>
    screenMacos.ScreenMacosCaptureCapabilityRowSchema.safeParse({
      ...proof.rows[0],
      silentBackgroundCaptureClaimed: true,
    })
  ),
  rejects('raw frame remote upload remains forbidden', () =>
    screenMacos.ScreenMacosCaptureCapabilityRowSchema.safeParse({
      ...proof.rows[0],
      rawFrameRemoteUploadAllowed: true,
    })
  ),
];

if (negativeChecks.some((check) => !check.rejected)) {
  throw new Error(`Unexpected macOS ScreenCaptureKit gate result: ${JSON.stringify(negativeChecks)}`);
}

const summary = {
  proof: proof.proofId,
  generatedAt,
  claim:
    'macOS ScreenCaptureKit source-doc readiness exists, but live macOS display/window capture, Screen Recording permission, PPPC/MDM, and deletion proof remain blocked before product readiness.',
  appleDocsVerified: [
    {
      ref: 'apple-developer-screencapturekit',
      url: 'https://developer.apple.com/documentation/ScreenCaptureKit',
      summary:
        'ScreenCaptureKit streams selected display, app, and window content through framework-managed shareable content, filters, and stream output.',
    },
    {
      ref: 'apple-developer-screencapturekit-content-filter',
      url: 'https://developer.apple.com/documentation/screencapturekit/sccontentfilter',
      summary:
        'SCContentFilter limits a capture stream to selected displays, applications, or windows instead of treating every capture as unscoped.',
    },
    {
      ref: 'apple-support-screen-recording-privacy',
      url: 'https://support.apple.com/en-us/120315',
      summary: 'macOS exposes privacy controls and user-visible reminders when software records or shares the screen.',
    },
  ],
  rows: proof.rows.map((row) => ({
    mode: row.mode,
    captureState: row.captureState,
    proofState: row.proofState,
    requiresScreenRecordingPermission: row.requiresScreenRecordingPermission,
    requiresUserVisibleCaptureIndicator: row.requiresUserVisibleCaptureIndicator,
    requiresScreenCaptureKitContentFilter: row.requiresScreenCaptureKitContentFilter,
    requiresPppcMdmReview: row.requiresPppcMdmReview,
    silentBackgroundCaptureClaimed: row.silentBackgroundCaptureClaimed,
    rawFrameRemoteUploadAllowed: row.rawFrameRemoteUploadAllowed,
    rawFrameRetentionDefault: row.rawFrameRetentionDefault,
    productMacosCaptureReady: row.productMacosCaptureReady,
    liveSessionProofRef: row.liveSessionProofRef,
    permissionProofRef: row.permissionProofRef,
    deletionProofRef: row.deletionProofRef,
  })),
  negativeChecks,
  gapStatus: {
    appleSourceDocsVerified: true,
    liveMacosDisplayProofExists: false,
    liveMacosWindowProofExists: false,
    screenRecordingPermissionProofExists: false,
    pppcMdmDeploymentProofExists: false,
    productMacosCaptureReady: proof.productMacosCaptureReady,
  },
  artifacts: {
    summary: relativePath(proofPath),
  },
  nonClaims: proof.nonClaims,
};

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-macos-capture-capability-proof-ok:${proofPath}`);

function rejects(name, parseAttempt) {
  return { name, rejected: !parseAttempt().success };
}

function relativePath(path) {
  return relative(repoRoot, path).replace(/\\/gu, '/');
}

function run(command, args) {
  const runner = process.platform === 'win32' ? 'cmd' : command;
  const runnerArgs = process.platform === 'win32' ? ['/c', command, ...args] : args;
  execFileSync(runner, runnerArgs, { cwd: repoRoot, stdio: 'inherit' });
}
