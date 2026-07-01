import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SocialAndroidNativeAppCapabilityMatrixSchema } from '../../packages/schema-domain/dist/social-android-native-app-capability-matrix.js';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const proofRoot = join(repoRoot, 'output/browser-plan-proof/social-16-android-native-app-capability-matrix');
const testResultPath = join(repoRoot, 'test-results/social-android-native-app-host-proof/proof.json');
const outputProofPath = join(proofRoot, '11-android-host-device-proof.json');
const observedAt = new Date().toISOString();

const sourceFiles = [
  'packages/schema-domain/src/social-android-native-app-capability-matrix-values.ts',
  'packages/schema-domain/src/social-android-native-app-capability-matrix.ts',
];
const builtFiles = [
  'packages/schema-domain/dist/social-android-native-app-capability-matrix-values.js',
  'packages/schema-domain/dist/social-android-native-app-capability-matrix.js',
];

const targetPackages = [
  { targetId: 'youtube', packageName: 'com.google.android.youtube' },
  { targetId: 'instagram', packageName: 'com.instagram.android' },
  { targetId: 'tiktok', packageName: 'com.zhiliaoapp.musically' },
  { targetId: 'facebook', packageName: 'com.facebook.katana' },
  { targetId: 'reddit', packageName: 'com.reddit.frontpage' },
  { targetId: 'snapchat', packageName: 'com.snapchat.android' },
];

assertBuiltContractsAreFresh();
mkdirSync(proofRoot, { recursive: true });

const adb = findAdb();
const adbVersion = adb ? command(['version'], { allowFailure: true, adbPath: adb.path }) : null;
const devices = adb ? listDevices(adb.path) : [];
const attachedDevices = devices.filter((device) => device.state === 'device');
const packageVisibility = [];

for (const device of attachedDevices) {
  for (const target of targetPackages) {
    packageVisibility.push(queryPackage(adb.path, device.serial, target));
  }
}

const matrix = buildCapabilityMatrix({
  generatedAt: observedAt,
  deviceAvailable: attachedDevices.length > 0,
  packageVisibility,
});
const parsed = SocialAndroidNativeAppCapabilityMatrixSchema.parse(matrix);
const negativeChecks = buildNegativeChecks(parsed);
if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected SOCIAL-16 Android host proof negative checks to reject dishonest native/runtime claims');
}

const proof = {
  schemaVersion: 1,
  proofId: 'social-android-native-app-host-proof',
  generatedAt: observedAt,
  branch: git(['branch', '--show-current']),
  commit: git(['rev-parse', 'HEAD']),
  baseCommit: git(['rev-parse', 'origin/main']),
  hostProofSummary: {
    adbInstalled: adb !== undefined,
    adbPathPersisted: false,
    adbPathSha256: adb ? sha256(adb.path) : null,
    adbVersionSha256: adbVersion ? sha256(adbVersion) : null,
    attachedDeviceCount: attachedDevices.length,
    realDeviceOrEmulatorInspected: attachedDevices.length > 0,
    packageVisibilityQueried: attachedDevices.length > 0,
    knownSocialPackageIdsQueriedOnly: true,
    rawInstalledPackageListPersisted: false,
    screenshotsCaptured: false,
    uiTreeCaptured: false,
    logcatCaptured: false,
    nativeRouteProofClaimed: false,
    perVideoOrReelBlockingClaimed: false,
    messageContentClaimed: false,
    accountIdentityClaimed: false,
    accessibilityContentCaptureClaimed: false,
    deviceOwnerEnrollmentClaimed: false,
    vpnContentInspectionClaimed: false,
    nativeRuntimeAdapterClaimed: false,
    platformConnectorClaimed: false,
    uiDeliveredClaimed: false,
    enforcementClaimed: false,
    resultState: attachedDevices.length > 0 ? 'device-package-visibility-proof' : 'manual-device-proof-required',
  },
  devices: devices.map((device) => ({
    serialRef: `redacted-android-device-ref-${sha256(device.serial).slice(0, 16)}`,
    state: device.state,
    rawSerialPersisted: false,
  })),
  packageVisibility,
  capabilityMatrix: parsed,
  negativeChecks,
};

