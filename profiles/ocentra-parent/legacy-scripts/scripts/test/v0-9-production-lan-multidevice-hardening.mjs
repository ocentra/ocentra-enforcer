import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const outputDir = join(process.cwd(), 'test-results', 'v0-9-production-lan-multidevice-hardening');
const evidencePath = join(outputDir, 'proof.json');

const proofSteps = [
  {
    label: 'discovery-challenge',
    command: ['node', 'scripts/test/v0-9-lan-discovery-challenge-mvp.mjs'],
    evidencePath: join(process.cwd(), 'test-results', 'v0-9-lan-discovery-challenge-mvp', 'proof.json'),
    requiredAssertions: [
      'wrong-origin-websocket-rejected-before-upgrade',
      'first-discovery-agent:anonymous-control-rejected',
      'first-discovery-agent:wrong-origin-proof-rejected',
      'first-discovery-agent:malformed-proof-rejected',
      'first-discovery-agent:stale-proof-rejected',
      'first-discovery-agent:expired-challenge-rejected-as-stale',
      'first-discovery-agent:challenge-preview-issued',
      'first-discovery-agent:challenge-proof-accepted',
      'first-discovery-agent:challenge-proof-replay-rejected',
      'first-discovery-agent:route-selected-after-challenge',
      'first-discovery-agent:rule-query-accepted-after-challenge',
      'second-discovery-agent:anonymous-control-rejected',
      'second-discovery-agent:wrong-origin-proof-rejected',
      'second-discovery-agent:malformed-proof-rejected',
      'second-discovery-agent:stale-proof-rejected',
      'second-discovery-agent:expired-challenge-rejected-as-stale',
      'second-discovery-agent:challenge-preview-issued',
      'second-discovery-agent:challenge-proof-accepted',
      'second-discovery-agent:challenge-proof-replay-rejected',
      'second-discovery-agent:route-selected-after-challenge',
      'second-discovery-agent:rule-query-accepted-after-challenge',
      'wrong-agent-port-challenge-rejected-as-wrong-device',
    ],
  },
  {
    label: 'pairing-control',
    command: ['node', 'scripts/test/v0-9-lan-pairing-control-mvp.mjs'],
    evidencePath: join(process.cwd(), 'test-results', 'v0-9-lan-pairing-control-mvp', 'proof.json'),
    requiredAssertions: [
      'wrong-origin-websocket-rejected-before-upgrade',
      'first-child-agent:anonymous-rejected',
      'first-child-agent:pairing-proof-accepted-unselected',
      'first-child-agent:unselected-control-rejected',
      'first-child-agent:route-selected',
      'first-child-agent:selected-route-trust-state-paired',
      'first-child-agent:rule-query-accepted',
      'first-child-agent:observer-rule-query-accepted',
      'first-child-agent:observer-write-rejected',
      'first-child-agent:controller-lease-renewed',
      'first-child-agent:controller-lease-released',
      'first-child-agent:controller-lease-reacquired',
      'first-child-agent:replay-rejected',
      'first-child-agent:stale-control-rejected',
      'first-child-agent:malformed-control-rejected',
      'first-child-agent:missing-controller-lease-rejected',
      'first-child-agent:expired-controller-lease-rejected',
      'first-child-agent:wrong-controller-rejected',
      'first-child-agent:controller-lease-takeover-denied',
      'first-child-agent:lan-ai-provider-advertised',
      'first-child-agent:lan-ai-job-degraded',
      'first-child-agent:observer-lan-ai-job-rejected',
      'first-child-agent:route-revoked',
      'first-child-agent:revoked-control-rejected',
      'second-child-agent:anonymous-rejected',
      'second-child-agent:pairing-proof-accepted-unselected',
      'second-child-agent:unselected-control-rejected',
      'second-child-agent:route-selected',
      'second-child-agent:selected-route-trust-state-paired',
      'second-child-agent:rule-query-accepted',
      'second-child-agent:observer-rule-query-accepted',
      'second-child-agent:observer-write-rejected',
      'second-child-agent:controller-lease-renewed',
      'second-child-agent:controller-lease-released',
      'second-child-agent:controller-lease-reacquired',
      'second-child-agent:replay-rejected',
      'second-child-agent:stale-control-rejected',
      'second-child-agent:malformed-control-rejected',
      'second-child-agent:missing-controller-lease-rejected',
      'second-child-agent:expired-controller-lease-rejected',
      'second-child-agent:wrong-controller-rejected',
      'second-child-agent:controller-lease-takeover-denied',
      'second-child-agent:lan-ai-provider-advertised',
      'second-child-agent:lan-ai-job-degraded',
      'second-child-agent:observer-lan-ai-job-rejected',
      'second-child-agent:controller-lease-takeover-accepted',
      'second-child-agent:restart-restores-selected-route',
      'second-child-agent:restart-restores-selected-route-trust-state',
      'second-child-agent:restart-recovered-approval-accepted',
      'wrong-agent-port-rejected-as-wrong-device',
    ],
  },
  {
    label: 'lan-ai-provider-pool',
    command: ['node', 'scripts/test/platform-roles-lan-ai-provider-pool.mjs'],
    evidencePath: join(process.cwd(), 'test-results', 'platform-roles-lan-ai-provider-pool', 'proof.json'),
    requiredAssertions: [
      'parent-desktop-controller-ai-provider:provider-advertised-available',
      'parent-desktop-controller-ai-provider:controller-job-completed-observer-job-rejected',
      'parent-desktop-controller-ai-provider:unsupported-capability-rejected',
      'parent-mobile-observer-scaffold:provider-unavailable',
      'parent-mobile-observer-scaffold:controller-job-degraded-with-provider-unavailable',
      'parent-mobile-observer-scaffold:observer-job-rejected',
      'parent-desktop-busy-ai-provider:provider-busy',
      'parent-desktop-busy-ai-provider:busy-job-degraded',
      'parent-desktop-degraded-ai-provider:provider-degraded',
      'parent-desktop-degraded-ai-provider:degraded-job-degraded',
    ],
  },
  {
    label: 'rust-selected-device-state',
    command: [
      'cargo',
      'test',
      '-p',
      'ocentra-parent-agent-service',
      'lan_pairing_status_reports_stale_and_offline_selected_device_state',
    ],
    proofAssertions: [
      'rust-service:selected-device-stale-status-reported',
      'rust-service:selected-device-stale-control-rejected',
      'rust-service:selected-device-offline-status-reported',
      'rust-service:selected-device-offline-control-rejected',
    ],
  },
  {
    label: 'rust-trusted-registry-expiry-and-reachability',
    command: ['cargo', 'test', '-p', 'ocentra-parent-agent-core', 'trusted_device_registry'],
    proofAssertions: [
      'rust-core:expired-pairing-rejected',
      'rust-core:selected-device-stale-rejected',
      'rust-core:selected-device-offline-rejected',
      'rust-core:trusted-registry-persistence-covered',
    ],
  },
];

