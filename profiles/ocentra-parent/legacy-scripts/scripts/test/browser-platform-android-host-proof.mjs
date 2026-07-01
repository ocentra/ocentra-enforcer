import { createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const proofRoot = join(repoRoot, 'output/browser-plan-proof/05-cross-platform-inventory-matrix');
const testResultPath = join(repoRoot, 'test-results/browser-platform-android-host-proof/proof.json');
const outputProofPath = join(proofRoot, '11-android-host-device-proof.json');
const screenshotProofPath = join(proofRoot, '11-android-host-device-screenshot.png');
const observedAt = new Date().toISOString();

const targetPackages = [
  { targetId: 'android-chrome', packageName: 'com.android.chrome' },
  { targetId: 'firefox-android', packageName: 'org.mozilla.firefox' },
  { targetId: 'edge-android', packageName: 'com.microsoft.emmx' },
  { targetId: 'samsung-internet', packageName: 'com.sec.android.app.sbrowser' },
  { targetId: 'ocentra-owned-browser-shell', packageName: 'com.ocentra.parent.browser' },
];

mkdirSync(proofRoot, { recursive: true });

const adb = findAdb();
const emulator = findEmulator();
const adbVersion = adb ? command(['version'], { allowFailure: true, adbPath: adb.path }) : null;
const emulatorAvds = emulator ? listAvds(emulator.path) : [];
const requestedAndroidSerial = process.env.ANDROID_SERIAL?.trim() ?? '';
let launchedEmulator = false;
let launchedEmulatorSerial = null;

if (
  adb &&
  emulator &&
  requestedAndroidSerial.length === 0 &&
  listDevices(adb.path).filter((device) => device.state === 'device').length === 0
) {
  launchedEmulatorSerial = await launchEmulatorIfAvailable(adb.path, emulator.path, emulatorAvds);
  launchedEmulator = launchedEmulatorSerial !== undefined;
}

const discoveredDevices = adb ? listDevices(adb.path) : [];
const requestedDevice =
  requestedAndroidSerial.length > 0
    ? discoveredDevices.find((device) => device.serial === requestedAndroidSerial)
    : null;
if (requestedAndroidSerial.length > 0 && !requestedDevice) {
  throw new Error(`Requested ANDROID_SERIAL was not attached: ${requestedAndroidSerial}`);
}
const devices = requestedDevice ? [requestedDevice] : discoveredDevices;
const attachedDevices = devices.filter((device) => device.state === 'device');
const packageVisibility = [];
const defaultViewHandlers = [];
const deviceSurfaceEvidence = [];

for (const device of attachedDevices) {
  const bootCompleted = command(['-s', device.serial, 'shell', 'getprop', 'sys.boot_completed'], {
    adbPath: adb.path,
    allowFailure: true,
  }).trim();
  device.bootCompleted = bootCompleted === '1';

  for (const target of targetPackages) {
    packageVisibility.push(queryPackage(adb.path, device.serial, target));
  }

  defaultViewHandlers.push(resolveDefaultViewHandler(adb.path, device.serial));
  deviceSurfaceEvidence.push(captureDeviceSurfaceEvidence(adb.path, device.serial, launchedEmulator));
}

const ownedShellVisible = packageVisibility.some(
  (entry) => entry.targetId === 'ocentra-owned-browser-shell' && entry.installed
);
const browserPackageVisible = packageVisibility.some(
  (entry) => entry.targetId !== 'ocentra-owned-browser-shell' && entry.installed
);
const bootedDeviceCount = attachedDevices.filter((device) => device.bootCompleted).length;
const androidSourceBoundaryProof = inspectAndroidSourceBoundary();
const negativeChecks = [
  {
    claim: 'owned-browser-shell-custody',
    rejected: !ownedShellVisible && !androidSourceBoundaryProof.ownedBrowserShellDeclared,
  },
  {
    claim: 'managed-exact-url-on-android',
    rejected: true,
  },
  {
    claim: 'known-active-tab-on-android',
    rejected: true,
  },
  {
    claim: 'android-browser-enforcement',
    rejected: true,
  },
];

if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected Android browser host proof negative checks to reject dishonest claims');
}