writeJson(testResultPath, proof);
writeJson(outputProofPath, proof);

console.log('social-android-native-app-host-proof-ok=true');
console.log(`proof=${testResultPath}`);
console.log(`outputProof=${outputProofPath}`);
console.log(`adbInstalled=${adb !== undefined}`);
console.log(`attachedDeviceCount=${attachedDevices.length}`);
console.log(`resultState=${proof.hostProofSummary.resultState}`);

function buildCapabilityMatrix({ generatedAt, deviceAvailable, packageVisibility }) {
  const packageVisibilityProofState =
    deviceAvailable && packageVisibility.length > 0
      ? 'existing-schema-domain-proof-ref'
      : 'manual-device-proof-required';
  const packageVisibilityCapabilityState =
    packageVisibilityProofState === 'existing-schema-domain-proof-ref'
      ? 'app-level-capable-with-proof'
      : 'manual-required';

  return {
    schemaVersion: 'social-android-native-app-capability-matrix',
    generatedAt,
    proofRefs: ['parent-proof-social-android-native-host-device'],
    rows: [
      matrixRow('android-package-visibility', {
        targetKind: 'social-native-app-presence',
        parentCapability: 'package-lifecycle',
        parentCapabilityStatus: 'manual-required',
        capabilityState: packageVisibilityCapabilityState,
        proofState: packageVisibilityProofState,
        policyScope:
          packageVisibilityProofState === 'existing-schema-domain-proof-ref' ? 'app-level-only' : 'manual-review-only',
        reasons: ['package-visibility-limited', 'route-level-unavailable', 'content-proof-unavailable'],
      }),
      matrixRow('android-usage-stats-foreground', {
        targetKind: 'social-native-app-foreground',
        parentCapability: 'usage-stats',
        capabilityState: 'permission-required',
        proofState: 'permission-grant-required',
        policyScope: 'app-level-only',
        reasons: ['usage-access-required', 'route-level-unavailable', 'content-proof-unavailable'],
      }),
      matrixRow('android-accessibility-route-hints', {
        targetKind: 'social-native-app-route-hint',
        parentCapability: 'accessibility-service',
        capabilityState: 'permission-required',
        proofState: 'permission-grant-required',
        policyScope: 'manual-review-only',
        reasons: ['accessibility-explicit-approval-required', 'route-level-unavailable', 'content-proof-unavailable'],
      }),
      matrixRow('android-vpn-domain-hints', {
        targetKind: 'social-native-app-domain-hint',
        parentCapability: 'vpn-dns-filtering',
        capabilityState: 'manual-required',
        proofState: 'adapter-not-implemented',
        policyScope: 'domain-level-only',
        reasons: ['vpn-domain-only', 'route-level-unavailable', 'content-proof-unavailable'],
      }),
      matrixRow('android-device-owner-app-control', {
        targetKind: 'social-native-app-blocking',
        parentCapability: 'device-owner-policy',
        capabilityState: 'manual-required',
        proofState: 'manual-device-proof-required',
        policyScope: 'manual-review-only',
        reasons: ['device-owner-required', 'route-level-unavailable', 'content-proof-unavailable'],
      }),
      matrixRow('android-managed-profile-config', {
        targetKind: 'social-native-app-managed-config',
        parentCapability: 'managed-profile',
        capabilityState: 'manual-required',
        proofState: 'manual-device-proof-required',
        policyScope: 'manual-review-only',
        reasons: ['managed-profile-required', 'route-level-unavailable', 'content-proof-unavailable'],
      }),
    ],
    claimBoundaries: {
      nativeRouteProof: 'not-claimed',
      perVideoOrReelBlocking: 'not-claimed',
      messageContent: 'not-claimed',
      accountIdentity: 'not-claimed',
      accessibilityContentCapture: 'not-claimed',
      deviceOwnerEnrollment: 'not-claimed',
      runtimeAdapter: 'not-claimed',
      enforcement: 'not-claimed',
      reviewerSummary:
        'Android native social app support remains app-level/manual-required until real device proof and permissions.',
    },
  };
}

