import { createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

import { BrowserGameMobileCapabilityMatrixSchema } from '@ocentra-parent/schema-domain/browser-game-android-ios-capability-matrix';

const repoRoot = process.cwd();
const proofId = 'browser-game-android-ios-capability-matrix-host-proof';
const outputDirectory = join(repoRoot, 'output', 'browser-plan-proof', 'game-23-android-ios-capability-matrix');
const resultDirectory = join(repoRoot, 'test-results', proofId);
const proofPath = join(resultDirectory, 'proof.json');
const outputProofPath = join(outputDirectory, '11-android-host-emulator-proof.json');
const apkPath = join(
  repoRoot,
  'target',
  'release-packages',
  'android',
  'ocentra-parent-agent-android-debug-latest.apk'
);
const packageName = 'ca.ocentra.parent.agent';
const expectedActivity = 'ca.ocentra.parent.agent/.MainActivity';
const expectedStatusText = 'Ocentra Parent Agent service scaffold is running.';
const knownBrowserPackageTargets = [
  ['browser-package-target-01', 'com.android.chrome'],
  ['browser-package-target-02', 'com.microsoft.emmx'],
  ['browser-package-target-03', 'org.mozilla.firefox'],
  ['browser-package-target-04', 'com.sec.android.app.sbrowser'],
];

const generatedAt = new Date().toISOString();
let launchedEmulator = false;
let selectedSerial = null;

try {
  await main();
} finally {
  if (launchedEmulator && selectedSerial !== undefined) {
    await adb(resolveAndroidTools(), selectedSerial, ['emu', 'kill'], { allowFailure: true });
  }
}

async function main() {
  mkdirSync(outputDirectory, { recursive: true });
  mkdirSync(resultDirectory, { recursive: true });

  const tools = resolveAndroidTools();
  const avds = listAvds(tools);
  if (avds.length === 0) {
    throw new Error('Expected at least one Android AVD for GAME-23 host proof');
  }

  const packageBuild = command(...npmCommand(['run', 'release:package:android']), {
    timeoutMs: 180_000,
  });
  if (packageBuild.exitCode !== 0) {
    throw new Error(`Android package build failed: ${packageBuild.stderr}`);
  }
  assertFileExists(apkPath, 'Android debug APK');

  selectedSerial = await ensureDevice(tools, avds[0]);
  await adb(tools, selectedSerial, ['install', '-r', apkPath], { timeoutMs: 120_000 });
  await adb(tools, selectedSerial, ['shell', 'input', 'keyevent', '224'], { allowFailure: true });
  await adb(tools, selectedSerial, ['shell', 'wm', 'dismiss-keyguard'], { allowFailure: true });

  const resolveActivity = await adbText(tools, selectedSerial, [
    'shell',
    'cmd',
    'package',
    'resolve-activity',
    '--brief',
    packageName,
  ]);
  if (!resolveActivity.includes('MainActivity')) {
    throw new Error('Expected Android launcher activity to resolve to MainActivity');
  }

  await adb(tools, selectedSerial, ['shell', 'am', 'start', '-n', expectedActivity], { timeoutMs: 60_000 });
  await delay(3_000);

  const uiTree = await adbText(tools, selectedSerial, ['exec-out', 'uiautomator', 'dump', '/dev/tty'], {
    timeoutMs: 60_000,
  });
  const statusTextObserved = uiTree.includes(expectedStatusText);
  if (!statusTextObserved) {
    throw new Error('Expected launched Android agent UI to expose the scaffold status text');
  }

  const packageDump = await adbText(tools, selectedSerial, ['shell', 'dumpsys', 'package', packageName], {
    timeoutMs: 60_000,
  });
  const deviceProps = await readDeviceProps(tools, selectedSerial);
  const packageVisibility = [];
  for (const [targetId, packageId] of knownBrowserPackageTargets) {
    packageVisibility.push(await queryPackageVisibility(tools, selectedSerial, targetId, packageId));
  }

  const matrix = buildCapabilityMatrix(generatedAt, {
    androidHostProofRef: 'parent-proof-browser-game-android-host-emulator',
    iosManualProofRef: 'parent-proof-browser-game-ios-entitlement-manual',
  });
  const parsed = BrowserGameMobileCapabilityMatrixSchema.parse(matrix);
  const negativeChecks = buildNegativeChecks(parsed);
  if (!negativeChecks.every((check) => check.rejected)) {
    throw new Error('Expected GAME-23 mobile capability negative checks to reject runtime/enforcement overclaims');
  }

  const proof = {
    schemaVersion: 1,
    proofId,
    generatedAt,
    branch: git(['rev-parse', '--abbrev-ref', 'HEAD']),
    commit: git(['rev-parse', 'HEAD']),
    baseCommit: git(['rev-parse', 'origin/main']),
    hostProofSummary: {
      androidSdkResolved: true,
      adbPathPersisted: false,
      emulatorPathPersisted: false,
      adbPathSha256: sha256(tools.adbPath),
      emulatorPathSha256: sha256(tools.emulatorPath),
      avdCount: avds.length,
      avdNamePersisted: false,
      selectedAvdSha256: sha256(avds[0]),
      launchedEmulator,
      packageBuildExitCode: packageBuild.exitCode,
      apkPathPersisted: false,
      apkSha256: sha256(readFileSync(apkPath)),
      deviceInspected: true,
      rawDeviceSerialPersisted: false,
      deviceSerialRef: `android-device-${sha256(selectedSerial).slice(0, 16)}`,
      packageInstalled: packageDump.includes(packageName),
      packageNamePersisted: false,
      packageNameSha256: sha256(packageName),
      launcherActivityResolved: true,
      launcherActivityRawPersisted: false,
      launcherActivitySha256: sha256(resolveActivity),
      appLaunched: true,
      uiTreeCaptured: true,
      uiTreeRawPersisted: false,
      uiTreeSha256: sha256(uiTree),
      statusTextObserved,
      screenshotCaptured: false,
      screenshotCaptureState: 'not-used-headless-emulator-screencap-was-black',
      knownBrowserPackageTargetsQueried: packageVisibility.length,
      rawInstalledPackageListPersisted: false,
      iosDeviceInspected: false,
      iosEntitlementProofClaimed: false,
      exactGameContentClaimed: false,
      cloudStreamFrameAnalysisClaimed: false,
      nativeGameControlClaimed: false,
      nativeLauncherControlClaimed: false,
      gameChatContentClaimed: false,
      perGameCloudTitleClaimed: false,
      runtimeSignalClaimed: false,
      appStoreOrPurchaseControlClaimed: false,
      uiDeliveryClaimed: false,
      enforcementClaimed: false,
      productChecklistUpgradeClaimed: false,
    },
    deviceProps,
    packageVisibility,
    capabilityMatrix: parsed,
    negativeChecks,
  };

  writeJson(proofPath, proof);
  writeJson(outputProofPath, proof);

  console.log('browser-game-android-ios-capability-matrix-host-proof-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`outputProof=${relativePath(outputProofPath)}`);
  console.log(`deviceInspected=true packageInstalled=true negativeChecks=${negativeChecks.length}`);
}

function resolveAndroidTools() {
  const sdkRoot =
    process.env.ANDROID_SDK_ROOT ??
    process.env.ANDROID_HOME ??
    (process.env.LOCALAPPDATA ? join(process.env.LOCALAPPDATA, 'Android', 'Sdk') : '');
  if (sdkRoot.length === 0) {
    throw new Error('Expected ANDROID_SDK_ROOT, ANDROID_HOME, or LOCALAPPDATA Android SDK path');
  }
  const adbPath = join(sdkRoot, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  const emulatorPath = join(sdkRoot, 'emulator', process.platform === 'win32' ? 'emulator.exe' : 'emulator');
  assertFileExists(adbPath, 'adb');
  assertFileExists(emulatorPath, 'Android emulator');
  return { adbPath, emulatorPath };
}

function listAvds(tools) {
  const result = command(tools.emulatorPath, ['-list-avds'], { timeoutMs: 30_000 });
  if (result.exitCode !== 0) {
    throw new Error(`emulator -list-avds failed: ${result.stderr}`);
  }
  return result.stdout
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean);
}

async function ensureDevice(tools, avd) {
  const existing = await findReadyDevice(tools);
  if (existing !== undefined) {
    await waitForBoot(tools, existing);
    return existing;
  }

  const emulator = spawn(
    tools.emulatorPath,
    ['-avd', avd, '-no-window', '-no-snapshot-save', '-no-audio', '-no-boot-anim', '-gpu', 'swiftshader_indirect'],
    {
      cwd: repoRoot,
      detached: process.platform !== 'win32',
      stdio: ['ignore', 'ignore', 'ignore'],
      windowsHide: true,
    }
  );
  launchedEmulator = true;
  emulator.unref();

  const serial = await waitForNewEmulatorDevice(tools);
  await waitForBoot(tools, serial);
  return serial;
}

async function findReadyDevice(tools) {
  const devices = await adbDevices(tools);
  return devices.find((device) => device.state === 'device')?.serial ?? null;
}

async function waitForNewEmulatorDevice(tools) {
  const deadline = Date.now() + 8 * 60_000;
  while (Date.now() < deadline) {
    const devices = await adbDevices(tools);
    const ready = devices.find((device) => device.state === 'device' && device.serial.startsWith('emulator-'));
    if (ready !== undefined) {
      return ready.serial;
    }
    await delay(2_000);
  }
  throw new Error('Timed out waiting for Android emulator device');
}

async function waitForBoot(tools, serial) {
  const deadline = Date.now() + 8 * 60_000;
  while (Date.now() < deadline) {
    const boot = (
      await adbText(tools, serial, ['shell', 'getprop', 'sys.boot_completed'], { allowFailure: true })
    ).trim();
    if (boot === '1') {
      return;
    }
    await delay(2_000);
  }
  throw new Error('Timed out waiting for Android emulator boot');
}

async function adbDevices(tools) {
  const output = await adbText(tools, null, ['devices']);
  return output
    .split(/\r?\n/u)
    .slice(1)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const [serial, state] = line.split(/\s+/u);
      return { serial, state };
    });
}

