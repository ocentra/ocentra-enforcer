import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = resolve(repoRoot, 'output', 'screen-plan-proof', 'local-platform-proof-batch');
const proofPath = join(outputDir, 'proof-summary.json');
const commandsPath = join(outputDir, '10-validation-commands.log');

const artifacts = {
  windowsActiveWindow: 'output/screen-plan-proof/real-capture/manual-parent-test-active-window/proof-summary.json',
  windowsScopeMatrix: 'output/screen-plan-proof/real-capture/scope-matrix/proof-summary.json',
  androidMediaProjection: 'output/screen-plan-proof/android-mediaprojection/proof-summary.json',
  androidCapability: 'output/screen-plan-proof/android/proof-summary.json',
  androidPhysicalTargetReadiness: 'output/screen-plan-proof/android-physical-target-readiness/proof-summary.json',
  androidPhysicalExternalGate: 'output/screen-plan-proof/android-physical-external-gate-analysis/proof-summary.json',
  linuxWslg: 'output/screen-plan-proof/linux-wslg/proof-summary.json',
  linuxWslgExternalGate: 'output/screen-plan-proof/linux-wslg-external-gate-analysis/proof-summary.json',
  linuxCapability: 'output/screen-plan-proof/linux/proof-summary.json',
  externalGates: 'output/screen-plan-proof/external-gates/proof-summary.json',
};

const args = new Set(process.argv.slice(2));
if (args.has('--run-local')) {
  runWindowsProofs();
  runAndroidProofs();
  runLinuxProofs();
} else {
  if (args.has('--run-windows')) {
    runWindowsProofs();
  }
  if (args.has('--run-android')) {
    runAndroidProofs();
  }
  if (args.has('--run-android-target-readiness')) {
    runAndroidTargetReadinessProof();
  }
  if (args.has('--run-android-physical')) {
    runAndroidPhysicalProofs();
  }
  if (args.has('--run-linux')) {
    runLinuxProofs();
  }
}

const windowsActiveWindow = readJson(artifacts.windowsActiveWindow);
const windowsScopeMatrix = readJson(artifacts.windowsScopeMatrix);
const androidMediaProjection = readJson(artifacts.androidMediaProjection);
const androidCapability = readJson(artifacts.androidCapability);
const androidPhysicalTargetReadiness = readOptionalJson(artifacts.androidPhysicalTargetReadiness);
const androidPhysicalExternalGate = readOptionalJson(artifacts.androidPhysicalExternalGate);
const linuxWslg = readJson(artifacts.linuxWslg);
const linuxWslgExternalGate = readJson(artifacts.linuxWslgExternalGate);
const linuxCapability = readJson(artifacts.linuxCapability);
const externalGates = readJson(artifacts.externalGates);
const hostInventory = collectHostInventory();

const rows = [
  windowsActiveWindowRow(windowsActiveWindow),
  windowsScopeMatrixRow(windowsScopeMatrix),
  androidEmulatorRow(androidMediaProjection, androidCapability, hostInventory),
  androidPhysicalRow(
    androidMediaProjection,
    androidCapability,
    androidPhysicalTargetReadiness,
    androidPhysicalExternalGate,
    externalGates,
    hostInventory
  ),
  ...linuxWslgRows(linuxWslg, linuxWslgExternalGate, linuxCapability, externalGates),
  linuxNativeWaylandRow(linuxCapability),
  appleExternalRow('macos-screencapturekit', 'macOS ScreenCaptureKit live permission and capture proof'),
  appleExternalRow('ios-replaykit', 'iOS ReplayKit physical-device capture proof'),
];