function matrixRow(surface, overrides) {
  return {
    surface,
    parentCapabilityStatus: 'manual-required',
    proofRefs: [`parent-proof-${surface}`],
    routeLevelProofClaimed: false,
    perVideoOrReelBlockingClaimed: false,
    messageContentClaimed: false,
    accountIdentityClaimed: false,
    accessibilityContentCaptureClaimed: false,
    deviceOwnerEnrollmentClaimed: false,
    vpnContentInspectionClaimed: false,
    nativeRuntimeAdapterClaimed: false,
    platformConnectorClaimed: false,
    uiDeliveredClaimed: false,
    enforcementClaimed: false,
    ...overrides,
  };
}

function buildNegativeChecks(matrix) {
  const claimFields = [
    'routeLevelProofClaimed',
    'perVideoOrReelBlockingClaimed',
    'messageContentClaimed',
    'accountIdentityClaimed',
    'accessibilityContentCaptureClaimed',
    'deviceOwnerEnrollmentClaimed',
    'vpnContentInspectionClaimed',
    'nativeRuntimeAdapterClaimed',
    'platformConnectorClaimed',
    'uiDeliveredClaimed',
    'enforcementClaimed',
  ];
  const rows = [];
  for (const field of claimFields) {
    rows.push({
      mutation: field,
      rejected: !SocialAndroidNativeAppCapabilityMatrixSchema.safeParse({
        ...matrix,
        rows: matrix.rows.map((row) =>
          row.surface === 'android-accessibility-route-hints' ? { ...row, [field]: true } : row
        ),
      }).success,
    });
  }
  rows.push({
    mutation: 'device-owner-upgrade-without-proof',
    rejected: !SocialAndroidNativeAppCapabilityMatrixSchema.safeParse({
      ...matrix,
      rows: matrix.rows.map((row) =>
        row.surface === 'android-device-owner-app-control'
          ? { ...row, capabilityState: 'app-level-capable-with-proof', proofState: 'existing-schema-domain-proof-ref' }
          : row
      ),
    }).success,
  });
  return rows;
}

function findAdb() {
  const output = commandWhere('adb');
  const candidate = output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find((line) => line.length > 0 && existsSync(line));
  return candidate ? { path: candidate } : null;
}

function listDevices(adbPath) {
  const output = command(['devices'], { adbPath, allowFailure: true });
  return output
    .split(/\r?\n/)
    .slice(1)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line) => {
      const [serial, state] = line.split(/\s+/);
      return { serial, state: state ?? 'unknown' };
    });
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
    routeLevelProofClaimed: false,
    contentProofClaimed: false,
    nativeControlClaimed: false,
  };
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

function commandWhere(binary) {
  try {
    return execFileSync('where.exe', [binary], { cwd: repoRoot, encoding: 'utf8' });
  } catch {
    return '';
  }
}

function assertBuiltContractsAreFresh() {
  for (const source of sourceFiles) {
    const built = builtFiles.find((candidate) =>
      candidate.endsWith(source.replace('src/', 'dist/').replace('.ts', '.js').split('/').at(-1))
    );
    if (!built) {
      continue;
    }
    const builtPath = join(repoRoot, built);
    if (!existsSync(builtPath)) {
      throw new Error(`Missing built contract file: ${built}`);
    }
    const sourceText = readFileSync(join(repoRoot, source), 'utf8');
    const builtText = readFileSync(builtPath, 'utf8');
    if (sourceText.includes('export ') && !builtText.includes('export ')) {
      throw new Error(`Built contract file is not importable for ${source}; run npm run build:contracts first`);
    }
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