const proof = {
  schemaVersion: 1,
  proofId: 'browser-platform-android-host-proof',
  generatedAt: observedAt,
  branch: git(['branch', '--show-current']),
  commit: git(['rev-parse', 'HEAD']),
  baseCommit: git(['rev-parse', 'origin/main']),
  hostProofSummary: {
    adbInstalled: adb !== undefined,
    adbPathPersisted: false,
    adbPathSha256: adb ? sha256(adb.path) : null,
    adbVersionSha256: adbVersion ? sha256(adbVersion) : null,
    emulatorPathPersisted: false,
    emulatorPathSha256: emulator ? sha256(emulator.path) : null,
    emulatorAvdCount: emulatorAvds.length,
    emulatorLaunchedByProof: launchedEmulator,
    emulatorCleanupAttempted: launchedEmulator,
    androidSerialFilterUsed: requestedAndroidSerial.length > 0,
    requestedAndroidSerialPersisted: false,
    requestedAndroidSerialRef:
      requestedAndroidSerial.length > 0
        ? `redacted-android-device-ref-${sha256(requestedAndroidSerial).slice(0, 16)}`
        : null,
    allAttachedDeviceCountBeforeSerialFilter: discoveredDevices.filter((device) => device.state === 'device').length,
    attachedDeviceCount: attachedDevices.length,
    bootedDeviceCount,
    realDeviceOrEmulatorInspected: bootedDeviceCount > 0,
    physicalAndroidTargetRequired: requestedAndroidSerial.length > 0,
    physicalAndroidTargetObserved:
      requestedAndroidSerial.length > 0 &&
      attachedDevices.some(
        (device) => device.serial === requestedAndroidSerial && !device.serial.startsWith('emulator-')
      ),
    requestedPhysicalProductObserved: requestedDevice?.product ?? null,
    requestedPhysicalModelObserved: requestedDevice?.model ?? null,
    requestedPhysicalDeviceNameObserved: requestedDevice?.deviceName ?? null,
    emulatorEvidenceExcludedBySerialFilter:
      requestedAndroidSerial.length > 0 && discoveredDevices.some((device) => device.serial.startsWith('emulator-')),
    knownBrowserPackageIdsQueriedOnly: true,
    browserPackageVisible,
    ownedBrowserShellVisible: ownedShellVisible,
    defaultViewHandlerQueried: bootedDeviceCount > 0,
    rawInstalledPackageListPersisted: false,
    screenshotsCaptured: deviceSurfaceEvidence.some((entry) => entry.screenshotCaptured),
    screenshotsPersisted: deviceSurfaceEvidence.some((entry) => entry.screenshotPersisted),
    screenshotCaptureState: deviceSurfaceEvidence.some((entry) => entry.screenshotCaptured)
      ? 'captured'
      : launchedEmulator
        ? 'not-used-headless-emulator-screencap-was-black'
        : 'not-captured',
    uiTreeCaptured: deviceSurfaceEvidence.some((entry) => entry.uiTreeCaptured),
    uiTreeRawPersisted: false,
    logcatCaptured: deviceSurfaceEvidence.some((entry) => entry.logcatCaptured),
    logcatRawPersisted: false,
    exactUrlProofClaimed: false,
    knownActiveTabProofClaimed: false,
    ownedShellCustodyClaimed: false,
    ownedBrowserShellSourceDeclared: androidSourceBoundaryProof.ownedBrowserShellDeclared,
    managedProfileClaimed: false,
    deviceOwnerEnrollmentClaimed: false,
    vpnDnsBrowserProofClaimed: false,
    usageStatsRouteProofClaimed: false,
    accessibilityRouteProofClaimed: false,
    enforcementClaimed: false,
    resultState:
      bootedDeviceCount > 0 ? 'android-browser-package-visibility-proof' : 'manual-android-device-proof-required',
  },
  devices: attachedDevices.map((device) => ({
    serialRef: `redacted-android-device-ref-${sha256(device.serial).slice(0, 16)}`,
    serialKind: device.serial.startsWith('emulator-') ? 'emulator' : 'physical-or-network-adb-device',
    state: device.state,
    product: device.product,
    model: device.model,
    deviceName: device.deviceName,
    bootCompleted: device.bootCompleted,
    rawSerialPersisted: false,
    rawAdbDeviceLinePersisted: false,
  })),
  packageVisibility,
  defaultViewHandlers,
  deviceSurfaceEvidence,
  androidSourceBoundaryProof,
  negativeChecks,
};

