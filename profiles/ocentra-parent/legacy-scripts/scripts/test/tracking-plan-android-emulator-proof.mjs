import { spawn, spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const proofMode = 'tracking-plan-android-emulator-proof';
const packageName = 'ca.ocentra.parent.agent';
const expectedActivity = 'ca.ocentra.parent.agent/.MainActivity';
const serviceName = 'OcentraParentAgentService';
const appLaunchText = 'Ocentra Parent Agent service scaffold is running.';
const androidDocumentedGeofenceLimitPerAppPerDeviceUser = 100;
const androidGeofenceLimitSourceUrl = 'https://developer.android.com/develop/sensors-and-location/location/geofencing';
const androidStatusGapProofRelativePath =
  'output/tracking-plan-proof/10-android-battery-connectivity-and-status-adapter/17-status-gap-proof.json';
const androidStatusGapEvidenceRelativePath = 'test-results/tracking-android-status-proof/proof.json';
const output08 = path.join(repoRoot, 'output', 'tracking-plan-proof', '08-android-foreground-location-adapter');
const output09 = path.join(
  repoRoot,
  'output',
  'tracking-plan-proof',
  '09-android-background-location-and-geofence-adapter'
);
const output10 = path.join(
  repoRoot,
  'output',
  'tracking-plan-proof',
  '10-android-battery-connectivity-and-status-adapter'
);
const resultDir = path.join(repoRoot, 'test-results', proofMode);
const proofPath = path.join(resultDir, 'proof.json');
const apkPath = path.join(
  repoRoot,
  'target',
  'release-packages',
  'android',
  'ocentra-parent-agent-android-debug-latest.apk'
);
const commands = [];
let startedEmulator = false;
let selectedSerial = null;

try {
  await main();
} finally {
  if (startedEmulator && selectedSerial !== undefined) {
    await shutdownEmulator(resolveAndroidTools(), selectedSerial);
  }
}

async function main() {
  const tools = resolveAndroidTools();
  await mkdirProofRoots();
  await runNpm(['run', 'release:package:android']);
  assertFileExists(apkPath, 'Android debug APK');

  selectedSerial = await ensureDevice(tools);
  await adb(tools, selectedSerial, ['logcat', '-c']);
  await adb(tools, selectedSerial, ['install', '-r', apkPath], {
    artifact: path.join(resultDir, '01-adb-install.txt'),
  });
  const preGrantPackageDump = await adbText(tools, selectedSerial, ['shell', 'dumpsys', 'package', packageName]);
  const preGrantPermissionState = parsePermissionState(preGrantPackageDump);
  const foregroundPermissionUx = await collectForegroundPermissionUx(tools, selectedSerial, preGrantPermissionState);
  await grantForegroundLocationPermissions(tools, selectedSerial, preGrantPermissionState);
  const backgroundSettingsPage = await collectBackgroundLocationSettingsPage(
    tools,
    selectedSerial,
    preGrantPermissionState
  );
  await grantBackgroundLocationPermission(tools, selectedSerial, preGrantPermissionState);
  await wakeAndDismissKeyguardForProof(tools, selectedSerial, path.join(resultDir, '02-wake-before-proof-drive.txt'));
  await forceStopPackageForProof(tools, selectedSerial);
  await resetGeofenceTransitionProof(tools, selectedSerial);
  await resetBackgroundLocationSampleProof(tools, selectedSerial);

  const device = await readDeviceMetadata(tools, selectedSerial);
  const packageDump = await adbText(tools, selectedSerial, ['shell', 'dumpsys', 'package', packageName]);
  const resolvedActivity = await adbText(tools, selectedSerial, [
    'shell',
    'cmd',
    'package',
    'resolve-activity',
    '--brief',
    packageName,
  ]);
  await writeText(path.join(resultDir, '02-resolve-activity.txt'), resolvedActivity);
  assertIncludes(resolvedActivity, 'MainActivity', 'resolved launcher activity');

  await seedEmulatorGeofenceLocation(tools, selectedSerial, {
    label: 'initial-outside-geofence',
    artifact: '16-geofence-transition-route.txt',
    longitude: '-122.090',
    latitude: '37.427',
  });
  await launchActivityForProof(tools, selectedSerial, path.join(resultDir, '03-launch-activity.txt'));
  await wakeAndDismissKeyguardForProof(tools, selectedSerial, path.join(resultDir, '03-launch-activity.txt'));
  await seedEmulatorForegroundLocation(tools, selectedSerial, preGrantPermissionState);
  await driveBackgroundLocationSample(tools, selectedSerial, preGrantPermissionState);
  await driveEmulatorGeofenceTransitions(tools, selectedSerial, preGrantPermissionState);
  await refreshActivityAfterGeofenceRoute(tools, selectedSerial);
  await delay(10_000);

  const runtime = await collectRuntimeArtifacts(tools, selectedSerial);
  const permissionState = parsePermissionState(packageDump);
  const proof = buildProof({
    device,
    packageDump,
    permissionState,
    resolvedActivity,
    runtime,
    tools,
    foregroundPermissionUx,
    backgroundSettingsPage,
  });
  await writeProofFiles(proof);
  assertFocusedProofStrength(proof);

  console.log('tracking-plan-android-emulator-proof-ok');
  console.log(`evidence=${relativePath(proofPath)}`);
}

function assertFocusedProofStrength(proof) {
  const foregroundProof = proof.workpackProof['08-android-foreground-location-adapter'];
  const backgroundProof = proof.workpackProof['09-android-background-location-and-geofence-adapter'];
  const statusProof = proof.workpackProof['10-android-battery-connectivity-and-status-adapter'];
  const failures = [];
  if (!proof.runtime.activity.packageFocused) {
    failures.push('package launch/focus was not observed during final runtime collection');
  }
  if (!proof.runtime.service.isForeground) {
    failures.push('foreground service was not observed during final runtime collection');
  }
  if (proof.runtime.backgroundLocationSample.sampleCount <= 0) {
    failures.push('background location sample storage was not observed');
  }
  if (
    proof.runtime.geofenceTransitions.enterCount <= 0 ||
    proof.runtime.geofenceTransitions.exitCount <= 0 ||
    proof.runtime.geofenceTransitions.transitionCount <= 0
  ) {
    failures.push('local geofence enter/exit transition rows were not observed');
  }
  if (proof.runtime.geofenceTransitions.dwellCount <= 0) {
    failures.push('app-owned local geofence dwell row was not observed');
  }
  if (!proof.runtime.geofenceTransitions.systemProximityRegistered) {
    failures.push('Android LocationManager addProximityAlert registration metadata was not observed');
  }
  if (!proof.runtime.activeGeofenceLimit.observed || !proof.runtime.activeGeofenceLimit.withinDocumentedLimit) {
    failures.push('active geofence limit representation was not observed within the documented limit');
  }
  if (!backgroundDegradedStatusProof().observed) {
    failures.push('WP10 degraded-status bridge proof was not observed');
  }
  if (
    ![
      'foreground_permission_granted_fused_current_sample_observed',
      'foreground_permission_granted_fused_sample_observed',
      'foreground_permission_ux_fused_current_sample_observed',
      'foreground_permission_ux_fused_sample_observed',
      'foreground_permission_granted_current_sample_observed',
      'foreground_permission_ux_current_sample_raw_coordinate_observed',
      'foreground_permission_granted_current_sample_raw_coordinate_observed',
    ].includes(foregroundProof.status) ||
    !backgroundProof.status.includes('system_registration_and_degraded_status_observed') ||
    statusProof.status !== 'emulator_scaffold_and_status_gap_bridge_observed' ||
    statusProof.reason !==
      'Emulator package launch, foreground service state, battery dump, connectivity dump, and WP10 low-power/app-restart/pending-upload/manual-required bridge were collected.'
  ) {
    failures.push(
      `workpack statuses regressed: WP08=${foregroundProof.status}; WP09=${backgroundProof.status}; WP10=${statusProof.status}`
    );
  }
  if (failures.length > 0) {
    throw new Error(`Tracking Android emulator proof regressed:\n- ${failures.join('\n- ')}`);
  }
}

async function collectForegroundPermissionUx(tools, serial, permissionState) {
  const resetLog = [];
  if (!permissionState.locationPermissionRequested) {
    const skipped = {
      observed: false,
      reason: 'foreground location permissions are not declared by the package',
      resetArtifact: relativePath(path.join(resultDir, '13-foreground-location-permission-ux-reset.txt')),
      uiArtifact: relativePath(path.join(resultDir, '13-foreground-location-permission-ux.xml')),
    };
    await writeText(
      path.join(resultDir, '13-foreground-location-permission-ux-reset.txt'),
      'SKIP foreground location permissions are not declared by the package.\n'
    );
    await writeText(
      path.join(resultDir, '13-foreground-location-permission-ux.xml'),
      'SKIP foreground location permissions are not declared by the package.\n'
    );
    await writeJson(path.join(resultDir, '13-foreground-location-permission-ux.json'), skipped);
    return skipped;
  }

  for (const permission of ['android.permission.ACCESS_FINE_LOCATION', 'android.permission.ACCESS_COARSE_LOCATION']) {
    if (permissionState.requested.includes(permission)) {
      const revoke = await adbMaybe(tools, serial, ['shell', 'pm', 'revoke', packageName, permission]);
      resetLog.push(`${revoke.exitCode === 0 ? 'PASS' : 'FAIL'} pm revoke ${permission}\n${revoke.output.trim()}`);
      const clearFlags = await adbMaybe(tools, serial, [
        'shell',
        'pm',
        'clear-permission-flags',
        packageName,
        permission,
        'user-set',
        'user-fixed',
      ]);
      resetLog.push(
        `${clearFlags.exitCode === 0 ? 'PASS' : 'FAIL'} pm clear-permission-flags ${permission}\n${clearFlags.output.trim()}`
      );
    }
  }
  await writeText(path.join(resultDir, '13-foreground-location-permission-ux-reset.txt'), resetLog.join('\n\n'));

  await adb(tools, serial, ['shell', 'am', 'start', '-n', expectedActivity], {
    artifact: path.join(resultDir, '13-foreground-location-permission-ux-launch.txt'),
  });
  await delay(2_000);
  const uiDump = await adbText(tools, serial, ['exec-out', 'uiautomator', 'dump', '/dev/tty']);
  await writeText(path.join(resultDir, '13-foreground-location-permission-ux.xml'), uiDump);
  const parsed = parseForegroundPermissionDialogUi(uiDump);
  const proof = {
    ...parsed,
    resetArtifact: relativePath(path.join(resultDir, '13-foreground-location-permission-ux-reset.txt')),
    launchArtifact: relativePath(path.join(resultDir, '13-foreground-location-permission-ux-launch.txt')),
    uiArtifact: relativePath(path.join(resultDir, '13-foreground-location-permission-ux.xml')),
  };
  await writeJson(path.join(resultDir, '13-foreground-location-permission-ux.json'), proof);
  await adbMaybe(tools, serial, ['shell', 'input', 'keyevent', '4']);
  await delay(1_000);
  return proof;
}

async function collectBackgroundLocationSettingsPage(tools, serial, permissionState) {
  const launchArtifact = path.join(resultDir, '24-background-location-settings-page-launch.txt');
  const uiArtifact = path.join(resultDir, '24-background-location-settings-page.xml');
  const activityArtifact = path.join(resultDir, '24-background-location-settings-page-activity.txt');
  const windowArtifact = path.join(resultDir, '24-background-location-settings-page-window.txt');
  const proofArtifact = path.join(resultDir, '24-background-location-settings-page.json');
  if (!permissionState.backgroundLocationPermissionRequested) {
    const skipped = {
      observed: false,
      reason: 'background location permission is not declared by the package',
      launchArtifact: relativePath(launchArtifact),
      uiArtifact: relativePath(uiArtifact),
      activityArtifact: relativePath(activityArtifact),
      windowArtifact: relativePath(windowArtifact),
    };
    await writeText(launchArtifact, 'SKIP background location permission is not declared by the package.\n');
    await writeText(uiArtifact, 'SKIP background location permission is not declared by the package.\n');
    await writeText(activityArtifact, 'SKIP background location permission is not declared by the package.\n');
    await writeText(windowArtifact, 'SKIP background location permission is not declared by the package.\n');
    await writeJson(proofArtifact, skipped);
    return skipped;
  }

  const launch = await adb(
    tools,
    serial,
    ['shell', 'am', 'start', '-a', 'android.settings.APPLICATION_DETAILS_SETTINGS', '-d', `package:${packageName}`],
    {
      artifact: launchArtifact,
    }
  );
  await delay(2_000);
  const uiDump = await adbText(tools, serial, ['exec-out', 'uiautomator', 'dump', '/dev/tty']);
  const activityDump = await adbText(tools, serial, ['shell', 'dumpsys', 'activity', 'activities']);
  const windowDump = await adbText(tools, serial, ['shell', 'dumpsys', 'window']);
  await writeText(uiArtifact, uiDump);
  await writeText(activityArtifact, activityDump);
  await writeText(windowArtifact, windowDump);
  const parsed = parseBackgroundLocationSettingsPageUi({ uiDump, activityDump, windowDump, launch });
  const proof = {
    ...parsed,
    launchArtifact: relativePath(launchArtifact),
    uiArtifact: relativePath(uiArtifact),
    activityArtifact: relativePath(activityArtifact),
    windowArtifact: relativePath(windowArtifact),
  };
  await writeJson(proofArtifact, proof);
  await adbMaybe(tools, serial, ['shell', 'input', 'keyevent', '4']);
  await adbMaybe(tools, serial, ['shell', 'am', 'start', '-n', expectedActivity]);
  await delay(1_000);
  return proof;
}

async function grantForegroundLocationPermissions(tools, serial, permissionState) {
  if (!permissionState.locationPermissionRequested) {
    await writeText(
      path.join(resultDir, '13-foreground-location-permission-grant.txt'),
      'SKIP foreground location permissions are not declared by the package.\n'
    );
    return;
  }

  const grantLog = [];
  for (const permission of ['android.permission.ACCESS_FINE_LOCATION', 'android.permission.ACCESS_COARSE_LOCATION']) {
    if (permissionState.requested.includes(permission)) {
      const result = await adbMaybe(tools, serial, ['shell', 'pm', 'grant', packageName, permission]);
      const status = result.exitCode === 0 ? 'PASS' : 'FAIL';
      grantLog.push(`${status} pm grant ${permission}\n${result.output.trim()}`);
    }
  }
  await writeText(path.join(resultDir, '13-foreground-location-permission-grant.txt'), grantLog.join('\n\n'));
}

async function grantBackgroundLocationPermission(tools, serial, permissionState) {
  if (!permissionState.backgroundLocationPermissionRequested) {
    await writeText(
      path.join(resultDir, '15-background-location-permission-grant.txt'),
      'SKIP background location permission is not declared by the package.\n'
    );
    return;
  }

  const permission = 'android.permission.ACCESS_BACKGROUND_LOCATION';
  const result = await adbMaybe(tools, serial, ['shell', 'pm', 'grant', packageName, permission]);
  await writeText(
    path.join(resultDir, '15-background-location-permission-grant.txt'),
    `${result.exitCode === 0 ? 'PASS' : 'FAIL'} pm grant ${permission}\n${result.output.trim()}`
  );
}

async function seedEmulatorForegroundLocation(tools, serial, permissionState) {
  if (!serial.startsWith('emulator-') || !permissionState.locationPermissionRequested) {
    await writeText(
      path.join(resultDir, '14-foreground-location-seed.txt'),
      'SKIP foreground location seed requires an Android emulator with declared foreground location permissions.\n'
    );
    return;
  }

  const result = await adbMaybe(tools, serial, ['emu', 'geo', 'fix', '-122.084', '37.422']);
  await writeText(
    path.join(resultDir, '14-foreground-location-seed.txt'),
    `${result.exitCode === 0 ? 'PASS' : 'FAIL'} adb emu geo fix synthetic foreground location seed\n${result.output.trim()}`
  );
}

async function resetGeofenceTransitionProof(tools, serial) {
  const result = await adbMaybe(tools, serial, [
    'shell',
    'run-as',
    packageName,
    'rm',
    '-f',
    'shared_prefs/tracking_geofence_transition_proof.xml',
  ]);
  await writeText(
    path.join(resultDir, '16-geofence-transition-route.txt'),
    `${result.exitCode === 0 ? 'PASS' : 'FAIL'} reset emulator geofence transition proof storage\n${result.output.trim()}`
  );
}

async function forceStopPackageForProof(tools, serial) {
  const result = await adbMaybe(tools, serial, ['shell', 'am', 'force-stop', packageName]);
  await writeText(
    path.join(resultDir, '02-force-stop-before-proof-drive.txt'),
    `${result.exitCode === 0 ? 'PASS' : 'FAIL'} am force-stop ${packageName} before clearing proof storage\n${result.output.trim()}`
  );
}

async function resetBackgroundLocationSampleProof(tools, serial) {
  const result = await adbMaybe(tools, serial, [
    'shell',
    'run-as',
    packageName,
    'rm',
    '-f',
    'shared_prefs/tracking_background_location_sample_proof.xml',
  ]);
  await writeText(
    path.join(resultDir, '20-background-location-sample-prefs.xml'),
    `${result.exitCode === 0 ? 'PASS' : 'FAIL'} reset emulator background location sample proof storage\n${result.output.trim()}`
  );
}

async function driveBackgroundLocationSample(tools, serial, permissionState) {
  if (!serial.startsWith('emulator-') || !permissionState.backgroundLocationPermissionRequested) {
    await appendText(
      path.join(resultDir, '20-background-location-sample-prefs.xml'),
      'SKIP background sample proof requires an Android emulator with declared background location permission.\n'
    );
    return;
  }

  await adb(tools, serial, ['shell', 'input', 'keyevent', '3'], {
    artifact: path.join(resultDir, '21-background-activity-for-sample.txt'),
  });
  await delay(2_000);
  const result = await adbMaybe(tools, serial, ['emu', 'geo', 'fix', '-122.083', '37.421']);
  await writeText(
    path.join(resultDir, '22-background-location-sample-route.txt'),
    `${result.exitCode === 0 ? 'PASS' : 'FAIL'} background-activity adb emu geo fix -122.083 37.421\n${result.output.trim()}`
  );
  await delay(5_000);
  await launchActivityForProof(tools, serial, path.join(resultDir, '23-relaunch-after-background-sample.txt'));
  await wakeAndDismissKeyguardForProof(tools, serial, path.join(resultDir, '23-relaunch-after-background-sample.txt'));
  await delay(2_000);
}

async function driveEmulatorGeofenceTransitions(tools, serial, permissionState) {
  if (!serial.startsWith('emulator-') || !permissionState.backgroundLocationPermissionRequested) {
    await appendText(
      path.join(resultDir, '16-geofence-transition-route.txt'),
      'SKIP geofence transition route requires an Android emulator with declared background location permission.\n'
    );
    return;
  }

  await seedEmulatorGeofenceLocation(tools, serial, {
    label: 'outside-geofence-after-registration',
    artifact: '16-geofence-transition-route.txt',
    longitude: '-122.090',
    latitude: '37.427',
  });
  await waitForGeofenceState(tools, serial, 'outside-geofence-baseline', (prefs) => prefs.hasInsideState);
  for (let attempt = 1; attempt <= 4; attempt += 1) {
    await seedEmulatorGeofenceLocation(tools, serial, {
      label: `inside-geofence-enter-${attempt}`,
      artifact: '16-geofence-transition-route.txt',
      longitude: '-122.084',
      latitude: '37.422',
    });
    const entered = await waitForGeofenceState(
      tools,
      serial,
      `inside-geofence-enter-${attempt}`,
      (prefs) => prefs.enterCount > 0
    );
    await delay(5_000);
    await seedEmulatorGeofenceLocation(tools, serial, {
      label: `inside-geofence-dwell-${attempt}`,
      artifact: '16-geofence-transition-route.txt',
      longitude: '-122.084',
      latitude: '37.422',
    });
    const dwelled = await waitForGeofenceState(
      tools,
      serial,
      `inside-geofence-dwell-${attempt}`,
      (prefs) => prefs.dwellCount > 0
    );
    await seedEmulatorGeofenceLocation(tools, serial, {
      label: `outside-geofence-exit-${attempt}`,
      artifact: '16-geofence-transition-route.txt',
      longitude: '-122.090',
      latitude: '37.427',
    });
    const exited = await waitForGeofenceState(
      tools,
      serial,
      `outside-geofence-exit-${attempt}`,
      (prefs) => prefs.exitCount > 0
    );
    if (entered && dwelled && exited) {
      return;
    }
  }
}

async function seedEmulatorGeofenceLocation(tools, serial, { label, artifact, longitude, latitude }) {
  if (!serial.startsWith('emulator-')) {
    await appendText(path.join(resultDir, artifact), `SKIP ${label} requires an Android emulator.\n`);
    return;
  }
  await launchActivityForProof(tools, serial, path.join(resultDir, '16-geofence-transition-launches.txt'));
  const result = await adbMaybe(tools, serial, ['emu', 'geo', 'fix', longitude, latitude]);
  await appendText(
    path.join(resultDir, artifact),
    `${result.exitCode === 0 ? 'PASS' : 'FAIL'} ${label} adb emu geo fix ${longitude} ${latitude}\n${result.output.trim()}\n`
  );
}

async function waitForGeofenceState(tools, serial, label, predicate) {
  for (let attempt = 1; attempt <= 6; attempt += 1) {
    await delay(2_000);
    const prefs = await readGeofenceTransitionPrefs(tools, serial);
    await appendText(
      path.join(resultDir, '16-geofence-transition-route.txt'),
      `POLL ${label} attempt ${attempt} transitionCount=${prefs.parsed.transitionCount} enterCount=${prefs.parsed.enterCount} exitCount=${prefs.parsed.exitCount} dwellCount=${prefs.parsed.dwellCount} insideState=${String(prefs.parsed.insideState)}\n`
    );
    if (predicate(prefs.parsed)) {
      return true;
    }
  }
  return false;
}

async function refreshActivityAfterGeofenceRoute(tools, serial) {
  await writeText(
    path.join(resultDir, '18-refresh-activity-after-geofence-route.txt'),
    'SKIP force-stop before refresh; preserve foreground-service proof state after Settings-page capture.\n'
  );
  await delay(1_000);
  await launchActivityForProof(tools, serial, path.join(resultDir, '19-relaunch-activity-after-geofence-route.txt'));
  await wakeAndDismissKeyguardForProof(
    tools,
    serial,
    path.join(resultDir, '19-relaunch-activity-after-geofence-route.txt')
  );
}

async function wakeAndDismissKeyguardForProof(tools, serial, artifactPath) {
  const wake = await adbMaybe(tools, serial, ['shell', 'input', 'keyevent', '224']);
  const dismiss = await adbMaybe(tools, serial, ['shell', 'wm', 'dismiss-keyguard']);
  await appendText(
    artifactPath,
    `${commandResultLog(wake, 'non-claiming screen wake keyevent 224')}\n${commandResultLog(
      dismiss,
      'non-claiming dismiss keyguard'
    )}\n`
  );
}

function commandResultLog(result, label) {
  const output = result.output.trim();
  return `${result.exitCode === 0 ? 'PASS' : 'WARN'} ${label}${output.length > 0 ? `\n${output}` : ''}`;
}

async function launchActivityForProof(tools, serial, artifactPath) {
  const attempts = [];
  for (let attempt = 1; attempt <= 12; attempt += 1) {
    const relaunch = await adbMaybe(tools, serial, ['shell', 'am', 'start', '-n', expectedActivity]);
    attempts.push(
      `${relaunch.exitCode === 0 ? 'PASS' : 'WARN'} attempt ${attempt} adb shell am start -n ${expectedActivity}\n${relaunch.output.trim()}`
    );
    if (relaunch.exitCode === 0) {
      await writeText(artifactPath, attempts.join('\n\n'));
      await delay(2_000);
      return;
    }
    const resolved = await adbMaybe(tools, serial, [
      'shell',
      'cmd',
      'package',
      'resolve-activity',
      '--brief',
      packageName,
    ]);
    attempts.push(
      `${resolved.exitCode === 0 ? 'INFO' : 'WARN'} attempt ${attempt} resolve-activity ${packageName}\n${resolved.output.trim()}`
    );
    await delay(5_000);
  }
  await writeText(artifactPath, attempts.join('\n\n'));
  throw new Error(`Unable to launch ${expectedActivity} after retrying; see ${relativePath(artifactPath)}`);
}

function resolveAndroidTools() {
  const sdkRoot = process.env.ANDROID_SDK_ROOT ?? process.env.ANDROID_HOME;
  if (sdkRoot === undefined || sdkRoot.length === 0) {
    throw new Error('ANDROID_SDK_ROOT or ANDROID_HOME is required for Android emulator proof.');
  }
  const adbPath = path.join(sdkRoot, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  const emulatorPath = path.join(sdkRoot, 'emulator', process.platform === 'win32' ? 'emulator.exe' : 'emulator');
  assertFileExists(adbPath, 'adb');
  assertFileExists(emulatorPath, 'Android emulator');
  return { adbPath, emulatorPath, sdkRoot };
}

async function mkdirProofRoots() {
  await mkdir(output08, { recursive: true });
  await mkdir(output09, { recursive: true });
  await mkdir(output10, { recursive: true });
  await mkdir(resultDir, { recursive: true });
}

async function ensureDevice(tools) {
  const explicitSerial = process.env.OCENTRA_PARENT_ANDROID_SERIAL;
  if (explicitSerial !== undefined && explicitSerial.length > 0) {
    await waitForBoot(tools, explicitSerial);
    return explicitSerial;
  }

  const existing = await findReadyEmulatorDevice(tools);
  if (existing !== undefined) {
    await waitForBoot(tools, existing);
    return existing;
  }

  const avd = process.env.OCENTRA_PARENT_ANDROID_AVD ?? firstAvd(tools);
  const emulator = spawn(tools.emulatorPath, emulatorArgs(avd), {
    cwd: repoRoot,
    detached: process.platform !== 'win32',
    stdio: ['ignore', 'ignore', 'ignore'],
    windowsHide: true,
  });
  startedEmulator = true;
  commands.push({ command: `${tools.emulatorPath} ${emulatorArgs(avd).join(' ')}`, exitCode: 0, artifact: null });
  emulator.unref();

  const serial = await waitForNewEmulatorDevice(tools);
  await waitForBoot(tools, serial);
  return serial;
}

function firstAvd(tools) {
  const result = spawnSync(tools.emulatorPath, ['-list-avds'], { cwd: repoRoot, encoding: 'utf8' });
  if ((result.status ?? 1) !== 0) {
    throw new Error(`emulator -list-avds failed: ${result.stderr}`);
  }
  const avd = result.stdout.split(/\r?\n/u).find((line) => line.trim().length > 0);
  if (avd === undefined) {
    throw new Error('No Android AVD is available for emulator proof.');
  }
  return avd.trim();
}

function emulatorArgs(avd) {
  return ['-avd', avd, '-no-window', '-no-snapshot-save', '-no-audio', '-no-boot-anim', '-gpu', 'swiftshader_indirect'];
}

async function findReadyEmulatorDevice(tools) {
  const devices = await adbDevices(tools);
  return devices.find((device) => device.state === 'device' && device.serial.startsWith('emulator-'))?.serial ?? null;
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
  throw new Error('Timed out waiting for Android emulator device.');
}

async function waitForBoot(tools, serial) {
  const deadline = Date.now() + 8 * 60_000;
  while (Date.now() < deadline) {
    const boot = (await adbText(tools, serial, ['shell', 'getprop', 'sys.boot_completed'])).trim();
    if (boot === '1') {
      return;
    }
    await delay(2_000);
  }
  throw new Error(`Timed out waiting for Android boot completion on ${serial}.`);
}

async function adbDevices(tools) {
  const output = await runCapture(tools.adbPath, ['devices']);
  return output
    .split(/\r?\n/u)
    .map((line) => line.match(/^(?<serial>\S+)\s+(?<state>\S+)$/u)?.groups)
    .filter((entry) => entry !== undefined)
    .map((entry) => ({ serial: entry.serial, state: entry.state }));
}

async function readDeviceMetadata(tools, serial) {
  const props = {
    serial,
    androidRelease: await getProp(tools, serial, 'ro.build.version.release'),
    androidSdk: await getProp(tools, serial, 'ro.build.version.sdk'),
    buildFingerprint: await getProp(tools, serial, 'ro.build.fingerprint'),
    productModel: await getProp(tools, serial, 'ro.product.model'),
    productManufacturer: await getProp(tools, serial, 'ro.product.manufacturer'),
    abi: await getProp(tools, serial, 'ro.product.cpu.abi'),
  };
  await writeJson(path.join(output08, '01-device-metadata.json'), props);
  return props;
}

async function getProp(tools, serial, name) {
  return (await adbText(tools, serial, ['shell', 'getprop', name])).trim();
}

async function collectRuntimeArtifacts(tools, serial) {
  const serviceDump = await adbText(tools, serial, ['shell', 'dumpsys', 'activity', 'services', packageName]);
  const activityDump = await adbText(tools, serial, ['shell', 'dumpsys', 'activity', 'activities']);
  const windowDump = await adbText(tools, serial, ['shell', 'dumpsys', 'window']);
  const packageDump = await adbText(tools, serial, ['shell', 'dumpsys', 'package', packageName]);
  const batteryDump = await adbText(tools, serial, ['shell', 'dumpsys', 'battery']);
  const connectivityDump = await adbText(tools, serial, ['shell', 'dumpsys', 'connectivity']);
  const uiDump = await adbText(tools, serial, ['exec-out', 'uiautomator', 'dump', '/dev/tty']);
  const geofencePrefs = await readGeofenceTransitionPrefs(tools, serial);
  const backgroundSamplePrefs = await readBackgroundLocationSamplePrefs(tools, serial);
  const activeGeofenceLimit = activeGeofenceLimitProof(geofencePrefs.parsed);
  const pidResult = await adbMaybe(tools, serial, ['shell', 'pidof', '-s', packageName]);
  const pid = pidResult.exitCode === 0 ? pidResult.output.trim() : '';
  const logcat =
    pid.length > 0
      ? await adbText(tools, serial, ['logcat', '--pid', pid, '-d'])
      : boundedFallbackLogcat(await adbText(tools, serial, ['logcat', '-d']));
  const screenshot = await adbBuffer(tools, serial, ['exec-out', 'screencap', '-p']);
  const screenshotInspection = inspectPngVisual(screenshot);

  await writeText(path.join(resultDir, '03-package-dump.txt'), packageDump);
  await writeText(path.join(resultDir, '04-service-dump.txt'), serviceDump);
  await writeText(path.join(resultDir, '05-activity-dump.txt'), activityDump);
  await writeText(path.join(resultDir, '06-window-dump.txt'), windowDump);
  await writeText(path.join(resultDir, '07-battery.txt'), batteryDump);
  await writeText(path.join(resultDir, '08-connectivity.txt'), connectivityDump);
  await writeText(path.join(resultDir, '09-ui.xml'), uiDump);
  await writeFile(path.join(resultDir, '10-screen.png'), screenshot);
  await writeText(path.join(resultDir, '11-logcat.txt'), logcat);
  await writeJson(path.join(resultDir, '12-screenshot-inspection.json'), screenshotInspection);
  await writeText(path.join(resultDir, '17-geofence-transition-prefs.xml'), geofencePrefs.raw);
  await writeJson(path.join(resultDir, '25-active-geofence-limit-proof.json'), activeGeofenceLimit);
  await writeText(path.join(resultDir, '20-background-location-sample-prefs.xml'), backgroundSamplePrefs.raw);

  return {
    pid,
    packageDump,
    service: parseServiceState(serviceDump),
    activity: parseActivityState(activityDump, windowDump),
    battery: parseKeyValueDump(batteryDump),
    connectivitySummary: summarizeConnectivity(connectivityDump),
    ui: parseUiState(uiDump),
    geofenceTransitions: geofencePrefs.parsed,
    activeGeofenceLimit,
    backgroundLocationSample: backgroundSamplePrefs.parsed,
    screenshotInspection,
    logcatFindings: parseLogcat(logcat),
    artifacts: runtimeArtifactPaths(),
  };
}

async function readGeofenceTransitionPrefs(tools, serial) {
  const result = await adbMaybe(tools, serial, [
    'shell',
    'run-as',
    packageName,
    'cat',
    'shared_prefs/tracking_geofence_transition_proof.xml',
  ]);
  const raw = result.exitCode === 0 ? result.output : `UNAVAILABLE geofence transition prefs\n${result.output}`;
  return { raw, parsed: parseGeofenceTransitionPrefs(raw) };
}

async function readBackgroundLocationSamplePrefs(tools, serial) {
  const result = await adbMaybe(tools, serial, [
    'shell',
    'run-as',
    packageName,
    'cat',
    'shared_prefs/tracking_background_location_sample_proof.xml',
  ]);
  const raw = result.exitCode === 0 ? result.output : `UNAVAILABLE background location sample prefs\n${result.output}`;
  return { raw, parsed: parseBackgroundLocationSamplePrefs(raw) };
}

function parsePermissionState(packageDump) {
  const requested = [...packageDump.matchAll(/^\s+(android\.permission\.[A-Z_]+)\s*$/gmu)].map((match) => match[1]);
  const grants = {};
  for (const match of packageDump.matchAll(/^\s+(android\.permission\.[A-Z_]+): granted=(true|false)/gmu)) {
    grants[match[1]] = match[2] === 'true';
  }
  return {
    requested,
    grants,
    foregroundServiceGranted: grants['android.permission.FOREGROUND_SERVICE'] === true,
    notificationGranted: grants['android.permission.POST_NOTIFICATIONS'] === true,
    locationPermissionRequested: requested.some(
      (permission) =>
        permission === 'android.permission.ACCESS_FINE_LOCATION' ||
        permission === 'android.permission.ACCESS_COARSE_LOCATION'
    ),
    foregroundLocationPermissionGranted:
      grants['android.permission.ACCESS_FINE_LOCATION'] === true ||
      grants['android.permission.ACCESS_COARSE_LOCATION'] === true,
    backgroundLocationPermissionRequested: requested.includes('android.permission.ACCESS_BACKGROUND_LOCATION'),
    backgroundLocationPermissionGranted: grants['android.permission.ACCESS_BACKGROUND_LOCATION'] === true,
  };
}

function parseServiceState(serviceDump) {
  return {
    serviceRecordPresent: serviceDump.includes(serviceName),
    isForeground: serviceDump.includes('isForeground=true'),
    foregroundNotification: /foregroundNoti=([^\n]+)/u.exec(serviceDump)?.[1]?.trim() ?? null,
    startForegroundCount: /startForegroundCount=(\d+)/u.exec(serviceDump)?.[1] ?? null,
  };
}

function parseActivityState(activityDump, windowDump) {
  const activityFocused =
    windowDump.includes(`${packageName}/.MainActivity`) ||
    windowDump.includes(`${packageName}/${packageName}.MainActivity`) ||
    windowDump.includes(`${packageName}/MainActivity`);
  return {
    packageFocused: activityFocused,
    currentFocus: /mCurrentFocus=([^\n]+)/u.exec(windowDump)?.[1]?.trim() ?? null,
    resumedActivity: /ResumedActivity:\s*([^\n]+)/u.exec(activityDump)?.[1]?.trim() ?? null,
  };
}

function parseUiState(uiDump) {
  const text = [...uiDump.matchAll(/text="([^"]*)"/gu)].map((match) => decodeXmlText(match[1])).join('\n');
  return {
    hasLaunchText: text.includes(appLaunchText),
    foregroundLocationPermissionStateText: text.includes('foreground-location-permission-granted')
      ? 'foreground-location-permission-granted'
      : text.includes('foreground-location-permission-required')
        ? 'foreground-location-permission-required'
        : null,
    foregroundLocationSampleStateText: text.includes('current-location-sample-observed-emulator-location-manager')
      ? 'current-location-sample-observed-emulator-location-manager'
      : text.includes('last-known-location-sample-observed')
        ? 'last-known-location-sample-observed'
        : text.includes('foreground-location-sample-manual-required')
          ? 'foreground-location-sample-manual-required'
          : null,
    foregroundLocationProvider: parseTextField(text, 'foregroundLocationProvider'),
    foregroundLocationObservedAtEpochMillis: parseNumberTextField(text, 'foregroundLocationObservedAtEpochMillis'),
    foregroundLocationAccuracyMeters: parseNumberTextField(text, 'foregroundLocationAccuracyMeters'),
    foregroundLocationSampleSource: parseTextField(text, 'foregroundLocationSampleSource'),
    foregroundLocationLatitude: parseNumberTextField(text, 'foregroundLocationLatitude'),
    foregroundLocationLongitude: parseNumberTextField(text, 'foregroundLocationLongitude'),
    fusedForegroundLocationSampleStateText: text.includes('current-fused-foreground-location-sample-observed-emulator')
      ? 'current-fused-foreground-location-sample-observed-emulator'
      : text.includes('last-known-fused-foreground-location-sample-observed')
        ? 'last-known-fused-foreground-location-sample-observed'
        : text.includes('fused-foreground-location-sample-manual-required')
          ? 'fused-foreground-location-sample-manual-required'
          : null,
    fusedForegroundLocationProvider: parseTextField(text, 'fusedForegroundLocationProvider'),
    fusedForegroundLocationObservedAtEpochMillis: parseNumberTextField(
      text,
      'fusedForegroundLocationObservedAtEpochMillis'
    ),
    fusedForegroundLocationAccuracyMeters: parseNumberTextField(text, 'fusedForegroundLocationAccuracyMeters'),
    fusedForegroundLocationSampleSource: parseTextField(text, 'fusedForegroundLocationSampleSource'),
    fusedForegroundLocationLatitude: parseNumberTextField(text, 'fusedForegroundLocationLatitude'),
    fusedForegroundLocationLongitude: parseNumberTextField(text, 'fusedForegroundLocationLongitude'),
    backgroundLocationPermissionStateText: text.includes('background-location-permission-granted')
      ? 'background-location-permission-granted'
      : text.includes('background-location-permission-required')
        ? 'background-location-permission-required'
        : null,
    backgroundGeofenceStateText: text.includes('background-geofence-transition-manual-required')
      ? 'background-geofence-transition-manual-required'
      : text.includes('background-geofence-transition-observed-emulator')
        ? 'background-geofence-transition-observed-emulator'
        : null,
    backgroundGeofenceTransitionCount: parseNumberTextField(text, 'backgroundGeofenceTransitionCount'),
    backgroundGeofenceEnterCount: parseNumberTextField(text, 'backgroundGeofenceEnterCount'),
    backgroundGeofenceExitCount: parseNumberTextField(text, 'backgroundGeofenceExitCount'),
    backgroundGeofenceDwellCount: parseNumberTextField(text, 'backgroundGeofenceDwellCount'),
    backgroundGeofenceDwellSource: parseTextField(text, 'backgroundGeofenceDwellSource'),
    backgroundGeofenceLastTransition: parseTextField(text, 'backgroundGeofenceLastTransition'),
    backgroundGeofenceSource: parseTextField(text, 'backgroundGeofenceSource'),
    backgroundLocationSampleStateText: text.includes('background-location-sample-observed-emulator-foreground-service')
      ? 'background-location-sample-observed-emulator-foreground-service'
      : text.includes('background-location-sample-manual-required')
        ? 'background-location-sample-manual-required'
        : null,
    backgroundLocationSampleCount: parseNumberTextField(text, 'backgroundLocationSampleCount'),
    backgroundLocationSampleProvider: parseTextField(text, 'backgroundLocationSampleProvider'),
    backgroundLocationSampleObservedAtEpochMillis: parseNumberTextField(
      text,
      'backgroundLocationSampleObservedAtEpochMillis'
    ),
    backgroundLocationSampleAccuracyMeters: parseNumberTextField(text, 'backgroundLocationSampleAccuracyMeters'),
    backgroundLocationSampleSource: parseTextField(text, 'backgroundLocationSampleSource'),
    backgroundLocationSampleActivityBackgrounded: parseBooleanTextField(
      text,
      'backgroundLocationSampleActivityBackgrounded'
    ),
    text,
  };
}

function parseForegroundPermissionDialogUi(uiDump) {
  const text = [...uiDump.matchAll(/text="([^"]*)"/gu)].map((match) => decodeXmlText(match[1]));
  const joinedText = text.join('\n');
  const permissionControllerObserved =
    uiDump.includes('com.google.android.permissioncontroller') || uiDump.includes('com.android.permissioncontroller');
  const foregroundLocationCopyObserved =
    /location/iu.test(joinedText) &&
    (/while using/iu.test(joinedText) || /only this time/iu.test(joinedText) || /allow/iu.test(joinedText));
  return {
    observed: permissionControllerObserved && foregroundLocationCopyObserved,
    permissionControllerObserved,
    foregroundLocationCopyObserved,
    allowWhileUsingOptionObserved: /while using/iu.test(joinedText),
    onlyThisTimeOptionObserved: /only this time/iu.test(joinedText),
    denyOptionObserved: /don.t allow|deny/iu.test(joinedText),
    text,
  };
}

function parseBackgroundLocationSettingsPageUi({ uiDump, activityDump, windowDump, launch }) {
  const text = [...uiDump.matchAll(/text="([^"]*)"/gu)].map((match) => decodeXmlText(match[1]));
  const joinedText = text.join('\n');
  const combinedSystemDumps = `${activityDump}\n${windowDump}\n${launch}`;
  const settingsPackageObserved =
    combinedSystemDumps.includes('com.android.settings') ||
    combinedSystemDumps.includes('com.google.android.settings') ||
    uiDump.includes('com.android.settings') ||
    uiDump.includes('com.google.android.settings');
  const settingsActivityObserved =
    /APPLICATION_DETAILS_SETTINGS|InstalledAppDetails|AppInfoDashboardFragment|AppDashboardFragment/iu.test(
      combinedSystemDumps
    );
  const packageRouteObserved = combinedSystemDumps.includes(packageName) || launch.includes('dat=package:');
  const appSettingsCopyObserved =
    /Ocentra Parent Agent|ca\.ocentra\.parent\.agent/iu.test(joinedText) &&
    /Permissions|App info|Location|Allow all the time|Allowed all the time|Not allowed/iu.test(joinedText);
  return {
    observed: settingsPackageObserved && (settingsActivityObserved || packageRouteObserved || appSettingsCopyObserved),
    settingsPackageObserved,
    settingsActivityObserved,
    packageRouteObserved,
    appSettingsCopyObserved,
    appLabelObserved: /Ocentra Parent Agent/iu.test(joinedText),
    packageNameObserved: /ca\.ocentra\.parent\.agent/iu.test(joinedText),
    permissionCopyObserved: /Permissions|Location|Allow all the time|Allowed all the time|Not allowed/iu.test(
      joinedText
    ),
    text,
  };
}

function parseBackgroundLocationSamplePrefs(raw) {
  return {
    sampleCount: parseXmlInt(raw, 'backgroundLocationSampleCount'),
    provider: parseXmlString(raw, 'backgroundLocationSampleProvider'),
    observedAtEpochMillis: parseXmlLong(raw, 'backgroundLocationSampleObservedAtEpochMillis'),
    accuracyMeters: parseXmlFloat(raw, 'backgroundLocationSampleAccuracyMeters'),
    source: parseXmlString(raw, 'backgroundLocationSampleSource'),
    activityBackgrounded: parseXmlBoolean(raw, 'backgroundLocationSampleActivityBackgrounded'),
  };
}

function parseGeofenceTransitionPrefs(raw) {
  return {
    registered: parseXmlBoolean(raw, 'registered'),
    source: parseXmlString(raw, 'source'),
    systemProximityRegistered: parseXmlBoolean(raw, 'systemProximityRegistered'),
    systemProximityRegistrationEpochMillis: parseXmlLong(raw, 'systemProximityRegistrationEpochMillis'),
    systemProximityRegistrationSource: parseXmlString(raw, 'systemProximityRegistrationSource'),
    systemProximityTransitionCount: parseXmlInt(raw, 'systemProximityTransitionCount'),
    systemProximityEnterCount: parseXmlInt(raw, 'systemProximityEnterCount'),
    systemProximityExitCount: parseXmlInt(raw, 'systemProximityExitCount'),
    systemProximityLastTransition: parseXmlString(raw, 'systemProximityLastTransition'),
    systemProximityLastTransitionEpochMillis: parseXmlLong(raw, 'systemProximityLastTransitionEpochMillis'),
    hasInsideState: parseXmlBoolean(raw, 'hasInsideState'),
    insideState: parseXmlBoolean(raw, 'insideState'),
    dwellCount: parseXmlInt(raw, 'dwellCount'),
    dwellLastObservedEpochMillis: parseXmlLong(raw, 'dwellLastObservedEpochMillis'),
    dwellInsideStartedEpochMillis: parseXmlLong(raw, 'dwellInsideStartedEpochMillis'),
    dwellSource: parseXmlString(raw, 'dwellSource'),
    transitionCount: parseXmlInt(raw, 'transitionCount'),
    enterCount: parseXmlInt(raw, 'enterCount'),
    exitCount: parseXmlInt(raw, 'exitCount'),
    lastTransition: parseXmlString(raw, 'lastTransition'),
    lastTransitionEpochMillis: parseXmlLong(raw, 'lastTransitionEpochMillis'),
    registrationEpochMillis: parseXmlLong(raw, 'registrationEpochMillis'),
  };
}

function activeGeofenceLimitProof(geofenceTransitions) {
  const activeGeofenceCount = geofenceTransitions.registered ? 1 : 0;
  return {
    observed: geofenceTransitions.registered,
    source: geofenceTransitions.source,
    activeGeofenceCount,
    documentedLimitPerAppPerDeviceUser: androidDocumentedGeofenceLimitPerAppPerDeviceUser,
    withinDocumentedLimit: activeGeofenceCount <= androidDocumentedGeofenceLimitPerAppPerDeviceUser,
    documentedLimitSourceUrl: androidGeofenceLimitSourceUrl,
    proofBoundary:
      activeGeofenceCount > 0
        ? 'app-owned-local-geofence-count-compared-to-android-documented-limit-only'
        : 'no-active-app-owned-local-geofence-observed',
    nonClaims: [
      'android-geofencing-api-limit-registration',
      'android-system-geofencing-delivery',
      'dwell-transition-delivery',
      'physical-device-behavior',
      'authority-enrolled-device-behavior',
    ],
  };
}

function systemProximityRegistrationProof(geofenceTransitions) {
  const transitionObserved = geofenceTransitions.systemProximityTransitionCount > 0;
  return {
    observed: geofenceTransitions.systemProximityRegistered,
    source: geofenceTransitions.systemProximityRegistrationSource,
    registeredAtEpochMillis: geofenceTransitions.systemProximityRegistrationEpochMillis,
    transitionObserved,
    transitionCount: geofenceTransitions.systemProximityTransitionCount,
    enterCount: geofenceTransitions.systemProximityEnterCount,
    exitCount: geofenceTransitions.systemProximityExitCount,
    lastTransition: geofenceTransitions.systemProximityLastTransition,
    lastTransitionEpochMillis: geofenceTransitions.systemProximityLastTransitionEpochMillis,
    proofBoundary: geofenceTransitions.systemProximityRegistered
      ? transitionObserved
        ? 'android-location-manager-add-proximity-alert-registration-and-broadcast-transition-observed'
        : 'android-location-manager-add-proximity-alert-registration-only-no-broadcast-transition-observed'
      : 'no-android-system-proximity-registration-observed',
    nonClaims: [
      ...(transitionObserved ? [] : ['android-system-geofencing-delivery']),
      'android-system-geofencing-dwell-transition',
      'physical-device-behavior',
      'authority-enrolled-device-behavior',
      'provider-delivery',
      'production-upload-worker',
      'product-ready-android-tracking',
    ],
  };
}

function parseXmlString(raw, name) {
  return new RegExp(`<string name="${name}">([^<]*)</string>`, 'u').exec(raw)?.[1] ?? null;
}

function parseXmlInt(raw, name) {
  const value = new RegExp(`<int name="${name}" value="(-?\\d+)"`, 'u').exec(raw)?.[1] ?? null;
  return value === null ? 0 : Number(value);
}

function parseXmlLong(raw, name) {
  const value = new RegExp(`<long name="${name}" value="(-?\\d+)"`, 'u').exec(raw)?.[1] ?? null;
  return value === null ? null : Number(value);
}

function parseXmlFloat(raw, name) {
  const value = new RegExp(`<float name="${name}" value="(-?[\\d.]+)"`, 'u').exec(raw)?.[1] ?? null;
  return value === null ? null : Number(value);
}

function parseXmlBoolean(raw, name) {
  const value = new RegExp(`<boolean name="${name}" value="(true|false)"`, 'u').exec(raw)?.[1] ?? null;
  return value === 'true';
}

function parseTextField(text, fieldName) {
  return new RegExp(`(?:^|\\n)${fieldName}:([^\\n]+)`, 'u').exec(text)?.[1] ?? null;
}

function parseNumberTextField(text, fieldName) {
  const value = parseTextField(text, fieldName);
  if (value === null) {
    return null;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function parseBooleanTextField(text, fieldName) {
  const value = parseTextField(text, fieldName);
  return value === null ? null : value === 'true';
}

function inspectPngVisual(buffer) {
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  if (!buffer.subarray(0, signature.length).equals(signature)) {
    return screenshotInspectionResult({
      reason: 'screencap output is not a PNG',
      visualClaimReady: false,
    });
  }

  const parsed = parsePngChunks(buffer);
  if (parsed.ihdr === null || parsed.idat.length === 0) {
    return screenshotInspectionResult({
      reason: 'PNG is missing IHDR or IDAT chunks',
      visualClaimReady: false,
    });
  }

  const { width, height, bitDepth, colorType, compression, filter, interlace } = parsed.ihdr;
  if (bitDepth !== 8 || compression !== 0 || filter !== 0 || interlace !== 0) {
    return screenshotInspectionResult({
      width,
      height,
      bitDepth,
      colorType,
      reason: 'PNG format is unsupported for contrast inspection',
      visualClaimReady: false,
    });
  }

  const bytesPerPixel = pngBytesPerPixel(colorType);
  if (bytesPerPixel === null) {
    return screenshotInspectionResult({
      width,
      height,
      bitDepth,
      colorType,
      reason: 'PNG color type is unsupported for contrast inspection',
      visualClaimReady: false,
    });
  }

  const rowBytes = width * bytesPerPixel;
  const inflated = inflateSync(Buffer.concat(parsed.idat));
  const expectedBytes = height * (rowBytes + 1);
  if (inflated.length < expectedBytes) {
    return screenshotInspectionResult({
      width,
      height,
      bitDepth,
      colorType,
      reason: 'PNG image data is shorter than expected',
      visualClaimReady: false,
    });
  }

  let sourceOffset = 0;
  let previousRow = Buffer.alloc(rowBytes);
  let minLuma = 255;
  let maxLuma = 0;
  let nonBlackPixelCount = 0;
  let transparentPixelCount = 0;
  const distinctColors = new Set();

  for (let rowIndex = 0; rowIndex < height; rowIndex += 1) {
    const filterType = inflated[sourceOffset];
    sourceOffset += 1;
    const encodedRow = inflated.subarray(sourceOffset, sourceOffset + rowBytes);
    sourceOffset += rowBytes;
    const row = unfilterPngRow(encodedRow, previousRow, bytesPerPixel, filterType);
    for (let pixelOffset = 0; pixelOffset < rowBytes; pixelOffset += bytesPerPixel) {
      const pixel = pngPixelRgb(row, pixelOffset, colorType);
      if (pixel.alpha === 0) {
        transparentPixelCount += 1;
        continue;
      }
      if (pixel.red !== 0 || pixel.green !== 0 || pixel.blue !== 0) {
        nonBlackPixelCount += 1;
      }
      const luma = Math.round(0.2126 * pixel.red + 0.7152 * pixel.green + 0.0722 * pixel.blue);
      minLuma = Math.min(minLuma, luma);
      maxLuma = Math.max(maxLuma, luma);
      if (distinctColors.size < 64) {
        distinctColors.add(`${pixel.red},${pixel.green},${pixel.blue},${pixel.alpha}`);
      }
    }
    previousRow = row;
  }

  const pixelCount = width * height;
  const visiblePixelCount = pixelCount - transparentPixelCount;
  const lumaRange = visiblePixelCount > 0 ? maxLuma - minLuma : 0;
  const isAllBlack = visiblePixelCount === 0 || nonBlackPixelCount === 0;
  const visualClaimReady = !isAllBlack && lumaRange >= 16 && distinctColors.size > 1;
  return screenshotInspectionResult({
    width,
    height,
    bitDepth,
    colorType,
    pixelCount,
    visiblePixelCount,
    nonBlackPixelCount,
    transparentPixelCount,
    distinctColorSampleCount: distinctColors.size,
    minLuma: visiblePixelCount > 0 ? minLuma : null,
    maxLuma: visiblePixelCount > 0 ? maxLuma : null,
    lumaRange,
    isAllBlack,
    visualClaimReady,
    reason: visualClaimReady
      ? 'PNG contains non-black pixels and visible luminance contrast'
      : visiblePixelCount === 0
        ? 'PNG has no visible pixels, so the headless screenshot cannot be claimed as visual evidence'
        : 'PNG did not contain enough visible contrast to claim screenshot evidence',
  });
}

function parsePngChunks(buffer) {
  let offset = 8;
  const idat = [];
  let ihdr = null;
  while (offset + 12 <= buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.subarray(offset + 4, offset + 8).toString('ascii');
    const dataStart = offset + 8;
    const dataEnd = dataStart + length;
    if (dataEnd + 4 > buffer.length) {
      break;
    }
    const data = buffer.subarray(dataStart, dataEnd);
    if (type === 'IHDR') {
      ihdr = {
        width: data.readUInt32BE(0),
        height: data.readUInt32BE(4),
        bitDepth: data[8],
        colorType: data[9],
        compression: data[10],
        filter: data[11],
        interlace: data[12],
      };
    } else if (type === 'IDAT') {
      idat.push(Buffer.from(data));
    } else if (type === 'IEND') {
      break;
    }
    offset = dataEnd + 4;
  }
  return { ihdr, idat };
}

function pngBytesPerPixel(colorType) {
  if (colorType === 0) {
    return 1;
  }
  if (colorType === 2) {
    return 3;
  }
  if (colorType === 4) {
    return 2;
  }
  if (colorType === 6) {
    return 4;
  }
  return null;
}

function unfilterPngRow(encodedRow, previousRow, bytesPerPixel, filterType) {
  const row = Buffer.alloc(encodedRow.length);
  for (let index = 0; index < encodedRow.length; index += 1) {
    const left = index >= bytesPerPixel ? row[index - bytesPerPixel] : 0;
    const up = previousRow[index] ?? 0;
    const upLeft = index >= bytesPerPixel ? previousRow[index - bytesPerPixel] : 0;
    row[index] = (encodedRow[index] + pngFilterAdjustment(filterType, left, up, upLeft)) & 0xff;
  }
  return row;
}

function pngFilterAdjustment(filterType, left, up, upLeft) {
  if (filterType === 0) {
    return 0;
  }
  if (filterType === 1) {
    return left;
  }
  if (filterType === 2) {
    return up;
  }
  if (filterType === 3) {
    return Math.floor((left + up) / 2);
  }
  if (filterType === 4) {
    return pngPaethPredictor(left, up, upLeft);
  }
  throw new Error(`Unsupported PNG filter type: ${filterType}`);
}

function pngPaethPredictor(left, up, upLeft) {
  const predicted = left + up - upLeft;
  const leftDistance = Math.abs(predicted - left);
  const upDistance = Math.abs(predicted - up);
  const upLeftDistance = Math.abs(predicted - upLeft);
  if (leftDistance <= upDistance && leftDistance <= upLeftDistance) {
    return left;
  }
  if (upDistance <= upLeftDistance) {
    return up;
  }
  return upLeft;
}

function pngPixelRgb(row, pixelOffset, colorType) {
  if (colorType === 0) {
    const gray = row[pixelOffset];
    return { red: gray, green: gray, blue: gray, alpha: 255 };
  }
  if (colorType === 2) {
    return { red: row[pixelOffset], green: row[pixelOffset + 1], blue: row[pixelOffset + 2], alpha: 255 };
  }
  if (colorType === 4) {
    const gray = row[pixelOffset];
    return { red: gray, green: gray, blue: gray, alpha: row[pixelOffset + 1] };
  }
  return {
    red: row[pixelOffset],
    green: row[pixelOffset + 1],
    blue: row[pixelOffset + 2],
    alpha: row[pixelOffset + 3],
  };
}

function screenshotInspectionResult(result) {
  return {
    schemaVersion: 1,
    ...result,
  };
}

function parseLogcat(logcat) {
  return {
    fatalExceptionCount: countMatches(logcat, /FATAL EXCEPTION/gu),
    androidRuntimeErrorCount: countMatches(logcat, /AndroidRuntime.*FATAL/gu),
    packageLogLines: countMatches(logcat, new RegExp(packageName.replaceAll('.', '\\.'), 'gu')),
    foregroundServiceAllowed: logcat.includes('Background started FGS: Allowed'),
  };
}

function boundedFallbackLogcat(logcat) {
  const relevantLines = logcat
    .split(/\r?\n/u)
    .filter(
      (line) => line.includes(packageName) || /AndroidRuntime|FATAL EXCEPTION|Background started FGS/u.test(line)
    );
  if (relevantLines.length > 0) {
    return [
      'PID-specific logcat unavailable; captured relevant fallback logcat lines only.',
      ...relevantLines.slice(-500),
    ].join('\n');
  }
  return [
    'PID-specific logcat unavailable; captured tail of fallback logcat.',
    ...logcat.split(/\r?\n/u).slice(-500),
  ].join('\n');
}

function parseKeyValueDump(dump) {
  const values = {};
  for (const line of dump.split(/\r?\n/u)) {
    const match = /^\s*(?<key>[A-Za-z0-9 _-]+):\s*(?<value>.*)$/u.exec(line);
    if (match?.groups !== undefined) {
      values[match.groups.key.trim().replaceAll(' ', '_')] = match.groups.value.trim();
    }
  }
  return values;
}

function summarizeConnectivity(connectivityDump) {
  return connectivityDump
    .split(/\r?\n/u)
    .filter((line) => /Active|NetworkAgentInfo|NetworkRequest|not connected|CONNECTED|DISCONNECTED/u.test(line))
    .slice(0, 25);
}

function runtimeArtifactPaths() {
  return {
    packageDump: relativePath(path.join(resultDir, '03-package-dump.txt')),
    serviceDump: relativePath(path.join(resultDir, '04-service-dump.txt')),
    activityDump: relativePath(path.join(resultDir, '05-activity-dump.txt')),
    windowDump: relativePath(path.join(resultDir, '06-window-dump.txt')),
    battery: relativePath(path.join(resultDir, '07-battery.txt')),
    connectivity: relativePath(path.join(resultDir, '08-connectivity.txt')),
    ui: relativePath(path.join(resultDir, '09-ui.xml')),
    screenshot: relativePath(path.join(resultDir, '10-screen.png')),
    logcat: relativePath(path.join(resultDir, '11-logcat.txt')),
    screenshotInspection: relativePath(path.join(resultDir, '12-screenshot-inspection.json')),
    foregroundLocationPermissionUxReset: relativePath(
      path.join(resultDir, '13-foreground-location-permission-ux-reset.txt')
    ),
    foregroundLocationPermissionUxLaunch: relativePath(
      path.join(resultDir, '13-foreground-location-permission-ux-launch.txt')
    ),
    foregroundLocationPermissionUx: relativePath(path.join(resultDir, '13-foreground-location-permission-ux.xml')),
    foregroundLocationPermissionUxProof: relativePath(
      path.join(resultDir, '13-foreground-location-permission-ux.json')
    ),
    foregroundLocationPermissionGrant: relativePath(
      path.join(resultDir, '13-foreground-location-permission-grant.txt')
    ),
    foregroundLocationSeed: relativePath(path.join(resultDir, '14-foreground-location-seed.txt')),
    backgroundLocationPermissionGrant: relativePath(
      path.join(resultDir, '15-background-location-permission-grant.txt')
    ),
    geofenceTransitionRoute: relativePath(path.join(resultDir, '16-geofence-transition-route.txt')),
    geofenceTransitionPrefs: relativePath(path.join(resultDir, '17-geofence-transition-prefs.xml')),
    activeGeofenceLimitProof: relativePath(path.join(resultDir, '25-active-geofence-limit-proof.json')),
    backgroundLocationSamplePrefs: relativePath(path.join(resultDir, '20-background-location-sample-prefs.xml')),
    backgroundActivityForSample: relativePath(path.join(resultDir, '21-background-activity-for-sample.txt')),
    backgroundLocationSampleRoute: relativePath(path.join(resultDir, '22-background-location-sample-route.txt')),
    relaunchAfterBackgroundSample: relativePath(path.join(resultDir, '23-relaunch-after-background-sample.txt')),
    backgroundLocationSettingsPageLaunch: relativePath(
      path.join(resultDir, '24-background-location-settings-page-launch.txt')
    ),
    backgroundLocationSettingsPage: relativePath(path.join(resultDir, '24-background-location-settings-page.xml')),
    backgroundLocationSettingsPageActivity: relativePath(
      path.join(resultDir, '24-background-location-settings-page-activity.txt')
    ),
    backgroundLocationSettingsPageWindow: relativePath(
      path.join(resultDir, '24-background-location-settings-page-window.txt')
    ),
    backgroundLocationSettingsPageProof: relativePath(
      path.join(resultDir, '24-background-location-settings-page.json')
    ),
  };
}

function buildProof({
  device,
  packageDump,
  permissionState,
  resolvedActivity,
  runtime,
  tools,
  foregroundPermissionUx,
  backgroundSettingsPage,
}) {
  const checkedAt = new Date().toISOString();
  return {
    schemaVersion: 1,
    checkedAt,
    commit: gitHead(),
    proofMode,
    requiredProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentStatus: runtime.screenshotInspection.visualClaimReady
      ? 'emulator_scaffold_observed'
      : 'emulator_scaffold_observed_nonvisual_screenshot',
    productClaimReady: false,
    androidSdkRoot: tools.sdkRoot,
    package: {
      packageName,
      expectedActivity,
      resolvedActivity: resolvedActivity.trim(),
      apk: relativePath(apkPath),
      versionName: /versionName=([^\s]+)/u.exec(packageDump)?.[1] ?? null,
      versionCode: /versionCode=(\d+)/u.exec(packageDump)?.[1] ?? null,
    },
    device,
    permissionState,
    foregroundPermissionUx,
    backgroundSettingsPage,
    runtime,
    workpackProof: workpackProofState(permissionState, runtime, foregroundPermissionUx, backgroundSettingsPage),
    commands,
    nonClaims: [
      ...([
        'current-fused-foreground-location-sample-observed-emulator',
        'last-known-fused-foreground-location-sample-observed',
      ].includes(runtime.ui.fusedForegroundLocationSampleStateText)
        ? []
        : ['This proof does not claim Android fused foreground location sample capture.']),
      'This proof does not claim product-ready Android background location sample behavior.',
      'This proof does not claim Android system geofencing, dwell transitions, or physical-device transition delivery.',
      'This proof does not claim notification delivery or alert provider behavior.',
      'This proof does not claim physical Android device behavior.',
      'This proof does not claim child-device enforcement, Device Owner, managed profile, or authority proof.',
    ],
  };
}

function workpackProofState(permissionState, runtime, foregroundPermissionUx, backgroundSettingsPage) {
  const foregroundSampleObserved =
    runtime.ui.foregroundLocationSampleStateText === 'last-known-location-sample-observed' ||
    runtime.ui.foregroundLocationSampleStateText === 'current-location-sample-observed-emulator-location-manager';
  const foregroundCurrentSampleObserved =
    runtime.ui.foregroundLocationSampleStateText === 'current-location-sample-observed-emulator-location-manager';
  const foregroundSampleMetadataObserved =
    foregroundSampleObserved &&
    runtime.ui.foregroundLocationProvider !== undefined &&
    runtime.ui.foregroundLocationObservedAtEpochMillis !== undefined &&
    runtime.ui.foregroundLocationAccuracyMeters !== undefined;
  const foregroundRawCoordinateObserved =
    foregroundCurrentSampleObserved &&
    runtime.ui.foregroundLocationLatitude !== undefined &&
    runtime.ui.foregroundLocationLongitude !== undefined &&
    runtime.ui.foregroundLocationSampleSource === 'android-location-manager-current-listener-emulator';
  const fusedForegroundCurrentSampleObserved =
    runtime.ui.fusedForegroundLocationSampleStateText ===
      'current-fused-foreground-location-sample-observed-emulator' &&
    runtime.ui.fusedForegroundLocationLatitude !== undefined &&
    runtime.ui.fusedForegroundLocationLongitude !== undefined &&
    runtime.ui.fusedForegroundLocationSampleSource === 'google-play-services-fused-current-emulator';
  const fusedForegroundSampleObserved =
    fusedForegroundCurrentSampleObserved ||
    (runtime.ui.fusedForegroundLocationSampleStateText === 'last-known-fused-foreground-location-sample-observed' &&
      runtime.ui.fusedForegroundLocationLatitude !== undefined &&
      runtime.ui.fusedForegroundLocationLongitude !== undefined &&
      runtime.ui.fusedForegroundLocationSampleSource === 'google-play-services-fused-last-known');
  const geofenceEnterExitObserved =
    runtime.geofenceTransitions.enterCount > 0 &&
    runtime.geofenceTransitions.exitCount > 0 &&
    runtime.geofenceTransitions.dwellCount > 0;
  const activeGeofenceLimitObserved =
    runtime.activeGeofenceLimit.observed && runtime.activeGeofenceLimit.withinDocumentedLimit;
  const systemProximityRegistrationObserved = runtime.geofenceTransitions.systemProximityRegistered;
  const backgroundSampleObserved =
    runtime.backgroundLocationSample.sampleCount > 0 &&
    runtime.backgroundLocationSample.provider !== undefined &&
    runtime.backgroundLocationSample.observedAtEpochMillis !== undefined;
  const backgroundDegradedStatusObserved = backgroundDegradedStatusProof().observed;
  return {
    '08-android-foreground-location-adapter': {
      status:
        foregroundPermissionUx.observed &&
        permissionState.foregroundLocationPermissionGranted &&
        fusedForegroundCurrentSampleObserved
          ? 'foreground_permission_ux_fused_current_sample_observed'
          : permissionState.foregroundLocationPermissionGranted && fusedForegroundCurrentSampleObserved
            ? 'foreground_permission_granted_fused_current_sample_observed'
            : foregroundPermissionUx.observed &&
                permissionState.foregroundLocationPermissionGranted &&
                fusedForegroundSampleObserved
              ? 'foreground_permission_ux_fused_sample_observed'
              : permissionState.foregroundLocationPermissionGranted && fusedForegroundSampleObserved
                ? 'foreground_permission_granted_fused_sample_observed'
                : foregroundPermissionUx.observed &&
                    permissionState.foregroundLocationPermissionGranted &&
                    foregroundRawCoordinateObserved
                  ? 'foreground_permission_ux_current_sample_raw_coordinate_observed'
                  : permissionState.foregroundLocationPermissionGranted && foregroundRawCoordinateObserved
                    ? 'foreground_permission_granted_current_sample_raw_coordinate_observed'
                    : permissionState.foregroundLocationPermissionGranted && foregroundSampleMetadataObserved
                      ? 'foreground_permission_granted_last_known_sample_metadata_observed'
                      : permissionState.foregroundLocationPermissionGranted && foregroundSampleObserved
                        ? 'foreground_permission_granted_last_known_sample_observed'
                        : permissionState.foregroundLocationPermissionGranted
                          ? 'foreground_permission_granted_sample_manual_required'
                          : 'manual_required',
      proofArtifact:
        'output/tracking-plan-proof/08-android-foreground-location-adapter/03-runtime-location-evidence.json',
      reason:
        foregroundPermissionUx.observed &&
        permissionState.foregroundLocationPermissionGranted &&
        fusedForegroundCurrentSampleObserved
          ? 'Foreground location permission UX dialog, permission grant, Google Play Services fused current foreground sample, provider, observed timestamp, accuracy, source, and raw latitude/longitude were observed on the emulator package; background/geofence, physical-device, authority, provider delivery, and product-ready tracking remain unclaimed.'
          : permissionState.foregroundLocationPermissionGranted && fusedForegroundCurrentSampleObserved
            ? 'Foreground location permission grant, Google Play Services fused current foreground sample, provider, observed timestamp, accuracy, source, and raw latitude/longitude were observed on the emulator package; background/geofence, physical-device, authority, provider delivery, and product-ready tracking remain unclaimed.'
            : foregroundPermissionUx.observed &&
                permissionState.foregroundLocationPermissionGranted &&
                fusedForegroundSampleObserved
              ? 'Foreground location permission UX dialog, permission grant, Google Play Services fused foreground sample, provider, observed timestamp, accuracy, source, and raw latitude/longitude were observed on the emulator package; background/geofence, physical-device, authority, provider delivery, and product-ready tracking remain unclaimed.'
              : permissionState.foregroundLocationPermissionGranted && fusedForegroundSampleObserved
                ? 'Foreground location permission grant, Google Play Services fused foreground sample, provider, observed timestamp, accuracy, source, and raw latitude/longitude were observed on the emulator package; background/geofence, physical-device, authority, provider delivery, and product-ready tracking remain unclaimed.'
                : foregroundPermissionUx.observed &&
                    permissionState.foregroundLocationPermissionGranted &&
                    foregroundRawCoordinateObserved
                  ? 'Foreground location permission UX dialog, permission grant, app-emitted emulator current LocationManager sample, provider, observed timestamp, accuracy, source, and raw latitude/longitude were observed on the emulator package; fused provider, background/geofence, physical-device, authority, provider delivery, and product-ready tracking remain unclaimed.'
                  : permissionState.foregroundLocationPermissionGranted && foregroundRawCoordinateObserved
                    ? 'Foreground location permission grant, app-emitted emulator current LocationManager sample, provider, observed timestamp, accuracy, source, and raw latitude/longitude were observed on the emulator package; fused provider, background/geofence, physical-device, authority, provider delivery, and product-ready tracking remain unclaimed.'
                    : permissionState.foregroundLocationPermissionGranted && foregroundSampleMetadataObserved
                      ? 'Foreground location permission grant, app-emitted last-known sample state, provider, observed timestamp, and accuracy were observed on the emulator package; raw coordinates, fused/current sample, background/geofence, physical-device, and product-ready tracking remain unclaimed.'
                      : permissionState.foregroundLocationPermissionGranted && foregroundSampleObserved
                        ? 'Foreground location permission grant and app-emitted last-known sample state were observed on the emulator package; fused/current sample, background/geofence, physical-device, and product-ready tracking remain unclaimed.'
                        : permissionState.foregroundLocationPermissionGranted
                          ? 'Foreground location permission grant was observed on the emulator package, but no app-emitted foreground location sample was captured.'
                          : permissionState.locationPermissionRequested
                            ? 'Package launched on emulator, but foreground location permission was not granted and no runtime foreground location evidence was emitted.'
                            : 'Package launched on emulator, but the current scaffold does not request foreground location permission.',
    },
    '09-android-background-location-and-geofence-adapter': {
      status:
        backgroundSettingsPage.observed &&
        permissionState.backgroundLocationPermissionGranted &&
        geofenceEnterExitObserved &&
        backgroundSampleObserved &&
        activeGeofenceLimitObserved &&
        systemProximityRegistrationObserved &&
        backgroundDegradedStatusObserved
          ? 'background_settings_page_permission_granted_emulator_sample_enter_exit_limit_system_registration_and_degraded_status_observed'
          : backgroundSettingsPage.observed &&
              permissionState.backgroundLocationPermissionGranted &&
              geofenceEnterExitObserved &&
              backgroundSampleObserved &&
              activeGeofenceLimitObserved &&
              backgroundDegradedStatusObserved
            ? 'background_settings_page_permission_granted_emulator_sample_enter_exit_limit_and_degraded_status_observed'
            : backgroundSettingsPage.observed &&
                permissionState.backgroundLocationPermissionGranted &&
                geofenceEnterExitObserved &&
                backgroundSampleObserved &&
                activeGeofenceLimitObserved
              ? 'background_settings_page_permission_granted_emulator_sample_enter_exit_and_limit_observed'
              : backgroundSettingsPage.observed &&
                  permissionState.backgroundLocationPermissionGranted &&
                  geofenceEnterExitObserved &&
                  backgroundSampleObserved
                ? 'background_settings_page_permission_granted_emulator_sample_and_enter_exit_observed'
                : permissionState.backgroundLocationPermissionGranted &&
                    geofenceEnterExitObserved &&
                    backgroundSampleObserved &&
                    activeGeofenceLimitObserved &&
                    systemProximityRegistrationObserved &&
                    backgroundDegradedStatusObserved
                  ? 'background_permission_granted_emulator_sample_enter_exit_limit_system_registration_and_degraded_status_observed'
                  : permissionState.backgroundLocationPermissionGranted &&
                      geofenceEnterExitObserved &&
                      backgroundSampleObserved &&
                      activeGeofenceLimitObserved &&
                      backgroundDegradedStatusObserved
                    ? 'background_permission_granted_emulator_sample_enter_exit_limit_and_degraded_status_observed'
                    : permissionState.backgroundLocationPermissionGranted &&
                        geofenceEnterExitObserved &&
                        backgroundSampleObserved &&
                        activeGeofenceLimitObserved
                      ? 'background_permission_granted_emulator_sample_enter_exit_and_limit_observed'
                      : permissionState.backgroundLocationPermissionGranted &&
                          geofenceEnterExitObserved &&
                          backgroundSampleObserved
                        ? 'background_permission_granted_emulator_sample_and_enter_exit_transition_observed'
                        : permissionState.backgroundLocationPermissionGranted && geofenceEnterExitObserved
                          ? 'background_permission_granted_emulator_enter_exit_transition_observed'
                          : permissionState.backgroundLocationPermissionGranted
                            ? 'background_permission_granted_geofence_transition_manual_required'
                            : permissionState.backgroundLocationPermissionRequested
                              ? 'background_permission_declared_geofence_transition_manual_required'
                              : 'manual_required',
      proofArtifact:
        'output/tracking-plan-proof/09-android-background-location-and-geofence-adapter/05-geofence-transition-proof.json',
      reason:
        backgroundSettingsPage.observed &&
        permissionState.backgroundLocationPermissionGranted &&
        geofenceEnterExitObserved &&
        backgroundSampleObserved &&
        activeGeofenceLimitObserved &&
        systemProximityRegistrationObserved &&
        backgroundDegradedStatusObserved
          ? 'Android app settings page routing, background location permission grant, emulator foreground-service LocationManager GPS-listener background-activity sample storage, emulator LocationManager GPS-listener local-geofence enter/exit/dwell rows, app-owned active geofence count within the Android documented per-app/per-device-user limit, Android LocationManager addProximityAlert registration, separate Android system proximity broadcast counters, and WP10 low-power/app-restart/pending-upload/manual-required status-gap bridge were observed; Android system geofence delivery remains unclaimed unless the separate system counter is nonzero, and Android system dwell, physical-device behavior, authority, provider delivery, production upload workers, and product-ready tracking remain unclaimed.'
          : backgroundSettingsPage.observed &&
              permissionState.backgroundLocationPermissionGranted &&
              geofenceEnterExitObserved &&
              backgroundSampleObserved &&
              activeGeofenceLimitObserved &&
              backgroundDegradedStatusObserved
            ? 'Android app settings page routing, background location permission grant, emulator foreground-service LocationManager GPS-listener background-activity sample storage, emulator LocationManager GPS-listener local-geofence enter/exit/dwell rows, app-owned active geofence count within the Android documented per-app/per-device-user limit, and WP10 low-power/app-restart/pending-upload/manual-required status-gap bridge were observed; Android system geofencing, Android system dwell, physical-device behavior, authority, provider delivery, production upload workers, and product-ready tracking remain unclaimed.'
            : backgroundSettingsPage.observed &&
                permissionState.backgroundLocationPermissionGranted &&
                geofenceEnterExitObserved &&
                backgroundSampleObserved &&
                activeGeofenceLimitObserved
              ? 'Android app settings page routing, background location permission grant, emulator foreground-service LocationManager GPS-listener background-activity sample storage, emulator LocationManager GPS-listener local-geofence enter/exit/dwell rows, and app-owned active geofence count within the Android documented per-app/per-device-user limit were observed; Android system geofencing, Android system dwell, physical-device behavior, authority, provider delivery, production upload workers, and product-ready tracking remain unclaimed.'
              : backgroundSettingsPage.observed &&
                  permissionState.backgroundLocationPermissionGranted &&
                  geofenceEnterExitObserved &&
                  backgroundSampleObserved
                ? 'Android app settings page routing, background location permission grant, emulator foreground-service LocationManager GPS-listener background-activity sample storage, and emulator LocationManager GPS-listener local-geofence enter/exit/dwell rows were observed; Android system geofencing, Android system dwell, physical-device behavior, authority, provider delivery, production upload workers, and product-ready tracking remain unclaimed.'
                : permissionState.backgroundLocationPermissionGranted &&
                    geofenceEnterExitObserved &&
                    backgroundSampleObserved &&
                    activeGeofenceLimitObserved &&
                    systemProximityRegistrationObserved &&
                    backgroundDegradedStatusObserved
                  ? 'Background location permission grant, emulator foreground-service LocationManager GPS-listener background-activity sample storage, emulator LocationManager GPS-listener local-geofence enter/exit/dwell rows, app-owned active geofence count within the Android documented per-app/per-device-user limit, Android LocationManager addProximityAlert registration, separate Android system proximity broadcast counters, and WP10 low-power/app-restart/pending-upload/manual-required status-gap bridge were observed through app-owned proof storage. This ATD emulator image does not expose an Android Settings activity, so Android app settings page routing remains unclaimed; Android system geofence delivery remains unclaimed unless the separate system counter is nonzero, and Android system dwell, physical-device behavior, authority, provider delivery, production upload workers, and product-ready tracking remain unclaimed.'
                  : permissionState.backgroundLocationPermissionGranted &&
                      geofenceEnterExitObserved &&
                      backgroundSampleObserved &&
                      activeGeofenceLimitObserved &&
                      backgroundDegradedStatusObserved
                    ? 'Background location permission grant, emulator foreground-service LocationManager GPS-listener background-activity sample storage, emulator LocationManager GPS-listener local-geofence enter/exit/dwell rows, app-owned active geofence count within the Android documented per-app/per-device-user limit, and WP10 low-power/app-restart/pending-upload/manual-required status-gap bridge were observed through app-owned proof storage. Android app settings page routing, system geofencing, Android system dwell, physical-device behavior, authority, provider delivery, production upload workers, and product-ready tracking remain unclaimed.'
                    : permissionState.backgroundLocationPermissionGranted &&
                        geofenceEnterExitObserved &&
                        backgroundSampleObserved &&
                        activeGeofenceLimitObserved
                      ? 'Background location permission grant, emulator foreground-service LocationManager GPS-listener background-activity sample storage, emulator LocationManager GPS-listener local-geofence enter/exit/dwell rows, and app-owned active geofence count within the Android documented per-app/per-device-user limit were observed through app-owned proof storage. Android app settings page routing, system geofencing, Android system dwell, physical-device behavior, authority, provider delivery, production upload workers, and product-ready tracking remain unclaimed.'
                      : permissionState.backgroundLocationPermissionGranted &&
                          geofenceEnterExitObserved &&
                          backgroundSampleObserved
                        ? 'Background location permission grant, emulator foreground-service LocationManager GPS-listener background-activity sample storage, and emulator LocationManager GPS-listener local-geofence enter/exit/dwell rows were observed through app-owned proof storage; Android system geofencing, Android system dwell, Android settings-page flow, physical-device behavior, authority, provider delivery, production upload workers, and product-ready tracking remain unclaimed.'
                        : permissionState.backgroundLocationPermissionGranted && geofenceEnterExitObserved
                          ? 'Background location permission grant and emulator LocationManager GPS-listener local-geofence enter/exit/dwell rows were observed through app-owned proof storage; background sample collection, Android system geofencing, Android system dwell, Android settings-page flow, physical-device behavior, authority, provider delivery, and product-ready tracking remain unclaimed.'
                          : permissionState.backgroundLocationPermissionGranted
                            ? 'Background location permission grant state was observed on the emulator package, but no background location sample, geofence transition, physical-device behavior, authority, or product-ready tracking is claimed.'
                            : permissionState.backgroundLocationPermissionRequested
                              ? 'Background permission is declared, but no background permission grant or geofence transition was observed.'
                              : 'No background location/geofence permission or transition adapter is present in the current scaffold.',
    },
    '10-android-battery-connectivity-and-status-adapter': {
      status:
        runtime.service.isForeground && backgroundDegradedStatusObserved
          ? 'emulator_scaffold_and_status_gap_bridge_observed'
          : 'emulator_scaffold_observed',
      proofArtifact:
        'output/tracking-plan-proof/10-android-battery-connectivity-and-status-adapter/04-device-status-proof.json',
      reason:
        runtime.service.isForeground && backgroundDegradedStatusObserved
          ? 'Emulator package launch, foreground service state, battery dump, connectivity dump, and WP10 low-power/app-restart/pending-upload/manual-required bridge were collected.'
          : runtime.service.isForeground
            ? 'Emulator package launch, foreground service state, battery dump, and connectivity dump were collected.'
            : 'Package launched, but foreground service state was not observed.',
    },
  };
}

async function writeProofFiles(proof) {
  await writeJson(proofPath, proof);
  for (const root of [output08, output09, output10]) {
    await writeText(path.join(root, '00-source-snapshot.md'), sourceSnapshotMarkdown(proof));
    await writeJson(path.join(root, '01-device-metadata.json'), proof.device);
  }
  await writeText(
    path.join(output08, '02-platform-permission-proof.md'),
    platformPermissionMarkdown(proof, 'foreground')
  );
  await writeJson(path.join(output08, '03-runtime-location-evidence.json'), foregroundLocationProof(proof));
  await writeText(
    path.join(output08, '15-manual-platform-proof.md'),
    manualProofMarkdown(proof, 'WP08 Android foreground')
  );
  await writeText(path.join(output08, '16-validation-commands.log'), commandLog());
  await writeText(
    path.join(output09, '02-platform-permission-proof.md'),
    platformPermissionMarkdown(proof, 'background')
  );
  await writeJson(path.join(output09, '05-geofence-transition-proof.json'), geofenceProof(proof));
  await writeText(
    path.join(output09, '15-manual-platform-proof.md'),
    manualProofMarkdown(proof, 'WP09 Android background/geofence')
  );
  await writeText(path.join(output09, '16-validation-commands.log'), commandLog());
  await writeJson(path.join(output10, '04-device-status-proof.json'), deviceStatusProof(proof));
  await writeText(
    path.join(output10, '15-manual-platform-proof.md'),
    manualProofMarkdown(proof, 'WP10 Android device status')
  );
  await writeText(path.join(output10, '16-validation-commands.log'), commandLog());
}

function sourceSnapshotMarkdown(proof) {
  return `# Android tracking emulator source snapshot

- Checked at: ${proof.checkedAt}
- Commit: ${proof.commit}
- Branch: ${gitBranch()}
- Proof command: \`npm run test:tracking-plan-android-emulator-proof\`
- Proof script: \`scripts/test/tracking-plan-android-emulator-proof.mjs\`
- APK: \`${proof.package.apk}\`
- Required proof tier: ${proof.requiredProofTier}
- Current proof tier: ${proof.currentProofTier}
- Product claim ready: ${String(proof.productClaimReady)}
`;
}

function foregroundLocationProof(proof) {
  const foregroundSampleObserved =
    proof.runtime.ui.foregroundLocationSampleStateText === 'last-known-location-sample-observed' ||
    proof.runtime.ui.foregroundLocationSampleStateText === 'current-location-sample-observed-emulator-location-manager';
  const foregroundRawCoordinateObserved =
    proof.runtime.ui.foregroundLocationSampleStateText ===
      'current-location-sample-observed-emulator-location-manager' &&
    proof.runtime.ui.foregroundLocationLatitude !== undefined &&
    proof.runtime.ui.foregroundLocationLongitude !== undefined;
  return {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    requiredProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentStatus: proof.workpackProof['08-android-foreground-location-adapter'].status,
    packageLaunchObserved: proof.runtime.activity.packageFocused,
    foregroundServiceObserved: proof.runtime.service.isForeground,
    locationEvidenceCaptured: foregroundSampleObserved,
    locationEvidenceBoundary: foregroundSampleObserved
      ? foregroundRawCoordinateObserved
        ? 'emulator-current-location-manager-sample-with-raw-coordinate-export'
        : 'app-ui-reported-last-known-sample-state-without-raw-coordinate-export'
      : 'no-app-emitted-location-sample-observed',
    foregroundLocationPermissionRequested: proof.permissionState.locationPermissionRequested,
    foregroundLocationPermissionUxDialogObserved: proof.foregroundPermissionUx.observed,
    foregroundLocationPermissionUx: proof.foregroundPermissionUx,
    foregroundLocationPermissionGranted: proof.permissionState.foregroundLocationPermissionGranted,
    foregroundLocationPermissionStateText: proof.runtime.ui.foregroundLocationPermissionStateText,
    foregroundLocationSampleStateText: proof.runtime.ui.foregroundLocationSampleStateText,
    foregroundLocationProvider: proof.runtime.ui.foregroundLocationProvider,
    foregroundLocationObservedAtEpochMillis: proof.runtime.ui.foregroundLocationObservedAtEpochMillis,
    foregroundLocationAccuracyMeters: proof.runtime.ui.foregroundLocationAccuracyMeters,
    foregroundLocationSampleSource: proof.runtime.ui.foregroundLocationSampleSource,
    foregroundLocationLatitude: proof.runtime.ui.foregroundLocationLatitude,
    foregroundLocationLongitude: proof.runtime.ui.foregroundLocationLongitude,
    foregroundRawCoordinateExportObserved: foregroundRawCoordinateObserved,
    fusedForegroundLocationSampleStateText: proof.runtime.ui.fusedForegroundLocationSampleStateText,
    fusedForegroundLocationProvider: proof.runtime.ui.fusedForegroundLocationProvider,
    fusedForegroundLocationObservedAtEpochMillis: proof.runtime.ui.fusedForegroundLocationObservedAtEpochMillis,
    fusedForegroundLocationAccuracyMeters: proof.runtime.ui.fusedForegroundLocationAccuracyMeters,
    fusedForegroundLocationSampleSource: proof.runtime.ui.fusedForegroundLocationSampleSource,
    fusedForegroundLocationLatitude: proof.runtime.ui.fusedForegroundLocationLatitude,
    fusedForegroundLocationLongitude: proof.runtime.ui.fusedForegroundLocationLongitude,
    fusedForegroundSampleClaimed:
      (proof.runtime.ui.fusedForegroundLocationSampleStateText ===
        'current-fused-foreground-location-sample-observed-emulator' ||
        proof.runtime.ui.fusedForegroundLocationSampleStateText ===
          'last-known-fused-foreground-location-sample-observed') &&
      proof.runtime.ui.fusedForegroundLocationLatitude !== undefined &&
      proof.runtime.ui.fusedForegroundLocationLongitude !== undefined,
    missingProofReason: proof.workpackProof['08-android-foreground-location-adapter'].reason,
    device: proof.device,
    artifacts: proof.runtime.artifacts,
  };
}

function geofenceProof(proof) {
  const geofenceTransitions = proof.runtime.geofenceTransitions;
  const backgroundSample = proof.runtime.backgroundLocationSample;
  const activeGeofenceLimit = proof.runtime.activeGeofenceLimit;
  const backgroundDegradedStatus = backgroundDegradedStatusProof();
  const systemProximityRegistration = systemProximityRegistrationProof(geofenceTransitions);
  return {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    requiredProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentStatus: proof.workpackProof['09-android-background-location-and-geofence-adapter'].status,
    packageLaunchObserved: proof.runtime.activity.packageFocused,
    foregroundServiceObserved: proof.runtime.service.isForeground,
    backgroundLocationPermissionRequested: proof.permissionState.backgroundLocationPermissionRequested,
    backgroundLocationPermissionGranted: proof.permissionState.backgroundLocationPermissionGranted,
    backgroundLocationSettingsPageObserved: proof.backgroundSettingsPage.observed,
    backgroundLocationSettingsPage: proof.backgroundSettingsPage,
    backgroundLocationPermissionStateText: proof.runtime.ui.backgroundLocationPermissionStateText,
    backgroundLocationSampleStateText: proof.runtime.ui.backgroundLocationSampleStateText,
    backgroundLocationSampleCaptured: backgroundSample.sampleCount > 0,
    backgroundLocationSampleCount: backgroundSample.sampleCount,
    backgroundLocationSampleProvider: backgroundSample.provider,
    backgroundLocationSampleObservedAtEpochMillis: backgroundSample.observedAtEpochMillis,
    backgroundLocationSampleAccuracyMeters: backgroundSample.accuracyMeters,
    backgroundLocationSampleSource: backgroundSample.source,
    backgroundLocationSampleActivityBackgrounded: backgroundSample.activityBackgrounded,
    backgroundLocationSampleBoundary:
      backgroundSample.sampleCount > 0
        ? 'emulator-foreground-service-location-manager-gps-listener-background-activity-sample-only'
        : 'no-emulator-background-location-sample-observed',
    backgroundGeofenceStateText: proof.runtime.ui.backgroundGeofenceStateText,
    geofenceTransitionCount: geofenceTransitions.transitionCount,
    geofenceEnterCount: geofenceTransitions.enterCount,
    geofenceExitCount: geofenceTransitions.exitCount,
    geofenceDwellCount: geofenceTransitions.dwellCount,
    geofenceDwellSource: geofenceTransitions.dwellSource,
    geofenceDwellLastObservedAtEpochMillis: geofenceTransitions.dwellLastObservedEpochMillis,
    geofenceDwellInsideStartedAtEpochMillis: geofenceTransitions.dwellInsideStartedEpochMillis,
    geofenceLastTransition: geofenceTransitions.lastTransition,
    geofenceSource: geofenceTransitions.source,
    geofenceRegistered: geofenceTransitions.registered,
    systemProximityRegistrationObserved: systemProximityRegistration.observed,
    systemProximityRegistration,
    systemProximityTransitionObserved: systemProximityRegistration.transitionObserved,
    systemProximityTransitionCount: systemProximityRegistration.transitionCount,
    systemProximityEnterCount: systemProximityRegistration.enterCount,
    systemProximityExitCount: systemProximityRegistration.exitCount,
    systemProximityTransitionBoundary: systemProximityRegistration.proofBoundary,
    geofenceHasInsideState: geofenceTransitions.hasInsideState,
    geofenceInsideState: geofenceTransitions.insideState,
    activeGeofenceCount: activeGeofenceLimit.activeGeofenceCount,
    activeGeofenceLimitWithinDocumentedLimit: activeGeofenceLimit.withinDocumentedLimit,
    activeGeofenceDocumentedLimitPerAppPerDeviceUser: activeGeofenceLimit.documentedLimitPerAppPerDeviceUser,
    activeGeofenceLimitSourceUrl: activeGeofenceLimit.documentedLimitSourceUrl,
    activeGeofenceLimitBoundary: activeGeofenceLimit.proofBoundary,
    backgroundDegradedStatusProof: backgroundDegradedStatus,
    geofenceTransitionBoundary:
      geofenceTransitions.enterCount > 0 && geofenceTransitions.exitCount > 0 && geofenceTransitions.dwellCount > 0
        ? 'emulator-location-manager-gps-listener-local-geofence-enter-exit-dwell-only'
        : 'no-emulator-geofence-transition-observed',
    missingProofReason: proof.workpackProof['09-android-background-location-and-geofence-adapter'].reason,
    device: proof.device,
    artifacts: proof.runtime.artifacts,
  };
}

function backgroundDegradedStatusProof() {
  const proofPath = path.join(repoRoot, androidStatusGapProofRelativePath);
  if (!existsSync(proofPath)) {
    return {
      observed: false,
      reason: 'WP10 Android status-gap proof artifact was not present when WP09 proof was generated.',
      proofArtifact: androidStatusGapProofRelativePath,
      evidenceArtifact: androidStatusGapEvidenceRelativePath,
      proofBoundary: 'no-background-degraded-status-proof-artifact-observed',
      nonClaims: backgroundDegradedStatusNonClaims(),
    };
  }
  const statusGapProof = JSON.parse(readFileSync(proofPath, 'utf8'));
  return {
    observed: true,
    proofArtifact: androidStatusGapProofRelativePath,
    evidenceArtifact: androidStatusGapEvidenceRelativePath,
    coveredCaseKinds: [
      statusGapProof.lowPower?.caseKind,
      statusGapProof.killedRestarted?.caseKind,
      statusGapProof.pendingUpload?.caseKind,
      statusGapProof.manualRequired?.caseKind,
    ].filter(Boolean),
    lowPowerClaimState: statusGapProof.lowPower?.claimState ?? null,
    appRestartClaimState: statusGapProof.killedRestarted?.claimState ?? null,
    pendingUploadClaimState: statusGapProof.pendingUpload?.claimState ?? null,
    manualRequiredClaimState: statusGapProof.manualRequired?.claimState ?? null,
    proofBoundary:
      'WP10 Rust parent Android status-gap proof covers low-power degradation, app restart auditability, pending-upload auditability, and manual-required platform rows; it does not prove Android system geofence delivery, Android system dwell, or physical-device background behavior.',
    nonClaims: backgroundDegradedStatusNonClaims(),
  };
}

function backgroundDegradedStatusNonClaims() {
  return [
    'No Android system geofence delivery is claimed from WP10 status rows.',
    'No dwell transition delivery is claimed from WP10 status rows.',
    'No physical-device background behavior is claimed from WP10 status rows.',
    'No provider delivery, authority, production upload worker, or product-ready Android tracking claim is made.',
  ];
}

function deviceStatusProof(proof) {
  return {
    schemaVersion: 1,
    checkedAt: proof.checkedAt,
    commit: proof.commit,
    requiredProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentProofTier: 'P3_LOCAL_DEV_MACHINE',
    currentStatus: proof.workpackProof['10-android-battery-connectivity-and-status-adapter'].status,
    packageLaunchObserved: proof.runtime.activity.packageFocused,
    foregroundServiceObserved: proof.runtime.service.isForeground,
    foregroundNotification: proof.runtime.service.foregroundNotification,
    battery: proof.runtime.battery,
    connectivitySummary: proof.runtime.connectivitySummary,
    statusGapBridge: backgroundDegradedStatusProof(),
    ui: proof.runtime.ui,
    screenshotInspection: proof.runtime.screenshotInspection,
    logcatFindings: proof.runtime.logcatFindings,
    device: proof.device,
    artifacts: proof.runtime.artifacts,
    nonClaims: [
      'No product tracking freshness claim is made from this emulator scaffold status proof.',
      'No physical-device background behavior, Android system geofence delivery, production upload worker, or product-ready Android tracking claim is made from the status-gap bridge.',
    ],
  };
}

function platformPermissionMarkdown(proof, mode) {
  return `# Android ${mode} permission proof

- Checked at: ${proof.checkedAt}
- Commit: ${proof.commit}
- Device: ${proof.device.productModel} / Android ${proof.device.androidRelease} API ${proof.device.androidSdk}
- Package: ${proof.package.packageName}
- Resolved activity: ${proof.package.resolvedActivity}

## Requested permissions

${proof.permissionState.requested.map((permission) => `- ${permission}: granted=${String(proof.permissionState.grants[permission] ?? false)}`).join('\n')}
${proof.permissionState.requested.length === 0 ? '_No requested Android permissions were found in the package dump._' : ''}

## Tracking claim boundary

- Foreground location permission requested: ${String(proof.permissionState.locationPermissionRequested)}
- Foreground location permission granted: ${String(proof.permissionState.foregroundLocationPermissionGranted)}
- Background location permission requested: ${String(proof.permissionState.backgroundLocationPermissionRequested)}
- Background location permission granted: ${String(proof.permissionState.backgroundLocationPermissionGranted)}
- Foreground service observed: ${String(proof.runtime.service.isForeground)}
- Product location/geofence claim ready: false
`;
}

function manualProofMarkdown(proof, label) {
  return `# ${label} manual platform proof

This proof was generated by \`npm run test:tracking-plan-android-emulator-proof\` on an Android emulator.

## Proved

- Debug APK built and installed.
- Launcher activity resolved and launched.
- Package process observed with pid ${proof.runtime.pid}.
- Foreground service observed: ${String(proof.runtime.service.isForeground)}.
- Foreground location permission granted: ${String(proof.permissionState.foregroundLocationPermissionGranted)}.
- Foreground location state text: ${proof.runtime.ui.foregroundLocationPermissionStateText ?? 'not-observed'}.
- Foreground sample state text: ${proof.runtime.ui.foregroundLocationSampleStateText ?? 'not-observed'}.
- Background location permission granted: ${String(proof.permissionState.backgroundLocationPermissionGranted)}.
- Background location settings page observed: ${String(proof.backgroundSettingsPage.observed)}.
- Background location state text: ${proof.runtime.ui.backgroundLocationPermissionStateText ?? 'not-observed'}.
- Background sample state text: ${proof.runtime.ui.backgroundLocationSampleStateText ?? 'not-observed'}.
- Background sample count: ${String(proof.runtime.backgroundLocationSample.sampleCount)}.
- Background sample provider: ${proof.runtime.backgroundLocationSample.provider ?? 'not-observed'}.
- Background sample source: ${proof.runtime.backgroundLocationSample.source ?? 'not-observed'}.
- Background sample activity backgrounded: ${String(proof.runtime.backgroundLocationSample.activityBackgrounded)}.
- Background geofence state text: ${proof.runtime.ui.backgroundGeofenceStateText ?? 'not-observed'}.
- Background geofence transition count: ${String(proof.runtime.geofenceTransitions.transitionCount)}.
- Background geofence enter count: ${String(proof.runtime.geofenceTransitions.enterCount)}.
- Background geofence exit count: ${String(proof.runtime.geofenceTransitions.exitCount)}.
- Background geofence app-owned dwell count: ${String(proof.runtime.geofenceTransitions.dwellCount)}.
- Background geofence app-owned dwell source: ${proof.runtime.geofenceTransitions.dwellSource ?? 'not-observed'}.
- Background geofence source: ${proof.runtime.geofenceTransitions.source ?? 'not-observed'}.
- Android system proximity broadcast transition count: ${String(proof.runtime.geofenceTransitions.systemProximityTransitionCount)}.
- Android system proximity broadcast source: ${proof.runtime.geofenceTransitions.systemProximityRegistrationSource ?? 'not-observed'}.
- Active app-owned local geofence count: ${String(proof.runtime.activeGeofenceLimit.activeGeofenceCount)} of documented Android per-app/per-device-user limit ${String(proof.runtime.activeGeofenceLimit.documentedLimitPerAppPerDeviceUser)}.
- Battery and connectivity dumps collected.
- UI tree collected and contains scaffold/manual-consent text: ${String(proof.runtime.ui.hasLaunchText)}.
- Screenshot visual contrast observed: ${String(proof.runtime.screenshotInspection.visualClaimReady)}.

## Not claimed

${proof.nonClaims.map((claim) => `- ${claim}`).join('\n')}
`;
}

function commandLog() {
  return `${commands.map((entry) => `${entry.exitCode === 0 ? 'PASS' : 'FAIL'} ${entry.command}`).join('\n')}\n`;
}

async function adb(tools, serial, args, options = {}) {
  const result = await runCommand(tools.adbPath, ['-s', serial, ...args], { capture: true });
  if (options.artifact !== undefined) {
    await writeText(options.artifact, result.output);
  }
  return result.output;
}

async function adbMaybe(tools, serial, args) {
  return runCommand(tools.adbPath, ['-s', serial, ...args], { capture: true, allowFailure: true });
}

async function adbText(tools, serial, args) {
  return adb(tools, serial, args);
}

async function adbBuffer(tools, serial, args) {
  const result = spawnSync(tools.adbPath, ['-s', serial, ...args], {
    cwd: repoRoot,
    encoding: null,
    maxBuffer: 20 * 1024 * 1024,
  });
  const exitCode = result.status ?? 1;
  commands.push({ command: `${tools.adbPath} -s ${serial} ${args.join(' ')}`, exitCode, artifact: null });
  if (exitCode !== 0) {
    throw new Error(`adb ${args.join(' ')} failed: ${String(result.stderr)}`);
  }
  return result.stdout;
}

async function runNpm(args) {
  await runCommand(
    process.platform === 'win32' ? 'cmd' : 'npm',
    process.platform === 'win32' ? ['/c', 'npm', ...args] : args
  );
}

async function runCapture(command, args) {
  return (await runCommand(command, args, { capture: true })).output;
}

async function runCommand(command, args, options = {}) {
  const output = [];
  const child = spawn(command, args, {
    cwd: repoRoot,
    stdio: ['ignore', options.capture ? 'pipe' : 'inherit', options.capture ? 'pipe' : 'inherit'],
    windowsHide: true,
  });
  if (options.capture === true) {
    child.stdout.on('data', (chunk) => output.push(String(chunk)));
    child.stderr.on('data', (chunk) => output.push(String(chunk)));
  }
  const exitCode = await waitForExit(child);
  const commandLine = [command, ...args].join(' ');
  commands.push({ command: commandLine, exitCode, artifact: null });
  if (exitCode !== 0 && options.allowFailure !== true) {
    throw new Error(`${commandLine} exited with ${exitCode}`);
  }
  return { exitCode, output: output.join('') };
}

function waitForExit(child) {
  return new Promise((resolve, reject) => {
    child.once('exit', (code, signal) => resolve(signal === null ? (code ?? 1) : 1));
    child.once('error', reject);
  });
}

async function shutdownEmulator(tools, serial) {
  try {
    await adb(tools, serial, ['emu', 'kill']);
  } catch {
    // The emulator may already be gone after a successful run.
  }
}

function gitHead() {
  const result = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, encoding: 'utf8' });
  if ((result.status ?? 1) !== 0) {
    throw new Error('git rev-parse HEAD failed');
  }
  return result.stdout.trim();
}

function gitBranch() {
  const result = spawnSync('git', ['branch', '--show-current'], { cwd: repoRoot, encoding: 'utf8' });
  if ((result.status ?? 1) !== 0) {
    throw new Error('git branch --show-current failed');
  }
  return result.stdout.trim();
}

async function writeJson(filePath, value) {
  await writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

async function writeText(filePath, value) {
  await writeFile(filePath, normalizeText(value), 'utf8');
}

async function appendText(filePath, value) {
  const existing = existsSync(filePath) ? await readFile(filePath, 'utf8') : '';
  await writeText(filePath, `${existing}${value}`);
}

function normalizeText(value) {
  const normalizedLines = value
    .replaceAll('\r\n', '\n')
    .replaceAll('\r', '\n')
    .split('\n')
    .map((line) => line.replace(/[ \t]+$/u, ''));
  const normalized = normalizedLines.join('\n');
  return normalized.endsWith('\n') ? normalized : `${normalized}\n`;
}

function assertFileExists(filePath, label) {
  if (!existsSync(filePath)) {
    throw new Error(`${label} not found: ${filePath}`);
  }
}

function assertIncludes(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`${label} did not include ${expected}`);
  }
}

function countMatches(value, pattern) {
  return [...value.matchAll(pattern)].length;
}

function decodeXmlText(value) {
  return value.replaceAll('&#10;', '\n').replaceAll('&quot;', '"').replaceAll('&amp;', '&');
}

function relativePath(filePath) {
  return path.relative(repoRoot, filePath).replaceAll('\\', '/');
}