const manualTwoDeviceChecklist = [
  {
    label: 'two-physical-hosts',
    commands: [
      'cargo build -p ocentra-parent-agent-service',
      'set OCENTRA_PARENT_AGENT_ADDR=0.0.0.0:4477',
      'set OCENTRA_PARENT_AGENT_ALLOWED_ORIGINS=http://127.0.0.1:4478,http://<parent-lan-ip>:4478',
      'set OCENTRA_PARENT_AGENT_LOCAL_NETWORK_ENABLED=true',
      'target\\debug\\ocentra-parent-agent-service.exe',
      'node scripts/test/v0-9-production-lan-multidevice-hardening.mjs',
    ],
    requiredArtifacts: [
      'child service stdout/stderr with listening address and no secret-bearing payloads',
      'test-results/v0-9-production-lan-multidevice-hardening/proof.json',
      'test-results/v0-9-lan-discovery-challenge-mvp/proof.json',
      'test-results/v0-9-lan-pairing-control-mvp/proof.json',
      'test-results/platform-roles-lan-ai-provider-pool/proof.json',
      'parent and child host names or IPs showing two distinct LAN devices',
      'firewall/router note proving the child port is reachable from the parent host',
      'offline or stale selected-device artifact from stopping/pausing the selected child service before a control command',
    ],
    currentStatus: 'manual-required-physical-devices-not-claimed-by-local-harness',
  },
];

await mkdir(outputDir, { recursive: true });

const checkedSteps = [];
for (const step of proofSteps) {
  await runStep(step);
  const checkedStep = {
    label: step.label,
    command: step.command.join(' '),
  };
  if (step.evidencePath !== undefined) {
    const evidence = JSON.parse(await readFile(step.evidencePath, 'utf8'));
    assertRequiredAssertions(step, evidence.assertions ?? []);
    checkedStep.evidencePath = relativeToWorkspace(step.evidencePath);
    checkedStep.assertionCount = evidence.assertions?.length ?? 0;
    checkedStep.requiredAssertions = step.requiredAssertions;
  } else {
    checkedStep.assertionCount = step.proofAssertions.length;
    checkedStep.requiredAssertions = step.proofAssertions;
  }
  checkedSteps.push(checkedStep);
}

