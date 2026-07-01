import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..', '..');
const outputDir = join(repoRoot, 'output', 'screen-plan-proof', 'android');
const proofPath = join(outputDir, 'proof-summary.json');
const emulatorProofPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  'android-mediaprojection',
  'proof-summary.json'
);

run('npm', ['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);

const screenAndroid = await import('@ocentra-parent/schema-domain/screen-android-mediaprojection-capability-proof');

const generatedAt = new Date().toISOString();
const emulatorProof = readJson(emulatorProofPath);
const physicalProofExists =
  emulatorProof.physicalDevice === true &&
  typeof emulatorProof.deviceInfo?.serial === 'string' &&
  !emulatorProof.deviceInfo.serial.startsWith('emulator-') &&
  emulatorProof.consentApproved === true &&
  emulatorProof.captured === true &&
  emulatorProof.rawTempDeleted === true;
const proof = screenAndroid.screenAndroidMediaProjectionCapabilityProof(
  generatedAt,
  physicalProofExists
    ? {
        physicalDeviceProofRef: 'output/screen-plan-proof/android-mediaprojection/proof-summary.json',
        deletionProofRef: 'output/screen-plan-proof/android-mediaprojection/03-android-capture-proof.json',
      }
    : undefined
);

if (!emulatorProof.consentApproved || !emulatorProof.captured || !emulatorProof.rawTempDeleted) {
  throw new Error(`Existing Android MediaProjection proof is not strong enough: ${JSON.stringify(emulatorProof)}`);
}

const negativeChecks = [
  rejects('physical readiness requires physical device proof', () =>
    screenAndroid.ScreenAndroidMediaProjectionCapabilityRowSchema.safeParse({
      ...proof.rows[1],
      captureState: 'ready',
      proofState: 'physicalDeviceVerified',
      deletionProofRef: 'screen-android-physical-mediaprojection-deletion-proof',
      productAndroidCaptureReady: true,
    })
  ),
  rejects('silent background capture remains not claimed', () =>
    screenAndroid.ScreenAndroidMediaProjectionCapabilityRowSchema.safeParse({
      ...proof.rows[1],
      silentBackgroundCaptureClaimed: true,
    })
  ),
  rejects('raw frame remote upload remains forbidden', () =>
    screenAndroid.ScreenAndroidMediaProjectionCapabilityRowSchema.safeParse({
      ...proof.rows[1],
      rawFrameRemoteUploadAllowed: true,
    })
  ),
  rejects('MediaProjection rows require per-session consent', () =>
    screenAndroid.ScreenAndroidMediaProjectionCapabilityRowSchema.safeParse({
      ...proof.rows[1],
      requiresUserConsentPerSession: false,
    })
  ),
  rejects('MediaProjection rows require stop callback behavior', () =>
    screenAndroid.ScreenAndroidMediaProjectionCapabilityRowSchema.safeParse({
      ...proof.rows[1],
      requiresStopCallbackOnUserStop: false,
    })
  ),
];

if (negativeChecks.some((check) => !check.rejected)) {
  throw new Error(`Unexpected Android MediaProjection gate result: ${JSON.stringify(negativeChecks)}`);
}

const summary = {
  proof: proof.proofId,
  generatedAt,
  claim:
    'Android MediaProjection emulator proof exists, but physical-device parity and silent background capture remain blocked before product readiness.',
  androidDocsVerified: [
    {
      ref: 'android-developer-media-projection',
      url: 'https://developer.android.com/media/grow/media-projection',
      summary:
        'MediaProjection captures display or app-window content as a media stream with user consent per session and foreground-service requirements for Android 14 targets.',
    },
    {
      ref: 'android-developer-app-screen-sharing',
      url: 'https://developer.android.com/about/versions/14/features/app-screen-sharing',
      summary: 'Android 14 app screen sharing can limit capture to a selected app window and exclude system UI.',
    },
  ],
  existingEmulatorProof: {
    path: relativePath(emulatorProofPath),
    deviceSerial: emulatorProof.deviceInfo?.serial ?? null,
    model: emulatorProof.deviceInfo?.model ?? null,
    api: emulatorProof.deviceInfo?.api ?? null,
    consentApproved: emulatorProof.consentApproved === true,
    captured: emulatorProof.captured === true,
    rawTempDeleted: emulatorProof.rawTempDeleted === true,
    physicalDeviceParityClaimed: physicalProofExists,
  },
  rows: proof.rows.map((row) => ({
    mode: row.mode,
    captureState: row.captureState,
    proofState: row.proofState,
    requiresUserConsentPerSession: row.requiresUserConsentPerSession,
    requiresForegroundServiceType: row.requiresForegroundServiceType,
    requiresStopCallbackOnUserStop: row.requiresStopCallbackOnUserStop,
    supportsAppWindowSelection: row.supportsAppWindowSelection,
    silentBackgroundCaptureClaimed: row.silentBackgroundCaptureClaimed,
    rawFrameRemoteUploadAllowed: row.rawFrameRemoteUploadAllowed,
    rawFrameRetentionDefault: row.rawFrameRetentionDefault,
    productAndroidCaptureReady: row.productAndroidCaptureReady,
    emulatorProofRef: row.emulatorProofRef,
    physicalDeviceProofRef: row.physicalDeviceProofRef,
    deletionProofRef: row.deletionProofRef,
  })),
  negativeChecks,
  gapStatus: {
    emulatorMediaProjectionProofExists: proof.emulatorCaptureProved,
    stopCallbackBehaviorDefined: proof.rows.every(
      (row) => row.mode === 'notClaimed' || row.requiresStopCallbackOnUserStop === true
    ),
    physicalAndroidDeviceProofExists: physicalProofExists,
    android14AppWindowPhysicalProofExists: false,
    silentBackgroundCaptureClaimed: false,
    productAndroidCaptureReady: proof.productAndroidCaptureReady,
  },
  artifacts: {
    summary: relativePath(proofPath),
  },
  nonClaims: proof.nonClaims,
};

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-android-mediaprojection-capability-proof-ok:${proofPath}`);

function rejects(name, parseAttempt) {
  return { name, rejected: !parseAttempt().success };
}

function readJson(path) {
  if (!existsSync(path)) {
    throw new Error(`Missing required proof artifact: ${path}`);
  }
  return JSON.parse(readFileSync(path, 'utf8'));
}

function relativePath(path) {
  return relative(repoRoot, path).replace(/\\/gu, '/');
}

function run(command, args) {
  const runner = process.platform === 'win32' ? 'cmd' : command;
  const runnerArgs = process.platform === 'win32' ? ['/c', command, ...args] : args;
  execFileSync(runner, runnerArgs, { cwd: repoRoot, stdio: 'inherit' });
}