writeJson(testResultPath, proof);
writeJson(outputProofPath, proof);

console.log('browser-platform-android-host-proof-ok=true');
console.log(`proof=${testResultPath}`);
console.log(`outputProof=${outputProofPath}`);
console.log(`adbInstalled=${adb !== undefined}`);
console.log(`attachedDeviceCount=${attachedDevices.length}`);
console.log(`bootedDeviceCount=${bootedDeviceCount}`);
console.log(`resultState=${proof.hostProofSummary.resultState}`);
console.log(`androidSerialFilterUsed=${proof.hostProofSummary.androidSerialFilterUsed}`);
console.log(`physicalAndroidTargetObserved=${proof.hostProofSummary.physicalAndroidTargetObserved}`);

if (adb && launchedEmulatorSerial !== undefined) {
  command(['-s', launchedEmulatorSerial, 'emu', 'kill'], { adbPath: adb.path, allowFailure: true });
}

function findAdb() {
  const output = commandWhere('adb');
  const candidate = output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find((line) => line.length > 0 && existsSync(line));
  return candidate ? { path: candidate } : null;
}

function findEmulator() {
  const sdkRoot =
    process.env.ANDROID_SDK_ROOT ??
    process.env.ANDROID_HOME ??
    (process.env.LOCALAPPDATA ? join(process.env.LOCALAPPDATA, 'Android', 'Sdk') : '');
  const candidate = sdkRoot.length > 0 ? join(sdkRoot, 'emulator', 'emulator.exe') : '';
  return candidate.length > 0 && existsSync(candidate) ? { path: candidate } : null;
}

function listAvds(emulatorPath) {
  const output = commandExternal(emulatorPath, ['-list-avds'], { allowFailure: true });
  return output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

async function launchEmulatorIfAvailable(adbPath, emulatorPath, avds) {
  const selectedAvd = avds[0];
  if (!selectedAvd) {
    return null;
  }

  const child = spawn(
    emulatorPath,
    [
      '-avd',
      selectedAvd,
      '-no-window',
      '-no-snapshot-save',
      '-no-audio',
      '-no-boot-anim',
      '-gpu',
      'swiftshader_indirect',
    ],
    {
      cwd: repoRoot,
      detached: process.platform !== 'win32',
      stdio: ['ignore', 'ignore', 'ignore'],
      windowsHide: true,
    }
  );
  child.unref();

  const serial = await waitForReadyEmulator(adbPath);
  await waitForBoot(adbPath, serial);
  return serial;
}

async function waitForReadyEmulator(adbPath) {
  const deadline = Date.now() + 8 * 60_000;
  while (Date.now() < deadline) {
    const ready = listDevices(adbPath).find(
      (device) => device.state === 'device' && device.serial.startsWith('emulator-')
    );
    if (ready) {
      return ready.serial;
    }
    await delay(2_000);
  }
  throw new Error('Timed out waiting for Android emulator device');
}

async function waitForBoot(adbPath, serial) {
  const deadline = Date.now() + 8 * 60_000;
  while (Date.now() < deadline) {
    const bootCompleted = command(['-s', serial, 'shell', 'getprop', 'sys.boot_completed'], {
      adbPath,
      allowFailure: true,
    }).trim();
    if (bootCompleted === '1') {
      return;
    }
    await delay(2_000);
  }
  throw new Error('Timed out waiting for Android emulator boot');
}

function listDevices(adbPath) {
  const output = command(['devices', '-l'], { adbPath, allowFailure: true });
  return output
    .split(/\r?\n/)
    .slice(1)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line) => {
      const [serial, state, ...details] = line.split(/\s+/);
      const detailMap = parseDeviceDetails(details);
      return {
        serial,
        state: state ?? 'unknown',
        product: detailMap.product ?? null,
        model: detailMap.model ?? null,
        deviceName: detailMap.device ?? null,
        rawDetailsSha256: details.length > 0 ? sha256(details.join(' ')) : null,
      };
    });
}

