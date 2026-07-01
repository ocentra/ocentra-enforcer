import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'child-android-permission-capability-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
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
    'tests/proof/child-android-permission-capability-proof.test.ts',
  ]);
  await runNpm(['run', 'release:package:android']);

  const sourceProof = await assertAndroidSourceProof();
  const packageArtifacts = await assertPackageArtifacts();
  const runtimeReadModel = await parseRuntimeReadModel(buildRuntimeReadModel());
  const matrixProof = await assertProofMatrix();
  const scriptWiring = await assertScriptWiring();

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode,
    commands,
    proofLabels,
    evidence: {
      sourceProof,
      packageArtifacts,
      contract: 'packages/schema-domain/src/child-android-permission-capability-proof.ts',
      contractTest: 'packages/schema-domain/tests/proof/child-android-permission-capability-proof.test.ts',
      matrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/child-android-permission-capability-proof-2026-05-31.md',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    matrixProof,
    scriptWiring,
    androidPermissionPackageProved: {
      packageDebugApk: 'ci-mechanical-proof: debug APK and checksum build through release:package:android',
      foregroundServicePermissions:
        'declared-in-manifest: foreground service permissions are present in AndroidManifest.xml',
      notificationPermission: 'manual-required: POST_NOTIFICATIONS is declared but runtime grant is not claimed',
      usageStatsPermission:
        'settings-grant-required: PACKAGE_USAGE_STATS is not declared by design and needs device settings proof',
      appPrivateStorage:
        'package-local-scaffold: app-private storage remains package-local without persistence behavior claim',
    },
    androidPermissionStillManual: [
      'POST_NOTIFICATIONS grant and notification delivery',
      'UsageStats settings grant and observation artifact',
      'AccessibilityService declaration, grant, and behavior',
      'VPN/DNS service declaration and filtering behavior',
      'device-owner enrollment and policy action',
      'managed-profile enrollment and behavior',
      'install/update/background/reboot/uninstall lifecycle behavior',
    ],
    nonClaims: [
      'emulator or physical-device behavior',
      'Android child enforcement parity',
      'automatic notification permission grant',
      'UsageStats collection',
      'accessibility behavior',
      'VPN or DNS filtering behavior',
      'device-owner or managed-profile control',
      'external LAN/WebSocket child-agent permission transport',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-android-permission-capability-proof-ok:${proofLabels.join(',')}`);
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
  const permissionBridge = await readRepoFile(
    'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidPermissionCapabilityProof.java'
  );

  assertIncludes(manifest, 'android.permission.FOREGROUND_SERVICE', 'foreground service permission');
  assertIncludes(manifest, 'android.permission.FOREGROUND_SERVICE_DATA_SYNC', 'foreground data sync permission');
  assertIncludes(manifest, 'android.permission.POST_NOTIFICATIONS', 'notification permission');
  assertNotIncludes(manifest, 'android.permission.PACKAGE_USAGE_STATS', 'UsageStats privileged permission');
  assertNotIncludes(manifest, 'VpnService', 'VPN service declaration');
  assertNotIncludes(manifest, 'DeviceAdminReceiver', 'device owner receiver declaration');
  assertIncludes(
    activity,
    'ChildAndroidPermissionCapabilityProof.createPermissionCapabilityBundle()',
    'activity permission proof'
  );
  assertIncludes(
    service,
    'ChildAndroidPermissionCapabilityProof.createPermissionCapabilityBundle()',
    'service permission proof'
  );
  assertIncludes(permissionBridge, 'child.android.permission.capability.snapshot.get', 'permission command');
  assertIncludes(permissionBridge, 'child.android.permission.runtime.manual-proof.reported', 'manual proof event');
  assertIncludes(permissionBridge, 'android.permission.PACKAGE_USAGE_STATS', 'UsageStats manual settings label');
  assertIncludes(permissionBridge, 'blocked-without-enrollment', 'enrollment blocked labels');
  assertIncludes(permissionBridge, 'manualPackageLifecyclePhases', 'manual lifecycle phases');
  proofLabels.push('android-wrapper.permission-capability-source-proof');

  return {
    manifest: 'platforms/android/agent/app/src/main/AndroidManifest.xml',
    activity: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/MainActivity.java',
    service: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/OcentraParentAgentService.java',
    permissionBridge:
      'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidPermissionCapabilityProof.java',
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
  proofLabels.push('android-package.permission-proof-debug-apk-and-checksum');

  return {
    versionName: workspacePackage.version,
    versionedApk: relativePath(versionedApkPath),
    latestApk: relativePath(latestApkPath),
    checksumState: 'ci-mechanical-proof',
  };
}

function buildRuntimeReadModel() {
  return {
    schemaVersion: proofMode,
    packageId: 'ca.ocentra.parent.agent',
    nativeBridgeClass: 'ca.ocentra.parent.agent.ChildAndroidPermissionCapabilityProof',
    protocolBridgeProof: {
      packageId: 'ca.ocentra.parent.agent',
      nativeBridgeClass: 'ca.ocentra.parent.agent.ChildAndroidPermissionCapabilityProof',
      bridgeState: 'package-local-scaffold',
      externalTransportState: 'not-implemented',
      commands: [
        'child.android.permission.capability.snapshot.get',
        'child.android.permission.package.proof.get',
        'child.android.permission.runtime.manual-proof.get',
      ],
      events: [
        'child.android.permission.capability.snapshot.reported',
        'child.android.permission.package.proof.reported',
        'child.android.permission.runtime.manual-proof.reported',
      ],
      runtimeOwner: 'android-native-wrapper',
      proofRequirement: 'permission bridge constants compile into the Android debug package',
      claimBoundary: 'permission bridge is package-local and not external child-agent transport',
    },
    permissionProofs: permissionProofs(),
    adapterProofs: adapterProofs(),
    packageLifecycleProofs: packageLifecycleProofs(),
    claimBoundaries: {
      packageLifecycle: 'debug APK build and checksum are proved; install update reboot uninstall remain manual',
      foregroundService:
        'foreground service permissions are declared, but runtime foreground behavior needs device proof',
      notifications: 'POST_NOTIFICATIONS is declared, but runtime grant and delivery are manual-required',
      usageStats: 'UsageStats needs settings grant and observation artifact before it is available',
      accessibility: 'no AccessibilityService declaration, grant, or behavior is claimed',
      vpnDns: 'no VPN service, DNS adapter, or filtering behavior is claimed',
      deviceOwner: 'device-owner remains blocked without enrollment and policy action proof',
      managedProfile: 'managed-profile remains blocked without enrollment proof',
      appPrivateStorage: 'app-private storage path is package-local scaffold only',
      backgroundLifecycle: 'background, reboot, and uninstall behavior require device proof',
      externalTransport: 'no LAN/WebSocket Android child-agent permission transport is claimed',
    },
    updatedAt: new Date().toISOString(),
  };
}

async function parseRuntimeReadModel(readModel) {
  const module = await importTsModule('packages/schema-domain/src/child-android-permission-capability-proof.ts');
  const parsed = module.ChildAndroidPermissionCapabilityReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.child-android-permission-capability-proof-parse');
  return parsed;
}

async function assertProofMatrix() {
  const matrix = JSON.parse(await readRepoFile('docs/expectations/pre-ai-proof-matrix.json'));
  const claim = matrix.claims.find((candidate) => candidate.id === proofMode);
  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === proofMode);
  if (!claim || !scenario) {
    throw new Error('Proof matrix is missing child-android-permission-capability-proof claim or scenario.');
  }
  assertArrayIncludes(matrix.requiredCompletedClaimIds, proofMode, 'required completed claim');
  assertArrayIncludes(scenario.ciCommands, `node scripts/test/${proofMode}.mjs`, 'scenario command');
  assertArrayIncludes(claim.ciProof.commands, `node scripts/test/${proofMode}.mjs`, 'claim command');
  proofLabels.push('proof-matrix.child-android-permission-capability-proof');
  return {
    claimId: claim.id,
    platformCoverage: claim.platformCoverage,
    runtimeSurfaceCoverage: claim.runtimeSurfaceCoverage,
  };
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-android-permission-capability-proof'];
  if (script !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:child-android-permission-capability-proof script.');
  }
  if (!schemaDomainPackage.exports['./child-android-permission-capability-proof']) {
    throw new Error('Missing schema-domain export for ./child-android-permission-capability-proof.');
  }
  proofLabels.push('package-scripts.child-android-permission-capability-proof');
  return {
    rootScript: 'test:child-android-permission-capability-proof',
    schemaDomainExport: './child-android-permission-capability-proof',
    sourceContract: 'packages/schema-domain/src/child-android-permission-capability-proof.ts',
  };
}

function permissionProofs() {
  return [
    permissionProof(
      'android.permission.FOREGROUND_SERVICE',
      'foreground-mobile-service',
      'declared-in-manifest',
      'not-applicable',
      'declared-in-manifest',
      'android-manifest'
    ),
    permissionProof(
      'android.permission.FOREGROUND_SERVICE_DATA_SYNC',
      'foreground-mobile-service',
      'declared-in-manifest',
      'not-applicable',
      'declared-in-manifest',
      'android-manifest'
    ),
    permissionProof(
      'android.permission.POST_NOTIFICATIONS',
      'notifications',
      'declared-in-manifest',
      'manual-runtime-required',
      'manual-required',
      'android-os-permission'
    ),
    permissionProof(
      'android.permission.PACKAGE_USAGE_STATS',
      'usage-stats',
      'not-declared-by-design',
      'manual-settings-required',
      'settings-grant-required',
      'manual-device-proof'
    ),
  ];
}

function adapterProofs() {
  return [
    adapterProof(
      'package-debug-apk',
      'package-lifecycle',
      'manual-required',
      'package-local-scaffold',
      'ci-mechanical-proof',
      'android-package-build'
    ),
    adapterProof(
      'foreground-service-permission',
      'foreground-mobile-service',
      'manual-required',
      'package-local-scaffold',
      'ci-mechanical-proof',
      'android-manifest'
    ),
    adapterProof(
      'post-notifications-permission',
      'notifications',
      'manual-required',
      'declared-in-manifest',
      'manual-required',
      'android-os-permission'
    ),
    adapterProof(
      'usage-stats-permission',
      'usage-stats',
      'manual-required',
      'not-declared',
      'settings-grant-required',
      'manual-device-proof'
    ),
    adapterProof(
      'accessibility-service',
      'accessibility-service',
      'not-implemented',
      'not-declared',
      'not-implemented',
      'android-accessibility-service'
    ),
    adapterProof(
      'vpn-dns-service',
      'vpn-dns-filtering',
      'not-implemented',
      'not-declared',
      'not-implemented',
      'android-vpn-service'
    ),
    adapterProof(
      'device-owner-policy',
      'device-owner-policy',
      'manual-required',
      'blocked-without-enrollment',
      'blocked',
      'android-policy-provider'
    ),
    adapterProof(
      'managed-profile',
      'managed-profile',
      'manual-required',
      'blocked-without-enrollment',
      'blocked',
      'android-policy-provider'
    ),
    adapterProof(
      'app-private-storage',
      'local-storage',
      'scaffold',
      'package-local-scaffold',
      'package-local-scaffold',
      'android-app-private-storage'
    ),
    adapterProof(
      'background-service-lifecycle',
      'background-execution',
      'manual-required',
      'not-implemented',
      'manual-required',
      'manual-device-proof'
    ),
  ];
}

function packageLifecycleProofs() {
  return [
    lifecycleProof('debug-apk-build', 'ci-mechanical-proof', 'android-package-build'),
    lifecycleProof('checksum', 'ci-mechanical-proof', 'android-package-build'),
    lifecycleProof('launcher-activity', 'ci-mechanical-proof', 'android-manifest'),
    lifecycleProof('foreground-service-registration', 'ci-mechanical-proof', 'android-manifest'),
    lifecycleProof('notification-permission-declared', 'ci-mechanical-proof', 'android-manifest'),
    lifecycleProof('app-private-storage-path', 'ci-mechanical-proof', 'android-app-private-storage'),
    lifecycleProof('background-service-start', 'manual-required', 'manual-device-proof'),
    lifecycleProof('install', 'manual-required', 'manual-device-proof'),
    lifecycleProof('update', 'manual-required', 'manual-device-proof'),
    lifecycleProof('reboot-recovery', 'manual-required', 'manual-device-proof'),
    lifecycleProof('uninstall', 'manual-required', 'manual-device-proof'),
  ];
}

function permissionProof(permission, parentCapability, declarationState, runtimeGrantState, proofState, runtimeOwner) {
  const proofRequirement = `${permission} remains ${runtimeGrantState} until device proof changes it`;
  return {
    permission,
    parentCapability,
    declarationState,
    runtimeGrantState,
    proofState,
    runtimeOwner,
    proofRequirement,
    claimBoundary: proofRequirement,
  };
}

function adapterProof(surface, parentCapability, parentCapabilityStatus, adapterState, proofState, runtimeOwner) {
  const proofRequirement = `${surface} remains ${adapterState} with ${proofState}`;
  return {
    surface,
    parentCapability,
    parentCapabilityStatus,
    adapterState,
    proofState,
    runtimeOwner,
    proofRequirement,
    claimBoundary: proofRequirement,
  };
}

function lifecycleProof(phase, proofState, runtimeOwner) {
  return {
    phase,
    proofState,
    runtimeOwner,
    proofRequirement: `${phase} proof state is ${proofState}`,
    claimBoundary: `${phase} does not upgrade Android runtime behavior without device evidence`,
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

function assertNotIncludes(value, expected, label) {
  if (value.includes(expected)) {
    throw new Error(`${label}: unexpectedly contains ${expected}`);
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
