import { createHash } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'child-android-device-proof-artifact-gate';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const commands = [];
const proofLabels = [];
const sourceProofModes = [
  'child-android-protocol-package-lifecycle-proof',
  'child-android-storage-protocol-capability-proof',
  'child-android-service-protocol-capability-proof',
  'child-android-permission-capability-proof',
  'child-android-privileged-capability-proof',
];

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
    'tests/proof/child-android-device-proof-artifact-gate.test.ts',
  ]);

  for (const sourceMode of sourceProofModes) {
    await runNpm(['run', `test:${sourceMode}`]);
  }

  const sourceProofs = await assertSourceProofOutputs();
  const packageArtifacts = await assertPackageArtifacts();
  const runtimeReadModel = await parseRuntimeReadModel(buildRuntimeReadModel(sourceProofs, packageArtifacts));
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
      sourceProofs,
      packageArtifacts,
      contract: 'packages/schema-domain/src/child-android-device-proof-artifact-gate.ts',
      contractTest: 'packages/schema-domain/tests/proof/child-android-device-proof-artifact-gate.test.ts',
      matrix: 'docs/expectations/pre-ai-proof-matrix.json',
      checkpoint: 'docs/checkpoints/child-android-device-proof-artifact-gate-2026-06-01.md',
      output: relativePath(proofPath),
    },
    runtimeReadModel,
    matrixProof,
    scriptWiring,
    androidDeviceProofGateProved: {
      sourceComposition: 'ci-mechanical-proof: five existing child Android proof outputs were generated and parsed',
      packageArtifacts: 'ci-mechanical-proof: debug APK and SHA-256 checksum are present as the Android child-agent artifact',
      androidMode: 'ci-mechanical-proof: chosen Android mode is explicit debug APK sideload only',
      statusBundles: 'package-local-scaffold: status bundle source artifacts exist but remain package-local only',
      addDevicePairingReadiness:
        'manual-required: parent-visible add-device/pairing entry exposes package, service, storage, protocol, permission, and privileged inputs',
    },
    androidDeviceProofStillManual: [
      'Parent add-device/pairing readiness entry before emulator or physical-device artifacts',
      'APK install, launcher runtime, and runtime service behavior on emulator or physical device',
      'POST_NOTIFICATIONS runtime grant and notification delivery',
      'UsageStats settings grant and observed usage events',
      'AccessibilityService declaration, grant, and behavior',
      'VPN service, DNS adapter, and filtering behavior',
      'Android uninstall and removal behavior on emulator or physical device',
      'device-owner enrollment and policy action',
      'managed-profile enrollment and behavior',
      'Play Store signing or release-track proof',
      'external LAN/WebSocket child-agent runtime transport',
    ],
    nonClaims: [
      'Android add-device/pairing readiness',
      'Android child device readiness',
      'Android child enforcement parity',
      'emulator or physical-device behavior',
      'real privileged permission behavior',
      'device-owner or managed-profile control',
      'Play Store signing or distribution',
      'external child-agent transport',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`child-android-device-proof-artifact-gate-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

async function assertSourceProofOutputs() {
  const proofs = [];
  for (const sourceMode of sourceProofModes) {
    const outputPath = join(repoRoot, 'test-results', sourceMode, 'proof.json');
    const proof = JSON.parse(await readFile(outputPath, 'utf8'));
    if (proof.proofMode !== sourceMode) {
      throw new Error(`${sourceMode}: proof output has wrong proofMode.`);
    }
    if (!Array.isArray(proof.proofLabels) || proof.proofLabels.length === 0) {
      throw new Error(`${sourceMode}: proof output is missing proof labels.`);
    }
    proofs.push({
      source: sourceMode,
      outputPath: relativePath(outputPath),
      command: `node scripts/test/${sourceMode}.mjs`,
      sourceStatus: 'ci-mechanical-proof',
    });
  }
  proofLabels.push('child-android-device-proof.source-proof-outputs');
  return proofs;
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
  proofLabels.push('android-package.device-proof-gate-debug-apk-and-checksum');

  return {
    versionName: workspacePackage.version,
    versionedApk: relativePath(versionedApkPath),
    latestApk: relativePath(latestApkPath),
    latestChecksum: relativePath(`${latestApkPath}.sha256`),
  };
}

function buildRuntimeReadModel(sourceProofs, packageArtifacts) {
  const checkedAt = new Date().toISOString();
  return {
    schemaVersion: proofMode,
    checkedAt,
    readinessDecision: 'manual-device-evidence-required-before-child-android-readiness',
    packageMechanicalProofState: 'ci-package-only',
    installAuthorityState: {
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
    addDevicePairingReadiness: {
      surface: 'parent-add-device-pairing',
      readinessState: 'manual-required',
      inputs: addDevicePairingInputs(),
      parentVisibleSummary:
        'Android add-device/pairing readiness remains manual-required until emulator or physical-device artifacts exist',
    },
    childAndroidDeviceReadinessState: 'manual-required',
    sourceProofs,
    artifactRequirements: artifactRequirements(checkedAt, packageArtifacts),
    manualEvidenceStatus: {
      custodyState: 'ci-artifacts-only',
      requiredArtifactCount: 17,
      ciArtifactCount: 3,
      collectedDeviceArtifactCount: 0,
      missingDeviceArtifactCount: 14,
      reviewerSummary: 'CI artifacts exist, but real Android device proof artifacts are not collected',
    },
    claimBoundaries: {
      addDevicePairingReadiness:
        'Parent-visible add-device/pairing readiness remains manual-required and is not remote-control proof',
      childAndroidDeviceReadiness:
        'Android child device readiness remains manual-required until emulator or physical-device artifacts exist',
      emulatorRuntime: 'no emulator install, runtime grant, foreground service, or UsageStats observation is claimed',
      physicalDeviceRuntime: 'no physical-device run, enrollment, managed profile, or privileged behavior is claimed',
      privilegedPermissions: 'UsageStats, Accessibility, VPN, and DNS remain manual-required or unavailable',
      deviceOwnerManagedProfile: 'device-owner and managed-profile states remain blocked without enrollment artifacts',
      playStoreSigning: 'Play Store signing and release-track proof remain planned and not collected',
      externalChildAgentTransport: 'no external LAN/WebSocket Android child-agent transport is claimed',
    },
    claimsProved: ['debug APK, checksum, and package-local status bundles are CI/package proof only'],
    claimsNotProved: [
      'Android add-device/pairing readiness remains manual-required without emulator or physical-device artifacts',
      'Android child device readiness remains manual-required without emulator or physical-device artifacts',
      'Android child enforcement parity is not proved by package-local proof outputs',
      'Android install, launch, removal, UsageStats grant, Accessibility, VPN/DNS, device-owner, managed-profile, signing, and external transport remain unproved',
    ],
  };
}

function addDevicePairingInputs() {
  return [
    addDevicePairingInput('package', 'child-android-protocol-package-lifecycle-proof', 'scaffold'),
    addDevicePairingInput('service', 'child-android-service-protocol-capability-proof', 'manual-required'),
    addDevicePairingInput('storage', 'child-android-storage-protocol-capability-proof', 'scaffold'),
    addDevicePairingInput('protocol', 'child-android-storage-protocol-capability-proof', 'scaffold'),
    addDevicePairingInput('permission', 'child-android-permission-capability-proof', 'manual-required'),
    addDevicePairingInput('privileged', 'child-android-privileged-capability-proof', 'not-implemented'),
  ];
}

function addDevicePairingInput(input, source, readinessState) {
  return {
    input,
    source,
    readinessState,
    parentVisibleSummary: `${input} add-device input remains ${readinessState}`,
  };
}

async function parseRuntimeReadModel(readModel) {
  const module = await importTsModule('packages/schema-domain/src/child-android-device-proof-artifact-gate.ts');
  const parsed = module.ChildAndroidDeviceProofArtifactGateReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.child-android-device-proof-artifact-gate-parse');
  return parsed;
}

async function assertProofMatrix() {
  const matrix = JSON.parse(await readRepoFile('docs/expectations/pre-ai-proof-matrix.json'));
  const claim = matrix.claims.find((candidate) => candidate.id === proofMode);
  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === proofMode);
  if (!claim || !scenario) {
    throw new Error('Proof matrix is missing child-android-device-proof-artifact-gate claim or scenario.');
  }
  assertArrayIncludes(matrix.requiredCompletedClaimIds, proofMode, 'required completed claim');
  assertArrayIncludes(scenario.ciCommands, `node scripts/test/${proofMode}.mjs`, 'scenario command');
  assertArrayIncludes(claim.ciProof.commands, `node scripts/test/${proofMode}.mjs`, 'claim command');
  proofLabels.push('proof-matrix.child-android-device-proof-artifact-gate');
  return {
    claimId: claim.id,
    platformCoverage: claim.platformCoverage,
    runtimeSurfaceCoverage: claim.runtimeSurfaceCoverage,
  };
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const script = packageJson.scripts['test:child-android-device-proof-artifact-gate'];
  if (script !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:child-android-device-proof-artifact-gate script.');
  }
  if (!schemaDomainPackage.exports['./child-android-device-proof-artifact-gate']) {
    throw new Error('Missing schema-domain export for ./child-android-device-proof-artifact-gate.');
  }
  proofLabels.push('package-scripts.child-android-device-proof-artifact-gate');
  return {
    rootScript: 'test:child-android-device-proof-artifact-gate',
    schemaDomainExport: './child-android-device-proof-artifact-gate',
    sourceContract: 'packages/schema-domain/src/child-android-device-proof-artifact-gate.ts',
  };
}

function artifactRequirements(checkedAt, packageArtifacts) {
  return [
    artifactRequirement(
      'debug-apk-build',
      'package-lifecycle',
      'manual-required',
      'ci-package-artifact',
      'ci-mechanical-proof',
      packageArtifacts.latestApk,
      checkedAt,
      'child-android-protocol-package-lifecycle-proof'
    ),
    artifactRequirement(
      'apk-sha256-checksum',
      'package-lifecycle',
      'manual-required',
      'ci-package-artifact',
      'ci-mechanical-proof',
      packageArtifacts.latestChecksum,
      checkedAt,
      'child-android-protocol-package-lifecycle-proof'
    ),
    artifactRequirement(
      'package-local-status-bundles',
      'typed-protocol-bridge',
      'scaffold',
      'package-local-status',
      'package-local-scaffold',
      'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/ChildAndroidPrivilegedCapabilityProof.java',
      checkedAt,
      'child-android-privileged-capability-proof'
    ),
    ...manualArtifactRequirements(),
  ];
}

function manualArtifactRequirements() {
  return [
    artifactRequirement(
      'real-device-install-artifact',
      'package-lifecycle',
      'manual-required',
      'emulator-device-artifact',
      'device-proof-required',
      null,
      null,
      'child-android-protocol-package-lifecycle-proof'
    ),
    artifactRequirement(
      'launch-activity-runtime-artifact',
      'package-lifecycle',
      'manual-required',
      'emulator-device-artifact',
      'device-proof-required',
      null,
      null,
      'child-android-protocol-package-lifecycle-proof'
    ),
    artifactRequirement(
      'foreground-service-runtime-artifact',
      'foreground-mobile-service',
      'manual-required',
      'emulator-device-artifact',
      'device-proof-required',
      null,
      null,
      'child-android-service-protocol-capability-proof'
    ),
    artifactRequirement(
      'notification-runtime-grant-artifact',
      'notifications',
      'manual-required',
      'permission-grant-artifact',
      'manual-required',
      null,
      null,
      'child-android-permission-capability-proof'
    ),
    artifactRequirement(
      'usage-stats-settings-grant-artifact',
      'usage-stats',
      'manual-required',
      'permission-grant-artifact',
      'settings-grant-required',
      null,
      null,
      'child-android-privileged-capability-proof'
    ),
    artifactRequirement(
      'usage-stats-observation-artifact',
      'usage-stats',
      'manual-required',
      'emulator-device-artifact',
      'device-proof-required',
      null,
      null,
      'child-android-privileged-capability-proof'
    ),
    artifactRequirement(
      'accessibility-service-grant-artifact',
      'accessibility-service',
      'not-implemented',
      'privileged-adapter-artifact',
      'not-implemented',
      null,
      null,
      'child-android-privileged-capability-proof'
    ),
    artifactRequirement(
      'vpn-service-grant-artifact',
      'vpn-dns-filtering',
      'not-implemented',
      'privileged-adapter-artifact',
      'not-implemented',
      null,
      null,
      'child-android-privileged-capability-proof'
    ),
    artifactRequirement(
      'dns-filtering-behavior-artifact',
      'vpn-dns-filtering',
      'not-implemented',
      'privileged-adapter-artifact',
      'not-implemented',
      null,
      null,
      'child-android-privileged-capability-proof'
    ),
    artifactRequirement(
      'package-removal-artifact',
      'package-lifecycle',
      'manual-required',
      'emulator-device-artifact',
      'device-proof-required',
      null,
      null,
      'child-android-protocol-package-lifecycle-proof'
    ),
    artifactRequirement(
      'device-owner-enrollment-artifact',
      'device-owner-policy',
      'manual-required',
      'enrollment-artifact',
      'blocked',
      null,
      null,
      'child-android-privileged-capability-proof'
    ),
    artifactRequirement(
      'managed-profile-enrollment-artifact',
      'managed-profile',
      'manual-required',
      'enrollment-artifact',
      'blocked',
      null,
      null,
      'child-android-privileged-capability-proof'
    ),
    artifactRequirement(
      'play-store-signing-artifact',
      'store-distribution',
      'planned',
      'store-signing-artifact',
      'planned',
      null,
      null,
      'child-android-protocol-package-lifecycle-proof'
    ),
    artifactRequirement(
      'external-child-agent-transport-artifact',
      'typed-protocol-bridge',
      'not-implemented',
      'external-transport-artifact',
      'not-implemented',
      null,
      null,
      'child-android-privileged-capability-proof'
    ),
  ];
}

function artifactRequirement(
  requirement,
  parentCapability,
  parentCapabilityStatus,
  artifactClass,
  artifactStatus,
  evidencePath,
  evidenceCapturedAt,
  source
) {
  return {
    requirement,
    parentCapability,
    parentCapabilityStatus,
    artifactClass,
    artifactStatus,
    requiredArtifactSummary: `${requirement} remains ${artifactStatus}`,
    evidencePath,
    evidenceCapturedAt,
    source,
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
