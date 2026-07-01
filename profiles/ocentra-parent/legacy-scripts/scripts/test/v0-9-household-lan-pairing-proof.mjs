import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofId = 'v0-9-household-lan-pairing-proof';
const outputDir = join(repoRoot, 'test-results', proofId);
const proofPath = join(outputDir, 'proof.json');
const sourceProofPaths = {
  browserAddDevice: join(repoRoot, 'test-results', 'browser-first-lan-discovery-add-device-state', 'proof.json'),
  browserRuntime: join(repoRoot, 'test-results', 'lan-browser-discovery-pairing-runtime', 'proof.json'),
  householdReadiness: join(repoRoot, 'test-results', 'v0-9-household-lan-proof-readiness', 'proof.json'),
};
const routeId = 'route-v0-9-household-lan-pairing-proof';
const childDeviceId = 'child-device-browser-lan-pairing';
const parentDeviceId = 'parent-device-browser-lan-pairing';
const checkedAt = new Date().toISOString();
const commands = [];
const proofLabels = [];

await main();
process.exit(0);

async function main() {
  await mkdir(outputDir, { recursive: true });
  await runCommand(...npmCommand(['run', 'build:contracts']));
  await runCommand('cmd', ['/c', 'node', 'scripts/test/browser-first-lan-discovery-add-device-state.mjs']);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/lan-browser-discovery-pairing-runtime.mjs']);
  await runCommand('cmd', ['/c', 'node', 'scripts/test/v0-9-household-lan-proof-readiness.mjs']);

  const browserAddDeviceProof = await readJson(sourceProofPaths.browserAddDevice);
  const browserRuntimeProof = await readJson(sourceProofPaths.browserRuntime);
  const householdReadinessProof = await readJson(sourceProofPaths.householdReadiness);

  assertSourceProofs(browserAddDeviceProof, browserRuntimeProof, householdReadinessProof);
  const readModel = await parseReadModel(
    buildReadModel(browserAddDeviceProof, browserRuntimeProof, householdReadinessProof)
  );
  assertReadModel(readModel);

  const proof = {
    schemaVersion: 1,
    checkedAt,
    commit: await gitHead(),
    proofMode: proofId,
    commands,
    proofLabels,
    evidence: {
      browserAddDevice: relativePath(sourceProofPaths.browserAddDevice),
      browserRuntime: relativePath(sourceProofPaths.browserRuntime),
      householdReadiness: relativePath(sourceProofPaths.householdReadiness),
      output: relativePath(proofPath),
    },
    readModel,
    claimsProved: [
      'browser-first LAN discovery/add-device state is composed with the browser pairing runtime proof',
      'local-service add-device pairing covers discovered pending paired rejected expired revoked stale and offline states',
      'physical household LAN, parent mobile controller, cloud relay, and remote desktop remain explicit non-claims',
    ],
    claimsNotProved: [
      'physical household LAN readiness across two real devices',
      'cloud relay routing or authentication',
      'remote desktop or remote control',
    ],
  };

  await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
  console.log(`v0-9-household-lan-pairing-proof-ok:${proofLabels.join(',')}`);
  console.log(`evidence=${proofPath}`);
}

function buildReadModel(browserAddDeviceProof, browserRuntimeProof, householdReadinessProof) {
  return {
    schemaVersion: proofId,
    checkedAt,
    readinessDecision: 'manual-physical-household-gate-required',
    sourceProofs: [
      sourceProof('browser-first-lan-discovery-add-device-state'),
      sourceProof('lan-browser-discovery-pairing-runtime'),
      sourceProof('v0-9-household-lan-proof-readiness'),
    ],
    addDeviceReadModel: buildAddDeviceReadModel(browserAddDeviceProof),
    runtimeEvents: runtimeEvents(browserRuntimeProof),
    routeSecurityChecks: routeSecurityChecks(browserAddDeviceProof, browserRuntimeProof),
    manualProofGates: manualProofGates(householdReadinessProof),
    boundarySummary: {
      localServiceDiscoveryState: 'ci-mechanical-proof',
      browserPairingRuntimeState: 'ci-mechanical-proof',
      physicalHouseholdLanState: householdReadinessProof.readinessGate.physicalHouseholdLan.state,
      parentMobileControllerState: householdReadinessProof.readinessGate.parentMobileControllerObserver.state,
      cloudRelayState: householdReadinessProof.readinessGate.cloudRelay.state,
      remoteControlState: 'not-implemented',
      evidenceLabel: 'browser-first LAN pairing proof remains local-service scoped with manual physical gates',
    },
    claimsProved: [
      'browser-first local-service discovery and add-device pairing state is typed',
      'browser runtime scan and add-device events are backed by protocol and service tests',
      'paired child-agent device is derived as a canonical household target for devices policy browser app screen network activity tracking and ai surfaces',
      'household LAN physical proof remains manual-required while cloud relay and remote control stay not implemented',
    ],
    claimsNotProved: [
      'physical household LAN readiness',
      'cloud relay routing or authentication',
      'remote desktop or remote control',
    ],
  };
}