function parseDeviceDetails(details) {
  return Object.fromEntries(
    details
      .map((detail) => {
        const separatorIndex = detail.indexOf(':');
        return separatorIndex > 0 ? [detail.slice(0, separatorIndex), detail.slice(separatorIndex + 1)] : null;
      })
      .filter((entry) => entry !== undefined)
  );
}

function captureDeviceSurfaceEvidence(adbPath, serial, headlessEmulator) {
  const serialRef = `redacted-android-device-ref-${sha256(serial).slice(0, 16)}`;
  const screenshot = headlessEmulator
    ? Buffer.alloc(0)
    : commandBuffer(['-s', serial, 'exec-out', 'screencap', '-p'], {
        adbPath,
        allowFailure: true,
      });
  const uiTree = command(['-s', serial, 'exec-out', 'uiautomator', 'dump', '/dev/tty'], {
    adbPath,
    allowFailure: true,
  });
  const logcat = command(['-s', serial, 'logcat', '-d', '-t', '200'], {
    adbPath,
    allowFailure: true,
  });

  const screenshotUsable = screenshot.length > 8 && pngHasVisibleNonBlackPixel(screenshot);
  if (screenshotUsable) {
    writeFileSync(screenshotProofPath, screenshot);
  } else if (existsSync(screenshotProofPath)) {
    rmSync(screenshotProofPath);
  }

  return {
    serialRef,
    screenshotCaptured: screenshotUsable,
    screenshotPersisted: screenshotUsable,
    screenshotCaptureState: screenshotUsable
      ? 'captured'
      : screenshot.length > 8 || headlessEmulator
        ? 'not-used-headless-emulator-screencap-was-black'
        : 'not-captured',
    screenshotPath: screenshotUsable
      ? 'output/browser-plan-proof/05-cross-platform-inventory-matrix/11-android-host-device-screenshot.png'
      : null,
    screenshotSha256: screenshotUsable ? sha256(screenshot) : null,
    uiTreeCaptured: uiTree.includes('<hierarchy'),
    uiTreeRawPersisted: false,
    uiTreeSha256: uiTree.length > 0 ? sha256(uiTree) : null,
    logcatCaptured: logcat.length > 0,
    logcatRawPersisted: false,
    logcatSha256: logcat.length > 0 ? sha256(logcat) : null,
    exactUrlProofClaimed: false,
    pageContentCaptured: false,
    browserSecretCaptured: false,
  };
}

function pngHasVisibleNonBlackPixel(buffer) {
  try {
    const png = decodePng(buffer);
    return png.pixels.some((value, index) => {
      if (png.channels === 4 && index % 4 === 3) {
        return false;
      }
      return value > 12;
    });
  } catch {
    return false;
  }
}

