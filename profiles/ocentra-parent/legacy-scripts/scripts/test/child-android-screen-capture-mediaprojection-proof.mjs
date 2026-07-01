import { spawn, spawnSync } from 'node:child_process';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const outputDir = join('output', 'screen-plan-proof', 'android-mediaprojection');
const packageId = 'ca.ocentra.parent.agent';
const proofExtra = 'ca.ocentra.parent.agent.START_SCREEN_CAPTURE_PROOF';
const proofFile = 'files/screen-capture-mediaprojection-proof.json';
const avdName = process.env.OCENTRA_ANDROID_AVD ?? 'Pixel_9_Pro_XL_API_35';
const requestedSerial = process.env.OCENTRA_ANDROID_SERIAL ?? process.env.ANDROID_SERIAL ?? null;

mkdirSync(outputDir, { recursive: true });

let device = firstOnlineDevice();
if (device === null && process.env.OCENTRA_ANDROID_START_EMULATOR === '1') {
  startEmulator();
  device = waitForDevice();
}
if (device === null) {
  throw new Error('No Android device/emulator is online; Android MediaProjection proof cannot be claimed.');
}
waitForAndroidReady(device);
ensureDeviceUnlocked(device);
rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });
buildDebugApk();
ensureDeviceUnlocked(device);

const deviceInfo = {
  serial: device,
  model: adb(['shell', 'getprop', 'ro.product.model']).stdout.trim(),
  api: adb(['shell', 'getprop', 'ro.build.version.sdk']).stdout.trim(),
  release: adb(['shell', 'getprop', 'ro.build.version.release']).stdout.trim(),
  physicalDevice: !device.startsWith('emulator-'),
};
writeJson(join(outputDir, '00-device.json'), deviceInfo);

const apk = join('platforms', 'android', 'agent', 'app', 'build', 'outputs', 'apk', 'debug', 'app-debug.apk');
resetScreenProjectionDialogs();
adb(['install', '-r', apk]);
resetScreenProjectionDialogs();
adb(['shell', 'run-as', packageId, 'rm', '-f', proofFile], { allowFailure: true });
adb(['shell', 'am', 'start', '-n', `${packageId}/.MainActivity`, '--ez', proofExtra, 'true']);

const consent = approveConsentDialog();
writeJson(join(outputDir, '01-consent-ui.json'), consent);
if (!consent.approved) {
  throw new Error(`Android MediaProjection explicit consent was not approved: ${JSON.stringify(consent)}`);
}
const chooser = approveAppSelectorIfShown();
writeJson(join(outputDir, '02-selector-ui.json'), chooser);
const proof = waitForProofJson();
writeJson(join(outputDir, '03-android-capture-proof.json'), proof);

const summary = {
  proof: 'child-android-screen-capture-mediaprojection-proof',
  outputDir,
  deviceInfo,
  consentApproved: consent.approved,
  selectorApproved: chooser.approved,
  selectorTarget: chooser.clickedText ?? null,
  captured: proof.status === 'captured',
  width: proof.width ?? null,
  height: proof.height ?? null,
  frameByteSize: proof.frameByteSize ?? null,
  rawTempDeleted: proof.rawTempDeleted === true,
  physicalDevice: deviceInfo.physicalDevice,
  degradedIsCaptureProof: false,
  nonClaims: [
    'This is Android emulator/device MediaProjection proof only.',
    'It does not claim silent background capture.',
    'It does not claim physical-device parity unless the target serial is a physical device recorded in 00-device.json.',
  ],
};
writeJson(join(outputDir, 'proof-summary.json'), summary);

if (!summary.consentApproved || !summary.captured || !summary.rawTempDeleted) {
  throw new Error(`Android MediaProjection proof failed: ${JSON.stringify(summary, null, 2)}`);
}

console.log(JSON.stringify(summary, null, 2));