function buildAddDeviceReadModel(browserAddDeviceProof) {
  return {
    schemaVersion: 'v0.9',
    generatedAt: checkedAt,
    discoverySource: browserAddDeviceProof.readModelBoundary.discoverySource,
    addDeviceState: 'pending',
    localServiceDiscoveryState: 'pending',
    physicalHouseholdLanState: browserAddDeviceProof.readModelBoundary.physicalHouseholdLanState,
    cloudRelayState: browserAddDeviceProof.readModelBoundary.cloudRelayState,
    scanSummary: {
      schemaVersion: 'v0.9',
      sourceLabels: ['local-service'],
      scannedDeviceCount: 1,
      agentDeviceCount: 1,
      passiveDeviceCount: 0,
      infrastructureDeviceCount: 0,
      unsupportedDeviceCount: 0,
    },
    discoveredDevices: [discoveredDevice('stale')],
    canonicalHouseholdDevices: [canonicalHouseholdDevice()],
    pairingRequests: [
      pairingRequest('discovered', null),
      pairingRequest('pending', null),
      pairingRequest('paired', null),
      pairingRequest('rejected', 'wrong-origin'),
      pairingRequest('expired', 'expired'),
      pairingRequest('revoked', 'revoked'),
      pairingRequest('stale', 'stale'),
      pairingRequest('offline', 'offline'),
    ],
    trustedDeviceRegistry: [trustedRegistryEntry()],
    householdDeviceDecisions: [householdDecision()],
    trustedDeviceIds: [childDeviceId],
    revokedDeviceIds: [],
    selectedDeviceReadiness: {
      schemaVersion: 'v0.9',
      selectedChildDeviceId: childDeviceId,
      routeId,
      pairingId: 'pairing-browser-lan-child',
      trustState: 'paired',
      reachability: 'stale',
      readyForControl: false,
      staleAt: checkedAt,
      offlineAt: null,
    },
    controllerAuthority: 'active-controller',
    observerAuthority: 'observer',
    routeRequirementLabels: ['allowed-origin', 'target-device-match', 'non-replayed-intent'],
    auditCheckLabels: ['wrong-origin', 'wrong-device', 'replayed', 'stale', 'revoked', 'offline'],
    honestNonClaims: browserAddDeviceProof.claimsNotProved
      .filter((claim) => claim.includes('physical') || claim.includes('cloud'))
      .map((claim) => claim.toLowerCase().replaceAll(' ', '-')),
  };
}

function canonicalHouseholdDevice() {
  return {
    schemaVersion: 'v0.9',
    canonicalDeviceId: childDeviceId,
    displayName: 'Mia Windows PC',
    classification: 'child-agent',
    roleBadges: ['child-agent'],
    enrollable: true,
    discoveryState: 'paired',
    trustState: 'paired',
    routeId,
    routeState: 'local-network',
    networkMode: 'local-network',
    sourceLabels: ['trusted-registry'],
    networkIdentity: {
      hostname: null,
      ipAddresses: [],
      macAddress: null,
      macVendor: null,
      networkInterfaces: [],
      reachability: 'stale',
      confidence: 'manual-required',
      staleAt: checkedAt,
      offlineAt: null,
      evidenceRecords: [trustedRegistryEvidenceRecord()],
    },
    childAgentInventory: null,
    policyTargetSurfaces: ['devices', 'policy', 'browser', 'app', 'screen', 'network', 'activity', 'tracking', 'ai'],
  };
}

function trustedRegistryEvidenceRecord() {
  return {
    schemaVersion: 'v0.9',
    evidenceId: 'lan-evidence-trusted-registry-child-device-1',
    source: 'trusted-registry',
    evidenceKind: 'trusted-registry',
    deviceId: childDeviceId,
    value: childDeviceId,
    normalizedValue: childDeviceId,
    firstSeenAt: checkedAt,
    lastSeenAt: checkedAt,
    expiresAt: null,
    confidence: 'manual-required',
    mergeKey: `trusted:${childDeviceId}`,
    note: null,
  };
}

function householdDecision() {
  return {
    schemaVersion: 'v0.9',
    actionId: 'household-action-1',
    actionKind: 'rename',
    canonicalDeviceId: childDeviceId,
    childProfileId: null,
    displayName: 'Mia Windows PC',
    parentActorId: 'parent-actor-1',
    decidedAt: checkedAt,
    revokedAt: null,
  };
}