const discoveryEvidence = await readStepEvidence('discovery-challenge');
const pairingEvidence = await readStepEvidence('pairing-control');
const providerEvidence = await readStepEvidence('lan-ai-provider-pool');
const localTwoServiceProof = buildLocalTwoServiceProof(pairingEvidence);
const controllerAuthorityProof = buildControllerAuthorityProof(pairingEvidence);
const parentMobileControllerObserverProof = buildParentMobileControllerObserverProof(providerEvidence);

const proof = {
  schemaVersion: 1,
  checkedAt: new Date().toISOString(),
  proofMode: 'local-multi-service-production-lan-hardening',
  checkedSteps,
  localTwoServiceProof,
  discoveryProof: {
    proofBoundary: 'local-real-service-discovery-processes',
    wrongOriginRejectedBeforeUpgrade: assertionPresent(
      discoveryEvidence,
      'wrong-origin-websocket-rejected-before-upgrade'
    ),
    wrongDeviceChallengeRejected: assertionPresent(
      discoveryEvidence,
      'wrong-agent-port-challenge-rejected-as-wrong-device'
    ),
    replayRejected: assertionsWithSuffix(discoveryEvidence, 'challenge-proof-replay-rejected'),
    staleOrExpiredRejected: assertionsWithSuffix(discoveryEvidence, 'stale-proof-rejected').concat(
      assertionsWithSuffix(discoveryEvidence, 'expired-challenge-rejected-as-stale')
    ),
  },
  controllerAuthorityProof,
  parentMobileControllerObserverProof,
  cloudRelayDecision: {
    state: 'not-implemented',
    proofBoundary: 'no-cloud-relay-contract-or-runtime-in-this-v0-9-proof',
  },
  claimsProvedLocally: [
    'production LAN states use explicit discovered/pending/paired/revoked/stale/offline/unavailable contract values',
    'trusted registry persists selected route and recovers it after restart',
    'selected route status reports selected pairing id, selected-route trust state, stale time, and offline time',
    'active controller write authority rejects observer writes, stale intents, replay, wrong device, and denied takeover',
    'active controller proof rejects observer writes, stale intents, replayed intents, wrong-device targets, missing or expired leases, wrong controllers, revoked pairings, and denied takeover',
    'direct discovery proof rejects wrong-origin proof, malformed proof, stale proof, expired challenge, replayed proof, and wrong-device challenge traffic',
    'selected-device stale and offline read-model states reject control through focused Rust service and core registry proof',
    'LAN AI provider routing covers authorized result, unsupported capability, busy, degraded, unavailable, and observer rejection',
    'LAN AI provider routing covers degraded provider state without claiming real household provider readiness',
    'revocation is observed before subsequent control rejection in the local proof artifact',
    'parent mobile controller/observer state remains backend-scaffold or manual-required instead of mobile UX parity',
  ],
  claimsNotProvedLocally: [
    'real household router discovery across two physical devices',
    'OS firewall prompts and mobile background behavior on Windows/macOS/Linux/Android/iOS',
    'device-owner policy, iOS Family Controls, app-store or MDM deployment behavior',
    'cloud relay routing, storage, or authentication behavior',
  ],
  manualTwoDeviceChecklist,
};

await writeFile(evidencePath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`v0-9-production-lan-multidevice-hardening-ok:${checkedSteps.map((step) => step.label).join(',')}`);

