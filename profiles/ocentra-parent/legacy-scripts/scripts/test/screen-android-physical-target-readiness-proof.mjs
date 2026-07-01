import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const requestedSerial = process.env.OCENTRA_ANDROID_SERIAL ?? process.env.ANDROID_SERIAL ?? '192.168.2.45:5555';
const outputDir = resolve(repoRoot, 'output', 'screen-plan-proof', 'android-physical-target-readiness');
const proofPath = join(outputDir, 'proof-summary.json');
const devicesLogPath = join(outputDir, '00-adb-devices.log');
const uiDumpPath = join(outputDir, '01-keyguard-ui.xml');

mkdirSync(outputDir, { recursive: true });

const devices = runAdb(['devices', '-l'], { serial: false });
writeFileSync(devicesLogPath, devices.stdout);

const targetLine = devices.stdout
  .split(/\r?\n/u)
  .map((line) => line.trim())
  .find((line) => line.startsWith(`${requestedSerial} `));
const targetOnline = targetLine !== undefined && /\bdevice\b/u.test(targetLine);
const physicalDevice = targetOnline && !requestedSerial.startsWith('emulator-');

if (!targetOnline) {
  throw new Error(`Physical Android target ${requestedSerial} is not online; adb devices output was recorded.`);
}

if (!physicalDevice) {
  throw new Error(`Physical Android target readiness cannot use an emulator serial: ${requestedSerial}`);
}

runAdb(['shell', 'input', 'keyevent', 'KEYCODE_WAKEUP'], { allowFailure: true });
runAdb(['shell', 'wm', 'dismiss-keyguard'], { allowFailure: true });
sleep(1000);

const uiDump = runAdb(['exec-out', 'uiautomator', 'dump', '/dev/tty'], { allowFailure: true }).stdout;
writeFileSync(uiDumpPath, uiDump);

const keyguardLocked = /keyguard_pin_view|password_entry|Use biometrics or enter PIN|PIN unlock/i.test(uiDump);
const model = getProp('ro.product.model');
const api = getProp('ro.build.version.sdk');
const release = getProp('ro.build.version.release');

const proof = {
  proof: 'screen-android-physical-target-readiness-proof',
  generatedAt: new Date().toISOString(),
  target: {
    requestedSerial,
    adbOnline: targetOnline,
    adbLine: targetLine,
    physicalDevice,
    model,
    api,
    release,
  },
  keyguard: {
    uiDumpPath: relativePath(uiDumpPath),
    uiDumpSha256: sha256(uiDump),
    lockedBehindCredentialPrompt: keyguardLocked,
    unlockRequiredBeforeMediaProjection: keyguardLocked,
  },
  readiness: {
    mediaProjectionProofRunnableNow: !keyguardLocked,
    physicalExternalGateCanRunNow: !keyguardLocked,
  },
  assertions: {
    targetObservedOnline: true,
    targetIsPhysicalAndroid: true,
    targetModelRecorded: model.length > 0,
    keyguardStateRecorded: uiDump.length > 0,
    lockedTargetDoesNotClaimCapture: keyguardLocked,
    mediaProjectionCaptureNotClaimed: true,
  },
  nextAction: keyguardLocked
    ? 'Unlock the physical Android target, then rerun scripts/test/screen-android-physical-external-gate-proof.mjs.'
    : 'Run scripts/test/screen-android-physical-external-gate-proof.mjs to capture MediaProjection proof.',
  nonClaims: [
    'This proof records physical Android target readiness only.',
    'It does not invoke MediaProjection, capture pixels, analyze content, or satisfy the Android physical external gate.',
    'A locked target is a blocker, not a degraded capture proof.',
  ],
};

writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`screen-android-physical-target-readiness-proof-ok:${relativePath(proofPath)}`);

function getProp(name) {
  return runAdb(['shell', 'getprop', name], { allowFailure: true }).stdout.trim();
}

function runAdb(args, options = {}) {
  const finalArgs = options.serial === false ? args : ['-s', requestedSerial, ...args];
  const result = spawnSync('adb', finalArgs, {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: false,
  });
  if (result.status !== 0 && options.allowFailure !== true) {
    throw new Error(`adb ${finalArgs.join(' ')} failed with ${result.status}\n${result.stdout}\n${result.stderr}`);
  }
  return {
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
    status: result.status,
  };
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}
