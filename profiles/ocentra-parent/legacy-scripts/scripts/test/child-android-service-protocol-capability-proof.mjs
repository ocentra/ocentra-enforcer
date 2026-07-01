import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'child-android-service-protocol-capability-proof';
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
    'tests/proof/child-android-service-protocol-proof.test.ts',
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
    proofMode,
    commands,
    proofLabels,
    evidence: {
      sourceProof,
      packageArtifacts,
      contract: 'packages/schema-domain/src/child-android-service-protocol-proof.ts',
      contractTest: 'packages/schema-domain/tests/proof/child-android-service-protocol-proof.test.ts',
      matrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/child-android-service-protocol-capability-proof-2026-05-31.md',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    matrixProof,
    scriptWiring,
    androidServiceProtocolProved: {
      foregroundServiceStatus:
        'ci-mechanical-proof: manifest declaration, Java start path, service status Bundle, and debug package compile',
      storageProtocolBridge:
        'scaffold-only: service bridge references storage protocol bridge without external transport',
      statusExportSurface: 'package-local-bundle: service status is exported only inside the native wrapper package',
      usageStatsLabel: 'permission-required: no permission grant or observation is claimed',
      deviceOwnerLabel: 'blocked: no enrollment or policy proof is claimed',
    },
    androidServiceStillUnimplemented: [
      'emulator or physical-device foreground service runtime behavior',
      'UsageStats permission grant and observation',
      'AccessibilityService declaration, grant, and behavior',
      'VPN/DNS adapter declaration and behavior',
      'device-owner enrollment and policy action',
      'managed-profile enrollment and behavior',
      'external LAN/WebSocket child-agent service transport',
    ],
    nonClaims: [
      'Android child enforcement parity',
      'device-owner behavior',
      'managed-profile behavior',
      'accessibility behavior',
      'VPN or DNS filtering behavior',
      'physical-device or emulator foreground service behavior',
      'remote status export or hosted child activity storage',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-android-service-protocol-capability-proof-ok:${proofLabels.join(',')}`);
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
  const serviceBridge = await readRepoFile(
    'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidServiceProtocolProof.java'
  );

  assertIncludes(manifest, 'android.permission.FOREGROUND_SERVICE', 'foreground service permission');
  assertIncludes(manifest, 'android:foregroundServiceType="dataSync"', 'foreground service type');
  assertNotIncludes(manifest, 'VpnService', 'VPN service declaration');
  assertNotIncludes(manifest, 'DeviceAdminReceiver', 'device owner receiver declaration');
  assertIncludes(activity, 'ChildAndroidServiceProtocolProof.createServiceProtocolBundle()', 'activity service proof');
  assertIncludes(service, 'ChildAndroidServiceProtocolProof.createServiceProtocolBundle()', 'service proof bundle');
  assertIncludes(service, 'startForeground(NOTIFICATION_ID, buildNotification())', 'foreground service start');
  assertIncludes(serviceBridge, 'child.android.service.status.get', 'service status command');
  assertIncludes(serviceBridge, 'child.android.service.protocol.proof.reported', 'service protocol proof event');
  assertIncludes(serviceBridge, 'ChildAndroidStorageProtocolProof.BRIDGE_STATE', 'storage bridge state reference');
  assertIncludes(serviceBridge, 'usage-stats-capability-label=permission-required', 'UsageStats permission label');
  assertIncludes(serviceBridge, 'accessibility-capability-label=unavailable', 'accessibility unavailable label');
  assertIncludes(serviceBridge, 'vpn-dns-capability-label=unavailable', 'VPN/DNS unavailable label');
  assertIncludes(serviceBridge, 'device-owner-capability-label=blocked', 'device owner blocked label');
  assertIncludes(serviceBridge, 'managed-profile-capability-label=blocked', 'managed profile blocked label');
  assertIncludes(serviceBridge, 'statusExportFields', 'status export fields');
  proofLabels.push('android-wrapper.service-protocol-source-proof');

  return {
    manifest: 'platforms/android/agent/app/src/main/AndroidManifest.xml',
    activity: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/MainActivity.java',
    service: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/OcentraParentAgentService.java',
    serviceBridge:
      'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidServiceProtocolProof.java',
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
  proofLabels.push('android-package.service-proof-debug-apk-and-checksum');

  return {
    versionName: workspacePackage.version,
    versionedApk: relativePath(versionedApkPath),
    latestApk: relativePath(latestApkPath),
    checksumState: 'ci-mechanical-proof',
  };
}

function buildRuntimeReadModel(packageArtifacts) {
  return {
    schemaVersion: proofMode,
    foregroundService: {
      packageId: 'ca.ocentra.parent.agent',
      serviceClass: 'ca.ocentra.parent.agent/.OcentraParentAgentService',
      notificationChannelId: 'ocentra_parent_agent',
      notificationId: 4477,
      foregroundServiceType: 'dataSync',
      serviceStatus: 'declared-started-by-package',
      runtimeOwner: 'android-foreground-service',
      proofRequirement: `foreground service source compiles into ${packageArtifacts.latestApk}`,
      claimBoundary: 'foreground service runtime still requires emulator or physical-device evidence',
    },
    protocolBridgeProof: {
      packageId: 'ca.ocentra.parent.agent',
      nativeBridgeClass: 'ca.ocentra.parent.agent.ChildAndroidServiceProtocolProof',
      storageBridgeClass: 'ca.ocentra.parent.agent.ChildAndroidStorageProtocolProof',
      bridgeState: 'package-local-scaffold',
      storageBridgeState: 'package-local-scaffold',
      externalTransportState: 'not-implemented',
      commands: [
        'child.android.service.status.get',
        'child.android.service.capability.labels.get',
        'child.android.service.status.export.get',
        'child.android.service.protocol.proof.get',
      ],
      events: [
        'child.android.service.status.reported',
        'child.android.service.capability.labels.reported',
        'child.android.service.status.export.reported',
        'child.android.service.protocol.proof.reported',
      ],
      runtimeOwner: 'android-native-wrapper',
      proofRequirement: 'service protocol bridge compiles into the Android debug package',
      claimBoundary: 'service protocol bridge is package-local and not LAN/WebSocket child-agent transport',
    },
    statusExportProof: {
      exportState: 'package-local-bundle',
      fields: [
        'schemaVersion',
        'packageId',
        'nativeBridgeClass',
        'foregroundServiceStatus',
        'storageBridgeState',
        'capabilityLabels',
        'commands',
        'events',
      ],
      runtimeOwner: 'status-export-bundle',
      proofRequirement: 'service status fields are exported only as native Bundle metadata',
      claimBoundary: 'status export is not remote transport or hosted child activity storage',
    },
    serviceSurfaces: serviceSurfaces(),
    claimBoundaries: {
      foregroundService: 'foreground service declaration and start path compile; runtime still needs device proof',
      storageProtocolBridge: 'storage protocol bridge is referenced as package-local scaffold only',
      statusExport: 'service status export is a local Bundle surface only',
      usageStats: 'UsageStats requires a user/device permission grant and observation artifact',
      accessibility: 'AccessibilityService is not implemented or declared by this proof',
      vpnDns: 'VPN/DNS filtering adapter is not implemented or declared by this proof',
      deviceOwner: 'device-owner policy remains blocked until enrollment and policy proof exist',
      managedProfile: 'managed-profile behavior remains blocked until enrollment proof exists',
      externalTransport: 'no LAN/WebSocket Android child-agent transport is claimed',
      childAndroidServiceRuntime: 'no emulator or physical-device foreground runtime behavior is claimed',
    },
    updatedAt: new Date().toISOString(),
  };
}

async function parseRuntimeReadModel(readModel) {
  const module = await importTsModule('packages/schema-domain/src/child-android-service-protocol-proof.ts');
  const parsed = module.ChildAndroidServiceProtocolReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.child-android-service-protocol-proof-parse');
  return parsed;
}

async function assertProofMatrix() {
  const matrix = JSON.parse(await readRepoFile('docs/expectations/pre-ai-proof-matrix.json'));
  const claim = matrix.claims.find((candidate) => candidate.id === proofMode);
  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === proofMode);
  if (!claim || !scenario) {
    throw new Error('Proof matrix is missing child-android-service-protocol-capability-proof claim or scenario.');
  }
  assertArrayIncludes(matrix.requiredCompletedClaimIds, proofMode, 'required completed claim');
  assertArrayIncludes(scenario.ciCommands, `node scripts/test/${proofMode}.mjs`, 'scenario command');
  assertArrayIncludes(claim.ciProof.commands, `node scripts/test/${proofMode}.mjs`, 'claim command');
  proofLabels.push('proof-matrix.child-android-service-protocol-proof');
  return {
    claimId: claim.id,
    platformCoverage: claim.platformCoverage,
    runtimeSurfaceCoverage: claim.runtimeSurfaceCoverage,
  };
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-android-service-protocol-capability-proof'];
  if (script !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:child-android-service-protocol-capability-proof script.');
  }
  if (!schemaDomainPackage.exports['./child-android-service-protocol-proof']) {
    throw new Error('Missing schema-domain export for ./child-android-service-protocol-proof.');
  }
  proofLabels.push('package-scripts.child-android-service-protocol-proof');
  return {
    rootScript: 'test:child-android-service-protocol-capability-proof',
    schemaDomainExport: './child-android-service-protocol-proof',
    sourceContract: 'packages/schema-domain/src/child-android-service-protocol-proof.ts',
  };
}

function serviceSurfaces() {
  return [
    serviceSurface(
      'foreground-service-status',
      'foreground-mobile-service',
      'manual-required',
      'scaffold-only',
      'ci-mechanical-proof',
      'android-foreground-service'
    ),
    serviceSurface(
      'storage-protocol-bridge',
      'typed-protocol-bridge',
      'scaffold',
      'scaffold-only',
      'ci-mechanical-proof',
      'agent-protocol'
    ),
    serviceSurface(
      'status-export-surface',
      'typed-protocol-bridge',
      'scaffold',
      'scaffold-only',
      'package-local-scaffold',
      'status-export-bundle'
    ),
    serviceSurface(
      'usage-stats-capability-label',
      'usage-stats',
      'manual-required',
      'permission-required',
      'permission-required',
      'android-os-permission'
    ),
    serviceSurface(
      'accessibility-capability-label',
      'accessibility-service',
      'not-implemented',
      'unavailable',
      'not-implemented',
      'android-accessibility-service'
    ),
    serviceSurface(
      'vpn-dns-capability-label',
      'vpn-dns-filtering',
      'not-implemented',
      'unavailable',
      'not-implemented',
      'android-vpn-service'
    ),
    serviceSurface(
      'device-owner-capability-label',
      'device-owner-policy',
      'manual-required',
      'blocked',
      'blocked',
      'android-policy-provider'
    ),
    serviceSurface(
      'managed-profile-capability-label',
      'managed-profile',
      'manual-required',
      'blocked',
      'blocked',
      'android-policy-provider'
    ),
  ];
}

function serviceSurface(surface, parentCapability, parentCapabilityStatus, capabilityLabel, proofState, runtimeOwner) {
  const proofRequirement = `${surface} remains ${capabilityLabel} until Android device proof changes it`;
  return {
    surface,
    parentCapability,
    parentCapabilityStatus,
    capabilityLabel,
    proofState,
    runtimeOwner,
    proofRequirement,
    claimBoundary: proofRequirement,
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