async function readDeviceProps(tools, serial) {
  const [sdk, release, model, manufacturer] = await Promise.all([
    adbText(tools, serial, ['shell', 'getprop', 'ro.build.version.sdk']),
    adbText(tools, serial, ['shell', 'getprop', 'ro.build.version.release']),
    adbText(tools, serial, ['shell', 'getprop', 'ro.product.model']),
    adbText(tools, serial, ['shell', 'getprop', 'ro.product.manufacturer']),
  ]);
  return {
    androidSdk: Number(sdk.trim()),
    androidRelease: release.trim(),
    modelPersisted: false,
    modelSha256: sha256(model.trim()),
    manufacturerPersisted: false,
    manufacturerSha256: sha256(manufacturer.trim()),
  };
}

async function queryPackageVisibility(tools, serial, targetId, packageId) {
  const result = await adbText(tools, serial, ['shell', 'pm', 'path', packageId], { allowFailure: true });
  return {
    targetId,
    packageIdPersisted: false,
    packageIdSha256: sha256(packageId),
    installed: result.includes('package:'),
    rawPackagePathPersisted: false,
    packagePathSha256: result.includes('package:') ? sha256(result.trim()) : null,
  };
}

function buildCapabilityMatrix(timestamp, proofRefs) {
  return {
    schemaVersion: 'browser-game-android-ios-capability-matrix',
    generatedAt: timestamp,
    proofRefs: [proofRefs.androidHostProofRef, proofRefs.iosManualProofRef],
    rows: [...androidRows(proofRefs.androidHostProofRef), ...iosRows(proofRefs.iosManualProofRef)],
    claimBoundaries: {
      exactGameContent: 'not-claimed',
      cloudStreamFrameAnalysis: 'not-claimed',
      nativeGameControl: 'not-claimed',
      nativeLauncherControl: 'not-claimed',
      gameChatContent: 'not-claimed',
      perGameCloudTitle: 'not-claimed',
      runtimeSignals: 'not-claimed',
      appStoreOrPurchaseControl: 'not-claimed',
      uiDelivery: 'not-claimed',
      enforcement: 'not-claimed',
      reviewerSummary:
        'Android host proof validates package install/launch only; browser-game mobile control remains manual-required or token-limited until owned-browser and iOS entitlement proof exists.',
    },
  };
}

