import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofMode = 'tracking-plan-pre-device-proof';
const outputDir = join(repoRoot, 'test-results', proofMode);
const proofPath = join(outputDir, 'proof.json');
const proofRoot = join(repoRoot, 'output', 'tracking-plan-proof', 'pre-device-gap-closure');
const commands = [];

const proofCommands = [
  ['tracking-contract', 'node', ['scripts/test/tracking-plan-contract-proof.mjs']],
  ['tracking-runtime', 'node', ['scripts/test/tracking-plan-runtime-proof.mjs']],
  ['tracking-service-read-model', 'node', ['scripts/test/tracking-plan-service-read-model-proof.mjs']],
  ['android-device-artifact-gate', 'npm', ['run', 'test:child-android-device-proof-artifact-gate']],
  ['ios-entitlement-capability', 'npm', ['run', 'test:child-ios-entitlement-capability-proof']],
  ['mobile-child-agent-capability', 'npm', ['run', 'test:mobile-child-agent-capability-proof']],
];

await main();

async function main() {
  await mkdir(outputDir, { recursive: true });
  await mkdir(proofRoot, { recursive: true });

  for (const [, command, args] of proofCommands) {
    await runCommand(command, args);
  }

  const proofs = await loadProofs();
  const docs = await assertDocsStillBlockDeviceClaims();
  const matrix = buildPreDeviceMatrix(proofs);
  assertPreDeviceMatrix(matrix);

  const artifactPlans = {
    androidStudio: androidStudioPlan(),
    iosSimulator: iosSimulatorPlan(),
    wslLocal: wslLocalPlan(),
    physicalDevices: physicalDevicePlan(),
  };
  const checkedAt = new Date().toISOString();
  const proof = {
    schemaVersion: 1,
    checkedAt,
    commit: await gitHead(),
    proofMode,
    commands,
    docs,
    matrix,
    artifactPlans,
    productClaimReady: false,
    preDeviceGateComplete: true,
    nextAllowedPhase:
      'Run Android Studio/emulator, WSL/local-host, and then physical Android/iOS manual proof one by one.',
    nonClaims: [
      'Android foreground or background location runtime behavior',
      'Android geofence delivery, Doze, killed-app, reboot, or OEM background reliability',
      'iOS Core Location foreground/background behavior',
      'iOS region monitoring, significant-change, visits, entitlement, signing, TestFlight, or App Store behavior',
      'hosted accessibility proof for the full parent/child tracking UI',
      'authority-enrolled Device Owner, supervised/MDM, AppLocker/App Control, or production pilot behavior',
    ],
  };

  await writeJson(proofPath, proof);
  await writeJson(join(proofRoot, 'proof-summary.json'), proof);
  await writeJson(join(proofRoot, 'android-studio-local-proof-plan.json'), artifactPlans.androidStudio);
  await writeJson(join(proofRoot, 'ios-simulator-local-proof-plan.json'), artifactPlans.iosSimulator);
  await writeJson(join(proofRoot, 'wsl-local-proof-plan.json'), artifactPlans.wslLocal);
  await writeJson(join(proofRoot, 'physical-device-manual-proof-plan.json'), artifactPlans.physicalDevices);
  await writeFile(
    join(proofRoot, '16-validation-commands.log'),
    `${commands.map((entry) => `${entry.command} exit=${entry.exitCode}`).join('\n')}\n`
  );
  await writeFile(join(proofRoot, 'README.md'), proofReadme(checkedAt));

  console.log('tracking-plan-pre-device-proof-ok');
  console.log(`evidence=${relativePath(proofPath)}`);
  console.log(`proofRoot=${relativePath(proofRoot)}`);
}

async function loadProofs() {
  const runtime = await readJson(
    'output/tracking-plan-proof/33-proof-gates-fixtures-rollout-and-pr-gate/00-run-metadata.json'
  );
  const service = await readJson(
    'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/18-service-read-model-proof.json'
  );
  const mobile = await readJson('test-results/mobile-child-agent-capability-proof/proof.json');
  const android = await readJson('test-results/child-android-device-proof-artifact-gate/proof.json');
  const ios = await readJson('test-results/child-ios-entitlement-capability-proof/proof.json');

  if (runtime.minimumSeriousMvpAudit.productCompleteClaimed !== false) {
    throw new Error('Runtime MVP audit must not claim product completion.');
  }
  if (service.productClaimReady !== false) {
    throw new Error('Tracking service proof must remain not product-claim-ready.');
  }
  if (android.runtimeReadModel.childAndroidDeviceReadinessState !== 'manual-required') {
    throw new Error('Android device readiness must remain manual-required before device proof.');
  }
  if (!ios.nonClaims.includes('background execution behavior')) {
    throw new Error('iOS entitlement proof must keep background execution as a non-claim.');
  }

  return { runtime, service, mobile, android, ios };
}