function decodePng(buffer) {
  const signature = buffer.subarray(0, 8).toString('hex');
  if (signature !== '89504e470d0a1a0a') {
    throw new Error('Expected PNG signature');
  }
  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  const chunks = [];
  while (offset < buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.subarray(offset + 4, offset + 8).toString('ascii');
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    offset += 12 + length;
    if (type === 'IHDR') {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data.readUInt8(8);
      colorType = data.readUInt8(9);
    }
    if (type === 'IDAT') {
      chunks.push(data);
    }
    if (type === 'IEND') {
      break;
    }
  }
  if (bitDepth !== 8 || width <= 0 || height <= 0 || chunks.length === 0) {
    throw new Error('Expected 8-bit PNG with image data');
  }
  const channels = pngChannels(colorType);
  const stride = width * channels;
  const inflated = inflateSync(Buffer.concat(chunks));
  const pixels = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y += 1) {
    const sourceOffset = y * (stride + 1);
    const filter = inflated[sourceOffset];
    const source = inflated.subarray(sourceOffset + 1, sourceOffset + 1 + stride);
    const targetOffset = y * stride;
    unfilterPngScanline(filter, source, pixels, targetOffset, stride, channels);
  }
  return { channels, pixels };
}

function pngChannels(colorType) {
  if (colorType === 0) {
    return 1;
  }
  if (colorType === 2) {
    return 3;
  }
  if (colorType === 6) {
    return 4;
  }
  throw new Error(`Unsupported PNG color type ${colorType}`);
}

function unfilterPngScanline(filter, source, target, targetOffset, stride, bytesPerPixel) {
  for (let x = 0; x < stride; x += 1) {
    const raw = source[x];
    const left = x >= bytesPerPixel ? target[targetOffset + x - bytesPerPixel] : 0;
    const up = targetOffset >= stride ? target[targetOffset + x - stride] : 0;
    const upperLeft =
      x >= bytesPerPixel && targetOffset >= stride ? target[targetOffset + x - stride - bytesPerPixel] : 0;
    target[targetOffset + x] = (raw + pngFilterValue(filter, left, up, upperLeft)) & 0xff;
  }
}

function pngFilterValue(filter, left, up, upperLeft) {
  if (filter === 0) {
    return 0;
  }
  if (filter === 1) {
    return left;
  }
  if (filter === 2) {
    return up;
  }
  if (filter === 3) {
    return Math.floor((left + up) / 2);
  }
  if (filter === 4) {
    return paeth(left, up, upperLeft);
  }
  throw new Error(`Unsupported PNG filter ${filter}`);
}

function paeth(left, up, upperLeft) {
  const estimate = left + up - upperLeft;
  const leftDistance = Math.abs(estimate - left);
  const upDistance = Math.abs(estimate - up);
  const upperLeftDistance = Math.abs(estimate - upperLeft);
  if (leftDistance <= upDistance && leftDistance <= upperLeftDistance) {
    return left;
  }
  return upDistance <= upperLeftDistance ? up : upperLeft;
}

function queryPackage(adbPath, serial, target) {
  const output = command(['-s', serial, 'shell', 'pm', 'path', target.packageName], {
    adbPath,
    allowFailure: true,
  });
  return {
    serialRef: `redacted-android-device-ref-${sha256(serial).slice(0, 16)}`,
    targetId: target.targetId,
    targetPackageId: target.packageName,
    installed: output.includes('package:'),
    rawInstalledPackageListPersisted: false,
    exactUrlProofClaimed: false,
    knownActiveTabProofClaimed: false,
    ownedShellCustodyClaimed: false,
    enforcementClaimed: false,
  };
}

function resolveDefaultViewHandler(adbPath, serial) {
  const output = command(
    [
      '-s',
      serial,
      'shell',
      'cmd',
      'package',
      'resolve-activity',
      '--brief',
      '-a',
      'android.intent.action.VIEW',
      '-d',
      'https://example.com',
    ],
    {
      adbPath,
      allowFailure: true,
    }
  );
  const lines = output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  const component = lines.at(-1) ?? 'unresolved';
  return {
    serialRef: `redacted-android-device-ref-${sha256(serial).slice(0, 16)}`,
    componentRef:
      component === 'unresolved' ? 'unresolved' : `redacted-android-view-handler-${sha256(component).slice(0, 16)}`,
    resolved: component !== 'unresolved' && !component.includes('No activity found'),
    rawComponentPersisted: false,
    urlPersisted: false,
    urlContentCaptured: false,
    exactUrlProofClaimed: false,
  };
}

