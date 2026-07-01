import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofId = 'v0-9-household-multi-device-proof-gates';
const outputDir = join(repoRoot, 'test-results', proofId);
const proofPath = join(outputDir, 'proof.json');
const sourceProofPaths = {
  productProof: join(
    repoRoot,
    'test-results',
    'v0-9-household-discovery-mobile-controller-product-proof',
    'proof.json'
  ),
  physicalArtifactGate: join(repoRoot, 'test-results', 'v0-9-household-physical-proof-artifact-gate', 'proof.json'),
  multideviceHardening: join(repoRoot, 'test-results', 'v0-9-production-lan-multidevice-hardening', 'proof.json'),
  householdDiscovery: join(repoRoot, 'test-results', 'v0-9-production-discovery-household-proof', 'proof.json'),
  productionMobile: join(repoRoot, 'test-results', 'v0-9-production-lan-mobile-controller-proof', 'proof.json'),
  mobileDiscovery: join(repoRoot, 'test-results', 'v0-9-mobile-controller-discovery-runtime-proof', 'proof.json'),
  mobileObserver: join(repoRoot, 'test-results', 'v0-9-mobile-controller-observer-runtime-proof', 'proof.json'),
  mobileHandoff: join(repoRoot, 'test-results', 'parent-mobile-controller-observer-handoff-proof', 'proof.json'),
};
const commands = [];
const proofLabels = [];
const routeCustodyGates = new Map([
  ['paired-route-accepted', 'paired-household-route-evidence'],
  ['failed-unpaired-rejected', 'failed-unpaired-household-route-evidence'],
  ['wrong-origin-rejected', 'allowed-origin-rejection-custody'],
  ['wrong-device-rejected', 'wrong-device-rejection-custody'],
  ['replay-rejected', 'replay-rejection-custody'],
  ['revoked-pairing-rejected', 'revocation-rejection-custody'],
  ['stale-source-rejected', 'stale-offline-rejection-custody'],
  ['offline-device-rejected', 'stale-offline-rejection-custody'],
  ['unavailable-route-rejected', 'unsupported-route-custody'],
]);

await main();
process.exit(0);