async function runStep(step) {
  await new Promise((resolve, reject) => {
    const child = spawn(step.command[0], step.command.slice(1), {
      cwd: process.cwd(),
      env: process.env,
      shell: false,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const chunks = [];
    child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
    child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${step.label} failed with exit code ${code}\n${chunks.join('')}`));
    });
  });
}

function assertRequiredAssertions(step, assertions) {
  for (const required of step.requiredAssertions) {
    if (!assertions.includes(required)) {
      throw new Error(`${step.label} evidence is missing required assertion ${required}`);
    }
  }
}

function relativeToWorkspace(path) {
  return path.replace(`${process.cwd()}\\`, '').replaceAll('\\', '/');
}

async function readStepEvidence(label) {
  const step = proofSteps.find((candidate) => candidate.label === label);
  if (!step?.evidencePath) {
    throw new Error(`Missing evidence step ${label}`);
  }
  return JSON.parse(await readFile(step.evidencePath, 'utf8'));
}

function buildLocalTwoServiceProof(pairingEvidence) {
  const services = pairingEvidence.services ?? [];
  if (services.length !== 2) {
    throw new Error(`Expected two local service processes in pairing proof, received ${services.length}`);
  }
  return {
    proofBoundary: 'local-two-service-mechanical-proof',
    serviceCount: services.length,
    services: services.map((service) => ({
      label: service.label,
      childDeviceId: service.childDeviceId,
      registryPersistence: service.registryPersistence,
    })),
    selectedRouteRecovery: assertionsWithSuffix(pairingEvidence, 'restart-restores-selected-route'),
    selectedRouteTrust: assertionsWithSuffix(pairingEvidence, 'selected-route-trust-state-paired').concat(
      assertionsWithSuffix(pairingEvidence, 'restart-restores-selected-route-trust-state')
    ),
    acceptedAfterRestart: assertionsWithSuffix(pairingEvidence, 'restart-recovered-approval-accepted'),
    wrongDeviceRejected: assertionPresent(pairingEvidence, 'wrong-agent-port-rejected-as-wrong-device'),
  };
}

function buildControllerAuthorityProof(pairingEvidence) {
  const assertions = pairingEvidence.assertions ?? [];
  const revocationIndex = assertions.indexOf('first-child-agent:route-revoked');
  const rejectedIndex = assertions.indexOf('first-child-agent:revoked-control-rejected');
  if (revocationIndex < 0 || rejectedIndex < 0 || revocationIndex > rejectedIndex) {
    throw new Error('Expected route revocation to be recorded before revoked control rejection.');
  }
  return {
    proofBoundary: 'local-controller-authority-real-service-proof',
    observerReadOnlyRejected: assertionsWithSuffix(pairingEvidence, 'observer-write-rejected'),
    observerReadAllowed: assertionsWithSuffix(pairingEvidence, 'observer-rule-query-accepted'),
    leaseLifecycle: assertionsWithSuffix(pairingEvidence, 'controller-lease-renewed')
      .concat(assertionsWithSuffix(pairingEvidence, 'controller-lease-released'))
      .concat(assertionsWithSuffix(pairingEvidence, 'controller-lease-reacquired')),
    takeover: assertionsWithSuffix(pairingEvidence, 'controller-lease-takeover-denied').concat(
      assertionsWithSuffix(pairingEvidence, 'controller-lease-takeover-accepted')
    ),
    dishonestStateRejections: assertionsWithSuffix(pairingEvidence, 'replay-rejected')
      .concat(assertionsWithSuffix(pairingEvidence, 'stale-control-rejected'))
      .concat(assertionsWithSuffix(pairingEvidence, 'missing-controller-lease-rejected'))
      .concat(assertionsWithSuffix(pairingEvidence, 'expired-controller-lease-rejected'))
      .concat(assertionsWithSuffix(pairingEvidence, 'wrong-controller-rejected')),
    revocationBeforeControl: {
      routeRevokedAssertion: assertions[revocationIndex],
      controlRejectedAssertion: assertions[rejectedIndex],
    },
  };
}

function buildParentMobileControllerObserverProof(providerEvidence) {
  return {
    proofBoundary: 'parent-mobile-backend-scaffold-without-mobile-ux-parity',
    observerReadOnlyRejected: assertionPresent(
      providerEvidence,
      'parent-mobile-observer-scaffold:observer-job-rejected'
    ),
    controllerJobDegraded: assertionPresent(
      providerEvidence,
      'parent-mobile-observer-scaffold:controller-job-degraded-with-provider-unavailable'
    ),
    providerUnavailable: assertionPresent(providerEvidence, 'parent-mobile-observer-scaffold:provider-unavailable'),
    mobileWriteAuthorityState: 'manual-required-real-mobile-package-proof',
  };
}

function assertionsWithSuffix(evidence, suffix) {
  return (evidence.assertions ?? []).filter((assertion) => assertion.endsWith(suffix));
}

function assertionPresent(evidence, assertion) {
  if (!(evidence.assertions ?? []).includes(assertion)) {
    throw new Error(`Expected evidence assertion ${assertion}`);
  }
  return assertion;
}