const proof = {
  proof: 'screen-local-platform-proof-batch',
  generatedAt: new Date().toISOString(),
  sourceArtifacts: artifacts,
  localHost: {
    platform: process.platform,
    windowsRunnableHere: process.platform === 'win32',
    androidRunnableHere: true,
    linuxRunnableHere: true,
    macosRunnableHere: process.platform === 'darwin',
    iosRunnableHere: false,
    inventory: hostInventory,
  },
  closure: {
    windowsCaptureComplete:
      rowsByStatus('windows-active-window', 'proved') && rowsByStatus('windows-scope-matrix', 'proved'),
    androidEmulatorCaptureComplete: rowsByStatus('android-mediaprojection-emulator', 'proved'),
    androidPhysicalCaptureComplete: rowsByStatus('android-mediaprojection-physical', 'proved'),
    linuxWslgCaptureComplete: rowsByStatus('linux-wslg-x11-selected-window', 'proved'),
    linuxWslgExternalGateComplete: rowsByStatus('linux-wslg-external-gate', 'proved'),
    nativeLinuxWaylandComplete: rowsByStatus('linux-native-wayland-pipewire', 'proved'),
    macosCaptureComplete: rowsByStatus('macos-screencapturekit', 'proved'),
    iosCaptureComplete: rowsByStatus('ios-replaykit', 'proved'),
    localWindowsAndroidLinuxProofsAccounted: true,
    productCompletePlatformCaptureReady: false,
  },
  rows,
  nonClaims: [
    'This batch verifies the local platform evidence already produced on the Windows worker host and records which gates remain external.',
    'Android emulator MediaProjection is not physical Android parity.',
    'Android physical-device proof requires a current online physical device in adb inventory, not only retained emulator artifacts.',
    'WSLg/X11 selected-window capture is not native Linux Wayland/PipeWire or root-display parity.',
    'WSLg/X11 external-gate proof retains a controlled visual inspection artifact and does not retain the raw product capture queue image.',
    'macOS and iOS native capture cannot be proved from this Windows worker host and remain external/CI/manual-required gates.',
    'This batch does not claim screen-plan product completion while any external platform gate remains external-required.',
  ],
};

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(commandsPath, validationCommands());
console.log(`screen-local-platform-proof-batch-ok:${relativePath(proofPath)}`);

function windowsActiveWindowRow(summary) {
  assert(summary.proof === 'screen-capture-real-proof', 'Windows active-window proof id mismatch');
  assert(summary.platform === 'win32', 'Windows active-window proof did not run on win32');
  assert(summary.captured === true, 'Windows active-window proof did not capture pixels');
  assert(summary.realCaptureProof === true, 'Windows active-window proof is not real');
  assert(summary.degradedIsCaptureProof === false, 'Windows active-window proof is degraded');
  return {
    platformGate: 'windows-active-window',
    status: 'proved',
    artifact: artifacts.windowsActiveWindow,
    capturedPixels: true,
    rawImageDeleted: true,
    productReadyContribution: 'local-windows-capture-ready',
  };
}

function windowsScopeMatrixRow(summary) {
  assert(summary.proof === 'screen-capture-scope-matrix-proof', 'Windows scope-matrix proof id mismatch');
  assert(summary.platform === 'win32', 'Windows scope-matrix proof did not run on win32');
  assert(summary.realCaptureRuns === 3, 'Windows scope-matrix expected three real capture runs');
  assert(summary.capturedRuns === 3, 'Windows scope-matrix did not capture all scopes');
  assert(summary.allRawImagesDeleted === true, 'Windows scope-matrix did not delete all raw images');
  assert(summary.allScopesMatched === true, 'Windows scope-matrix scopes did not match');
  return {
    platformGate: 'windows-scope-matrix',
    status: 'proved',
    artifact: artifacts.windowsScopeMatrix,
    capturedPixels: true,
    rawImageDeleted: true,
    productReadyContribution: 'local-windows-scope-ready',
  };
}