function buildDebugApk() {
  const command = process.platform === 'win32' ? 'gradlew.bat' : './gradlew';
  const result = spawnSync(command, [':app:assembleDebug'], {
    cwd: join(process.cwd(), 'platforms', 'android', 'agent'),
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  writeFileSync(join(outputDir, 'gradle-stdout.log'), result.stdout ?? '');
  writeFileSync(join(outputDir, 'gradle-stderr.log'), result.stderr ?? '');
  if (result.status !== 0) {
    throw new Error(`Android debug APK build failed with status ${result.status}`);
  }
}

function resetScreenProjectionDialogs() {
  adb(['shell', 'input', 'keyevent', 'KEYCODE_BACK'], { allowFailure: true });
  adb(['shell', 'input', 'keyevent', 'KEYCODE_BACK'], { allowFailure: true });
  adb(['shell', 'input', 'keyevent', 'KEYCODE_BACK'], { allowFailure: true });
  adb(['shell', 'am', 'force-stop', packageId], { allowFailure: true });
}

function startEmulator() {
  const emulatorPath =
    process.env.OCENTRA_ANDROID_EMULATOR ??
    join(process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT, 'emulator', 'emulator.exe');
  const child = spawn(
    emulatorPath,
    [
      '-avd',
      avdName,
      '-no-window',
      '-no-audio',
      '-no-snapshot-save',
      '-gpu',
      process.env.OCENTRA_ANDROID_EMULATOR_GPU ?? 'swiftshader_indirect',
    ],
    {
      detached: true,
      stdio: 'ignore',
      windowsHide: true,
    }
  );
  child.unref();
}

function waitForDevice() {
  for (let attempt = 0; attempt < 150; attempt += 1) {
    const device = firstOnlineDevice();
    if (device !== undefined) {
      return device;
    }
    sleep(2000);
  }
  return null;
}

function waitForAndroidReady(device) {
  for (let attempt = 0; attempt < 150; attempt += 1) {
    const bootCompleted = adb(['shell', 'getprop', 'sys.boot_completed'], { allowFailure: true }).stdout.trim();
    const packageManagerReady =
      adb(['shell', 'cmd', 'package', 'path', 'android'], { allowFailure: true }).status === 0;
    const windowReady = adb(['shell', 'dumpsys', 'window'], { allowFailure: true }).stdout.includes('mCurrentFocus');
    if (bootCompleted === '1' && packageManagerReady && windowReady) {
      return;
    }
    sleep(2000);
  }
  throw new Error(`Android device/emulator did not become UI-ready for MediaProjection proof: ${device}`);
}

function ensureDeviceUnlocked(device) {
  adb(['shell', 'input', 'keyevent', 'KEYCODE_WAKEUP'], { allowFailure: true });
  adb(['shell', 'wm', 'dismiss-keyguard'], { allowFailure: true });
  sleep(1000);
  const dump = dumpUi();
  writeFileSync(join(outputDir, '00-unlock-state-ui.xml'), dump);
  if (/keyguard_pin_view|password_entry|Use biometrics or enter PIN|PIN unlock/i.test(dump)) {
    throw new Error(
      `Android physical proof target ${device} is locked behind keyguard/PIN; unlock the phone before rerunning physical MediaProjection proof.`
    );
  }
}

function firstOnlineDevice() {
  const result = adb(['devices'], { allowFailure: true });
  const lines = result.stdout.split(/\r?\n/).slice(1);
  const online = lines
    .map((line) => line.trim())
    .filter((line) => line.endsWith('\tdevice'))
    .map((line) => line.split(/\s+/)[0]);
  if (requestedSerial !== undefined) {
    if (!online.includes(requestedSerial)) {
      throw new Error(
        `Requested Android serial ${requestedSerial} is not online; online devices are ${JSON.stringify(online)}.`
      );
    }
    return requestedSerial;
  }
  return online.find((serial) => serial.startsWith('emulator-')) ?? online[0] ?? null;
}

function approveConsentDialog() {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    let dump = dumpUi();
    writeFileSync(join(outputDir, `consent-ui-${String(attempt).padStart(2, '0')}.xml`), dump);
    const captureScopeChoice = selectEntireScreenModeIfAvailable(dump, attempt);
    if (captureScopeChoice.selected) {
      dump = dumpUi();
      writeFileSync(join(outputDir, `consent-ui-${String(attempt).padStart(2, '0')}-entire-screen.xml`), dump);
    }
    const button = findClickableByText(dump, /(Start now|Start|Allow|OK|Record|Share)/i);
    if (button !== undefined) {
      adb(['shell', 'input', 'tap', String(button.x), String(button.y)]);
      return {
        approved: true,
        attempt,
        captureScopeChoice,
        clickedText: button.text,
        x: button.x,
        y: button.y,
      };
    }
    sleep(1000);
  }
  return { approved: false };
}

function selectEntireScreenModeIfAvailable(xml, attempt) {
  if (!/screen_share_mode_spinner/i.test(xml) || !/A single app/i.test(xml)) {
    return { selected: false, reason: 'not-required' };
  }
  const spinner = findNode(
    xml,
    (node) => attr(node, 'resource-id') === 'com.android.systemui:id/screen_share_mode_spinner',
    true
  );
  if (spinner === null) {
    return { selected: false, reason: 'spinner-not-found' };
  }
  adb(['shell', 'input', 'tap', String(spinner.x), String(spinner.y)]);
  sleep(500);
  const options = dumpUi();
  writeFileSync(join(outputDir, `consent-mode-ui-${String(attempt).padStart(2, '0')}.xml`), options);
  const entireScreen = findNode(options, (node) => /Entire screen/i.test(attr(node, 'text') ?? ''), false);
  if (entireScreen === null) {
    return { selected: false, reason: 'entire-screen-option-not-found' };
  }
  adb(['shell', 'input', 'tap', String(entireScreen.x), String(entireScreen.y)]);
  sleep(500);
  return {
    selected: true,
    text: entireScreen.text,
    x: entireScreen.x,
    y: entireScreen.y,
  };
}

function approveAppSelectorIfShown() {
  for (let attempt = 0; attempt < 25; attempt += 1) {
    const dump = dumpUi();
    writeFileSync(join(outputDir, `selector-ui-${String(attempt).padStart(2, '0')}.xml`), dump);
    if (!/Share or record an app|Apps list|media_projection/i.test(dump)) {
      return { approved: false, reason: 'selector-not-shown', attempt };
    }
    const target =
      findClickableByText(dump, /Ocentra Parent Agent/i) ??
      findClickableByContentDescription(dump, /Ocentra Parent Agent/i) ??
      findClickableByPackageOrRecentTask(dump);
    if (target !== undefined) {
      adb(['shell', 'input', 'tap', String(target.x), String(target.y)]);
      return {
        approved: true,
        attempt,
        clickedText: target.text,
        resourceId: target.resourceId,
        x: target.x,
        y: target.y,
      };
    }
    sleep(1000);
  }
  return { approved: false, reason: 'selector-target-not-found' };
}

function dumpUi() {
  adb(['shell', 'uiautomator', 'dump', '/sdcard/ocentra-screen-capture-ui.xml'], { allowFailure: true });
  return adb(['exec-out', 'cat', '/sdcard/ocentra-screen-capture-ui.xml'], { allowFailure: true }).stdout;
}

function findClickableByText(xml, pattern) {
  return findNode(
    xml,
    (node) => {
      const text = attr(node, 'text') ?? attr(node, 'content-desc') ?? '';
      return pattern.test(text);
    },
    true
  );
}

function findClickableByContentDescription(xml, pattern) {
  return findNode(xml, (node) => pattern.test(attr(node, 'content-desc') ?? ''), true);
}

function findClickableByPackageOrRecentTask(xml) {
  const recentTask = findNode(
    xml,
    (node) => {
      const description = attr(node, 'content-desc') ?? '';
      return description === 'Ocentra' || /Ocentra Parent Agent/i.test(description);
    },
    true
  );
  if (recentTask !== undefined) {
    return recentTask;
  }
  return null;
}

function findNode(xml, accepts, requireClickable) {
  const nodePattern = /<node\b[^>]*>/g;
  let match;
  let fallback = null;
  while ((match = nodePattern.exec(xml)) !== undefined) {
    const node = match[0];
    const text = attr(node, 'text') ?? attr(node, 'content-desc') ?? '';
    const bounds = attr(node, 'bounds');
    const clickable = attr(node, 'clickable') === 'true';
    const enabled = attr(node, 'enabled') === 'true';
    if ((requireClickable && !clickable) || !enabled || !accepts(node) || bounds === null) {
      continue;
    }
    const parsed = /\[(\d+),(\d+)\]\[(\d+),(\d+)\]/.exec(bounds);
    if (parsed === null) {
      continue;
    }
    const [, left, top, right, bottom] = parsed.map(Number);
    const candidate = {
      text,
      resourceId: attr(node, 'resource-id'),
      x: Math.round((left + right) / 2),
      y: Math.round((top + bottom) / 2),
    };
    if (candidate.resourceId === 'android:id/button1') {
      return candidate;
    }
    fallback ??= candidate;
  }
  return fallback;
}

function waitForProofJson() {
  for (let attempt = 0; attempt < 40; attempt += 1) {
    const result = adb(['exec-out', 'run-as', packageId, 'cat', proofFile], { allowFailure: true });
    if (result.status === 0 && result.stdout.trim().startsWith('{')) {
      return JSON.parse(result.stdout);
    }
    sleep(1000);
  }
  throw new Error('Android MediaProjection proof JSON was not written.');
}

function adb(args, options = {}) {
  let result = spawnAdb(args);
  if (
    result.status !== 0 &&
    options.allowFailure !== true &&
    /device offline|device still connecting|more than one device\/emulator/i.test(result.stderr || result.stdout)
  ) {
    waitForDevice();
    result = spawnAdb(args);
  }
  if (result.status !== 0 && options.allowFailure !== true) {
    throw new Error(`adb ${args.join(' ')} failed: ${result.stderr || result.stdout}`);
  }
  return result;
}

function spawnAdb(args) {
  const serialArgs = requestedSerial === null ? [] : ['-s', requestedSerial];
  return spawnSync('adb', [...serialArgs, ...args], {
    cwd: process.cwd(),
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
}

function attr(node, name) {
  const match = new RegExp(`${name}="([^"]*)"`).exec(node);
  return match?.[1] ?? null;
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}