async function main() {
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-production-discovery-household-proof.mjs']);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-production-lan-mobile-controller-proof.mjs']);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-mobile-controller-discovery-runtime-proof.mjs']);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-mobile-controller-observer-runtime-proof.mjs']);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-production-lan-multidevice-hardening.mjs']);

  const householdDiscoveryProof = await readJson(sourceProofPaths.householdDiscovery);
  const productionMobileProof = await readJson(sourceProofPaths.productionMobile);
  const mobileDiscoveryProof = await readJson(sourceProofPaths.mobileDiscovery);
  const mobileObserverProof = await readJson(sourceProofPaths.mobileObserver);
  const mobileHandoffProof = await readJson(sourceProofPaths.mobileHandoff);
  const multideviceHardeningProof = await readJson(sourceProofPaths.multideviceHardening);
  const productProof = await buildProductProof(
    householdDiscoveryProof,
    productionMobileProof,
    mobileDiscoveryProof,
    mobileObserverProof,
    mobileHandoffProof
  );
  const physicalArtifactGateProof = await buildPhysicalArtifactGateProof(productProof);
  await writeFile(sourceProofPaths.productProof, `${JSON.stringify(productProof, null, 2)}\n`);
  await writeFile(sourceProofPaths.physicalArtifactGate, `${JSON.stringify(physicalArtifactGateProof, null, 2)}\n`);

  assertSourceProofs(productProof, physicalArtifactGateProof, multideviceHardeningProof);
  const readModel = await parseReadModel(
    buildReadModel(productProof, physicalArtifactGateProof, multideviceHardeningProof)
  );
  assertReadModel(readModel, multideviceHardeningProof);

  const proof = {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: proofId,
    commands,
    proofLabels,
    evidence: {
      productProof: relativePath(sourceProofPaths.productProof),
      physicalArtifactGate: relativePath(sourceProofPaths.physicalArtifactGate),
      multideviceHardening: relativePath(sourceProofPaths.multideviceHardening),
      output: relativePath(proofPath),
    },
    readModel,
    claimsProved: [
      'typed household multi-device gates compose product proof route custody with the physical artifact manual gate',
      'selected and trusted device storage security follows through from local multidevice proof output',
      'portal-visible household device spine exposes registry, route, provider, and artifact readiness gates',
      'cloud relay remains not implemented and manual-decision-gated',
    ],
    claimsNotProved: [
      'remote desktop or remote control',
      'physical household LAN readiness across two real devices',
      'router, firewall, and local network permission evidence from household hardware',
      'real parent mobile product UX or remote cloud control',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-9-household-multi-device-proof-gates-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

function buildReadModel(productProof, physicalArtifactGateProof, multideviceHardeningProof) {
  const productReadModel = productProof.readModel;
  const physicalReadModel = physicalArtifactGateProof.readModel;
  return {
    schemaVersion: proofId,
    checkedAt: productReadModel.checkedAt,
    readinessDecision: 'manual-gate-required-before-household-multi-device-readiness',
    householdMultiDeviceReadinessState: physicalReadModel.physicalHouseholdLanClaimState,
    localMultiServiceProofState: 'ci-mechanical-proof',
    sourceProofs: [
      sourceProof('v0-9-household-discovery-mobile-controller-product-proof'),
      sourceProof('v0-9-household-physical-proof-artifact-gate'),
      sourceProof('v0-9-production-lan-multidevice-hardening'),
    ],
    physicalArtifactRequirements: physicalReadModel.artifactRequirements,
    manualEvidenceStatus: physicalReadModel.manualEvidenceStatus,
    routeCustody: [...routeCustodyGates.entries()].map(([check, custodyGate]) =>
      routeCustodyEvidence(requireRouteCheck(productReadModel, check), custodyGate)
    ),
    selectedTrustedDeviceStorage: selectedTrustedDeviceStorage(productReadModel.selectedTrustedDeviceEvidence),
    cloudRelayBoundary: {
      implementationState: productReadModel.manualProofBoundary.cloudRelayImplementation,
      remoteControlState: 'not-implemented',
      decision: productReadModel.manualProofBoundary.cloudRelayDecision,
      manualDecisionLabel: 'cloud relay remains unimplemented until a separate authenticated relay proof is chosen',
    },
    portalDeviceSpine: portalDeviceSpine(productReadModel, physicalReadModel, multideviceHardeningProof),
    claimsProved: [
      'local multi-service route proof is mapped into explicit household multi-device readiness gates',
      'paired, failed-unpaired, allowed-origin, wrong-device, replay, revocation, stale, offline, and unsupported route custody are explicit',
      'selected/trusted-device storage and rejection evidence remains tied to the existing product proof',
      'browser LAN add-device pairing spine exposes discovery, pairing state, registry, route state, provider readiness, and artifact gates',
    ],
    claimsNotProved: [
      'remote desktop or remote control',
      'physical household LAN readiness',
      'real parent mobile controller product UX',
      'cloud relay routing or authentication',
    ],
  };
}

async function buildProductProof(
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
  const readModel = await parseProductReadModel(
    buildProductReadModel(householdDiscoveryProof, productionMobileProof, mobileDiscoveryProof, mobileObserverProof)
  );
  proofLabels.push('source-proof.product-read-model-synthesized-from-real-proof-files');
  return {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'v0-9-household-discovery-mobile-controller-product-proof',
    commands: ['synthesized from current lower-level V0.9 proof files'],
    proofLabels: ['aggregate-read-model.honest'],
    evidence: {
      householdDiscovery: relativePath(sourceProofPaths.householdDiscovery),
      productionMobile: relativePath(sourceProofPaths.productionMobile),
      mobileDiscovery: relativePath(sourceProofPaths.mobileDiscovery),
      mobileObserver: relativePath(sourceProofPaths.mobileObserver),
      mobileHandoff: relativePath(sourceProofPaths.mobileHandoff),
    },
    readModel,
  };
}

function buildProductReadModel(
  householdDiscoveryProof,
  productionMobileProof,
  mobileDiscoveryProof,
  mobileObserverProof
) {
  const householdReadModel = householdDiscoveryProof.productionDiscoveryHouseholdReadModel;
  const discoveryReadModel = mobileDiscoveryProof.runtimeReadModel;
  const observerReadModel = mobileObserverProof.runtimeReadModel;
  const routeChecks = householdReadModel.routeChecks.filter(isAggregateRouteCheck).map(routeCheckEvidence);
  const observerOperations = observerReadModel.mobileReadModels.flatMap((mobileReadModel) =>
    mobileReadModel.operationProofs.map((entry) => observerOperationEvidence(entry, mobileReadModel.platform))
  );
  return {
    schemaVersion: 'v0-9-household-discovery-mobile-controller-product-proof',
    checkedAt: householdReadModel.checkedAt,
    sourceProofs: productSourceProofInputs(),
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

async function buildPhysicalArtifactGateProof(productProof) {
  const readModel = await parsePhysicalArtifactGateReadModel(buildPhysicalReadModel(productProof));
  proofLabels.push('source-proof.physical-artifact-gate-synthesized-from-current-product-read-model');
  return {
    schemaVersion: 1,
    checkedAt: new Date().toISOString(),
    commit: await gitHead(),
    proofMode: 'v0-9-household-physical-proof-artifact-gate',
    commands: ['synthesized from current V0.9 product read model'],
    proofLabels: ['artifact-gate.manual-required-read-model'],
    evidence: {
      aggregateHouseholdMobileProof: relativePath(sourceProofPaths.productProof),
      output: relativePath(sourceProofPaths.physicalArtifactGate),
    },
    readModel,
  };
}

function buildPhysicalReadModel(productProof) {
  const aggregateReadModel = productProof.readModel;
  const pairedRoute = requireRouteCheck(aggregateReadModel, 'paired-route-accepted');
  const revokedRoute = requireRouteCheck(aggregateReadModel, 'revoked-pairing-rejected');
  const staleRoute = requireRouteCheck(aggregateReadModel, 'stale-source-rejected');
  const androidRoute = requireMobileRoute(aggregateReadModel, 'android');
  const iosRoute = requireMobileRoute(aggregateReadModel, 'ios');
  const artifactRequirements = manualArtifactRequirements();
  return {
    schemaVersion: 'v0-9-household-physical-proof-artifact-gate',
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

function portalDeviceSpine(productReadModel, physicalReadModel, multideviceHardeningProof) {
  const pairedRoute = requireRouteCheck(productReadModel, 'paired-route-accepted');
  const offlineRoute = requireRouteCheck(productReadModel, 'offline-device-rejected');
  const staleRoute = requireRouteCheck(productReadModel, 'stale-source-rejected');
  const observerRoute = requireMobileRoute(productReadModel, 'android');
  const manualTakeoverRoute = requireMobileRoute(productReadModel, 'ios');
  const providerProof = multideviceHardeningProof.parentMobileControllerObserverProof;
  const selectedEvidence = productReadModel.selectedTrustedDeviceEvidence;

  return {
    lanDiscoveryBoundary: {
      sourceState: 'local-service-discovery-proof',
      discoverableDeviceState: 'ci-mechanical-proof',
      physicalLanDiscoveryState: 'manual-required',
      evidenceLabel:
        'browser LAN adapter uses local service discovery while physical household scan is manual-required',
    },
    householdDeviceRegistry: {
      registryProofState: 'ci-mechanical-proof',
      devices: [
        visibleDevice('paired household child route', pairedRoute.routeId, 'paired', pairedRoute.proofState),
        visibleDevice('offline household child route', offlineRoute.routeId, 'offline', offlineRoute.proofState),
        visibleDevice('stale household child route', staleRoute.routeId, 'stale', staleRoute.proofState),
        visibleDevice('manual household mobile controller route', null, 'manual-required', 'manual-required'),
      ],
      evidenceLabel: 'household registry read model exposes paired offline stale and manual-required devices',
    },
    addDevicePairingRequests: [
      pairingRequest('discovered', pairedRoute.routeId, null, 'ci-mechanical-proof', pairedRoute.proofLabel),
      pairingRequest('pending', pairedRoute.routeId, null, 'ci-mechanical-proof', pairedRoute.proofLabel),
      pairingRequest('paired', pairedRoute.routeId, null, pairedRoute.proofState, pairedRoute.proofLabel),
      pairingRequest(
        'rejected',
        requireRouteCheck(productReadModel, 'failed-unpaired-rejected').routeId,
        'anonymous',
        'ci-mechanical-proof',
        requireRouteCheck(productReadModel, 'failed-unpaired-rejected').proofLabel
      ),
      pairingRequest('expired', staleRoute.routeId, 'stale', staleRoute.proofState, staleRoute.proofLabel),
      pairingRequest(
        'revoked',
        requireRouteCheck(productReadModel, 'revoked-pairing-rejected').routeId,
        'revoked',
        'ci-mechanical-proof',
        requireRouteCheck(productReadModel, 'revoked-pairing-rejected').proofLabel
      ),
      pairingRequest('stale', staleRoute.routeId, 'stale', staleRoute.proofState, staleRoute.proofLabel),
      pairingRequest('offline', offlineRoute.routeId, 'offline', offlineRoute.proofState, offlineRoute.proofLabel),
    ],
    trustedDeviceRegistry: {
      registryProofState: 'ci-mechanical-proof',
      entries: [
        trustedRegistryEntry(pairedRoute.routeId, 'paired', 'paired', pairedRoute.proofLabel),
        trustedRegistryEntry(staleRoute.routeId, 'stale', 'stale', staleRoute.proofLabel),
        trustedRegistryEntry(offlineRoute.routeId, 'offline', 'offline', offlineRoute.proofLabel),
      ],
      selectedRouteRecoveryLabelCount: selectedEvidence.selectedRouteRecoveryLabels.length,
      trustedRegistryLabelCount: selectedEvidence.trustedRegistryLabels.length,
      evidenceLabel: 'trusted-device registry read model is ready for browser portal consumption',
    },
    selectedDeviceReadiness: {
      selectedRouteId: pairedRoute.routeId,
      selectedDeviceState: 'paired',
      routeProofState: pairedRoute.proofState,
      physicalArtifactStatus: 'manual-required',
      manualRequiredLabel: 'selected device readiness still requires physical household evidence',
    },
    routeState: {
      currentControllerRouteId: pairedRoute.routeId,
      currentObserverRouteId: observerRoute.routeId,
      controllerCommandAuthorityState: 'active-controller-backend-proof',
      observerCommandAuthorityState: observerRoute.commandAuthorityState,
      manualControllerTakeoverState: manualTakeoverRoute.commandAuthorityState,
      evidenceLabel: 'portal adapter route state includes current controller observer and manual takeover states',
    },
    lanAiProviderReadiness: {
      readinessState: 'mobile-provider-degraded',
      localProviderState: 'ci-mechanical-proof',
      mobileProviderState: 'degraded',
      physicalProviderArtifactStatus: 'manual-required',
      evidenceLabels: [
        'lan-ai-provider-pool local desktop provider proof is present',
        providerProof.providerUnavailable,
        providerProof.controllerJobDegraded,
      ],
    },
    artifactReadinessGates: {
      requiredArtifactCount: physicalReadModel.manualEvidenceStatus.requiredArtifactCount,
      collectedArtifactCount: physicalReadModel.manualEvidenceStatus.collectedArtifactCount,
      missingArtifactCount: physicalReadModel.manualEvidenceStatus.missingArtifactCount,
      physicalReadinessState: physicalReadModel.physicalHouseholdLanClaimState,
      cloudRelayState: multideviceHardeningProof.cloudRelayDecision.state,
      evidenceLabel: 'physical artifacts remain manual-required and cloud relay remains not implemented',
    },
    adapterBoundaryLabel: 'non-visual portal adapter can consume this spine without visual UI or product UX claims',
  };
}

function sourceProof(source) {
  return {
    source,
    path: `test-results/${source}/proof.json`,
    command: `node scripts/test/${source}.mjs`,
  };
}

function productSourceProofInputs() {
  return [
    sourceProof('v0-9-production-discovery-household-proof'),
    sourceProof('v0-9-production-lan-mobile-controller-proof'),
    sourceProof('v0-9-mobile-controller-discovery-runtime-proof'),
    sourceProof('v0-9-mobile-controller-observer-runtime-proof'),
    sourceProof('parent-mobile-controller-observer-handoff-proof'),
  ];
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
  return routeCustodyGates.has(entry.check);
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

function routeCustodyEvidence(routeCheck, custodyGate) {
  return {
    check: routeCheck.check,
    custodyGate,
    routeId: routeCheck.routeId,
    rejectionReason: routeCheck.rejectionReason,
    proofState: routeCheck.proofState,
    manualArtifactStatus: 'manual-required',
    evidenceLabel: `${routeCheck.check}:${routeCheck.proofLabel}`,
  };
}

function selectedTrustedDeviceStorage(evidence) {
  return {
    storageState: evidence.storageState,
    securityState: evidence.securityState,
    selectedRouteRecoveryLabelCount: evidence.selectedRouteRecoveryLabels.length,
    trustedRegistryLabelCount: evidence.trustedRegistryLabels.length,
    selectedRouteTrustLabelCount: evidence.selectedRouteTrustLabels.length,
    selectedDeviceRejectionLabelCount: evidence.selectedDeviceRejectionLabels.length,
    wrongDeviceRejectionLabel: evidence.wrongDeviceRejectionLabel,
    manualArtifactStatus: 'manual-required',
    evidenceLabel: evidence.proofLabel,
  };
}

function visibleDevice(deviceLabel, routeId, deviceState, routeProofState) {
  return {
    deviceLabel,
    routeId,
    deviceState,
    routeProofState,
    artifactGateStatus: 'manual-required',
    evidenceLabel: `${deviceState}:${deviceLabel}`,
  };
}

function pairingRequest(requestState, routeId, rejectionReason, proofState, proofLabel) {
  return {
    requestState,
    routeId,
    rejectionReason,
    proofState,
    manualArtifactStatus: 'manual-required',
    evidenceLabel: `add-device ${requestState}:${proofLabel}`,
  };
}

function trustedRegistryEntry(routeId, pairingState, deviceState, proofLabel) {
  return {
    routeId,
    pairingState,
    deviceState,
    registryProofState: 'ci-mechanical-proof',
    evidenceLabel: `trusted-registry ${pairingState}:${proofLabel}`,
  };
}

async function parseReadModel(readModel) {
  const module = await import('@ocentra-parent/schema-domain/v0-9-household-multi-device-proof-gates');
  proofLabels.push('rust-parent-runtime.v0.9-household-multi-device-proof-gates-parse');
  return module.V09HouseholdMultiDeviceProofGateReadModelSchema.parse(readModel);
}

async function parseProductReadModel(readModel) {
  const modulePath = join(
    repoRoot,
    'packages',
    'rust-parent-runtime',
    'dist',
    'v0-9-household-discovery-mobile-controller-product-proof.js'
  );
  const module = await import(`file:///${modulePath.replaceAll('\\', '/')}`);
  return module.V09HouseholdDiscoveryMobileControllerProductProofReadModelSchema.parse(readModel);
}

async function parsePhysicalArtifactGateReadModel(readModel) {
  const modulePath = join(
    repoRoot,
    'packages',
    'rust-parent-runtime',
    'dist',
    'v0-9-household-physical-proof-artifact-gate.js'
  );
  const module = await import(`file:///${modulePath.replaceAll('\\', '/')}`);
  return module.V09HouseholdPhysicalProofArtifactGateReadModelSchema.parse(readModel);
}

function assertSourceProofs(productProof, physicalArtifactGateProof, multideviceHardeningProof) {
  assertEqual(productProof.proofMode, 'v0-9-household-discovery-mobile-controller-product-proof', 'product proof mode');
  assertEqual(
    physicalArtifactGateProof.proofMode,
    'v0-9-household-physical-proof-artifact-gate',
    'physical artifact gate proof mode'
  );
  assertEqual(
    multideviceHardeningProof.proofMode,
    'local-multi-service-production-lan-hardening',
    'multidevice hardening proof mode'
  );
  proofLabels.push('source-proofs.product-physical-multidevice-present');
}

function assertReadModel(readModel, multideviceHardeningProof) {
  assertEqual(readModel.householdMultiDeviceReadinessState, 'manual-required', 'multi-device readiness state');
  assertEqual(readModel.cloudRelayBoundary.implementationState, 'not-implemented', 'cloud relay implementation');
  assertEqual(readModel.cloudRelayBoundary.remoteControlState, 'not-implemented', 'cloud relay remote control');
  assertEqual(readModel.manualEvidenceStatus.collectedArtifactCount, 0, 'manual evidence collected count');
  assertArrayIncludes(
    readModel.routeCustody.map((entry) => entry.custodyGate),
    'allowed-origin-rejection-custody',
    'allowed origin custody'
  );
  assertArrayIncludes(
    readModel.routeCustody.map((entry) => entry.custodyGate),
    'wrong-device-rejection-custody',
    'wrong device custody'
  );
  assertArrayIncludes(
    readModel.routeCustody.map((entry) => entry.custodyGate),
    'revocation-rejection-custody',
    'revocation custody'
  );
  assertEqual(
    readModel.selectedTrustedDeviceStorage.storageState,
    'ci-mechanical-proof',
    'selected trusted-device storage'
  );
  assertEqual(multideviceHardeningProof.localTwoServiceProof.serviceCount, 2, 'local multidevice service count');
  assertEqual(multideviceHardeningProof.cloudRelayDecision.state, 'not-implemented', 'source cloud relay state');
  assertEqual(
    readModel.portalDeviceSpine.lanDiscoveryBoundary.sourceState,
    'local-service-discovery-proof',
    'portal LAN discovery source'
  );
  assertEqual(
    readModel.portalDeviceSpine.lanDiscoveryBoundary.physicalLanDiscoveryState,
    'manual-required',
    'portal physical LAN discovery state'
  );
  assertArrayIncludes(
    readModel.portalDeviceSpine.householdDeviceRegistry.devices.map((entry) => entry.deviceState),
    'paired',
    'portal device paired state'
  );
  assertArrayIncludes(
    readModel.portalDeviceSpine.householdDeviceRegistry.devices.map((entry) => entry.deviceState),
    'offline',
    'portal device offline state'
  );
  assertArrayIncludes(
    readModel.portalDeviceSpine.householdDeviceRegistry.devices.map((entry) => entry.deviceState),
    'stale',
    'portal device stale state'
  );
  assertArrayIncludes(
    readModel.portalDeviceSpine.householdDeviceRegistry.devices.map((entry) => entry.deviceState),
    'manual-required',
    'portal device manual-required state'
  );
  for (const state of ['discovered', 'pending', 'paired', 'rejected', 'expired', 'revoked', 'stale', 'offline']) {
    assertArrayIncludes(
      readModel.portalDeviceSpine.addDevicePairingRequests.map((entry) => entry.requestState),
      state,
      `portal add-device state ${state}`
    );
  }
  assertArrayIncludes(
    readModel.portalDeviceSpine.trustedDeviceRegistry.entries.map((entry) => entry.deviceState),
    'paired',
    'trusted registry paired state'
  );
  assertArrayIncludes(
    readModel.portalDeviceSpine.trustedDeviceRegistry.entries.map((entry) => entry.deviceState),
    'offline',
    'trusted registry offline state'
  );
  assertEqual(
    readModel.portalDeviceSpine.selectedDeviceReadiness.physicalArtifactStatus,
    'manual-required',
    'selected device physical readiness'
  );
  assertEqual(
    readModel.portalDeviceSpine.routeState.controllerCommandAuthorityState,
    'active-controller-backend-proof',
    'portal controller route state'
  );
  assertEqual(
    readModel.portalDeviceSpine.routeState.observerCommandAuthorityState,
    'observer-read-only',
    'portal observer route state'
  );
  assertEqual(
    readModel.portalDeviceSpine.lanAiProviderReadiness.mobileProviderState,
    'degraded',
    'portal mobile provider readiness'
  );
  assertEqual(
    readModel.portalDeviceSpine.artifactReadinessGates.physicalReadinessState,
    'manual-required',
    'portal physical artifact readiness'
  );
  proofLabels.push('route-custody.paired-and-rejected-states');
  proofLabels.push('selected-trusted-device.storage-security-follow-through');
  proofLabels.push('browser-lan-adapter.local-discovery-manual-physical-boundary');
  proofLabels.push('add-device-pairing.states-complete');
  proofLabels.push('trusted-device-registry.portal-read-model');
  proofLabels.push('portal-device-spine.device-readiness-states');
  proofLabels.push('portal-device-spine.route-and-provider-readiness');
  proofLabels.push('portal-device-spine.artifact-gates-manual');
  proofLabels.push('cloud-relay.not-implemented-manual-decision');
}

function requireRouteCheck(readModel, check) {
  const routeCheck = readModel.routeChecks.find((entry) => entry.check === check);
  if (!routeCheck) {
    throw new Error(`Missing product route check ${check}.`);
  }
  return routeCheck;
}

function requireMobileRoute(readModel, platform) {
  const route = readModel.mobileRoutes.find((entry) => entry.platform === platform);
  if (!route) {
    throw new Error(`Missing product mobile route ${platform}.`);
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
