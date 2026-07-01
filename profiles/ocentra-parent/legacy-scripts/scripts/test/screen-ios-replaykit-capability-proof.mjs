import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..', '..');
const outputDir = join(repoRoot, 'output', 'screen-plan-proof', 'ios');
const proofPath = join(outputDir, 'proof-summary.json');

run('npm', ['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);

const screenIos = await import('@ocentra-parent/schema-domain/screen-ios-replaykit-capability-proof');

const generatedAt = new Date().toISOString();
const proof = screenIos.screenIosReplayKitCapabilityProof(generatedAt);
const negativeChecks = [
  rejects('product readiness requires physical device proof', () =>
    screenIos.ScreenIosReplayKitCapabilityRowSchema.safeParse({
      ...proof.rows[0],
      captureState: 'ready',
      proofState: 'physicalDeviceVerified',
      deletionProofRef: 'screen-ios-replaykit-deletion-proof',
      productCaptureReady: true,
    })
  ),
  rejects('product readiness requires deletion proof', () =>
    screenIos.ScreenIosReplayKitCapabilityRowSchema.safeParse({
      ...proof.rows[0],
      captureState: 'ready',
      proofState: 'physicalDeviceVerified',
      physicalDeviceProofRef: 'screen-ios-replaykit-physical-device-proof',
      productCaptureReady: true,
    })
  ),
  rejects('silent arbitrary other-app background capture is not claimable', () =>
    screenIos.ScreenIosReplayKitCapabilityRowSchema.safeParse({
      ...proof.rows[0],
      arbitraryBackgroundOtherAppCaptureClaimed: true,
    })
  ),
  rejects('raw frame remote upload remains forbidden', () =>
    screenIos.ScreenIosReplayKitCapabilityRowSchema.safeParse({
      ...proof.rows[0],
      rawFrameRemoteUploadAllowed: true,
    })
  ),
];

if (negativeChecks.some((check) => !check.rejected)) {
  throw new Error(`Unexpected iOS ReplayKit gate result: ${JSON.stringify(negativeChecks)}`);
}

const summary = {
  proof: proof.proofId,
  generatedAt,
  claim:
    'iOS screen capture remains explicit ReplayKit session or broadcast-extension work only; arbitrary silent other-app background capture is not claimed.',
  appleDocsVerified: [
    {
      ref: 'apple-developer-replaykit',
      url: 'https://developer.apple.com/documentation/replaykit',
      summary: 'ReplayKit records or streams screen video and app or microphone audio through ReplayKit APIs.',
    },
    {
      ref: 'apple-developer-rpscreenrecorder',
      url: 'https://developer.apple.com/documentation/replaykit/rpscreenrecorder',
      summary: 'RPScreenRecorder is an app shared recorder path for explicit start-and-stop recording or capture.',
    },
    {
      ref: 'apple-developer-rpbroadcastsamplehandler',
      url: 'https://developer.apple.com/documentation/replaykit/rpbroadcastsamplehandler',
      summary: 'RPBroadcastSampleHandler processes ReplayKit sample buffers in a broadcast upload extension.',
    },
  ],
  rows: proof.rows.map((row) => ({
    mode: row.mode,
    captureState: row.captureState,
    proofState: row.proofState,
    requiresExplicitUserStart: row.requiresExplicitUserStart,
    requiresReplayKitUi: row.requiresReplayKitUi,
    requiresBroadcastUploadExtension: row.requiresBroadcastUploadExtension,
    arbitraryBackgroundOtherAppCaptureClaimed: row.arbitraryBackgroundOtherAppCaptureClaimed,
    rawFrameRemoteUploadAllowed: row.rawFrameRemoteUploadAllowed,
    rawFrameRetentionDefault: row.rawFrameRetentionDefault,
    productCaptureReady: row.productCaptureReady,
    physicalDeviceProofRef: row.physicalDeviceProofRef,
    deletionProofRef: row.deletionProofRef,
  })),
  negativeChecks,
  gapStatus: {
    physicalIosDeviceReplayKitProofExists: false,
    replayKitBroadcastExtensionRuntimeProofExists: false,
    replayKitDeletionProofExists: false,
    arbitraryBackgroundOtherAppCaptureClaimed: false,
    productIosCaptureReady: proof.productIosCaptureReady,
  },
  artifacts: {
    summary: relativePath(proofPath),
  },
  nonClaims: proof.nonClaims,
};

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-ios-replaykit-capability-proof-ok:${proofPath}`);

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