async function assertDocsStillBlockDeviceClaims() {
  const checklist = await readText('docs/plans/tracking-plan/implementation-checklist.md');
  const feature = await readText('docs/features/location-geofence-device-status.md');
  assertIncludes(
    checklist,
    '- [ ] Android background claims have real device permission/background proof.',
    'Android unchecked gate'
  );
  assertIncludes(
    checklist,
    '- [ ] iOS background/region claims have real device permission/background',
    'iOS unchecked gate'
  );
  assertIncludes(feature, '- [ ] Android permission/background runtime proof.', 'feature Android gap');
  assertIncludes(feature, '- [ ] iOS entitlement/background proof.', 'feature iOS gap');
  assertIncludes(
    feature,
    'Location is not implied by LAN presence, IP address, or pairing.',
    'feature no LAN/IP claim'
  );
  assertIncludes(checklist, 'DOC_DELTA queue', 'central product checklist update is queued outside this worker branch');
  return {
    implementationChecklist: 'docs/plans/tracking-plan/implementation-checklist.md',
    featureDoc: 'docs/features/location-geofence-device-status.md',
    productChecklist:
      'Queued outside this branch via C:/Users/sujan/.codex/ocentra-parent-hub/lanes/codex-a/product-doc-deltas.ndjson',
  };
}

function buildPreDeviceMatrix({ runtime, service, mobile, android, ios }) {
  return [
    matrixRow('tracking-contracts', 'P0_CONTRACT', 'P0_CONTRACT', 'proved', 'output/tracking-plan-proof/', 'none'),
    matrixRow(
      'tracking-runtime-fixtures',
      'P1_FIXTURE_SIMULATION',
      runtime.currentProofTier,
      runtime.currentStatus,
      'output/tracking-plan-proof/33-proof-gates-fixtures-rollout-and-pr-gate/00-run-metadata.json',
      'product-complete and physical-device claims remain blocked'
    ),
    matrixRow(
      'tracking-service-read-model',
      'P2_HOSTED_CI',
      service.currentProofTier,
      service.currentStatus,
      'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/18-service-read-model-proof.json',
      'richer read models, deletion/tombstone replay, hosted UI/a11y, and platform replay remain pending'
    ),
    matrixRow(
      'mobile-child-agent-scaffold',
      'P2_HOSTED_CI',
      'P2_HOSTED_CI',
      'proved',
      mobile.evidence.output,
      'mobile child-agent parity remains manual-required'
    ),
    matrixRow(
      'android-studio-emulator-pass',
      'P3_LOCAL_DEV_MACHINE',
      'P2_HOSTED_CI',
      trackingStatus(android.runtimeReadModel.childAndroidDeviceReadinessState),
      android.evidence.output,
      'emulator install, runtime permission grant, foreground service, and logcat evidence must be collected locally'
    ),
    matrixRow(
      'ios-simulator-pass',
      'P3_LOCAL_DEV_MACHINE',
      'P2_HOSTED_CI',
      'manual_required',
      ios.evidence.output,
      'local xcodebuild simulator run and screenshots are not collected on this Windows lane'
    ),
    matrixRow(
      'android-physical-device-pass',
      'P4_PHYSICAL_DEVICE',
      'P2_HOSTED_CI',
      'manual_required',
      'output/tracking-plan-proof/pre-device-gap-closure/physical-device-manual-proof-plan.json',
      'real Android location/geofence/battery/killed/reboot proof is required next'
    ),
    matrixRow(
      'ios-physical-device-pass',
      'P4_PHYSICAL_DEVICE',
      'P2_HOSTED_CI',
      'manual_required',
      'output/tracking-plan-proof/pre-device-gap-closure/physical-device-manual-proof-plan.json',
      'real iOS authorization/location/region/background proof is required next'
    ),
    matrixRow(
      'authority-enrolled-device-pass',
      'P5_AUTHORITY_ENROLLED_DEVICE',
      'P0_CONTRACT',
      'authority_required',
      'output/tracking-plan-proof/pre-device-gap-closure/physical-device-manual-proof-plan.json',
      'Device Owner, supervised/MDM, AppLocker/App Control, or equivalent authority is not enrolled'
    ),
  ];
}

function assertPreDeviceMatrix(matrix) {
  const falseClaim = matrix.find(
    (row) =>
      ['android-physical-device-pass', 'ios-physical-device-pass', 'authority-enrolled-device-pass'].includes(row.id) &&
      row.currentStatus === 'proved'
  );
  if (falseClaim !== undefined) {
    throw new Error(`${falseClaim.id} cannot be proved before device or authority artifacts exist.`);
  }
}

