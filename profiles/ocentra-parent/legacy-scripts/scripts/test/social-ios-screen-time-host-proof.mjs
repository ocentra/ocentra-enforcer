import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SocialIosScreenTimeCapabilityMatrixSchema } from '../../packages/schema-domain/dist/social-ios-screen-time-capability-matrix.js';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const proofRoot = join(repoRoot, 'output/browser-plan-proof/social-17-ios-screentime-managedsettings-matrix');
const testResultPath = join(repoRoot, 'test-results/social-ios-screen-time-host-proof/proof.json');
const outputProofPath = join(proofRoot, '11-ios-host-tooling-proof.json');
const observedAt = new Date().toISOString();

const sourceFiles = [
  'packages/schema-domain/src/social-ios-screen-time-capability-matrix-values.ts',
  'packages/schema-domain/src/social-ios-screen-time-capability-matrix.ts',
];
const builtFiles = [
  'packages/schema-domain/dist/social-ios-screen-time-capability-matrix-values.js',
  'packages/schema-domain/dist/social-ios-screen-time-capability-matrix.js',
];

const iosToolBinaries = ['xcrun', 'xcodebuild', 'idevice_id', 'ios-deploy'];

assertBuiltContractsAreFresh();
mkdirSync(proofRoot, { recursive: true });

const host = {
  platform: process.platform,
  isDarwinHost: process.platform === 'darwin',
  rawEnvironmentPersisted: false,
};
const tools = iosToolBinaries.map((binary) => findTool(binary));
const xcrunVersion = commandIfAvailable('xcrun', ['--version']);
const xcodebuildVersion = commandIfAvailable('xcodebuild', ['-version']);
const ideviceList = commandIfAvailable('idevice_id', ['-l']);
const attachedDeviceRefs = parseIdeviceList(ideviceList.output);

const matrix = SocialIosScreenTimeCapabilityMatrixSchema.parse(buildCapabilityMatrix(observedAt));
const negativeChecks = buildNegativeChecks(matrix);
if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected SOCIAL-17 iOS host proof negative checks to reject dishonest runtime claims');
}

const appleToolingAvailable = tools.some((tool) => tool.found);
const resultState =
  host.isDarwinHost && appleToolingAvailable && attachedDeviceRefs.length > 0
    ? 'manual-device-proof-required'
    : 'host-tooling-unavailable';

const proof = {
  schemaVersion: 1,
  proofId: 'social-ios-screen-time-host-proof',
  generatedAt: observedAt,
  branch: git(['branch', '--show-current']),
  commit: git(['rev-parse', 'HEAD']),
  baseCommit: git(['rev-parse', 'origin/main']),
  hostProofSummary: {
    host,
    appleToolingAvailable,
    xcrunAvailable: toolFound('xcrun'),
    xcodebuildAvailable: toolFound('xcodebuild'),
    libimobiledeviceAvailable: toolFound('idevice_id'),
    iosDeployAvailable: toolFound('ios-deploy'),
    xcrunVersionSha256: xcrunVersion.available ? sha256(xcrunVersion.output) : null,
    xcodebuildVersionSha256: xcodebuildVersion.available ? sha256(xcodebuildVersion.output) : null,
    attachedDeviceCount: attachedDeviceRefs.length,
    realDeviceInspected: false,
    simulatorInspected: false,
    appleEntitlementProofPresent: false,
    familyControlsAuthorizationClaimed: false,
    tokenSelectionClaimed: false,
    deviceActivityRuntimeClaimed: false,
    managedSettingsRuntimeClaimed: false,
    rawApplicationIdentityClaimed: false,
    nativeRouteProofClaimed: false,
    perVideoOrReelBlockingClaimed: false,
    messageContentClaimed: false,
    accountIdentityClaimed: false,
    screenContentCaptureClaimed: false,
    platformConnectorClaimed: false,
    uiDeliveredClaimed: false,
    enforcementClaimed: false,
    resultState,
  },
  tools,
  attachedDeviceRefs,
  capabilityMatrix: matrix,
  negativeChecks,
};

writeJson(testResultPath, proof);
writeJson(outputProofPath, proof);

console.log('social-ios-screen-time-host-proof-ok=true');
console.log(`proof=${testResultPath}`);
console.log(`outputProof=${outputProofPath}`);
console.log(`isDarwinHost=${host.isDarwinHost}`);
console.log(`appleToolingAvailable=${appleToolingAvailable}`);
console.log(`attachedDeviceCount=${attachedDeviceRefs.length}`);
console.log(`resultState=${resultState}`);

function buildCapabilityMatrix(generatedAt) {
  return {
    schemaVersion: 'social-ios-screen-time-capability-matrix',
    generatedAt,
    proofRefs: ['parent-proof-social-ios-screentime-host-tooling'],
    rows: [...familyControlsRows(), ...managedSettingsRows()],
    claimBoundaries: {
      familyControlsAuthorization: 'not-claimed',
      rawApplicationIdentity: 'not-claimed',
      nativeRouteProof: 'not-claimed',
      perVideoOrReelBlocking: 'not-claimed',
      messageContent: 'not-claimed',
      accountIdentity: 'not-claimed',
      screenContentCapture: 'not-claimed',
      runtimeAdapter: 'not-claimed',
      connectorAuthorization: 'not-claimed',
      uiDelivery: 'not-claimed',
      enforcement: 'not-claimed',
      reviewerSummary:
        'iOS social support remains token and shield capability mapping only until Apple entitlement and device proof.',
    },
  };
}

