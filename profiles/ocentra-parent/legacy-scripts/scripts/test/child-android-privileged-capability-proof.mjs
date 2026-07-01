import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'child-android-privileged-capability-proof';
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
    'tests/proof/child-android-privileged-capability-proof.test.ts',
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
      contract: 'packages/schema-domain/src/child-android-privileged-capability-proof.ts',
      contractTest: 'packages/schema-domain/tests/proof/child-android-privileged-capability-proof.test.ts',
      matrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/child-android-privileged-capability-proof-2026-05-31.md',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    matrixProof,
    scriptWiring,
    androidPrivilegedCapabilityProved: {
      privilegedStatusBundle:
        'package-local-scaffold: native privileged capability labels compile into the Android debug package',
      usageStatsSettings:
        'settings-grant-required: UsageStats requires user settings grant before availability is claimed',
      usageStatsObservation: 'manual-device-proof: observed usage events require emulator or physical-device artifacts',
      accessibilityService: 'not-implemented: no AccessibilityService declaration, grant, or behavior is claimed',
      vpnDnsFiltering: 'not-implemented: no VPN service, DNS adapter, or filtering behavior is claimed',
      deviceOwner: 'blocked: no device-owner enrollment or policy action is claimed',
      managedProfile: 'blocked: no managed-profile enrollment or behavior is claimed',
    },
    androidPrivilegedStillManual: [
      'UsageStats settings grant and observed usage events',
      'AccessibilityService declaration, user grant, and behavior',
      'VPN service declaration, user grant, and DNS filtering behavior',
      'device-owner enrollment and policy action',
      'managed-profile enrollment and behavior',
      'emulator or physical-device runtime behavior',
      'external LAN/WebSocket child-agent privileged transport',
    ],
    nonClaims: [
      'emulator or physical-device behavior',
      'Android child enforcement parity',
      'UsageStats collection',
      'accessibility behavior',
      'VPN or DNS filtering behavior',
      'device-owner or managed-profile control',
      'device enrollment',
      'external LAN/WebSocket child-agent privileged transport',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-android-privileged-capability-proof-ok:${proofLabels.join(',')}`);
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
  const privilegedBridge = await readRepoFile(
    'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidPrivilegedCapabilityProof.java'
  );

  assertNotIncludes(manifest, 'android.permission.PACKAGE_USAGE_STATS', 'UsageStats privileged permission');
  assertNotIncludes(manifest, 'VpnService', 'VPN service declaration');
  assertNotIncludes(manifest, 'android.permission.BIND_VPN_SERVICE', 'VPN service binding');
  assertNotIncludes(manifest, 'DeviceAdminReceiver', 'device owner receiver declaration');
  assertIncludes(
    activity,
    'ChildAndroidPrivilegedCapabilityProof.createPrivilegedCapabilityBundle()',
    'activity privileged proof'
  );
  assertIncludes(
    service,
    'ChildAndroidPrivilegedCapabilityProof.createPrivilegedCapabilityBundle()',
    'service privileged proof'
  );
  assertIncludes(privilegedBridge, 'child.android.privileged.capability.snapshot.get', 'privileged command');
  assertIncludes(privilegedBridge, 'child.android.privileged.enrollment-proof.reported', 'enrollment event');
  assertIncludes(privilegedBridge, 'usage-stats-settings-access=settings-grant-required', 'UsageStats settings label');
  assertIncludes(privilegedBridge, 'usage-stats-observation=manual-device-proof', 'UsageStats observation label');
  assertIncludes(privilegedBridge, 'accessibility-service-adapter=not-implemented', 'accessibility label');
  assertIncludes(privilegedBridge, 'vpn-service-adapter=not-implemented', 'VPN label');
  assertIncludes(privilegedBridge, 'dns-filtering-adapter=not-implemented', 'DNS label');
  assertIncludes(privilegedBridge, 'device-owner-enrollment=blocked', 'device-owner label');
  assertIncludes(privilegedBridge, 'managed-profile-enrollment=blocked', 'managed-profile label');
  assertIncludes(privilegedBridge, 'manualDeviceProofRequired', 'manual device proof labels');
  assertIncludes(privilegedBridge, 'childAgentParityState', 'child agent parity non-claim');
  assertIncludes(privilegedBridge, 'not-claimed', 'child agent parity state');
  proofLabels.push('android-wrapper.privileged-capability-source-proof');

  return {
    manifest: 'platforms/android/agent/app/src/main/AndroidManifest.xml',
    activity: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/MainActivity.java',
    service: 'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/OcentraParentAgentService.java',
    privilegedBridge:
      'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidPrivilegedCapabilityProof.java',
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
  proofLabels.push('android-package.privileged-proof-debug-apk-and-checksum');

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
    nativeBridgeClass: 'ca.ocentra.parent.agent.ChildAndroidPrivilegedCapabilityProof',
    protocolBridgeProof: {
      packageId: 'ca.ocentra.parent.agent',
      nativeBridgeClass: 'ca.ocentra.parent.agent.ChildAndroidPrivilegedCapabilityProof',
      bridgeState: 'package-local-scaffold',
      externalTransportState: 'not-implemented',
      commands: [
        'child.android.privileged.capability.snapshot.get',
        'child.android.privileged.settings-proof.get',
        'child.android.privileged.enrollment-proof.get',
      ],
      events: [
        'child.android.privileged.capability.snapshot.reported',
        'child.android.privileged.settings-proof.reported',
        'child.android.privileged.enrollment-proof.reported',
      ],
      runtimeOwner: 'android-native-wrapper',
      proofRequirement: 'privileged capability status labels compile into the Android debug package',
      claimBoundary: 'privileged proof is package-local and not external child-agent transport',
    },
    privilegedSurfaceProofs: privilegedSurfaceProofs(),
    claimBoundaries: {
      usageStats: 'UsageStats requires user settings grant and observation artifact before support is claimed',
      accessibility: 'AccessibilityService remains not declared and not implemented',
      vpnDns: 'VPN service and DNS filtering remain not declared and not implemented',
      deviceOwner: 'device-owner policy remains blocked without enrollment and policy action proof',
      managedProfile: 'managed profile remains blocked without enrollment proof',
      statusBundle: 'native status Bundle labels are package-local scaffold proof only',
      physicalDevice: 'physical-device install and runtime behavior remain device-proof-required',
      externalTransport: 'no LAN or WebSocket child-agent privileged transport is claimed',
    },
    updatedAt: new Date().toISOString(),
  };
}

async function parseRuntimeReadModel(readModel) {
  const module = await importTsModule('packages/schema-domain/src/child-android-privileged-capability-proof.ts');
  const parsed = module.ChildAndroidPrivilegedCapabilityReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.child-android-privileged-capability-proof-parse');
  return parsed;
}

async function assertProofMatrix() {
  const matrix = JSON.parse(await readRepoFile('docs/expectations/pre-ai-proof-matrix.json'));
  const claim = matrix.claims.find((candidate) => candidate.id === proofMode);
  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === proofMode);
  if (!claim || !scenario) {
    throw new Error('Proof matrix is missing child-android-privileged-capability-proof claim or scenario.');
  }
  assertArrayIncludes(matrix.requiredCompletedClaimIds, proofMode, 'required completed claim');
  assertArrayIncludes(scenario.ciCommands, `node scripts/test/${proofMode}.mjs`, 'scenario command');
  assertArrayIncludes(claim.ciProof.commands, `node scripts/test/${proofMode}.mjs`, 'claim command');
  proofLabels.push('proof-matrix.child-android-privileged-capability-proof');
  return {
    claimId: claim.id,
    platformCoverage: claim.platformCoverage,
    runtimeSurfaceCoverage: claim.runtimeSurfaceCoverage,
  };
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-android-privileged-capability-proof'];
  if (script !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:child-android-privileged-capability-proof script.');
  }
  if (!schemaDomainPackage.exports['./child-android-privileged-capability-proof']) {
    throw new Error('Missing schema-domain export for ./child-android-privileged-capability-proof.');
  }
  proofLabels.push('package-scripts.child-android-privileged-capability-proof');
  return {
    rootScript: 'test:child-android-privileged-capability-proof',
    schemaDomainExport: './child-android-privileged-capability-proof',
    sourceContract: 'packages/schema-domain/src/child-android-privileged-capability-proof.ts',
  };
}

function privilegedSurfaceProofs() {
  return [
    surfaceProof(
      'usage-stats-settings-access',
      'usage-stats',
      'manual-required',
      'not-declared-by-design',
      'manual-settings-required',
      'settings-grant-required',
      'android-settings-panel'
    ),
    surfaceProof(
      'usage-stats-observation',
      'usage-stats',
      'manual-required',
      'not-applicable',
      'manual-device-required',
      'manual-device-proof',
      'android-usage-stats-manager'
    ),
    surfaceProof(
      'accessibility-service-adapter',
      'accessibility-service',
      'not-implemented',
      'not-declared',
      'unavailable',
      'not-implemented',
      'android-accessibility-service'
    ),
    surfaceProof(
      'vpn-service-adapter',
      'vpn-dns-filtering',
      'not-implemented',
      'not-declared',
      'unavailable',
      'not-implemented',
      'android-vpn-service'
    ),
    surfaceProof(
      'dns-filtering-adapter',
      'vpn-dns-filtering',
      'not-implemented',
      'not-declared',
      'not-implemented',
      'not-implemented',
      'android-dns-filtering'
    ),
    surfaceProof(
      'device-owner-enrollment',
      'device-owner-policy',
      'manual-required',
      'not-declared',
      'blocked',
      'blocked',
      'android-device-policy-manager'
    ),
    surfaceProof(
      'managed-profile-enrollment',
      'managed-profile',
      'manual-required',
      'not-declared',
      'blocked',
      'blocked',
      'android-managed-profile-owner'
    ),
    surfaceProof(
      'privileged-status-bundle',
      'typed-protocol-bridge',
      'scaffold',
      'status-bundle-label',
      'not-applicable',
      'package-local-scaffold',
      'android-native-wrapper'
    ),
    surfaceProof(
      'physical-device-proof',
      'package-lifecycle',
      'manual-required',
      'not-applicable',
      'manual-device-required',
      'device-proof-required',
      'manual-device-proof'
    ),
    surfaceProof(
      'external-child-agent-transport',
      'typed-protocol-bridge',
      'not-implemented',
      'not-applicable',
      'not-implemented',
      'not-implemented',
      'external-child-agent-transport'
    ),
  ];
}

function surfaceProof(
  surface,
  parentCapability,
  parentCapabilityStatus,
  declarationState,
  runtimeGrantState,
  proofState,
  runtimeOwner
) {
  const proofRequirement = `${surface} remains ${proofState} until device artifacts change it`;
  return {
    surface,
    parentCapability,
    parentCapabilityStatus,
    declarationState,
    runtimeGrantState,
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