function androidStudioPlan() {
  return {
    requiredTier: 'P3_LOCAL_DEV_MACHINE',
    currentStatus: 'manual_required_until_emulator_or_android_studio_run',
    commands: [
      'npm run release:package:android',
      'adb devices',
      'adb install -r target/release-packages/android/ocentra-parent-agent-android-debug-latest.apk',
      'adb shell monkey -p ca.ocentra.parent.agent 1',
      'adb logcat -d > output/tracking-plan-proof/android-foreground-location/10-logcat.txt',
    ],
    requiredArtifacts: [
      'output/tracking-plan-proof/android-foreground-location/01-device-metadata.json',
      'output/tracking-plan-proof/android-foreground-location/02-permission-state.json',
      'output/tracking-plan-proof/android-foreground-location/03-runtime-location-evidence.json',
      'output/tracking-plan-proof/android-device-status/04-device-status-proof.json',
      'output/tracking-plan-proof/android-background-geofence/05-geofence-transitions.ndjson',
    ],
  };
}

function iosSimulatorPlan() {
  return {
    requiredTier: 'P3_LOCAL_DEV_MACHINE',
    currentStatus: 'manual_required_until_mac_xcode_or_ios_simulator_run',
    commands: [
      'bash scripts/release/ios/build-simulator-app.sh',
      'xcrun simctl install booted target/release-packages/ios/OcentraParentAgent.app',
      'xcrun simctl launch booted ca.ocentra.parent.agent',
    ],
    requiredArtifacts: [
      'output/tracking-plan-proof/11-ios-core-location-foreground-adapter/02-platform-permission-proof.md',
      'output/tracking-plan-proof/11-ios-core-location-foreground-adapter/03-runtime-location-evidence.json',
      'output/tracking-plan-proof/12-ios-background-region-significant-change-adapter/15-manual-platform-proof.md',
    ],
  };
}

function wslLocalPlan() {
  return {
    requiredTier: 'P3_LOCAL_DEV_MACHINE',
    currentStatus: 'optional_local_replay_required_before_linux_or_wsl_claim',
    commands: [
      'npm run build:contracts',
      'node scripts/test/tracking-plan-service-read-model-proof.mjs',
      'cargo test -p ocentra-parent-agent-core tracking_read_model',
    ],
    nonClaims: ['WSL cannot prove Android/iOS physical background behavior or mobile OS permissions.'],
  };
}

function physicalDevicePlan() {
  return {
    androidRequiredTier: 'P4_PHYSICAL_DEVICE',
    iosRequiredTier: 'P4_PHYSICAL_DEVICE',
    authorityRequiredTier: 'P5_AUTHORITY_ENROLLED_DEVICE',
    currentStatus: 'manual_required_or_authority_required',
    androidProofRoot: 'output/tracking-plan-proof/android-background-geofence/',
    iosProofRoot: 'output/tracking-plan-proof/ios-region-monitoring/',
    requiredBeforeProductClaim: [
      'device metadata',
      'permission or authorization state',
      'input scenario',
      'runtime evidence rows',
      'journal/read-model rows',
      'policy/action result',
      'screenshots',
      'logcat or xcode logs',
      'result summary',
    ],
  };
}

function matrixRow(id, requiredProofTier, currentProofTier, currentStatus, artifactPath, missingProofReason) {
  return { id, requiredProofTier, currentProofTier, currentStatus, artifactPath, missingProofReason };
}

function trackingStatus(status) {
  return status.replaceAll('-', '_');
}

function proofReadme(checkedAt) {
  return [
    '# Tracking Pre-Device Gap Closure Proof',
    '',
    `Generated: ${checkedAt}`,
    '',
    'This proof closes the pre-device accounting gap. It proves the current P0/P1/P2 tracking and mobile scaffold boundaries, then writes exact Android Studio, iOS simulator, WSL/local, physical-device, and authority proof requirements.',
    '',
    'It does not claim Android/iOS physical background behavior, enrolled-device authority, hosted full UI accessibility, or production pilot readiness.',
    '',
  ].join('\n');
}

async function runCommand(command, args) {
  const actualCommand = process.platform === 'win32' && command === 'npm' ? 'cmd' : command;
  const actualArgs = process.platform === 'win32' && command === 'npm' ? ['/c', 'npm', ...args] : args;
  const commandLine = [command, ...args].join(' ');
  await new Promise((resolve, reject) => {
    const child = spawn(actualCommand, actualArgs, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => {
      commands.push({ command: commandLine, exitCode: code });
      code === 0 ? resolve() : reject(new Error(`${commandLine} exited with ${code}`));
    });
    child.once('error', reject);
  });
}

async function readJson(path) {
  return JSON.parse(await readText(path));
}

async function readText(path) {
  return readFile(join(repoRoot, path), 'utf8');
}

async function writeJson(path, value) {
  await mkdir(join(path, '..'), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
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
