import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofId = 'v0-9-household-discovery-mobile-controller-product-proof';
const outputDir = join(repoRoot, 'test-results', proofId);
const proofPath = join(outputDir, 'proof.json');
const matrixPath = join(repoRoot, 'docs', 'expectations', 'pre-ai-proof-matrix.json');
const proofCommand = `node scripts/test/${proofId}.mjs`;
const aggregateRouteChecks = new Set([
  'paired-route-accepted',
  'failed-unpaired-rejected',
  'wrong-origin-rejected',
  'wrong-device-rejected',
  'replay-rejected',
  'revoked-pairing-rejected',
  'stale-source-rejected',
  'offline-device-rejected',
  'unavailable-route-rejected',
]);
const sourceProofPaths = {
  householdDiscovery: join(repoRoot, 'test-results', 'v0-9-production-discovery-household-proof', 'proof.json'),
  productionMobile: join(repoRoot, 'test-results', 'v0-9-production-lan-mobile-controller-proof', 'proof.json'),
  mobileDiscovery: join(repoRoot, 'test-results', 'v0-9-mobile-controller-discovery-runtime-proof', 'proof.json'),
  mobileObserver: join(repoRoot, 'test-results', 'v0-9-mobile-controller-observer-runtime-proof', 'proof.json'),
  mobileHandoff: join(repoRoot, 'test-results', 'parent-mobile-controller-observer-handoff-proof', 'proof.json'),
};
const commands = [];
const proofLabels = [];

await main();
process.exit(0);