function androidRows(proofRef) {
  return [
    matrixRow('android', 'android-owned-browser-shell', proofRef, androidOwnedBrowserShell()),
    matrixRow('android', 'android-managed-webview-shell', proofRef, androidOwnedBrowserShell()),
    matrixRow('android', 'android-custom-tabs', proofRef, {
      targetKind: 'browser-game-web-surface',
      parentCapability: 'managed-browser-control',
      capabilityState: 'manual-required',
      proofState: 'adapter-not-implemented',
      policyScope: 'manual-review-only',
      reasons: ['android-custom-tabs-not-owned', 'unmanaged-exact-url-unavailable'],
    }),
    matrixRow('android', 'android-installed-browser-app', proofRef, {
      targetKind: 'installed-browser-app',
      parentCapability: 'package-lifecycle',
      capabilityState: 'manual-required',
      proofState: 'manual-device-proof-required',
      policyScope: 'app-level-only',
      reasons: ['unmanaged-exact-url-unavailable', 'manual-device-proof-required'],
    }),
    matrixRow(
      'android',
      'android-cloud-gaming-browser-session',
      proofRef,
      cloudGamingBrowserSession('managed-browser-control')
    ),
    matrixRow('android', 'android-device-owner-browser-policy', proofRef, {
      targetKind: 'device-owner-browser-policy',
      parentCapability: 'device-owner-policy',
      capabilityState: 'manual-required',
      proofState: 'manual-device-proof-required',
      policyScope: 'manual-review-only',
      reasons: ['device-owner-required', 'app-store-control-unavailable'],
    }),
  ];
}