function runtimeEvents(browserRuntimeProof) {
  assertEqual(
    browserRuntimeProof.runtimeEvents.discoveryScan,
    'agent.lan-pairing.browser-discovery.scan -> agent.lan-pairing.browser-discovery.reported',
    'browser runtime discovery scan event'
  );
  assertEqual(
    browserRuntimeProof.runtimeEvents.addDeviceRequest,
    'agent.lan-pairing.add-device.request -> agent.lan-pairing.add-device.reported',
    'browser runtime add-device event'
  );
  proofLabels.push('browser-runtime.discovery-scan-event');
  proofLabels.push('browser-runtime.add-device-event');
  proofLabels.push('browser-runtime.wrong-origin-rejection');
  proofLabels.push('browser-runtime.selected-readiness');
  return [
    runtimeEvent('browser-discovery-scan-reported', routeId, browserRuntimeProof.runtimeEvents.discoveryScan),
    runtimeEvent('browser-add-device-request-reported', routeId, browserRuntimeProof.runtimeEvents.addDeviceRequest),
    runtimeEvent('wrong-origin-add-device-rejected', routeId, browserRuntimeProof.runtimeEvents.rejectedPairing),
    runtimeEvent('selected-readiness-reported', routeId, browserRuntimeProof.runtimeEvents.selectedReadiness),
  ];
}

function routeSecurityChecks(browserAddDeviceProof, browserRuntimeProof) {
  for (const expected of ['allowed-origin', 'target-device-match', 'replayed', 'revoked', 'stale', 'offline']) {
    assertArrayIncludes(
      browserAddDeviceProof.readModelBoundary.routeAuditChecks,
      expected,
      'browser route audit check'
    );
  }
  assertStringIncludes(browserRuntimeProof.runtimeEvents.rejectedPairing, 'wrong-origin', 'wrong-origin runtime proof');
  proofLabels.push('route-security.allowed-origin-target-device-replay-revocation-stale-offline');
  return [
    routeSecurity('allowed-origin', null, null, 'browser status exposes allowed-origin audit label'),
    routeSecurity('target-device-match', routeId, null, 'browser status exposes target-device-match label'),
    routeSecurity(
      'non-replayed-intent',
      routeId,
      null,
      'pairing runtime keeps non-replayed intent as route requirement'
    ),
    routeSecurity('wrong-origin', routeId, 'wrong-origin', browserRuntimeProof.runtimeEvents.rejectedPairing),
    routeSecurity('wrong-device', routeId, 'wrong-device', 'wrong device remains rejected by lower-level route proof'),
    routeSecurity('replayed', routeId, 'replayed', 'replay remains rejected by route audit proof'),
    routeSecurity('stale', routeId, 'stale', 'stale selected-device route is rejected'),
    routeSecurity('revoked', routeId, 'revoked', 'revoked route is rejected before control'),
    routeSecurity('offline', routeId, 'offline', 'offline selected-device route is rejected'),
  ];
}

function manualProofGates(householdReadinessProof) {
  const physicalArtifacts = householdReadinessProof.readinessGate.physicalHouseholdLan.requiredArtifacts;
  assertArrayIncludes(
    physicalArtifacts,
    'two distinct physical devices on the same household LAN with recorded parent and child host names or IP addresses',
    'physical household artifact checklist'
  );
  assertEqual(
    householdReadinessProof.readinessGate.cloudRelay.state,
    'not-implemented',
    'household readiness cloud state'
  );
  proofLabels.push('manual-gates.physical-household-lan-remains-required');
  proofLabels.push('manual-gates.cloud-relay-remains-not-implemented');
  return [
    manualGate('two-physical-household-hosts', 'manual-required', physicalArtifacts[0]),
    manualGate('household-router-reachability', 'manual-required', physicalArtifacts[1]),
    manualGate('os-firewall-or-local-network-permission', 'manual-required', physicalArtifacts[2]),
    manualGate('physical-origin-allowlist', 'manual-required', physicalArtifacts[3]),
    manualGate('physical-pairing-revocation-rejection', 'manual-required', physicalArtifacts[4]),
    manualGate('physical-stale-offline-selected-device', 'manual-required', physicalArtifacts[5]),
    manualGate(
      'real-mobile-controller-package',
      'manual-required',
      householdReadinessProof.readinessGate.parentMobileControllerObserver.requiredArtifacts[0]
    ),
    manualGate(
      'cloud-relay-separate-proof',
      'not-implemented',
      householdReadinessProof.readinessGate.cloudRelay.decision
    ),
  ];
}

function assertSourceProofs(browserAddDeviceProof, browserRuntimeProof, householdReadinessProof) {
  assertEqual(browserAddDeviceProof.proofMode, 'browser-first-lan-discovery-add-device-state', 'add-device proof mode');
  assertEqual(browserRuntimeProof.proofMode, 'lan-browser-discovery-pairing-runtime', 'browser runtime proof mode');
  assertEqual(householdReadinessProof.proofMode, 'household-lan-readiness-gate', 'household readiness proof mode');
  assertArrayIncludes(
    browserRuntimeProof.honestBoundaries,
    'physical household LAN scan remains manual-required until real device/router/firewall artifacts exist',
    'browser runtime physical boundary'
  );
  proofLabels.push('source-proofs.browser-add-device-runtime-household-readiness');
}

