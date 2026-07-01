import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofId = 'v0-9-household-physical-proof-artifact-gate';
const outputDir = join(repoRoot, 'test-results', proofId);
const proofPath = join(outputDir, 'proof.json');
const sourceProofPath = join(
  repoRoot,
  'test-results',
  'v0-9-household-discovery-mobile-controller-product-proof',
  'proof.json'
);
const commands = [];
const proofLabels = [];

await main();
process.exit(0);

async function main() {
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-household-discovery-mobile-controller-product-proof.mjs']);

  const aggregateProof = await readJson(sourceProofPath);
  assertAggregateProof(aggregateProof);

  const readModel = await parseArtifactGateReadModel(buildReadModel(aggregateProof));
  assertReadModel(readModel);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: proofId,
    commands,
    proofLabels,
    evidence: {
      aggregateHouseholdMobileProof: relativePath(sourceProofPath),
      output: relativePath(proofPath),
    },
    readModel,
    claimsProved: [
      'typed artifact gate identifies every physical two-device evidence requirement left before household LAN readiness can be claimed',
      'selected-route, observer read-only, controller takeover, revoked route, and stale/offline route health are composed from existing V0.9 proof output',
      'cloud relay remains not implemented and separate from physical household LAN proof',
    ],
    claimsNotProved: [
      'physical household LAN readiness across two real devices',
      'manual two-device router, firewall, and mobile package artifacts',
      'cloud relay routing, authentication, storage, or remote control',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-9-household-physical-proof-artifact-gate-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

function buildReadModel(aggregateProof) {
  const aggregateReadModel = aggregateProof.readModel;
  const pairedRoute = requireRouteCheck(aggregateReadModel, 'paired-route-accepted');
  const revokedRoute = requireRouteCheck(aggregateReadModel, 'revoked-pairing-rejected');
  const staleRoute = requireRouteCheck(aggregateReadModel, 'stale-source-rejected');
  const androidRoute = requireMobileRoute(aggregateReadModel, 'android');
  const iosRoute = requireMobileRoute(aggregateReadModel, 'ios');
  const artifactRequirements = manualArtifactRequirements();

  return {
    schemaVersion: proofId,
    checkedAt: aggregateReadModel.checkedAt,
    readinessDecision: 'manual-evidence-required-before-physical-household-lan-readiness',
    physicalHouseholdLanClaimState: aggregateReadModel.manualProofBoundary.physicalHouseholdLan,
    cloudRelayState: aggregateReadModel.manualProofBoundary.cloudRelayImplementation,
    sourceProofs: [
      sourceProof('v0-9-household-discovery-mobile-controller-product-proof'),
      sourceProof('v0-9-production-discovery-household-proof'),
      sourceProof('v0-9-production-lan-mobile-controller-proof'),
    ],
    artifactRequirements,
    deviceReadiness: [
      deviceReadiness('discovered-child-agent', pairedRoute, 'ci-mechanical-proof'),
      deviceReadiness('selected-child-route', pairedRoute, 'ci-mechanical-proof'),
      deviceReadiness('parent-controller-origin', pairedRoute, 'ci-mechanical-proof'),
      {
        check: 'parent-observer-route',
        routeId: androidRoute.routeId,
        discoveryState: androidRoute.discoveryState,
        trustState: 'paired',
        reachability: androidRoute.reachability,
        runtimeProofState: androidRoute.proofState,
        physicalArtifactStatus: 'manual-required',
        evidenceLabel: androidRoute.proofLabel,
      },
    ],
    routeHealth: [
      routeHealth(
        'selected-route-accepted',
        pairedRoute.routeId,
        'active-controller-backend-proof',
        null,
        'ci-mechanical-proof'
      ),
      routeHealth(
        'observer-read-only',
        androidRoute.routeId,
        androidRoute.commandAuthorityState,
        'observer-read-only',
        androidRoute.proofState
      ),
      routeHealth(
        'controller-takeover-manual-required',
        iosRoute.routeId,
        iosRoute.commandAuthorityState,
        'takeover-denied',
        iosRoute.proofState
      ),
      routeHealth(
        'revoked-route-rejected',
        revokedRoute.routeId,
        'active-controller-backend-proof',
        revokedRoute.rejectionReason,
        revokedRoute.proofState
      ),
      routeHealth(
        'stale-offline-route-rejected',
        staleRoute.routeId,
        'unavailable',
        staleRoute.rejectionReason,
        staleRoute.proofState
      ),
    ],
    manualEvidenceStatus: {
      custodyState: 'not-collected',
      requiredArtifactCount: artifactRequirements.length,
      collectedArtifactCount: 0,
      missingArtifactCount: artifactRequirements.length,
      reviewerSummary: 'manual evidence bundle is not collected, reviewed, or ready for a physical readiness claim',
    },
    claimsProved: ['local V0.9 proof output is mapped to explicit physical household artifact gates'],
    claimsNotProved: [
      'physical household LAN readiness',
      'two-device router and firewall artifact bundle',
      'real parent mobile write authority from Android or iOS package',
      'cloud relay implementation',
    ],
  };
}

function sourceProof(source) {
  const script =
    source === 'v0-9-household-discovery-mobile-controller-product-proof'
      ? 'v0-9-household-discovery-mobile-controller-product-proof'
      : source;
  return {
    source,
    path: `test-results/${source}/proof.json`,
    command: `node scripts/test/${script}.mjs`,
  };
}

function manualArtifactRequirements() {
  return [
    artifactRequirement('two-physical-household-hosts', 'two named household devices on the same LAN'),
    artifactRequirement('same-router-or-subnet-evidence', 'router, subnet, or network artifact tying both devices'),
    artifactRequirement(
      'child-service-router-reachability',
      'physical child service reachable through household router'
    ),
    artifactRequirement('os-firewall-or-local-network-permission', 'OS firewall or local-network permission artifact'),
    artifactRequirement('controller-origin-allowlist-artifact', 'physical controller origin allowlist evidence'),
    artifactRequirement('selected-device-route-recovery', 'selected route recovery after child service restart'),
    artifactRequirement(
      'controller-observer-route-health',
      'controller and observer route health from physical clients'
    ),
    artifactRequirement('revoked-route-rejection', 'revoked route rejected before control is accepted'),
    artifactRequirement('stale-offline-device-rejection', 'stale and offline selected device rejection artifacts'),
    artifactRequirement('real-mobile-controller-package', 'real Android or iOS parent package route artifact'),
    artifactRequirement('manual-evidence-custody-record', 'reviewable custody record for the manual evidence bundle'),
  ];
}

function artifactRequirement(requirement, requiredArtifactSummary) {
  return {
    requirement,
    status: 'manual-required',
    requiredArtifactSummary,
    evidencePath: null,
    evidenceCapturedAt: null,
  };
}

function deviceReadiness(check, routeEvidence, runtimeProofState) {
  return {
    check,
    routeId: routeEvidence.routeId,
    discoveryState: check === 'discovered-child-agent' ? 'discovered' : routeEvidence.discoveryState,
    trustState: routeEvidence.trustState,
    reachability: routeEvidence.reachability,
    runtimeProofState,
    physicalArtifactStatus: 'manual-required',
    evidenceLabel: `${check}:${routeEvidence.proofLabel}`,
  };
}

function routeHealth(check, routeId, commandAuthorityState, rejectionReason, runtimeProofState) {
  return {
    check,
    routeId,
    commandAuthorityState,
    rejectionReason,
    runtimeProofState,
    physicalArtifactStatus: 'manual-required',
    evidenceLabel: `${check}:${runtimeProofState}`,
  };
}

async function parseArtifactGateReadModel(readModel) {
  const modulePath = join(
    repoRoot,
    'packages',
    'rust-parent-runtime',
    'dist',
    'v0-9-household-physical-proof-artifact-gate.js'
  );
  const module = await import(`file:///${modulePath.replaceAll('\\', '/')}`);
  proofLabels.push('rust-parent-runtime.v0.9-household-physical-proof-artifact-gate-parse');
  return module.V09HouseholdPhysicalProofArtifactGateReadModelSchema.parse(readModel);
}

function assertAggregateProof(proof) {
  assertEqual(proof.proofMode, 'v0-9-household-discovery-mobile-controller-product-proof', 'aggregate proof mode');
  assertEqual(proof.readModel.manualProofBoundary.physicalHouseholdLan, 'manual-required', 'physical LAN boundary');
  assertEqual(proof.readModel.manualProofBoundary.cloudRelayImplementation, 'not-implemented', 'cloud relay boundary');
  assertArrayIncludes(proof.readModel.claimsNotProved, 'physical household LAN readiness', 'physical non-claim');
  proofLabels.push('source-proof.physical-household-non-claim-preserved');
}

function assertReadModel(readModel) {
  assertEqual(readModel.physicalHouseholdLanClaimState, 'manual-required', 'physical LAN claim state');
  assertEqual(readModel.cloudRelayState, 'not-implemented', 'cloud relay state');
  assertEqual(readModel.manualEvidenceStatus.collectedArtifactCount, 0, 'collected artifact count');
  assertEqual(readModel.manualEvidenceStatus.missingArtifactCount, 11, 'missing artifact count');
  assertArrayIncludes(
    readModel.artifactRequirements.map((entry) => entry.requirement),
    'manual-evidence-custody-record',
    'manual custody gate'
  );
  assertArrayIncludes(
    readModel.routeHealth.map((entry) => entry.check),
    'controller-takeover-manual-required',
    'controller takeover route health'
  );
  proofLabels.push('artifact-gate.manual-required-read-model');
}

function requireRouteCheck(readModel, check) {
  const routeCheck = readModel.routeChecks.find((entry) => entry.check === check);
  if (!routeCheck) {
    throw new Error(`Missing aggregate route check ${check}.`);
  }
  return routeCheck;
}

function requireMobileRoute(readModel, platform) {
  const route = readModel.mobileRoutes.find((entry) => entry.platform === platform);
  if (!route) {
    throw new Error(`Missing aggregate mobile route ${platform}.`);
  }
  return route;
}

async function runCommand(commandName, args) {
  commands.push([commandName, ...args].join(' '));
  await new Promise((resolve, reject) => {
    const child = spawn(commandName, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${commandName} exited with ${code}`))));
    child.once('error', reject);
  });
}

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'));
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

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
}

function assertArrayIncludes(values, expected, label) {
  if (!Array.isArray(values) || !values.includes(expected)) {
    throw new Error(`${label}: expected ${expected}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