function androidEmulatorRow(summary, capability, inventory) {
  assert(summary.proof === 'child-android-screen-capture-mediaprojection-proof', 'Android proof id mismatch');
  assert(summary.consentApproved === true, 'Android proof missing explicit consent');
  assert(summary.captured === true, 'Android proof did not capture pixels');
  assert(summary.rawTempDeleted === true, 'Android proof did not delete raw temp frame');
  assert(capability.gapStatus?.emulatorMediaProjectionProofExists === true, 'Android capability lost emulator proof');
  const retainedSerial = String(summary.deviceInfo?.serial ?? '');
  const retainedEmulatorStillOnline =
    retainedSerial.length > 0 && inventory.android.onlineEmulatorSerials.includes(retainedSerial);
  return {
    platformGate: 'android-mediaprojection-emulator',
    status: 'proved',
    artifact: artifacts.androidMediaProjection,
    capabilityArtifact: artifacts.androidCapability,
    capturedPixels: true,
    rawImageDeleted: true,
    deviceSerial: summary.deviceInfo?.serial ?? null,
    deviceModel: summary.deviceInfo?.model ?? null,
    api: summary.deviceInfo?.api ?? null,
    currentHostDeviceOnline: retainedEmulatorStillOnline,
    currentHostAvds: inventory.android.avds,
    productReadyContribution: 'android-emulator-proof-only',
  };
}

function androidPhysicalRow(summary, capability, targetReadiness, externalGateSummary, externalGateProof, inventory) {
  const serial = String(summary.deviceInfo?.serial ?? '');
  const physicalDevice = serial.length > 0 && !serial.startsWith('emulator-');
  const physicalDeviceOnline = inventory.android.onlinePhysicalSerials.length > 0;
  const targetReady = targetReadiness?.readiness?.mediaProjectionProofRunnableNow === true;
  const externalGateSatisfied =
    externalGateSummary?.assertions?.androidExternalGateSatisfied === true &&
    externalGateProof.gateResults?.some(
      (gate) => gate.gateId === 'android-physical-mediaprojection-capture' && gate.status === 'satisfied'
    ) === true;
  return {
    platformGate: 'android-mediaprojection-physical',
    status:
      physicalDevice &&
      physicalDeviceOnline &&
      capability.gapStatus?.physicalAndroidDeviceProofExists === true &&
      externalGateSatisfied
        ? 'proved'
        : 'external-required',
    artifact: artifacts.androidMediaProjection,
    capabilityArtifact: artifacts.androidCapability,
    targetReadinessArtifact: artifacts.androidPhysicalTargetReadiness,
    externalGateArtifact: artifacts.androidPhysicalExternalGate,
    capturedPixels: physicalDevice && physicalDeviceOnline && summary.captured === true,
    rawImageDeleted: physicalDevice && physicalDeviceOnline && summary.rawTempDeleted === true,
    localVlmAnalyzedCapturedArtifact: externalGateSummary?.assertions?.localVlmAnalyzedRetainedArtifact === true,
    physicalTargetObservedOnline: targetReadiness?.assertions?.targetObservedOnline === true,
    physicalTargetLockedBehindKeyguard: targetReadiness?.keyguard?.lockedBehindCredentialPrompt === true,
    physicalTargetRunnableNow: targetReady,
    deviceSerial: serial || null,
    currentHostOnlinePhysicalSerials: inventory.android.onlinePhysicalSerials,
    productReadyContribution: 'required-for-android-physical-product-claim',
    reason: physicalDevice
      ? 'Physical Android device proof still requires capability and external gate proof to record physical parity.'
      : targetReadiness?.keyguard?.lockedBehindCredentialPrompt === true
        ? 'Physical Android target is online but locked behind keyguard/PIN; unlock the target and rerun physical MediaProjection proof before product claim.'
        : 'Current proof is emulator-only or no physical Android is online; connect physical Android and rerun MediaProjection proof before product claim.',
  };
}