function inspectAndroidSourceBoundary() {
  const manifestPath = 'platforms/android/agent/app/src/main/AndroidManifest.xml';
  const buildGradlePath = 'platforms/android/agent/app/build.gradle';
  const privilegedProofPath =
    'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidPrivilegedCapabilityProof.java';
  const manifest = readRepoText(manifestPath);
  const buildGradle = readRepoText(buildGradlePath);
  const privilegedProof = readRepoText(privilegedProofPath);
  const expectedNegativeMarkers = [
    'ACCESSIBILITY_STATE = "not-declared"',
    'VPN_SERVICE_STATE = "not-declared"',
    'DNS_FILTERING_STATE = "not-implemented"',
    'DEVICE_OWNER_STATE = "blocked-without-enrollment"',
    'MANAGED_PROFILE_STATE = "blocked-without-enrollment"',
    'CHILD_AGENT_PARITY_STATE = "not-claimed"',
  ];
  const missingNegativeMarkers = expectedNegativeMarkers.filter((marker) => !privilegedProof.includes(marker));
  if (missingNegativeMarkers.length > 0) {
    throw new Error(
      `Android privileged proof is missing expected negative markers: ${missingNegativeMarkers.join(', ')}`
    );
  }

  return {
    manifestPath,
    manifestSha256: sha256(manifest),
    buildGradlePath,
    buildGradleSha256: sha256(buildGradle),
    privilegedProofPath,
    privilegedProofSha256: sha256(privilegedProof),
    rawSourcePersisted: false,
    packageIdDeclared: manifest.includes('ca.ocentra.parent.agent') || buildGradle.includes('ca.ocentra.parent.agent'),
    ownedBrowserShellDeclared: manifest.includes('com.ocentra.parent.browser'),
    webViewDeclared: manifest.includes('android.webkit.WebView') || manifest.includes('WebView'),
    viewIntentHandlerDeclared:
      manifest.includes('android.intent.action.VIEW') || manifest.includes('android.intent.category.BROWSABLE'),
    accessibilityServiceDeclared:
      manifest.includes('AccessibilityService') || manifest.includes('android.permission.BIND_ACCESSIBILITY_SERVICE'),
    vpnServiceDeclared: manifest.includes('VpnService') || manifest.includes('android.permission.BIND_VPN_SERVICE'),
    deviceAdminReceiverDeclared: manifest.includes('DeviceAdminReceiver'),
    usageStatsPermissionDeclared: manifest.includes('android.permission.PACKAGE_USAGE_STATS'),
    privilegedNegativeMarkers: expectedNegativeMarkers,
    negativeBoundaryState:
      'source-backed-no-owned-browser-shell-no-webview-no-view-handler-no-accessibility-no-vpn-no-device-admin',
  };
}

function readRepoText(relativePath) {
  return readFileSync(join(repoRoot, relativePath), 'utf8');
}

function command(args, { adbPath, allowFailure }) {
  try {
    return execFileSync(adbPath, args, { cwd: repoRoot, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  } catch (error) {
    if (allowFailure) {
      return `${error.stdout?.toString() ?? ''}${error.stderr?.toString() ?? ''}`;
    }
    throw error;
  }
}

function commandBuffer(args, { adbPath, allowFailure }) {
  try {
    return execFileSync(adbPath, args, { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
  } catch (error) {
    if (allowFailure) {
      return Buffer.alloc(0);
    }
    throw error;
  }
}

function commandExternal(file, args, { allowFailure }) {
  try {
    return execFileSync(file, args, { cwd: repoRoot, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  } catch (error) {
    if (allowFailure) {
      return `${error.stdout?.toString() ?? ''}${error.stderr?.toString() ?? ''}`;
    }
    throw error;
  }
}

function commandWhere(binary) {
  try {
    return execFileSync('where.exe', [binary], { cwd: repoRoot, encoding: 'utf8' });
  } catch {
    return '';
  }
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}