function iosRows(proofRef) {
  return [
    matrixRow('ios', 'ios-family-controls-authorization', proofRef, {
      targetKind: 'application-token',
      parentCapability: 'family-controls-entitlement',
      capabilityState: 'entitlement-required',
      proofState: 'platform-entitlement-required',
      policyScope: 'manual-review-only',
      reasons: ['family-controls-entitlement-required', 'native-game-boundary'],
    }),
    matrixRow('ios', 'ios-safari-web-domain-token', proofRef, {
      targetKind: 'web-domain-token',
      parentCapability: 'screen-time-api',
      capabilityState: 'domain-token-limited',
      proofState: 'platform-entitlement-required',
      policyScope: 'web-domain-token-level',
      reasons: ['web-domain-token-limited', 'cloud-title-unavailable'],
    }),
    matrixRow('ios', 'ios-application-token', proofRef, iosApplicationToken()),
    matrixRow('ios', 'ios-managed-browser-shell', proofRef, androidOwnedBrowserShell()),
    matrixRow('ios', 'ios-cloud-gaming-web-session', proofRef, cloudGamingBrowserSession('screen-time-api')),
    matrixRow('ios', 'ios-webclip-pwa', proofRef, {
      ...iosApplicationToken(),
      targetKind: 'webclip-or-pwa',
      reasons: ['opaque-application-token-required', 'native-game-boundary', 'cloud-title-unavailable'],
    }),
  ];
}

function androidOwnedBrowserShell() {
  return {
    targetKind: 'owned-browser-shell',
    parentCapability: 'managed-browser-control',
    capabilityState: 'manual-device-proof-required',
    proofState: 'manual-device-proof-required',
    policyScope: 'owned-browser-shell-only',
    reasons: ['owned-shell-required', 'managed-browser-required', 'manual-device-proof-required'],
  };
}

function cloudGamingBrowserSession(parentCapability) {
  return {
    targetKind: 'cloud-gaming-web-session',
    parentCapability,
    capabilityState: 'manual-required',
    proofState:
      parentCapability === 'screen-time-api' ? 'platform-entitlement-required' : 'manual-device-proof-required',
    policyScope: 'manual-review-only',
    reasons: ['cloud-title-unavailable', 'content-frame-analysis-unavailable', 'runtime-signal-unavailable'],
  };
}