function linuxWslgRows(summary, externalGateSummary, capability, externalGateProof) {
  assert(summary.proof === 'screen-capture-linux-wslg-proof', 'Linux WSLg proof id mismatch');
  assert(
    externalGateSummary.proof === 'screen-linux-wslg-external-gate-proof',
    'Linux WSLg external gate proof id mismatch'
  );
  assert(summary.selectedWindow?.captured === true, 'Linux WSLg proof did not capture selected window');
  assert(summary.selectedWindow?.actualScope === 'selectedWindow', 'Linux WSLg proof scope mismatch');
  assert(summary.custody?.rawImageDeleted === true, 'Linux WSLg proof did not delete raw image');
  assert(summary.custody?.existsAfterDelete === false, 'Linux WSLg raw image still exists');
  assert(externalGateSummary.assertions?.linuxExternalGateSatisfied === true, 'Linux external gate is not satisfied');
  assert(
    externalGateProof.gateResults?.some(
      (gate) => gate.gateId === 'linux-desktop-session-capture' && gate.status === 'satisfied'
    ) === true,
    'External gate proof does not show the Linux desktop-session gate satisfied'
  );
  assert(capability.gapStatus?.wslgX11SelectedWindowProofExists === true, 'Linux capability lost WSLg proof');
  return [
    {
      platformGate: 'linux-wslg-x11-selected-window',
      status: 'proved',
      artifact: artifacts.linuxWslg,
      capabilityArtifact: artifacts.linuxCapability,
      capturedPixels: true,
      rawImageDeleted: true,
      display: summary.session?.display ?? null,
      productReadyContribution: 'wslg-selected-window-proof-only',
    },
    {
      platformGate: 'linux-wslg-external-gate',
      status: 'proved',
      artifact: artifacts.linuxWslg,
      externalGateArtifact: artifacts.linuxWslgExternalGate,
      externalGateProof: artifacts.externalGates,
      retainedInspectionArtifact: externalGateSummary.retainedInspectionArtifact?.path ?? null,
      capturedPixels: true,
      localVlmAnalyzedCapturedArtifact: true,
      rawImageDeleted: true,
      display: summary.session?.display ?? null,
      productReadyContribution: 'wslg-x11-selected-window-external-gate-ready',
    },
  ];
}

function linuxNativeWaylandRow(capability) {
  return {
    platformGate: 'linux-native-wayland-pipewire',
    status: capability.gapStatus?.nativeWaylandPipeWireProofExists === true ? 'proved' : 'external-required',
    artifact: artifacts.linuxCapability,
    capturedPixels: capability.gapStatus?.nativeWaylandPipeWireProofExists === true,
    rawImageDeleted: capability.gapStatus?.nativeWaylandPipeWireProofExists === true,
    productReadyContribution: 'required-for-native-linux-wayland-product-claim',
    reason: 'Native Linux Wayland/PipeWire portal proof is separate from WSLg/X11 selected-window proof.',
  };
}

function appleExternalRow(platformGate, reason) {
  return {
    platformGate,
    status:
      process.platform === 'darwin' && platformGate === 'macos-screencapturekit' ? 'not-run-here' : 'external-required',
    artifact: null,
    capturedPixels: false,
    rawImageDeleted: false,
    productReadyContribution: 'required-for-apple-platform-product-claim',
    reason,
  };
}

function rowsByStatus(platformGate, status) {
  return rows.find((row) => row.platformGate === platformGate)?.status === status;
}

function runWindowsProofs() {
  runNode('scripts/test/screen-capture-real-proof.mjs');
  runNode('scripts/test/screen-capture-scope-matrix-proof.mjs');
}

function runAndroidProofs() {
  runNode('scripts/test/child-android-screen-capture-mediaprojection-proof.mjs');
  runNode('scripts/test/screen-android-mediaprojection-capability-proof.mjs');
}

function runAndroidTargetReadinessProof() {
  runNode('scripts/test/screen-android-physical-target-readiness-proof.mjs');
}

function runAndroidPhysicalProofs() {
  runNode('scripts/test/screen-android-physical-external-gate-proof.mjs');
}

function runLinuxProofs() {
  runNode('scripts/test/screen-linux-capture-capability-proof.mjs');
  runNode('scripts/test/screen-linux-wslg-external-gate-proof.mjs');
}

function runNode(script) {
  execFileSync(process.execPath, [script], { cwd: repoRoot, stdio: 'inherit' });
}