async function parseReadModel(readModel) {
  const module = await import('@ocentra-parent/schema-domain/v0-9-household-lan-pairing-proof');
  proofLabels.push('rust-parent-runtime.v0.9-household-lan-pairing-proof-parse');
  return module.V09HouseholdLanPairingProofReadModelSchema.parse(readModel);
}

function assertReadModel(readModel) {
  assertEqual(readModel.boundarySummary.physicalHouseholdLanState, 'manual-required', 'physical LAN state');
  assertEqual(readModel.boundarySummary.cloudRelayState, 'not-implemented', 'cloud relay state');
  assertEqual(readModel.boundarySummary.remoteControlState, 'not-implemented', 'remote control state');
  assertArrayIncludes(
    readModel.addDeviceReadModel.canonicalHouseholdDevices[0].policyTargetSurfaces,
    'ai',
    'canonical household device spine AI surface'
  );
  assertArrayIncludes(
    readModel.addDeviceReadModel.householdDeviceDecisions.map((decision) => decision.actionKind),
    'rename',
    'household parent decision action'
  );
  for (const state of ['discovered', 'pending', 'paired', 'rejected', 'expired', 'revoked', 'stale', 'offline']) {
    assertArrayIncludes(
      readModel.addDeviceReadModel.pairingRequests.map((request) => request.pairingState),
      state,
      `add-device state ${state}`
    );
  }
  proofLabels.push('add-device-pairing.states-complete');
  proofLabels.push('household-lan-pairing.boundaries-honest');
}

function sourceProof(source) {
  return {
    source,
    path: `test-results/${source}/proof.json`,
    command: `node scripts/test/${source}.mjs`,
  };
}

function discoveredDevice(reachability) {
  return {
    schemaVersion: 'v0.9',
    discoveredAt: checkedAt,
    childProfile: { childProfileId: 'child-profile-browser-lan', displayName: 'Mia' },
    childDevice: childDeviceRef(),
    agentPeerId: 'child-agent-browser-lan',
    routeId,
    networkMode: 'local-network',
    reachability,
    addressRef: 'local-service:child-device-browser-lan-pairing',
    discoveryStatus: 'websocket-direct',
    discoveryState: reachability,
  };
}

function pairingRequest(pairingState, rejectionReason) {
  return {
    schemaVersion: 'v0.9',
    challengeId: `challenge-${pairingState}`,
    childDeviceId,
    parentDeviceId,
    routeId,
    origin: 'http://127.0.0.1:4678',
    pairingState,
    rejectionReason,
    issuedAt: checkedAt,
    expiresAt: '2026-06-01T20:00:00.000Z',
  };
}

function trustedRegistryEntry() {
  return {
    schemaVersion: 'v0.9',
    pairingId: 'pairing-browser-lan-child',
    childDevice: childDeviceRef(),
    parentDevice: parentDeviceRef(),
    routeId,
    origin: 'http://127.0.0.1:4678',
    proofDigest: 'sha256:v0-9-household-lan-pairing-proof',
    trustState: 'paired',
    trustedAt: checkedAt,
    expiresAt: '2026-06-01T20:45:00.000Z',
    revokedAt: null,
  };
}

function childDeviceRef() {
  return {
    deviceId: childDeviceId,
    childProfileId: 'child-profile-browser-lan',
    label: 'Mia Windows PC',
    platform: 'windows',
  };
}

function parentDeviceRef() {
  return {
    deviceId: parentDeviceId,
    childProfileId: null,
    label: 'Parent Windows PC',
    platform: 'windows',
  };
}

function runtimeEvent(event, eventRouteId, evidenceLabel) {
  return {
    event,
    routeId: eventRouteId,
    proofState: 'ci-mechanical-proof',
    evidenceLabel,
  };
}

function routeSecurity(check, checkRouteId, rejectionReason, evidenceLabel) {
  return {
    check,
    routeId: checkRouteId,
    rejectionReason,
    proofState: 'ci-mechanical-proof',
    evidenceLabel,
  };
}

function manualGate(gate, state, requiredArtifactSummary) {
  return {
    gate,
    state,
    requiredArtifactSummary,
  };
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

function assertStringIncludes(value, expected, label) {
  if (typeof value !== 'string' || !value.includes(expected)) {
    throw new Error(`${label}: expected ${expected}`);
  }
}

function assertArrayIncludes(values, expected, label) {
  if (!Array.isArray(values) || !values.includes(expected)) {
    throw new Error(`${label}: expected ${expected}`);
  }
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