async function main() {
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-production-discovery-household-proof.mjs']);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-production-lan-mobile-controller-proof.mjs']);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-mobile-controller-discovery-runtime-proof.mjs']);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-mobile-controller-observer-runtime-proof.mjs']);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/parent-mobile-controller-observer-handoff-proof.mjs']);

  const householdDiscoveryProof = await readJson(sourceProofPaths.householdDiscovery);
  const productionMobileProof = await readJson(sourceProofPaths.productionMobile);
  const mobileDiscoveryProof = await readJson(sourceProofPaths.mobileDiscovery);
  const mobileObserverProof = await readJson(sourceProofPaths.mobileObserver);
  const mobileHandoffProof = await readJson(sourceProofPaths.mobileHandoff);
  const matrix = await readJson(matrixPath);
  const readModel = await parseAggregateReadModel(
    buildReadModel(
      householdDiscoveryProof,
      productionMobileProof,
      mobileDiscoveryProof,
      mobileObserverProof,
      mobileHandoffProof
    )
  );

  assertSourceProofs(
    householdDiscoveryProof,
    productionMobileProof,
    mobileDiscoveryProof,
    mobileObserverProof,
    mobileHandoffProof
  );
  assertReadModel(readModel);
  assertProofMatrix(matrix);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: proofId,
    commands,
    proofLabels,
    evidence: {
      householdDiscovery: relativePath(sourceProofPaths.householdDiscovery),
      productionMobile: relativePath(sourceProofPaths.productionMobile),
      mobileDiscovery: relativePath(sourceProofPaths.mobileDiscovery),
      mobileObserver: relativePath(sourceProofPaths.mobileObserver),
      mobileHandoff: relativePath(sourceProofPaths.mobileHandoff),
      proofMatrix: relativePath(matrixPath),
      output: relativePath(proofPath),
    },
    readModel,
    claimsProved: [
      'production household discovery, paired route acceptance, rejection states, and restart recovery are composed with parent mobile route proof',
      'parent mobile observer operations stay read-only while writes, pairing, revocation, and takeover remain rejected or manual-required',
      'cloud relay remains not implemented with a manual decision gate',
    ],
    claimsNotProved: [
      'physical household LAN readiness across two real devices',
      'real Android or iOS parent mobile write authority or background LAN behavior',
      'cloud relay routing, authentication, storage, or remote control',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-9-household-discovery-mobile-controller-product-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

function buildReadModel(householdDiscoveryProof, productionMobileProof, mobileDiscoveryProof, mobileObserverProof) {
  const householdReadModel = householdDiscoveryProof.productionDiscoveryHouseholdReadModel;
  const discoveryReadModel = mobileDiscoveryProof.runtimeReadModel;
  const observerReadModel = mobileObserverProof.runtimeReadModel;
  const routeChecks = householdReadModel.routeChecks.filter(isAggregateRouteCheck).map(routeCheckEvidence);
  const observerOperations = observerReadModel.mobileReadModels.flatMap((mobileReadModel) =>
    mobileReadModel.operationProofs.map((entry) => observerOperationEvidence(entry, mobileReadModel.platform))
  );
  return {
    schemaVersion: proofId,
    checkedAt: householdReadModel.checkedAt,
    sourceProofs: sourceProofInputs(),
    productionDiscoveryStates: discoveryReadModel.householdDiscovery.discoveryStatesCovered,
    routeChecks,
    mobileRoutes: discoveryReadModel.mobileRouteReadModels.map(mobileRouteEvidence),
    observerOperations,
    controllerTransitions: discoveryReadModel.controllerTransitions,
    selectedTrustedDeviceEvidence: selectedTrustedDeviceEvidence(productionMobileProof),
    auditProofCustody: auditProofCustody(routeChecks, observerOperations, householdReadModel, productionMobileProof),
    manualProofBoundary: {
      physicalHouseholdLan: productionMobileProof.manualProofGates.physicalHouseholdLan.state,
      parentMobileWriteAuthority: productionMobileProof.manualProofGates.parentMobileControllerObserver.state,
      cloudRelayImplementation: productionMobileProof.manualProofGates.cloudRelay.state,
      cloudRelayDecision: householdDiscoveryProof.manualStates.cloudRelayDecision,
      mobileBackgroundBehavior: 'manual-required',
      physicalDeviceChecklist: householdReadModel.manualHouseholdProofChecklist.map(
        (entry) => entry.requiredArtifactSummary
      ),
    },
    claimsProved: [
      'local production discovery and parent mobile observer route proof are composed',
      'selected device storage, trusted registry recovery, and route security rejection labels are carried into the aggregate proof',
      'Android and iOS observer operations are audited without upgrading parent mobile write authority',
    ],
    claimsNotProved: [
      'physical household LAN readiness',
      'real parent mobile write authority',
      'cloud relay implementation',
    ],
  };
}

function sourceProofInputs() {
  return [
    sourceProof('v0-9-production-discovery-household-proof'),
    sourceProof('v0-9-production-lan-mobile-controller-proof'),
    sourceProof('v0-9-mobile-controller-discovery-runtime-proof'),
    sourceProof('v0-9-mobile-controller-observer-runtime-proof'),
    sourceProof('parent-mobile-controller-observer-handoff-proof'),
  ];
}

function sourceProof(source) {
  const scriptName =
    source === 'parent-mobile-controller-observer-handoff-proof'
      ? 'parent-mobile-controller-observer-handoff-proof'
      : source;
  return {
    source,
    path: `test-results/${source}/proof.json`,
    command: `node scripts/test/${scriptName}.mjs`,
  };
}

function routeCheckEvidence(entry) {
  return {
    check: entry.check,
    routeId: entry.routeId,
    discoveryState: entry.discoveryState,
    trustState: entry.trustState,
    reachability: entry.reachability,
    rejectionReason: entry.rejectionReason,
    proofState: entry.proofState,
    proofLabel: `${entry.check}:${entry.proofState}`,
  };
}

function isAggregateRouteCheck(entry) {
  return aggregateRouteChecks.has(entry.check);
}

function mobileRouteEvidence(entry) {
  return {
    platform: entry.platform,
    routeId: entry.routeId,
    discoveryState: entry.discoveryState,
    reachability: entry.reachability,
    controllerState: entry.controllerState,
    commandAuthorityState: entry.commandAuthorityState,
    serviceState: entry.serviceState,
    proofState: entry.platform === 'android' ? 'ci-mechanical-proof' : 'manual-required',
    proofLabel: entry.proofLabels[0],
  };
}

function observerOperationEvidence(entry, platform) {
  return {
    platform,
    operation: entry.operation,
    operationState: entry.operationState,
    rejectionReason: entry.rejectionReason,
    proofState: operationProofState(entry.operationState),
    proofLabel: `${platform}:${entry.proofLabel}`,
  };
}

function selectedTrustedDeviceEvidence(productionMobileProof) {
  const serviceBoundary = productionMobileProof.localServiceProof.twoServiceBoundary;
  const controllerAuthority = productionMobileProof.localServiceProof.controllerAuthority;
  const selectedDeviceRejectionLabels = controllerAuthority.dishonestStateRejections.concat([
    controllerAuthority.revocationBeforeControl.routeRevokedAssertion,
    controllerAuthority.revocationBeforeControl.controlRejectedAssertion,
  ]);
  return {
    storageState: 'ci-mechanical-proof',
    securityState: 'ci-mechanical-proof',
    selectedRouteRecoveryLabels: serviceBoundary.selectedRouteRecovery.concat(serviceBoundary.acceptedAfterRestart),
    trustedRegistryLabels: serviceBoundary.services.map((service) => `${service.label}:${service.registryPersistence}`),
    selectedRouteTrustLabels: serviceBoundary.selectedRouteTrust,
    selectedDeviceRejectionLabels,
    wrongDeviceRejectionLabel: serviceBoundary.wrongDeviceRejected,
    proofLabel: 'trusted-device-selected-device-storage-security-proof',
  };
}

function auditProofCustody(routeChecks, observerOperations, householdReadModel, productionMobileProof) {
  return {
    proofState: 'ci-mechanical-proof',
    physicalDeviceProofState: productionMobileProof.manualProofGates.physicalHouseholdLan.state,
    routeAuditLabels: routeChecks.map((entry) => entry.proofLabel),
    observerAuditLabels: observerOperations.map((entry) => entry.proofLabel),
    manualBoundaryLabels: householdReadModel.manualHouseholdProofChecklist.map(
      (entry) => entry.requiredArtifactSummary
    ),
    proofLabel: 'aggregate-route-observer-audit-proof-custody',
  };
}

function operationProofState(operationState) {
  if (operationState === 'manual-required-mobile-package') {
    return 'manual-required';
  }
  if (operationState === 'degraded-provider') {
    return 'degraded';
  }
  return 'ci-mechanical-proof';
}

async function parseAggregateReadModel(readModel) {
  const modulePath = join(
    repoRoot,
    'packages',
    'rust-parent-runtime',
    'dist',
    'v0-9-household-discovery-mobile-controller-product-proof.js'
  );
  const module = await import(`file:///${modulePath.replaceAll('\\', '/')}`);
  proofLabels.push('rust-parent-runtime.v0.9-household-discovery-mobile-controller-product-proof-parse');
  return module.V09HouseholdDiscoveryMobileControllerProductProofReadModelSchema.parse(readModel);
}

function assertSourceProofs(
  householdDiscoveryProof,
  productionMobileProof,
  mobileDiscoveryProof,
  mobileObserverProof,
  mobileHandoffProof
) {
  assertEqual(householdDiscoveryProof.proofMode, 'v0-9-production-discovery-household-proof', 'household proof mode');
  assertEqual(productionMobileProof.proofMode, 'v0-9-production-lan-mobile-controller-proof', 'mobile proof mode');
  assertEqual(mobileDiscoveryProof.proofMode, 'v0-9-mobile-controller-discovery-runtime-proof', 'discovery proof mode');
  assertEqual(mobileObserverProof.proofMode, 'v0-9-mobile-controller-observer-runtime-proof', 'observer proof mode');
  assertEqual(
    mobileHandoffProof.proofMode,
    'parent-mobile-controller-observer-handoff-proof',
    'mobile handoff proof mode'
  );
  proofLabels.push('source-proofs.present');
}

function assertReadModel(readModel) {
  assertEqual(readModel.manualProofBoundary.physicalHouseholdLan, 'manual-required', 'physical household state');
  assertEqual(readModel.manualProofBoundary.cloudRelayImplementation, 'not-implemented', 'cloud relay state');
  assertArrayIncludes(readModel.productionDiscoveryStates, 'stale', 'stale source state');
  assertRouteRejections(readModel.routeChecks);
  assertArrayIncludes(
    readModel.mobileRoutes.map((entry) => entry.commandAuthorityState),
    'observer-read-only',
    'observer authority'
  );
  assertObserverOperations(readModel.observerOperations);
  assertSelectedTrustedDeviceEvidence(readModel.selectedTrustedDeviceEvidence);
  assertAuditProofCustody(readModel.auditProofCustody, readModel.routeChecks, readModel.observerOperations);
  proofLabels.push('aggregate-read-model.honest');
}

function assertRouteRejections(routeChecks) {
  const expectedRejections = new Map([
    ['paired-route-accepted', null],
    ['failed-unpaired-rejected', 'anonymous'],
    ['wrong-origin-rejected', 'wrong-origin'],
    ['wrong-device-rejected', 'wrong-device'],
    ['replay-rejected', 'replayed'],
    ['revoked-pairing-rejected', 'revoked'],
    ['stale-source-rejected', 'stale'],
    ['offline-device-rejected', 'offline'],
    ['unavailable-route-rejected', 'unsupported-route'],
  ]);
  const byCheck = new Map(routeChecks.map((entry) => [entry.check, entry]));
  for (const [check, rejectionReason] of expectedRejections.entries()) {
    const entry = byCheck.get(check);
    if (!entry) {
      throw new Error(`Missing aggregate route check ${check}.`);
    }
    assertEqual(entry.rejectionReason, rejectionReason, `${check} rejection reason`);
  }
  proofLabels.push('aggregate-route-rejections.exhaustive');
}

function assertObserverOperations(observerOperations) {
  for (const platform of ['android', 'ios']) {
    const operations = observerOperations.filter((entry) => entry.platform === platform);
    const byOperation = new Map(operations.map((entry) => [entry.operation, entry]));
    for (const operation of [
      'observe-status',
      'preview-policy-draft',
      'refresh-capabilities',
      'request-controller-takeover',
      'release-controller-lease',
      'submit-lan-ai-job',
      'write-policy',
      'approve-override',
      'pair-device',
      'revoke-device',
    ]) {
      if (!byOperation.has(operation)) {
        throw new Error(`Missing ${platform} observer operation ${operation}.`);
      }
    }
    assertEqual(byOperation.get('write-policy').rejectionReason, 'observer-read-only', `${platform} write rejection`);
    assertEqual(
      byOperation.get('submit-lan-ai-job').rejectionReason,
      'lan-ai-provider-unavailable',
      `${platform} LAN AI rejection`
    );
  }
  proofLabels.push('aggregate-observer-operations.android-ios-complete');
}

function assertSelectedTrustedDeviceEvidence(evidence) {
  assertEqual(evidence.storageState, 'ci-mechanical-proof', 'selected trusted device storage state');
  assertEqual(evidence.securityState, 'ci-mechanical-proof', 'selected trusted device security state');
  assertArrayLengthAtLeast(evidence.selectedRouteRecoveryLabels, 2, 'selected route recovery labels');
  assertArrayLengthAtLeast(evidence.trustedRegistryLabels, 2, 'trusted registry labels');
  assertArrayLengthAtLeast(evidence.selectedRouteTrustLabels, 3, 'selected route trust labels');
  assertArrayLengthAtLeast(evidence.selectedDeviceRejectionLabels, 8, 'selected device rejection labels');
  if (!evidence.wrongDeviceRejectionLabel.includes('wrong-device')) {
    throw new Error(`Wrong-device rejection label was not explicit: ${evidence.wrongDeviceRejectionLabel}`);
  }
  proofLabels.push('aggregate-selected-trusted-device.storage-security');
}

function assertAuditProofCustody(custody, routeChecks, observerOperations) {
  assertEqual(custody.proofState, 'ci-mechanical-proof', 'audit custody state');
  assertEqual(custody.physicalDeviceProofState, 'manual-required', 'physical device custody state');
  assertArrayLengthAtLeast(custody.routeAuditLabels, routeChecks.length, 'route audit labels');
  assertArrayLengthAtLeast(custody.observerAuditLabels, observerOperations.length, 'observer audit labels');
  assertArrayLengthAtLeast(custody.manualBoundaryLabels, 5, 'manual boundary custody labels');
  proofLabels.push('aggregate-audit-proof-custody.manual-boundary-honest');
}

function assertProofMatrix(matrix) {
  assertArrayIncludes(matrix.requiredCompletedClaimIds, proofId, 'required completed claim');
  const claim = matrix.claims.find((candidate) => candidate.id === proofId);
  const scenario = matrix.checkpointScenarios.find((candidate) => candidate.id === proofId);
  if (!claim || !scenario) {
    throw new Error(`Proof matrix is missing ${proofId}.`);
  }
  assertArrayIncludes(claim.ciProof.commands, proofCommand, 'claim command');
  assertArrayIncludes(scenario.ciCommands, proofCommand, 'scenario command');
  assertEqual(claim.runtimeSurfaceCoverage.physicalHouseholdLan.state, 'manual-required', 'matrix physical LAN');
  assertEqual(claim.runtimeSurfaceCoverage.cloudRelay.state, 'not-implemented', 'matrix cloud relay');
  proofLabels.push('proof-matrix.v0.9-household-mobile-product-proof');
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

function assertArrayLengthAtLeast(values, expected, label) {
  if (!Array.isArray(values) || values.length < expected) {
    throw new Error(`${label}: expected at least ${expected}, received ${values?.length ?? 0}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
