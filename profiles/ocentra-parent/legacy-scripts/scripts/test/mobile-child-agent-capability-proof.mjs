import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { pathToFileURL } from 'node:url';
import { spawn } from 'node:child_process';
import { tsImport } from 'tsx/esm/api';

const repoRoot = process.cwd();
const proofMode = 'mobile-child-agent-capability-proof';
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
  'child-android-device-proof-artifact-gate',
  'child-ios-entitlement-capability-proof',
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
    'tests/proof/mobile-child-agent-capability-proof.test.ts',
  ]);

  const scriptWiring = await assertScriptWiring();
  const sourceProofWiring = await assertSourceProofWiring();
  const documentationProof = await assertDocumentationProof();
  const runtimeReadModel = await parseRuntimeReadModel(buildRuntimeReadModel());

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode,
    commands,
    proofLabels,
    evidence: {
      contract: 'packages/schema-domain/src/mobile-child-agent-capability-proof.ts',
      contractTest: 'packages/schema-domain/tests/proof/mobile-child-agent-capability-proof.test.ts',
      output: relativePath(proofPath),
      scriptWiring,
      sourceProofWiring,
      documentationProof,
    },
    runtimeReadModel,
    mobileChildAgentCapabilityProved: {
      android:
        'aggregate read model composes Android package, service, storage, permission, privileged, and device-gate proof states without claiming device parity',
      ios: 'aggregate read model composes iOS simulator, entitlement, signing, TestFlight, and device-proof states without claiming Apple entitlement behavior',
      packageRuntimeHooks:
        'debug APK/checksum, Android package-local status, iOS Xcode target, and iOS simulator status are CI/source proof only',
    },
    mobileChildAgentStillManual: [
      'Android emulator or physical-device install and foreground-service runtime',
      'Android notification grant and delivery',
      'Android UsageStats settings grant and observed event behavior',
      'Android AccessibilityService, VPN/DNS, Device Owner, and managed profile behavior',
      'Android Play signing and release-track proof',
      'iOS Family Controls, DeviceActivity, Screen Time, and Network Extension entitlement behavior',
      'iOS notifications, background execution, signing, TestFlight, App Store, and physical-device behavior',
      'external child-agent LAN/WebSocket transport on Android or iOS',
    ],
    nonClaims: [
      'mobile child-agent parity',
      'privileged Android OS behavior',
      'Apple entitlement approval or device behavior',
      'store distribution',
      'external mobile child-agent transport',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`mobile-child-agent-capability-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

async function assertScriptWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const schemaDomainPackage = JSON.parse(await readRepoFile('packages/schema-domain/package.json'));
  const rootScript = packageJson.scripts['test:mobile-child-agent-capability-proof'];
  if (rootScript !== `node scripts/test/${proofMode}.mjs`) {
    throw new Error('Missing root test:mobile-child-agent-capability-proof script.');
  }
  if (!schemaDomainPackage.exports['./mobile-child-agent-capability-proof']) {
    throw new Error('Missing schema-domain export for ./mobile-child-agent-capability-proof.');
  }
  proofLabels.push('package-scripts.mobile-child-agent-capability-proof');
  return {
    rootScript: 'test:mobile-child-agent-capability-proof',
    schemaDomainExport: './mobile-child-agent-capability-proof',
    sourceContract: 'packages/schema-domain/src/mobile-child-agent-capability-proof.ts',
  };
}

async function assertSourceProofWiring() {
  const packageJson = JSON.parse(await readRepoFile('package.json'));
  const proofs = [];
  for (const sourceMode of sourceProofModes) {
    const script = packageJson.scripts[`test:${sourceMode}`];
    if (script !== `node scripts/test/${sourceMode}.mjs`) {
      throw new Error(`Missing source proof script test:${sourceMode}.`);
    }
    proofs.push({
      source: sourceMode,
      command: `npm run test:${sourceMode}`,
      status: 'ci-mechanical-proof',
      outputPath: `test-results/${sourceMode}/proof.json`,
    });
  }
  proofLabels.push('source-proof-scripts.mobile-child-agent-capability-proof');
  return proofs;
}

async function assertDocumentationProof() {
  const remoteFeature = await readRepoFile('docs/features/remote-lan-mobile-platforms.md');
  const childServiceFeature = await readRepoFile('docs/features/child-agent-local-service.md');
  const releaseFeature = await readRepoFile('docs/features/production-distribution-support.md');
  const checklist = await readRepoFile('docs/product-capability-checklist.md');
  const androidReadme = await readRepoFile('platforms/android/README.md');
  const iosReadme = await readRepoFile('platforms/ios/README.md');

  assertIncludes(remoteFeature, proofMode, 'remote/mobile feature proof note');
  assertIncludes(childServiceFeature, proofMode, 'child service feature proof note');
  assertIncludes(releaseFeature, proofMode, 'release feature proof note');
  assertIncludes(checklist, proofMode, 'product checklist proof note');
  assertIncludes(androidReadme, proofMode, 'Android README proof note');
  assertIncludes(iosReadme, proofMode, 'iOS README proof note');
  proofLabels.push('docs.mobile-child-agent-capability-proof');

  return {
    remoteFeature: 'docs/features/remote-lan-mobile-platforms.md',
    childServiceFeature: 'docs/features/child-agent-local-service.md',
    releaseFeature: 'docs/features/production-distribution-support.md',
    checklist: 'docs/product-capability-checklist.md',
    androidReadme: 'platforms/android/README.md',
    iosReadme: 'platforms/ios/README.md',
  };
}

function buildRuntimeReadModel() {
  return {
    schemaVersion: proofMode,
    checkedAt: new Date().toISOString(),
    platforms: [
      {
        platform: 'android-child-agent',
        childAgentReadiness: 'manual-device-proof-required',
        packageRuntimeState: 'package-local-scaffold',
        privilegedOsState: 'blocked',
        externalTransportState: 'not-implemented',
        reviewerSummary: 'Android child-agent capability remains package-local until device proof artifacts exist',
      },
      {
        platform: 'ios-child-agent',
        childAgentReadiness: 'entitlement-review-required',
        packageRuntimeState: 'simulator-scaffold',
        privilegedOsState: 'entitlement-required',
        externalTransportState: 'not-implemented',
        reviewerSummary: 'iOS child-agent capability remains simulator and entitlement-review scoped',
      },
    ],
    sourceProofs: sourceProofModes.map((sourceMode) => ({
      source: sourceMode,
      status: 'ci-mechanical-proof',
      command: `npm run test:${sourceMode}`,
      outputPath: `test-results/${sourceMode}/proof.json`,
    })),
    capabilityRows: capabilityRows(),
    packageRuntimeHooks: packageRuntimeHooks(),
    claimBoundaries: {
      parentMobileScope: 'separate-parent-mobile-workstream',
      childAndroidParity: 'not-claimed',
      childIosParity: 'not-claimed',
      privilegedOsBehavior: 'not-claimed',
      externalChildAgentTransport: 'not-claimed',
      storeDistribution: 'not-claimed',
      reviewerSummary:
        'Mobile child-agent parity requires real device, entitlement, signing, and store proof artifacts',
    },
    knownManualGaps: [
      'Android emulator install and launch evidence',
      'Android physical-device install and foreground service evidence',
      'Android POST_NOTIFICATIONS grant and delivery evidence',
      'Android UsageStats settings grant and observed event evidence',
      'Android AccessibilityService declaration, grant, and behavior',
      'Android VPN service and DNS filtering behavior',
      'Android Device Owner enrollment and policy action',
      'Android managed profile enrollment and behavior',
      'Android Play signing or release-track evidence',
      'iOS Family Controls entitlement approval and behavior',
      'iOS DeviceActivity schedule and event behavior',
      'iOS Network Extension entitlement and filtering behavior',
      'iOS notification and background execution behavior',
      'iOS signing, TestFlight, App Store, and physical-device evidence',
    ],
  };
}

function capabilityRows() {
  return [
    androidRow(
      'android-foreground-service',
      'foreground-mobile-service',
      'manual-required',
      'device-proof-required',
      'child-android-service-protocol-capability-proof'
    ),
    androidRow(
      'android-storage-protocol-bridge',
      'local-storage',
      'scaffold',
      'package-local-scaffold',
      'child-android-storage-protocol-capability-proof'
    ),
    androidRow(
      'android-typed-protocol-bridge',
      'typed-protocol-bridge',
      'scaffold',
      'package-local-scaffold',
      'child-android-storage-protocol-capability-proof'
    ),
    androidRow(
      'android-notifications',
      'notifications',
      'manual-required',
      'manual-required',
      'child-android-permission-capability-proof'
    ),
    androidRow(
      'android-usage-stats',
      'usage-stats',
      'manual-required',
      'settings-grant-required',
      'child-android-privileged-capability-proof'
    ),
    androidRow(
      'android-accessibility-service',
      'accessibility-service',
      'not-implemented',
      'not-implemented',
      'child-android-privileged-capability-proof'
    ),
    androidRow(
      'android-vpn-dns',
      'vpn-dns-filtering',
      'not-implemented',
      'not-implemented',
      'child-android-privileged-capability-proof'
    ),
    androidRow(
      'android-device-owner',
      'device-owner-policy',
      'manual-required',
      'blocked',
      'child-android-privileged-capability-proof'
    ),
    androidRow(
      'android-managed-profile',
      'managed-profile',
      'manual-required',
      'blocked',
      'child-android-privileged-capability-proof'
    ),
    androidRow(
      'android-device-proof',
      'package-lifecycle',
      'manual-required',
      'device-proof-required',
      'child-android-device-proof-artifact-gate'
    ),
    androidRow(
      'android-play-signing',
      'store-distribution',
      'planned',
      'planned',
      'child-android-device-proof-artifact-gate'
    ),
    androidRow(
      'android-external-transport',
      'typed-protocol-bridge',
      'not-implemented',
      'not-implemented',
      'child-android-device-proof-artifact-gate'
    ),
    iosRow('ios-simulator-status-surface', 'typed-protocol-bridge', 'scaffold', 'simulator-scaffold'),
    iosRow('ios-family-controls', 'family-controls-entitlement', 'manual-required', 'entitlement-required'),
    iosRow('ios-device-activity', 'device-activity', 'manual-required', 'entitlement-required'),
    iosRow('ios-screen-time', 'screen-time-api', 'manual-required', 'entitlement-required'),
    iosRow('ios-network-extension', 'network-extension', 'manual-required', 'entitlement-required'),
    iosRow('ios-notifications', 'notifications', 'manual-required', 'manual-required'),
    iosRow('ios-background-execution', 'background-execution', 'manual-required', 'manual-required'),
    iosRow('ios-signing', 'signing-entitlements', 'manual-required', 'signing-required'),
    iosRow('ios-testflight', 'testflight-distribution', 'manual-required', 'device-proof-required'),
    iosRow('ios-device-proof', 'package-lifecycle', 'manual-required', 'device-proof-required'),
    iosRow('ios-app-store', 'store-distribution', 'planned', 'planned'),
    iosRow('ios-external-transport', 'typed-protocol-bridge', 'not-implemented', 'not-implemented'),
  ];
}

function packageRuntimeHooks() {
  return [
    runtimeHook(
      'android-debug-apk-checksum',
      'android-child-agent',
      'ci-mechanical-proof',
      'target/release-packages/android/ocentra-parent-agent-android-debug-latest.apk.sha256',
      'child-android-device-proof-artifact-gate'
    ),
    runtimeHook(
      'android-package-local-status',
      'android-child-agent',
      'package-local-scaffold',
      'platforms/android/agent/app/src/main/java/ca/ocentra/parent/agent/OcentraParentAgentService.java',
      'child-android-device-proof-artifact-gate'
    ),
    runtimeHook(
      'android-device-install',
      'android-child-agent',
      'device-proof-required',
      null,
      'child-android-device-proof-artifact-gate'
    ),
    runtimeHook(
      'android-play-signing',
      'android-child-agent',
      'planned',
      null,
      'child-android-device-proof-artifact-gate'
    ),
    runtimeHook(
      'ios-xcode-target',
      'ios-child-agent',
      'ci-mechanical-proof',
      'platforms/ios/OcentraParentAgent.xcodeproj/project.pbxproj',
      'child-ios-entitlement-capability-proof'
    ),
    runtimeHook(
      'ios-simulator-status',
      'ios-child-agent',
      'simulator-scaffold',
      'platforms/ios/OcentraParentAgent/AgentStatusViewController.swift',
      'child-ios-entitlement-capability-proof'
    ),
    runtimeHook(
      'ios-signing-profile',
      'ios-child-agent',
      'signing-required',
      null,
      'child-ios-entitlement-capability-proof'
    ),
    runtimeHook(
      'ios-testflight-device',
      'ios-child-agent',
      'device-proof-required',
      null,
      'child-ios-entitlement-capability-proof'
    ),
  ];
}

function androidRow(surface, parentCapability, parentCapabilityStatus, proofState, source) {
  return capabilityRow(surface, 'android-child-agent', parentCapability, parentCapabilityStatus, proofState, source);
}

function iosRow(surface, parentCapability, parentCapabilityStatus, proofState) {
  return capabilityRow(
    surface,
    'ios-child-agent',
    parentCapability,
    parentCapabilityStatus,
    proofState,
    'child-ios-entitlement-capability-proof'
  );
}

function capabilityRow(surface, platform, parentCapability, parentCapabilityStatus, proofState, source) {
  const proofRequirement = `${surface} remains ${proofState} until required platform artifacts change it`;
  return {
    surface,
    platform,
    parentCapability,
    parentCapabilityStatus,
    proofState,
    source,
    proofRequirement,
    claimBoundary: proofRequirement,
  };
}

function runtimeHook(hook, platform, hookState, evidencePath, source) {
  return { hook, platform, hookState, evidencePath, source };
}

async function parseRuntimeReadModel(readModel) {
  const module = await importTsModule('packages/schema-domain/src/mobile-child-agent-capability-proof.ts');
  const parsed = module.MobileChildAgentCapabilityReadModelSchema.parse(readModel);
  proofLabels.push('schema-domain.mobile-child-agent-capability-proof-parse');
  return parsed;
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

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