function familyControlsRows() {
  return [
    matrixRow('ios-family-controls-authorization', {
      targetKind: 'social-ios-family-authorization',
      parentCapability: 'family-controls-entitlement',
      capabilityState: 'entitlement-required',
      proofState: 'apple-entitlement-required',
      policyScope: 'manual-review-only',
      reasons: ['family-controls-entitlement-required', 'family-authorization-required'],
    }),
    matrixRow('ios-application-token-selection', {
      targetKind: 'social-ios-app-token',
      parentCapability: 'family-controls-entitlement',
      capabilityState: 'token-selection-required',
      proofState: 'family-authorization-required',
      policyScope: 'app-token-level',
      reasons: [
        'family-authorization-required',
        'opaque-token-required',
        'raw-app-identity-unavailable',
        'route-level-unavailable',
        'content-proof-unavailable',
      ],
    }),
    matrixRow('ios-web-domain-token-selection', {
      targetKind: 'social-ios-web-domain-token',
      parentCapability: 'family-controls-entitlement',
      capabilityState: 'token-selection-required',
      proofState: 'family-authorization-required',
      policyScope: 'web-domain-token-level',
      reasons: [
        'family-authorization-required',
        'opaque-token-required',
        'web-domain-token-limited',
        'route-level-unavailable',
        'content-proof-unavailable',
      ],
    }),
  ];
}

function managedSettingsRows() {
  return [
    matrixRow('ios-device-activity-monitor', {
      targetKind: 'social-ios-device-activity',
      parentCapability: 'device-activity',
      capabilityState: 'manual-device-proof-required',
      proofState: 'apple-entitlement-required',
      policyScope: 'category-token-level',
      reasons: ['device-activity-entitlement-required', 'route-level-unavailable', 'content-proof-unavailable'],
    }),
    matrixRow('ios-managed-settings-application-shield', {
      targetKind: 'social-ios-application-shield',
      parentCapability: 'screen-time-api',
      capabilityState: 'manual-device-proof-required',
      proofState: 'apple-entitlement-required',
      policyScope: 'app-token-level',
      reasons: [
        'managed-settings-entitlement-required',
        'shield-state-device-proof-required',
        'route-level-unavailable',
        'content-proof-unavailable',
      ],
    }),
    matrixRow('ios-managed-settings-web-domain-shield', {
      targetKind: 'social-ios-web-domain-shield',
      parentCapability: 'screen-time-api',
      capabilityState: 'manual-device-proof-required',
      proofState: 'apple-entitlement-required',
      policyScope: 'web-domain-token-level',
      reasons: [
        'managed-settings-entitlement-required',
        'shield-state-device-proof-required',
        'web-domain-token-limited',
        'route-level-unavailable',
        'content-proof-unavailable',
      ],
    }),
  ];
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
    rawApplicationIdentityClaimed: false,
    screenContentCaptureClaimed: false,
    deviceActivityRuntimeClaimed: false,
    managedSettingsRuntimeClaimed: false,
    entitlementApprovalClaimed: false,
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
    'rawApplicationIdentityClaimed',
    'screenContentCaptureClaimed',
    'deviceActivityRuntimeClaimed',
    'managedSettingsRuntimeClaimed',
    'entitlementApprovalClaimed',
    'platformConnectorClaimed',
    'uiDeliveredClaimed',
    'enforcementClaimed',
  ];
  const rows = claimFields.map((field) => ({
    mutation: field,
    rejected: !SocialIosScreenTimeCapabilityMatrixSchema.safeParse({
      ...matrix,
      rows: matrix.rows.map((row) =>
        row.surface === 'ios-managed-settings-application-shield' ? { ...row, [field]: true } : row
      ),
    }).success,
  }));
  rows.push({
    mutation: 'entitlement-upgrade-without-apple-proof',
    rejected: !SocialIosScreenTimeCapabilityMatrixSchema.safeParse({
      ...matrix,
      rows: matrix.rows.map((row) =>
        row.surface === 'ios-family-controls-authorization'
          ? {
              ...row,
              capabilityState: 'authorization-required',
              proofState: 'existing-ios-entitlement-proof-ref',
            }
          : row
      ),
    }).success,
  });
  return rows;
}

function findTool(binary) {
  const output = commandWhere(binary);
  const candidate = output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find((line) => line.length > 0 && existsSync(line));
  return {
    binary,
    found: candidate !== undefined,
    pathPersisted: false,
    pathSha256: candidate ? sha256(candidate) : null,
  };
}

function toolFound(binary) {
  return tools.some((tool) => tool.binary === binary && tool.found);
}

function commandIfAvailable(binary, args) {
  if (!toolFound(binary)) {
    return { available: false, output: '' };
  }
  try {
    return {
      available: true,
      output: execFileSync(binary, args, { cwd: repoRoot, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }),
    };
  } catch (error) {
    return {
      available: true,
      output: `${error.stdout?.toString() ?? ''}${error.stderr?.toString() ?? ''}`,
    };
  }
}

function commandWhere(binary) {
  try {
    return execFileSync('where.exe', [binary], {
      cwd: repoRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  } catch {
    return '';
  }
}

function parseIdeviceList(output) {
  return output
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((serial) => ({
      serialRef: `redacted-ios-device-ref-${sha256(serial).slice(0, 16)}`,
      rawSerialPersisted: false,
    }));
}

function assertBuiltContractsAreFresh() {
  for (const source of sourceFiles) {
    const sourcePath = join(repoRoot, source);
    const sourceMtime = statSync(sourcePath).mtimeMs;
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
    if (statSync(builtPath).mtimeMs + 1_000 < sourceMtime) {
      throw new Error(`Built contract file is stale for ${source}; run npm run build:contracts first`);
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