function iosApplicationToken() {
  return {
    targetKind: 'application-token',
    parentCapability: 'family-controls-entitlement',
    capabilityState: 'app-token-limited',
    proofState: 'platform-entitlement-required',
    policyScope: 'app-token-level',
    reasons: ['opaque-application-token-required', 'native-game-boundary'],
  };
}

function matrixRow(platform, surface, proofRef, overrides) {
  return {
    platform,
    surface,
    parentCapabilityStatus: 'manual-required',
    proofRefs: [proofRef, `parent-proof-${surface}`],
    exactGameContentClaimed: false,
    cloudStreamFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    nativeLauncherControlClaimed: false,
    gameChatContentClaimed: false,
    perGameCloudTitleClaimed: false,
    runtimeSignalClaimed: false,
    appStoreOrPurchaseControlClaimed: false,
    uiDeliveredClaimed: false,
    enforcementClaimed: false,
    ...overrides,
  };
}

function buildNegativeChecks(matrix) {
  const claimFields = [
    'exactGameContentClaimed',
    'cloudStreamFrameAnalysisClaimed',
    'nativeGameControlClaimed',
    'nativeLauncherControlClaimed',
    'gameChatContentClaimed',
    'perGameCloudTitleClaimed',
    'runtimeSignalClaimed',
    'appStoreOrPurchaseControlClaimed',
    'uiDeliveredClaimed',
    'enforcementClaimed',
  ];
  const androidCloudRow = matrix.rows.find((row) => row.surface === 'android-cloud-gaming-browser-session');
  const checks = claimFields.map((field) => ({
    name: field,
    rejected: !BrowserGameMobileCapabilityMatrixSchema.safeParse({
      ...matrix,
      rows: matrix.rows.map((row) => (row.surface === androidCloudRow.surface ? { ...row, [field]: true } : row)),
    }).success,
  }));
  return [
    ...checks,
    {
      name: 'unsupportedAndroidOwnedShellUpgrade',
      rejected: !BrowserGameMobileCapabilityMatrixSchema.safeParse({
        ...matrix,
        rows: matrix.rows.map((row) =>
          row.surface === 'android-owned-browser-shell'
            ? {
                ...row,
                capabilityState: 'owned-browser-shell-capable-with-proof',
                proofState: 'existing-rust-parent-proof-ref',
              }
            : row
        ),
      }).success,
    },
    {
      name: 'missingIosWebclipSurface',
      rejected: !BrowserGameMobileCapabilityMatrixSchema.safeParse({
        ...matrix,
        rows: matrix.rows.filter((row) => row.surface !== 'ios-webclip-pwa'),
      }).success,
    },
  ];
}

async function adb(tools, serial, args, options = {}) {
  return adbText(tools, serial, args, options);
}

async function adbText(tools, serial, args, options = {}) {
  const allArgs = serial === null ? args : ['-s', serial, ...args];
  const result = command(tools.adbPath, allArgs, {
    timeoutMs: options.timeoutMs ?? 30_000,
    allowFailure: options.allowFailure,
  });
  if (result.exitCode !== 0 && options.allowFailure !== true) {
    throw new Error(`adb ${args.join(' ')} failed: ${result.stderr}`);
  }
  return result.stdout;
}

function command(file, args, options = {}) {
  try {
    const stdout = execFileSync(file, args, {
      cwd: repoRoot,
      timeout: options.timeoutMs ?? 30_000,
      windowsHide: true,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    return { exitCode: 0, stdout, stderr: '' };
  } catch (error) {
    if (options.allowFailure === true) {
      return {
        exitCode: error.status ?? 1,
        stdout: String(error.stdout ?? ''),
        stderr: String(error.stderr ?? error.message),
      };
    }
    throw error;
  }
}

function assertFileExists(path, label) {
  if (!existsSync(path)) {
    throw new Error(`Missing ${label}: ${relativePath(path)}`);
  }
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
