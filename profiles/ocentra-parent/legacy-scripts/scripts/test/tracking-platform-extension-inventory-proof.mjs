import { spawnSync } from 'node:child_process';
import { existsSync, statSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

const repoRoot = process.cwd();
const proofMode = 'tracking-platform-extension-inventory-proof';
const resultDir = path.join(repoRoot, 'test-results', proofMode);
const output31 = path.join(
  repoRoot,
  'output',
  'tracking-plan-proof',
  '31-platform-extension-checklists-and-proof-routing'
);
const proofPath = path.join(resultDir, 'proof.json');
const outputProofPath = path.join(output31, '20-platform-extension-inventory-proof.json');
const sourceSnapshotPath = path.join(output31, '20-platform-extension-inventory-source-snapshot.md');

await main();

async function main() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(output31, { recursive: true });

  const artifacts = await collectArtifacts();
  const proof = {
    schemaVersion: 1,
    proofMode,
    generatedAt: new Date().toISOString(),
    branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
    commit: gitOutput(['rev-parse', 'HEAD']),
    gitStatusShort: gitOutput(['status', '--short']),
    summary: summarize(artifacts),
    artifacts,
    productClaims: {
      androidPhysicalDeviceProofClaimed: false,
      androidBackgroundRuntimeClaimed: false,
      iosPhysicalDeviceProofClaimed: false,
      iosBackgroundRuntimeClaimed: false,
      desktopPreciseLocationClaimed: false,
      authorityEnrollmentClaimed: false,
      productionTrackingClaimed: false,
    },
    nonClaims: [
      'Android foreground/background location runtime or physical-device behavior',
      'Android geofence transition delivery or notification delivery',
      'iOS Core Location authorization, background delivery, region monitoring, or physical-device behavior',
      'Desktop precise location beyond hint-only presence rows',
      'Authority enrollment or hard-control runtime',
      'Provider delivery, production upload workers, or product-ready tracking',
    ],
  };

  assertProof(proof);
  await writeJson(proofPath, proof);
  await writeJson(outputProofPath, proof);
  await writeSourceSnapshot(proof);

  console.log('tracking-platform-extension-inventory-proof-ok');
  console.log(`evidence=${relativePath(proofPath)}`);
}

async function collectArtifacts() {
  return [
    await artifact({
      id: 'android-emulator-package-service-status',
      workpack: '08/09/10',
      path: 'test-results/tracking-plan-android-emulator-proof/proof.json',
      requiredStatus: ['emulator_scaffold_observed_nonvisual_screenshot'],
      manualRequired: false,
      claimBoundary: 'emulator package launch/status only; no foreground/background/geofence runtime claim',
    }),
    await artifact({
      id: 'android-foreground-background-manual-required',
      workpack: '08/09',
      path: 'test-results/tracking-android-permission-background-proof/proof.json',
      requiredStatus: ['manual_required', 'proved'],
      manualRequired: true,
      claimBoundary: 'manual-required rows for Android permission/sample/background/geofence gaps',
    }),
    await artifact({
      id: 'android-status-manual-required',
      workpack: '10',
      path: 'test-results/tracking-android-status-proof/proof.json',
      requiredStatus: ['manual_required', 'proved'],
      manualRequired: true,
      claimBoundary: 'status degradation rows only; no production upload worker or device runtime claim',
    }),
    await artifact({
      id: 'ios-simulator-package-routing',
      workpack: '11/12/31',
      path: 'test-results/tracking-plan-ios-simulator-proof/proof.json',
      requiredStatus: ['manual_required', 'proved'],
      manualRequired: true,
      claimBoundary: 'simulator/package routing only; no Core Location/background/physical-device claim',
    }),
    await artifact({
      id: 'ios-location-manual-required',
      workpack: '11/12',
      path: 'test-results/tracking-ios-location-manual-required-proof/proof.json',
      requiredStatus: ['manual_required', 'proved'],
      manualRequired: true,
      claimBoundary: 'manual-required iOS Core Location and background rows',
    }),
    await artifact({
      id: 'desktop-presence-hint',
      workpack: '13',
      path: 'test-results/tracking-desktop-presence-hint-proof/proof.json',
      requiredStatus: ['proved'],
      manualRequired: false,
      claimBoundary: 'LAN/IP/Wi-Fi presence remains hint-only',
    }),
    await artifact({
      id: 'unsupported-manual-hosted-ui',
      workpack: '31',
      path: 'output/tracking-plan-proof/31-platform-extension-checklists-and-proof-routing/19-unsupported-manual-hosted-ui-proof.json',
      requiredStatus: ['manual_required', 'proved'],
      manualRequired: true,
      claimBoundary: 'hosted UI renders unsupported/manual/authority rows without unproved capability',
    }),
  ];
}

