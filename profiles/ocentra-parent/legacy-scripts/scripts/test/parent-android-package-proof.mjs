import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'parent-android-package-proof');
const proofPath = join(outputDir, 'proof.json');
const packageRoot = join(repoRoot, 'target', 'release-packages', 'parent-android');
const latestApkPath = join(packageRoot, 'ocentra-parent-mobile-android-debug-latest.apk');
const latestChecksumPath = `${latestApkPath}.sha256`;
const commands = [];

const parentPackageName = 'ca.ocentra.parent.mobile';
const parentLaunchTarget = `${parentPackageName}/.MainActivity`;

main();

function main() {
  mkdirSync(outputDir, { recursive: true });

  runNpmScript('release:package:parent-android');
  runNpmScript('test:parent-mobile-package-source-artifact-proof');

  const artifactProof = assertPackageArtifacts();
  const sourceBoundaryProof = assertSourceBoundary();
  const installProof = detectInstallProof(artifactProof.latestApk.absolutePath);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: gitHead(),
    packageAnchor: 'release:package:parent-android',
    commands,
    evidence: {
      proofScript: relative(repoRoot, join(repoRoot, 'scripts', 'test', 'parent-android-package-proof.mjs')),
      buildScript: relative(
        repoRoot,
        join(repoRoot, 'scripts', 'release', 'parent-android', 'build-parent-mobile-package.mjs')
      ),
      sourceArtifactProof: relative(
        repoRoot,
        join(repoRoot, 'scripts', 'test', 'parent-mobile-package-source-artifact-proof.mjs')
      ),
      apkArtifact: artifactProof.latestApk.relativePath,
      apkChecksum: artifactProof.latestChecksum.relativePath,
      output: relative(repoRoot, proofPath),
    },
    artifactProof,
    sourceBoundaryProof,
    installOrSimulatorProof: installProof,
    storeAndManualRequiredTruth: {
      signingState: 'manual-required',
      storeDistributionState: 'manual-required',
      artifactRequirement: 'signed Play-ready Android artifact required before any store claim',
      manualRequiredStates: [
        'device or booted emulator install-and-launch proof',
        'release signing separate from debug packaging',
        'Google Play upload, track, and store review proof',
      ],
    },
    noClaimBoundary: [
      'child-runtime distribution not claimed',
      'Android package build does not imply iOS readiness',
      'Android package build does not imply desktop readiness',
      'debug APK artifact does not imply Google Play or production release readiness',
    ],
  };

  writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`, 'utf8');
  console.log(`parent-android-package-proof-ok:${installProof.state}`);
  console.log(`evidence=${proofPath}`);
}

function assertPackageArtifacts() {
  assert.equal(existsSync(packageRoot), true, 'parent Android package root should exist after build');
  assert.equal(existsSync(latestApkPath), true, 'latest parent Android APK should exist');
  assert.equal(existsSync(latestChecksumPath), true, 'latest parent Android checksum should exist');

  const versionedApk = findVersionedApk();
  const versionedChecksumPath = `${versionedApk}.sha256`;
  assert.equal(existsSync(versionedChecksumPath), true, 'versioned parent Android checksum should exist');

  return {
    packageRoot: relative(repoRoot, packageRoot),
    latestApk: fileProof(latestApkPath),
    latestChecksum: fileProof(latestChecksumPath),
    versionedApk: fileProof(versionedApk),
    versionedChecksum: fileProof(versionedChecksumPath),
    packageId: parentPackageName,
    launchTarget: parentLaunchTarget,
  };
}

function assertSourceBoundary() {
  const buildFile = readRepoFile('platforms/android/parent/app/build.gradle');
  const manifest = readRepoFile('platforms/android/parent/app/src/main/AndroidManifest.xml');
  const activity = readRepoFile('platforms/android/parent/app/src/main/java/ca/ocentra/parent/mobile/MainActivity.java');
  const strings = readRepoFile('platforms/android/parent/app/src/main/res/values/strings.xml');
  const releaseScript = readRepoFile('scripts/release/parent-android/build-parent-mobile-package.mjs');
  const smokeScript = readRepoFile('scripts/smoke/android-apk-smoke.sh');

  assert.match(buildFile, /namespace = 'ca\.ocentra\.parent\.mobile'/u);
  assert.match(buildFile, /applicationId = 'ca\.ocentra\.parent\.mobile'/u);
  assert.match(manifest, /android\.intent\.action\.MAIN/u);
  assert.match(manifest, /android\.intent\.category\.LAUNCHER/u);
  assert.doesNotMatch(manifest, /OcentraParentAgentService/u);
  assert.match(activity, /package ca\.ocentra\.parent\.mobile;/u);
  assert.match(strings, /Ocentra Parent Mobile Android scaffold/u);
  assert.match(releaseScript, /platforms',\s*'android',\s*'parent'/u);
  assert.match(releaseScript, /ocentra-parent-mobile-android-debug-latest\.apk/u);
  assert.match(smokeScript, /\$\{2:-ca\.ocentra\.parent\.agent\}/u);
  assert.match(smokeScript, /\$\{3:-\$package_name\/\.MainActivity\}/u);

  return {
    packageId: parentPackageName,
    launchTarget: parentLaunchTarget,
    childRuntimeLeak: 'not-detected',
    explicitSmokeInvocationRequired: true,
    evidenceFiles: [
      'platforms/android/parent/app/build.gradle',
      'platforms/android/parent/app/src/main/AndroidManifest.xml',
      'platforms/android/parent/app/src/main/java/ca/ocentra/parent/mobile/MainActivity.java',
      'scripts/release/parent-android/build-parent-mobile-package.mjs',
      'scripts/smoke/android-apk-smoke.sh',
    ],
  };
}

function detectInstallProof(apkPath) {
  const adbDevices = runCommand('adb', ['devices'], { allowFailure: true });
  const emulatorAvds = runCommand('cmd', ['/c', 'emulator -list-avds'], { allowFailure: true });
  const attachedDevices = parseAdbDevices(adbDevices.stdout);
  const availableAvds = parseAvdNames(emulatorAvds.stdout);

  if (attachedDevices.length === 0) {
    return {
      state: 'manual-required',
      reason: 'android-device-or-booted-emulator-required',
      attachedDevices,
      availableAvds,
      smokeCommand: `bash scripts/smoke/android-apk-smoke.sh ${relative(repoRoot, apkPath)} ${parentPackageName} ${parentLaunchTarget}`,
      smokeResult: 'not-run',
      note:
        'No attached device or booted emulator was available in this workspace, so install/launch remains explicit manual-required proof.',
    };
  }

  const smoke = runCommand(
    'bash',
    ['scripts/smoke/android-apk-smoke.sh', apkPath, parentPackageName, parentLaunchTarget],
    { allowFailure: true }
  );

  if (smoke.status !== 0) {
    return {
      state: 'blocked',
      reason: 'android-install-smoke-failed',
      attachedDevices,
      availableAvds,
      smokeCommand: `bash scripts/smoke/android-apk-smoke.sh ${relative(repoRoot, apkPath)} ${parentPackageName} ${parentLaunchTarget}`,
      smokeResult: commandRecord('bash', ['scripts/smoke/android-apk-smoke.sh', apkPath, parentPackageName, parentLaunchTarget], smoke),
      note: 'A device was available, but the parent Android install smoke failed and must be fixed before claiming install proof.',
    };
  }

  return {
    state: 'proved',
    reason: 'android-install-smoke-passed',
    attachedDevices,
    availableAvds,
    smokeCommand: `bash scripts/smoke/android-apk-smoke.sh ${relative(repoRoot, apkPath)} ${parentPackageName} ${parentLaunchTarget}`,
    smokeResult: commandRecord('bash', ['scripts/smoke/android-apk-smoke.sh', apkPath, parentPackageName, parentLaunchTarget], smoke),
    note: 'APK install, launch, process presence, and uninstall were proved through the Android smoke path.',
  };
}

function findVersionedApk() {
  const candidates = readdirSync(packageRoot)
    .filter(
      (fileName) =>
        /^ocentra-parent-mobile-android-debug-v.+\.apk$/u.test(fileName) &&
        !fileName.endsWith('latest.apk')
    )
    .sort();

  if (candidates.length === 0) {
    throw new Error(`Missing versioned parent Android APK in ${packageRoot}`);
  }

  return join(packageRoot, candidates.at(-1));
}

function readRepoFile(path) {
  return readFileSync(join(repoRoot, path), 'utf8');
}

function fileProof(path) {
  const stats = statSync(path);
  return {
    relativePath: relative(repoRoot, path),
    absolutePath: path,
    sizeBytes: stats.size,
    modifiedAt: stats.mtime.toISOString(),
  };
}

function parseAdbDevices(stdout) {
  return stdout
    .split(/\r?\n/u)
    .slice(1)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const [serial = '', state = 'unknown'] = line.split(/\s+/u);
      return { serial, state };
    });
}

function parseAvdNames(stdout) {
  return stdout
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean);
}

function runNpmScript(scriptName) {
  if (process.env.npm_execpath) {
    const result = runCommand(process.execPath, [process.env.npm_execpath, 'run', scriptName]);
    assert.equal(result.status, 0, `${scriptName} should pass`);
    return;
  }

  if (process.platform === 'win32') {
    const result = runCommand('cmd', ['/c', 'npm', 'run', scriptName]);
    assert.equal(result.status, 0, `${scriptName} should pass`);
    return;
  }

  const result = runCommand('npm', ['run', scriptName]);
  assert.equal(result.status, 0, `${scriptName} should pass`);
}

function gitHead() {
  const result = runCommand('git', ['rev-parse', 'HEAD']);
  assert.equal(result.status, 0, 'git rev-parse HEAD should pass');
  return result.stdout.trim();
}

function runCommand(command, args, { allowFailure = false } = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: 'utf8',
    windowsHide: true,
  });

  const record = commandRecord(command, args, result);
  commands.push(record);

  if (!allowFailure && result.status !== 0) {
    throw new Error(`Command failed: ${record.command}`);
  }

  return result;
}

function commandRecord(command, args, result) {
  return {
    command: [command, ...args].join(' '),
    status: result.status ?? (result.error ? 1 : 0),
    stdoutTail: tail(result.stdout ?? ''),
    stderrTail: tail(result.stderr ?? ''),
    error: result.error?.message ?? null,
  };
}

function tail(text, maxLines = 20) {
  const lines = text.split(/\r?\n/u).filter((line) => line.length > 0);
  return lines.slice(-maxLines);
}