function collectHostInventory() {
  const adbDevices = tryExec('adb', ['devices', '-l']);
  const avds = tryExec(resolveAndroidTool('emulator'), ['-list-avds']);
  const wslUname = tryExec('wsl.exe', ['bash', '-lc', 'uname -sr']);
  const wslDisplay = tryExec('wsl.exe', ['bash', '-lc', 'printf "%s" "${DISPLAY:-}"']);
  const androidDevices = parseAdbDevices(adbDevices.stdout);
  return {
    android: {
      sdkRoot: process.env.ANDROID_SDK_ROOT ?? process.env.ANDROID_HOME ?? null,
      adbAvailable: adbDevices.ok,
      adbError: adbDevices.ok ? null : adbDevices.error,
      avdToolAvailable: avds.ok,
      avdError: avds.ok ? null : avds.error,
      avds: avds.stdout
        .split(/\r?\n/u)
        .map((line) => line.trim())
        .filter((line) => line.length > 0),
      devices: androidDevices,
      onlineEmulatorSerials: androidDevices
        .filter((device) => device.state === 'device' && device.serial.startsWith('emulator-'))
        .map((device) => device.serial),
      onlinePhysicalSerials: androidDevices
        .filter((device) => device.state === 'device' && !device.serial.startsWith('emulator-'))
        .map((device) => device.serial),
    },
    linux: {
      wslAvailable: wslUname.ok,
      wslError: wslUname.ok ? null : wslUname.error,
      wslKernel: wslUname.stdout.trim() || null,
      display: wslDisplay.stdout.trim() || null,
    },
  };
}

function parseAdbDevices(output) {
  return output
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith('List of devices'))
    .map((line) => {
      const [serial, state, ...details] = line.split(/\s+/u);
      return {
        serial,
        state: state ?? 'unknown',
        details: details.join(' '),
      };
    });
}

function resolveAndroidTool(tool) {
  const sdkRoot = process.env.ANDROID_SDK_ROOT ?? process.env.ANDROID_HOME;
  if (sdkRoot === undefined || sdkRoot.length === 0) {
    return tool;
  }
  if (tool === 'emulator') {
    return join(sdkRoot, 'emulator', process.platform === 'win32' ? 'emulator.exe' : 'emulator');
  }
  return tool;
}

function tryExec(command, args) {
  try {
    return {
      ok: true,
      stdout: execFileSync(command, args, {
        cwd: repoRoot,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
        timeout: 10_000,
      }),
      error: null,
    };
  } catch (error) {
    return {
      ok: false,
      stdout: String(error.stdout ?? ''),
      error: String(error.stderr ?? error.message ?? error),
    };
  }
}

function readJson(path) {
  const absolute = resolve(repoRoot, path);
  assert(existsSync(absolute), `missing proof artifact ${path}`);
  return JSON.parse(readFileSync(absolute, 'utf8'));
}

function readOptionalJson(path) {
  const absolute = resolve(repoRoot, path);
  return existsSync(absolute) ? JSON.parse(readFileSync(absolute, 'utf8')) : null;
}

function validationCommands() {
  return [
    'node --check scripts/test/screen-local-platform-proof-batch.mjs',
    'node --check scripts/test/screen-linux-wslg-external-gate-proof.mjs',
    'node scripts/test/screen-linux-wslg-external-gate-proof.mjs',
    'node scripts/test/screen-local-platform-proof-batch.mjs',
    'node scripts/test/screen-local-platform-proof-batch.mjs --run-windows',
    'node scripts/test/screen-local-platform-proof-batch.mjs --run-android',
    'OCENTRA_ANDROID_SERIAL=192.168.2.45:5555 node scripts/test/screen-local-platform-proof-batch.mjs --run-android-target-readiness',
    'OCENTRA_ANDROID_SERIAL=192.168.2.45:5555 node scripts/test/screen-local-platform-proof-batch.mjs --run-android-physical',
    'node scripts/test/screen-local-platform-proof-batch.mjs --run-linux',
    'node scripts/test/screen-local-platform-proof-batch.mjs --run-local',
    '',
  ].join('\n');
}

function relativePath(path) {
  return relative(repoRoot, path).replace(/\\/gu, '/');
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