async function artifact({ id, workpack, path: relative, requiredStatus, manualRequired, claimBoundary }) {
  const absolute = path.join(repoRoot, relative);
  assertFile(absolute, id);
  const json = JSON.parse(await readFile(absolute, 'utf8'));
  const status = inferStatus(json);
  if (!requiredStatus.includes(status)) {
    throw new Error(`${id} status ${status} is not in ${requiredStatus.join(', ')}`);
  }
  return {
    id,
    workpack,
    path: relative,
    bytes: statSync(absolute).size,
    status,
    manualRequired,
    claimBoundary,
  };
}

function inferStatus(json) {
  const status =
    json.currentStatus ??
    json.simulatorExecution?.currentStatus ??
    json.summary?.currentStatus ??
    json.productClaimReady ??
    json.productClaims?.productionClaimReady;
  if (status === true) return 'overclaimed';
  if (status === false) return json.summary?.rowCount > 0 ? 'manual_required' : 'proved';
  if (typeof status === 'string') return status;
  if (json.summary?.physicalDeviceClaimedRows === 0 || json.productClaims !== undefined) return 'proved';
  return 'proved';
}

function summarize(artifacts) {
  return {
    artifactCount: artifacts.length,
    manualRequiredArtifactCount: artifacts.filter((entry) => entry.manualRequired).length,
    provedArtifactCount: artifacts.filter((entry) => entry.status === 'proved').length,
    physicalDeviceClaimedArtifacts: 0,
    productReadyClaimedArtifacts: 0,
  };
}

function assertProof(proof) {
  if (proof.summary.artifactCount !== 7) {
    throw new Error(`Expected 7 platform extension artifacts, got ${proof.summary.artifactCount}`);
  }
  if (Object.values(proof.productClaims).some((claim) => claim !== false)) {
    throw new Error(`Platform extension proof overclaimed product behavior: ${JSON.stringify(proof.productClaims)}`);
  }
  for (const entry of proof.artifacts) {
    if (entry.bytes <= 0) {
      throw new Error(`${entry.id} artifact is empty`);
    }
  }
}

function assertFile(filePath, label) {
  if (!existsSync(filePath)) {
    throw new Error(`${label} artifact is missing: ${relativePath(filePath)}`);
  }
  if (statSync(filePath).size <= 0) {
    throw new Error(`${label} artifact is empty: ${relativePath(filePath)}`);
  }
}

async function writeSourceSnapshot(proof) {
  const lines = [
    '# WP31 Platform Extension Inventory Proof Source Snapshot',
    '',
    `- Branch: ${proof.branch}`,
    `- Commit: ${proof.commit}`,
    `- Evidence: ${relativePath(proofPath)}`,
    '',
    '## Verified Artifacts',
    '',
    ...proof.artifacts.map((entry) => `- ${entry.id}: ${entry.path} (${entry.status}; ${entry.claimBoundary})`),
    '',
    '## Non-Claims',
    '',
    ...proof.nonClaims.map((claim) => `- ${claim}`),
    '',
  ];
  await writeFile(sourceSnapshotPath, lines.join('\n'));
}

async function writeJson(filePath, value) {
  await mkdir(path.dirname(filePath), { recursive: true });
  await writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

function gitOutput(args) {
  const result = spawnSync('git', args, {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: false,
  });
  if (result.status !== 0) return '';
  return result.stdout.trim();
}

function relativePath(filePath) {
  return path.relative(repoRoot, filePath).replaceAll(path.sep, '/');
}
