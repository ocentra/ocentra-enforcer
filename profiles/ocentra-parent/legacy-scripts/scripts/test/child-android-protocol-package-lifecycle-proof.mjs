import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'test-results', 'child-android-protocol-package-lifecycle-proof');
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });

  await runNpm([
    'exec',
    '--workspace',
    '@ocentra-parent/schema-domain',
    '--',
    'vitest',
    'run',
    'tests/proof/child-android-lifecycle-proof.test.ts',
  ]);
  await runNpm(['run', 'release:package:android']);

  const sourceProof = await assertAndroidSourceProof();
  const packageArtifacts = await assertPackageArtifacts();
  const runtimeReadModel = await parseRuntimeReadModel(buildRuntimeReadModel(packageArtifacts));
  const matrixProof = await assertProofMatrix();
  const scriptWiring = await assertScriptWiring();

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'child-android-protocol-package-lifecycle-proof',
    commands,
    proofLabels,
    evidence: {
      sourceProof,
      packageArtifacts,
      contract: 'packages/schema-domain/src/child-android-lifecycle-proof.ts',
      contractTest: 'packages/schema-domain/tests/proof/child-android-lifecycle-proof.test.ts',
      matrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/child-android-protocol-package-lifecycle-proof-2026-05-31.md',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    matrixProof,
    scriptWiring,
    androidCapabilitiesProved: {
      childAgentArtifact: 'ci-mechanical-proof: debug APK is built and checksummed as the Android child-agent artifact',
      androidMode: 'ci-mechanical-proof: chosen Android mode is explicit debug APK sideload, not managed-profile or device-owner',
      foregroundService: 'ci-mechanical-proof: manifest declaration, Java start path, and debug package compile',
      typedProtocolBridge: 'ci-mechanical-proof: native wrapper exposes lifecycle/capability/package proof commands',
      packageLifecycle: 'ci-mechanical-proof for debug APK build and checksum only',
      localStorage: 'scaffold only',
    },
    androidCapabilitiesStillManual: [
      'APK install under debug APK sideload mode',
      'launcher activity runtime launch on emulator or physical device',
      'notification permission grant and delivery',
      'UsageStats permission grant and observations',
      'AccessibilityService grant and behavior',
      'VPN/DNS filtering adapter behavior',
      'device-owner enrollment and policy behavior',
      'managed-profile enrollment and behavior',
      'APK update/background/reboot/uninstall lifecycle on emulator or physical device',
    ],
    nonClaims: [
      'child Android enforcement parity',
      'device-owner behavior',
      'accessibility behavior',
      'VPN or DNS filtering behavior',
      'physical-device or emulator runtime behavior',
      'Google Play signing or store distribution',
      'LAN/WebSocket child-agent transport from the Android package',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-android-protocol-package-lifecycle-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

async function assertAndroidSourceProof() {
  const manifest = await readRepoFile('platforms/android/agent/app/src/main/AndroidManifest.xml');
  const activity = await readRepoFile(
    'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/MainActivity.java'
  );
  const service = await readRepoFile(
    'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/OcentraParentAgentService.java'
  );
  const bridge = await readRepoFile(
    'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidLifecycleProof.java'
  );
  const buildGradle = await readRepoFile('platforms/android/agent/app/build.gradle');
  const releaseScript = await readRepoFile('scripts/release/android/build-agent-package.mjs');

  assertIncludes(manifest, 'android.permission.FOREGROUND_SERVICE', 'foreground service permission');
  assertIncludes(manifest, 'android.permission.FOREGROUND_SERVICE_DATA_SYNC', 'foreground data sync permission');
  assertIncludes(manifest, 'android.permission.POST_NOTIFICATIONS', 'notification permission');
  assertIncludes(manifest, 'android.intent.action.MAIN', 'launcher activity');
  assertIncludes(manifest, 'android:foregroundServiceType="dataSync"', 'foreground service type');
  assertIncludes(activity, 'ChildAndroidLifecycleProof.createStatusBundle()', 'activity bridge status bundle');
  assertIncludes(service, 'ChildAndroidLifecycleProof.createStatusBundle()', 'service bridge status bundle');
  assertIncludes(service, 'startForeground(NOTIFICATION_ID, buildNotification())', 'service foreground start');
  assertIncludes(bridge, 'child.android.lifecycle.snapshot.get', 'lifecycle bridge command');
  assertIncludes(bridge, 'child.android.package.lifecycle.proof.reported', 'package lifecycle event');
  assertIncludes(bridge, 'manualRequiredCapabilities', 'manual capability boundary');
  assertIncludes(buildGradle, "applicationId = 'ca.ocentra.parent.agent'", 'Android application id');
  assertIncludes(buildGradle, 'minSdk = 26', 'Android min SDK');
  assertIncludes(buildGradle, 'targetSdk = 35', 'Android target SDK');
  assertIncludes(releaseScript, 'gradlew.bat assembleDebug', 'Android Gradle package command');
  assertIncludes(releaseScript, 'writeChecksum', 'Android checksum write');

  proofLabels.push('android-wrapper.lifecycle-bridge-source-proof');
  return {
    manifest: 'platforms/android/agent/app/src/main/AndroidManifest.xml',
    activity: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/MainActivity.java',
    service: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/OcentraParentAgentService.java',
    bridge: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidLifecycleProof.java',
    releaseScript: 'scripts/release/android/build-agent-package.mjs',
  };
}

async function assertPackageArtifacts() {
  const workspacePackage = JSON.parse(await readRepoFile('package.json'));
  const apkName = `ocentra-parent-agent-android-debug-v${workspacePackage.version}.apk`;
  const versionedApkPath = join(repoRoot, 'target', 'release-packages', 'android', apkName);
  const latestApkPath = join(
    repoRoot,
    'target',
    'release-packages',
    'android',
    'ocentra-parent-agent-android-debug-latest.apk'
  );

  await assertApkWithChecksum(versionedApkPath);
  await assertApkWithChecksum(latestApkPath);
  proofLabels.push('android-package.debug-apk-and-checksum-proof');

  return {
    versionName: workspacePackage.version,
    versionedApk: relativePath(versionedApkPath),
    latestApk: relativePath(latestApkPath),
    checksumState: 'ci-mechanical-proof',
  };
}

async function assertApkWithChecksum(apkPath) {
  if (!existsSync(apkPath)) {
    throw new Error(`Missing Android APK artifact: ${apkPath}`);
  }
  const checksumPath = `${apkPath}.sha256`;
  if (!existsSync(checksumPath)) {
    throw new Error(`Missing Android APK checksum artifact: ${checksumPath}`);
  }
  const apkBytes = await readFile(apkPath);
  const expected = createHash('sha256').update(apkBytes).digest('hex').toUpperCase();
  const checksum = await readFile(checksumPath, 'utf8');
  assertIncludes(checksum, expected, `${relativePath(checksumPath)} checksum`);
}

function buildRuntimeReadModel(packageArtifacts) {
  return {
    schemaVersion: 'child-android-protocol-package-lifecycle-proof',
    packageProof: {
      packageId: 'ca.ocentra.parent.agent',
      applicationId: 'ca.ocentra.parent.agent',
      launchActivity: 'ca.ocentra.parent.agent/.MainActivity',
      foregroundService: 'ca.ocentra.parent.agent/.OcentraParentAgentService',
      nativeBridgeClass: 'ca.ocentra.parent.agent.ChildAndroidLifecycleProof',
      minSdk: 26,
      targetSdk: 35,
      versionName: packageArtifacts.versionName,
      debugApkPath: packageArtifacts.versionedApk,
      latestApkPath: packageArtifacts.latestApk,
      checksumState: 'ci-mechanical-proof',
      releaseCommand: 'cmd /c npm run release:package:android',
    },
    protocolBridgeProof: {
      bridgeState: 'package-local-scaffold',
      externalTransportState: 'not-implemented',
      commands: [
        'child.android.lifecycle.snapshot.get',
        'child.android.capabilities.snapshot.get',
        'child.android.package.lifecycle.proof.get',
      ],
      events: [
        'child.android.lifecycle.snapshot.reported',
        'child.android.capability.snapshot.reported',
        'child.android.package.lifecycle.proof.reported',
      ],
      nativeBridgeClass: 'ca.ocentra.parent.agent.ChildAndroidLifecycleProof',
      runtimeOwner: 'android-native-wrapper',
      proofRequirement: 'native package bridge constants and status bundle compile into the debug APK',
      claimBoundary: 'package-local lifecycle bridge is not LAN/WebSocket child-agent transport',
    },
    capabilityProofs: capabilityProofs(),
    packageLifecycleAssertions: lifecycleAssertions(),
    permissionProofs: [
      permissionProof('android.permission.FOREGROUND_SERVICE', 'declared-in-manifest', 'not-applicable'),
      permissionProof('android.permission.FOREGROUND_SERVICE_DATA_SYNC', 'declared-in-manifest', 'not-applicable'),
      permissionProof('android.permission.POST_NOTIFICATIONS', 'declared-in-manifest', 'manual-required'),
    ],
    installAuthorityProof: {
      childAgentArtifactState: 'debug-apk-built',
      installMode: 'debug-apk-sideload',
      installState: 'manual-install-proof-required',
      launchState: 'manual-launch-proof-required',
      removalState: 'manual-removal-proof-required',
      deviceOwnerAuthorityState: 'manual-required',
      managedProfileAuthorityState: 'manual-required',
      childAgentArtifactBoundary:
        'debug APK is the Android child-agent artifact proved by CI package output and checksum only',
      installModeBoundary:
        'proof is limited to debug APK sideload mode and does not claim managed-profile or device-owner packaging',
      installStateBoundary: 'Android install remains manual-required until emulator or physical-device proof exists',
      launchStateBoundary: 'Android launcher runtime remains manual-required until emulator or physical-device proof exists',
      removalStateBoundary: 'Android uninstall and removal behavior remain manual-required until device proof exists',
      deviceOwnerBoundary: 'manual-required; no device-owner claim is made without enrollment evidence',
      managedProfileBoundary: 'manual-required; no managed-profile claim is made without enrollment evidence',
    },
    claimBoundaries: {
      childAndroidEnforcementParity: 'not claimed; this proof covers package-local bridge mechanics only',
      foregroundServiceRuntime:
        'foreground service declaration and Java path compile; runtime behavior needs device proof',
      notificationRuntime: 'notification permission is declared; grant and delivery remain manual-required',
      accessibility: 'manual-required; no AccessibilityService behavior is claimed',
      vpnDns: 'manual-required; no VPN or DNS filtering adapter is claimed',
      deviceOwner: 'manual-required; no device-owner enrollment or policy action is claimed',
      managedProfile: 'manual-required; no managed-profile enrollment proof is present',
      usageStats: 'manual-required; no UsageStats permission grant or observation is claimed',
      packageLifecycle: 'debug APK build/checksum is proved; install/update/background/reboot/uninstall remain manual',
      physicalDevice: 'manual-required; no emulator or physical-device run is claimed by CI',
      storeDistribution: 'planned; Google Play signing and release tracks are not wired',
    },
    updatedAt: new Date().toISOString(),
  };
}

async function parseRuntimeReadModel(readModel) {
  const module = await importTsModule('packages/schema-domain/src/child-android-lifecycle-proof.ts');
  const parsed = module.ChildAndroidLifecycleReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.child-android-lifecycle-proof-parse');
  return parsed;
}

async function assertProofMatrix() {
  const matrix = JSON.parse(await readRepoFile('docs/expectations/pre-ai-proof-matrix.json'));
  const claim = matrix.claims.find((candidate) => candidate.id === 'child-android-protocol-package-lifecycle-proof');
  const scenario = matrix.checkpointScenarios.find(
    (candidate) => candidate.id === 'child-android-protocol-package-lifecycle-proof'
  );
  if (!claim || !scenario) {
    throw new Error('Proof matrix is missing child-android-protocol-package-lifecycle-proof claim or scenario.');
  }
  assertArrayIncludes(
    matrix.requiredCompletedClaimIds,
    'child-android-protocol-package-lifecycle-proof',
    'required completed claim'
  );
  assertArrayIncludes(
    claim.ciProof.commands,
    'node scripts/test/child-android-protocol-package-lifecycle-proof.mjs',
    'claim command'
  );
  assertArrayIncludes(
    scenario.ciCommands,
    'node scripts/test/child-android-protocol-package-lifecycle-proof.mjs',
    'scenario command'
  );
  proofLabels.push('proof-matrix.child-android-lifecycle-proof');
  return {
    claimId: claim.id,
    platformCoverage: claim.platformCoverage,
    runtimeSurfaceCoverage: claim.runtimeSurfaceCoverage,
  };
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-android-protocol-package-lifecycle-proof'];
  if (script !== 'node scripts/test/child-android-protocol-package-lifecycle-proof.mjs') {
    throw new Error('Missing root test:child-android-protocol-package-lifecycle-proof script.');
  }
  if (!schemaDomainPackage.exports['./child-android-lifecycle-proof']) {
    throw new Error('Missing schema-domain export for ./child-android-lifecycle-proof.');
  }
  proofLabels.push('package-scripts.child-android-lifecycle-proof');
  return {
    rootScript: 'test:child-android-protocol-package-lifecycle-proof',
    schemaDomainExport: './child-android-lifecycle-proof',
    sourceContract: 'packages/schema-domain/src/child-android-lifecycle-proof.ts',
  };
}

function capabilityProofs() {
  return [
    capabilityProof(
      'foreground-mobile-service',
      'manual-required',
      'ci-mechanical-proof',
      'android-native-wrapper',
      'foreground service is declared and started by package code, but device runtime proof is still required'
    ),
    capabilityProof(
      'notifications',
      'manual-required',
      'manual-required',
      'android-os-permission',
      'notification permission grant and delivery require emulator or physical-device evidence'
    ),
    capabilityProof(
      'local-storage',
      'scaffold',
      'scaffold',
      'android-native-wrapper',
      'local storage remains scaffold until device persistence proof exists'
    ),
    capabilityProof(
      'typed-protocol-bridge',
      'scaffold',
      'ci-mechanical-proof',
      'android-native-wrapper',
      'typed package-local lifecycle bridge compiles but no LAN/WebSocket transport is claimed'
    ),
    capabilityProof(
      'usage-stats',
      'manual-required',
      'manual-required',
      'manual-device-proof',
      'UsageStats needs a real permission grant and observation artifact'
    ),
    capabilityProof(
      'accessibility-service',
      'manual-required',
      'manual-required',
      'manual-device-proof',
      'Accessibility requires explicit service grant and device behavior proof'
    ),
    capabilityProof(
      'vpn-dns-filtering',
      'manual-required',
      'manual-required',
      'manual-device-proof',
      'VPN or DNS filtering needs approved adapter and device proof'
    ),
    capabilityProof(
      'device-owner-policy',
      'manual-required',
      'manual-required',
      'manual-device-proof',
      'device-owner policy requires enrollment and policy action proof'
    ),
    capabilityProof(
      'managed-profile',
      'manual-required',
      'manual-required',
      'manual-device-proof',
      'managed profile behavior requires enrollment proof'
    ),
    capabilityProof(
      'package-lifecycle',
      'manual-required',
      'ci-mechanical-proof',
      'android-package-build',
      'debug APK build and checksum are CI proof, while install/update/background/reboot/uninstall remain manual'
    ),
    capabilityProof(
      'store-distribution',
      'planned',
      'planned',
      'store-distribution',
      'Google Play signing and release tracks are planned, not wired'
    ),
  ];
}

function lifecycleAssertions() {
  return [
    lifecycleAssertion('debug-apk-build', 'ci-mechanical-proof', 'android-package-build'),
    lifecycleAssertion('checksum', 'ci-mechanical-proof', 'android-package-build'),
    lifecycleAssertion('launcher-activity', 'ci-mechanical-proof', 'android-manifest'),
    lifecycleAssertion('foreground-service-registration', 'ci-mechanical-proof', 'android-manifest'),
    lifecycleAssertion('notification-permission-declared', 'ci-mechanical-proof', 'android-manifest'),
    lifecycleAssertion('install', 'manual-required', 'manual-device-proof'),
    lifecycleAssertion('update', 'manual-required', 'manual-device-proof'),
    lifecycleAssertion('background-execution', 'manual-required', 'manual-device-proof'),
    lifecycleAssertion('reboot-recovery', 'manual-required', 'manual-device-proof'),
    lifecycleAssertion('uninstall', 'manual-required', 'manual-device-proof'),
  ];
}

function capabilityProof(capability, parentCapabilityStatus, proofState, runtimeOwner, proofRequirement) {
  return {
    capability,
    parentCapability: capability,
    parentCapabilityStatus,
    proofState,
    runtimeOwner,
    proofRequirement,
    claimBoundary: proofRequirement,
  };
}

function lifecycleAssertion(phase, proofState, runtimeOwner) {
  return {
    phase,
    proofState,
    runtimeOwner,
    proofRequirement: `${phase} proof state remains ${proofState}`,
    claimBoundary: `${phase} does not upgrade unproven Android runtime behavior`,
  };
}

function permissionProof(permission, declarationState, runtimeGrantState) {
  return {
    permission,
    declarationState,
    runtimeGrantState,
    proofRequirement: `${permission} declaration is parsed while runtime grant remains ${runtimeGrantState}`,
  };
}

async function readRepoFile(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

async function runNpm(args) {
  await runCommand(...npmCommand([...args]));
}

async function runCommand(commandName, args) {
  commands.push([commandName, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`${commandName} ${args.join(' ')} exited with ${code}`))
    );
    child.once('error', reject);
  });
}

async function importTsModule(relativePath) {
  return tsImport(pathToFileURL(join(repoRoot, relativePath)).href, import.meta.url);
}

async function gitHead() {
  const chunks = [];
  await new Promise((resolve, reject) => {
    const child = spawn('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error('git rev-parse HEAD failed'))));
    child.once('error', reject);
  });
  return chunks.join('').trim();
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function assertIncludes(value, expected, label) {
  if (!value.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function assertArrayIncludes(values, expected, label) {
  if (!Array.isArray(values) || !values.includes(expected)) {
    throw new Error(`${label}: missing ${expected}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
