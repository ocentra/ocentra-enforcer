import { spawn } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { AgentEventEnvelopeSchema } from '@ocentra-parent/schema-domain/agent-command-event-contracts';
import {
  ParentDevEnv,
  ParentDevHost,
  ParentDevValue,
  createAgentAddress,
  createAgentHealthUrl,
  createAgentWebSocketUrl,
  createHttpOrigin,
  isLikelyParentAgentOccupant,
} from '../dev/local-dev-config.mjs';
import { ensurePortFree } from '../dev/port-utils.mjs';
import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const outputDir = join(process.cwd(), 'test-results', 'v0-9-lan-discovery-challenge-mvp');
const evidencePath = join(outputDir, 'proof.json');
const allowedOrigin = createHttpOrigin(ParentDevHost.Loopback);
const wrongOrigin = createHttpOrigin(ParentDevHost.Loopback, 9478);
const issuedAt = '2026-05-26T20:20:00.000Z';
const expiresAt = '2099-05-26T20:25:00.000Z';
const expiredAt = '2026-05-25T20:19:00.000Z';
const platform = 'windows';
const parentDeviceId = 'parent-device-v09-discovery';
const controllerLeaseId = 'controller-lease-v09-discovery';
const controllerDeviceId = 'parent-controller-v09-discovery';
const parentActorId = 'parent-actor-v09-discovery';
const parentAuthorityActiveController = 'active-controller';
const controllerLeaseExpiresAt = '2099-05-26T20:24:00.000Z';
const webSocketEventTimeoutMs = 20000;
const sensitiveMarkers = [
  'activityDigest',
  'activity.sqlite',
  'decryptedEvidence',
  'journalPath',
  'rawEvidence',
  'rawProofSecret',
  'rawToken',
  'sqlitePath',
];

const agents = [
  {
    label: 'first-discovery-agent',
    port: 4494,
    childDeviceId: 'child-device-v09-discovery-first',
    pairingId: 'pairing-v09-discovery-first',
    routeId: 'route-v09-discovery-first-local-network',
    evidenceReferenceIds: 'activity-event-v09-discovery-first',
  },
  {
    label: 'second-discovery-agent',
    port: 4495,
    childDeviceId: 'child-device-v09-discovery-second',
    pairingId: 'pairing-v09-discovery-second',
    routeId: 'route-v09-discovery-second-local-network',
    evidenceReferenceIds: 'activity-event-v09-discovery-second',
  },
];

await rm(outputDir, { recursive: true, force: true });
await mkdir(outputDir, { recursive: true });

for (const agent of agents) {
  await ensurePortFree(agent.port, isLikelyParentAgentOccupant, console.log, ParentDevHost.Wildcard);
}

const services = agents.map(spawnAgentService);
const assertions = [];

try {
  await Promise.all(services.map((service) => waitForHttp(service.healthUrl, service)));
  await assertWrongOriginWebSocketRejected(services[0]);
  assertions.push('wrong-origin-websocket-rejected-before-upgrade');

  for (const service of services) {
    const labels = await runDiscoveryChallengeCeremony(service);
    assertions.push(...labels);
  }

  await assertWrongAgentPortChallengeRejected(services[0], services[1]);
  assertions.push('wrong-agent-port-challenge-rejected-as-wrong-device');

  await writeEvidence(assertions, services);
  console.log(`v0-9-lan-discovery-challenge-mvp-ok:${assertions.join(',')}`);
} finally {
  await Promise.allSettled(services.map((service) => stopProcessTreeAndWait(service.child)));
}

function spawnAgentService(agent) {
  const registryPath = join(outputDir, `${agent.label}-registry.json`);
  const service = spawn(resolveDebugAgentServicePath(), [], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      [ParentDevEnv.AgentAddress]: createAgentAddress(agent.port, ParentDevHost.Wildcard),
      [ParentDevEnv.AgentAllowedOrigins]: allowedOrigin,
      [ParentDevEnv.AgentLocalNetworkEnabled]: ParentDevValue.True,
      OCENTRA_PARENT_AGENT_LAN_CHILD_DEVICE_ID: agent.childDeviceId,
      OCENTRA_PARENT_AGENT_LAN_PAIRING_REGISTRY_PATH: registryPath,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  return {
    ...agent,
    child: service,
    healthUrl: createAgentHealthUrl(agent.port),
    registryPath,
    serviceOutput: collectOutput(service),
    wsUrl: createAgentWebSocketUrl(agent.port),
  };
}

async function runDiscoveryChallengeCeremony(service) {
  const socket = await openWebSocket(service, allowedOrigin);
  try {
    const labels = [];
    const anonymous = await sendCommand(socket, buildHealthCommand(service, 'anonymous-before-challenge', {}));
    assertEvent(anonymous, 'agent.command.rejected');
    assertPayloadValue(anonymous.payload, 'rejectionReason', 'anonymous');
    labels.push(`${service.label}:anonymous-control-rejected`);

    const wrongOriginProof = await issueChallengeAndSubmitProof(socket, service, 'wrong-origin-proof', {
      origin: wrongOrigin,
    });
    assertEvent(wrongOriginProof, 'agent.command.rejected');
    assertPayloadValue(wrongOriginProof.payload, 'auditEventType', 'pairing-proof-rejected');
    assertPayloadValue(wrongOriginProof.payload, 'rejectionReason', 'wrong-origin');
    labels.push(`${service.label}:wrong-origin-proof-rejected`);

    const malformedProof = await issueChallengeAndSubmitProof(socket, service, 'malformed-proof', {
      proofDigest: undefined,
    });
    assertEvent(malformedProof, 'agent.command.rejected');
    assertPayloadValue(malformedProof.payload, 'rejectionReason', 'malformed');
    labels.push(`${service.label}:malformed-proof-rejected`);

    const staleProof = await issueChallengeAndSubmitProof(socket, service, 'stale-proof', {
      staleAt: expiredAt,
    });
    assertEvent(staleProof, 'agent.command.rejected');
    assertPayloadValue(staleProof.payload, 'rejectionReason', 'stale');
    labels.push(`${service.label}:stale-proof-rejected`);

    const expiredChallenge = await sendCommand(
      socket,
      buildChallengeCommand(service, 'expired-challenge', { staleAt: expiredAt })
    );
    assertEvent(expiredChallenge, 'agent.command.rejected');
    assertPayloadValue(expiredChallenge.payload, 'rejectionReason', 'stale');
    labels.push(`${service.label}:expired-challenge-rejected-as-stale`);

    const challenge = await issueChallenge(socket, service, 'accepted-proof');
    assertChallengePreview(challenge.payload, service);
    labels.push(`${service.label}:challenge-preview-issued`);

    const acceptedProof = await sendCommand(socket, buildPairingCommand(service, challenge.payload, 'accepted-proof'));
    assertEvent(acceptedProof, 'agent.lan-pairing.status.reported');
    assertPayloadValue(acceptedProof.payload, 'auditEventType', 'pairing-proof-accepted');
    assertPayloadValue(acceptedProof.payload, 'trustedDeviceIds', service.childDeviceId);
    assertPayloadValue(acceptedProof.payload, 'discoveryState', 'paired');
    labels.push(`${service.label}:challenge-proof-accepted`);

    const replayedProof = await sendCommand(socket, buildPairingCommand(service, challenge.payload, 'replayed-proof'));
    assertEvent(replayedProof, 'agent.command.rejected');
    assertPayloadValue(replayedProof.payload, 'rejectionReason', 'replayed');
    labels.push(`${service.label}:challenge-proof-replay-rejected`);

    const selected = await sendCommand(socket, buildRouteSelectCommand(service, 'route-select-after-challenge'));
    assertEvent(selected, 'agent.lan-pairing.status.reported');
    assertPayloadValue(selected.payload, 'selectedChildDeviceId', service.childDeviceId);
    assertPayloadValue(selected.payload, 'discoveryState', 'paired');
    labels.push(`${service.label}:route-selected-after-challenge`);

    const acceptedControl = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'accepted-rule-query-after-challenge',
        intentPayload(service, 'intent-rule-query-after-challenge', 'rule-query')
      )
    );
    assertEvent(acceptedControl, 'agent.health.reported');
    assertPayloadValue(acceptedControl.payload, 'controlState', 'accepted');
    assertPayloadValue(acceptedControl.payload, 'intentKind', 'rule-query');
    labels.push(`${service.label}:rule-query-accepted-after-challenge`);

    return labels;
  } finally {
    socket.close();
  }
}

async function issueChallengeAndSubmitProof(socket, service, messageSuffix, overrides) {
  const challenge = await issueChallenge(socket, service, messageSuffix);
  assertChallengePreview(challenge.payload, service);
  return sendCommand(socket, buildPairingCommand(service, challenge.payload, messageSuffix, overrides));
}

async function issueChallenge(socket, service, messageSuffix) {
  const challenge = await sendCommand(socket, buildChallengeCommand(service, messageSuffix));
  assertEvent(challenge, 'agent.lan-pairing.status.reported');
  assertPayloadValue(challenge.payload, 'auditEventType', 'pairing-challenge-issued');
  return challenge;
}

async function assertWrongAgentPortChallengeRejected(firstService, secondService) {
  const socket = await openWebSocket(firstService, allowedOrigin);
  try {
    const wrongPortChallenge = await sendCommand(
      socket,
      buildChallengeCommand(secondService, 'wrong-agent-port-challenge')
    );
    assertEvent(wrongPortChallenge, 'agent.command.rejected');
    assertPayloadValue(wrongPortChallenge.payload, 'rejectionReason', 'wrong-device');
  } finally {
    socket.close();
  }
}

async function assertWrongOriginWebSocketRejected(service) {
  await new Promise((resolve, reject) => {
    const socket = new WebSocket(service.wsUrl, { headers: { Origin: wrongOrigin } });
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error(`${service.label} accepted wrong origin longer than expected`));
    }, 5000);

    socket.addEventListener('open', () => {
      clearTimeout(timer);
      socket.close();
      reject(new Error(`${service.label} unexpectedly opened with wrong origin`));
    });
    socket.addEventListener('error', () => {
      clearTimeout(timer);
      resolve();
    });
    socket.addEventListener('close', () => {
      clearTimeout(timer);
      resolve();
    });
  });
}

async function openWebSocket(service, origin) {
  const socket = new WebSocket(service.wsUrl, { headers: { Origin: origin } });
  attachEventQueue(socket);
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error(`${service.label} WebSocket open timed out`));
    }, webSocketEventTimeoutMs);
    socket.addEventListener('open', () => {
      clearTimeout(timer);
      resolve();
    });
    socket.addEventListener('error', () => {
      clearTimeout(timer);
      reject(new Error(`${service.label} WebSocket failed to open`));
    });
  });
  return socket;
}

async function sendCommand(socket, command) {
  socket.send(JSON.stringify(command));
  for (;;) {
    const event = await nextEvent(socket, command.messageId);
    if (event.event !== 'agent.connection.ready') {
      return event;
    }
  }
}

function nextEvent(socket, messageId) {
  if (typeof socket.nextParsedAgentEvent === 'function') {
    return socket.nextParsedAgentEvent(messageId);
  }
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      socket.removeEventListener('message', onMessage);
      reject(new Error(`Timed out waiting for LAN discovery event after ${messageId}`));
    }, webSocketEventTimeoutMs);
    const onMessage = (message) => {
      clearTimeout(timer);
      try {
        resolve(AgentEventEnvelopeSchema.parse(JSON.parse(String(message.data))));
      } catch (error) {
        reject(error);
      }
    };
    socket.addEventListener('message', onMessage, { once: true });
  });
}

function attachEventQueue(socket) {
  const queuedEvents = [];
  const pendingResolvers = [];
  socket.nextParsedAgentEvent = (messageId) =>
    new Promise((resolve, reject) => {
      const queued = queuedEvents.shift();
      if (queued !== undefined) {
        if (queued instanceof Error) {
          reject(queued);
          return;
        }
        resolve(queued);
        return;
      }
      const timer = setTimeout(() => {
        const index = pendingResolvers.findIndex((pending) => pending.resolve === resolve);
        if (index >= 0) {
          pendingResolvers.splice(index, 1);
        }
        reject(new Error(`Timed out waiting for LAN discovery event after ${messageId}`));
      }, webSocketEventTimeoutMs);
      pendingResolvers.push({
        reject,
        resolve: (event) => {
          clearTimeout(timer);
          resolve(event);
        },
      });
    });
  socket.addEventListener('message', (message) => {
    try {
      const event = AgentEventEnvelopeSchema.parse(JSON.parse(String(message.data)));
      const pending = pendingResolvers.shift();
      if (pending) {
        pending.resolve(event);
        return;
      }
      queuedEvents.push(event);
    } catch (error) {
      const pending = pendingResolvers.shift();
      if (pending) {
        pending.reject(error);
      } else {
        queuedEvents.push(error);
      }
    }
  });
}

async function waitForHttp(url, service) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30000) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
    } catch {
      await delay(250);
    }
  }
  throw new Error(`Timed out waiting for ${url}\n${service.serviceOutput()}`);
}

function buildChallengeCommand(service, messageSuffix, overrides = {}) {
  return buildCommand(service, messageSuffix, 'agent.lan-pairing.status.get', {
    childDeviceId: service.childDeviceId,
    parentDeviceId,
    routeId: service.routeId,
    origin: allowedOrigin,
    startedAt: issuedAt,
    staleAt: expiresAt,
    ...overrides,
  });
}

function buildPairingCommand(service, challengePayload, messageSuffix, overrides = {}) {
  const payload = {
    pairingId: service.pairingId,
    challengeId: challengePayload.challengeId,
    childDeviceId: service.childDeviceId,
    parentDeviceId,
    routeId: service.routeId,
    origin: allowedOrigin,
    proofDigest: challengePayload.proofDigest,
    evidenceReferenceIds: service.evidenceReferenceIds,
    startedAt: issuedAt,
    staleAt: expiresAt,
    ...overrides,
  };
  for (const [key, value] of Object.entries(payload)) {
    if (value === undefined) {
      delete payload[key];
    }
  }
  return buildCommand(service, messageSuffix, 'agent.lan-pairing.proof.submit', payload);
}

function buildRouteSelectCommand(service, intentId) {
  return buildCommand(
    service,
    intentId,
    'agent.lan-pairing.route.select',
    intentPayload(service, intentId, 'configuration-update')
  );
}

function buildHealthCommand(service, messageSuffix, payload) {
  return buildCommand(service, messageSuffix, 'agent.health.check', payload);
}

function buildCommand(service, messageSuffix, command, payload) {
  return {
    schemaVersion: 1,
    messageId: `${service.label}-${messageSuffix}`,
    sentAt: new Date().toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: service.childDeviceId, platform, route: 'local-network' },
    command,
    payload,
  };
}

function intentPayload(service, intentId, intentKind) {
  return {
    intentId,
    intentKind,
    pairingId: service.pairingId,
    childDeviceId: service.childDeviceId,
    routeId: service.routeId,
    origin: allowedOrigin,
    proofDigest: service.lastProofDigest ?? '',
    evidenceReferenceIds: service.evidenceReferenceIds,
    startedAt: issuedAt,
    staleAt: expiresAt,
    controllerLeaseId,
    controllerDeviceId,
    parentActorId,
    parentAuthority: parentAuthorityActiveController,
    controllerLeaseIssuedAt: issuedAt,
    controllerLeaseExpiresAt,
  };
}

function assertChallengePreview(payload, service) {
  assertPayloadValue(payload, 'transport', 'websocket');
  assertPayloadValue(payload, 'discoveryStatus', 'websocket-direct');
  assertPayloadValue(payload, 'challengeStatus', 'websocket-direct');
  assertPayloadValue(payload, 'proofPreviewStatus', 'websocket-direct');
  assertPayloadValue(payload, 'discoveryState', 'pending');
  assertPayloadValue(payload, 'pairingState', 'pairing');
  assertPayloadValue(payload, 'childDeviceId', service.childDeviceId);
  assertPayloadValue(payload, 'parentDeviceId', parentDeviceId);
  assertPayloadValue(payload, 'routeId', service.routeId);
  assertPayloadValue(payload, 'origin', allowedOrigin);
  assertPayloadValue(payload, 'staleAt', expiresAt);
  assertPayloadValue(
    payload,
    'unsupportedHttpEndpoints',
    [
      '/api/lan-pairing/discovery',
      '/api/lan-pairing/challenge',
      '/api/lan-pairing/proof',
      '/api/lan-pairing/control',
      '/api/lan-pairing/registry',
    ].join(',')
  );
  if (!String(payload.challengeId ?? '').startsWith('challenge-direct-')) {
    throw new Error(`Expected direct challenge id, received ${payload.challengeId}`);
  }
  if (!String(payload.proofDigest ?? '').startsWith('sha256:direct-preview:')) {
    throw new Error(`Expected direct proof digest, received ${payload.proofDigest}`);
  }
  service.lastProofDigest = payload.proofDigest;
  assertNoSensitiveMarkers(payload);
}

function assertNoSensitiveMarkers(payload) {
  const serialized = JSON.stringify(payload);
  for (const marker of sensitiveMarkers) {
    if (serialized.includes(marker)) {
      throw new Error(`LAN challenge payload leaked ${marker}`);
    }
  }
}

function assertEvent(event, expected) {
  if (event.event !== expected) {
    throw new Error(`Expected event ${expected}, received ${event.event}: ${JSON.stringify(event.payload)}`);
  }
}

function assertPayloadValue(payload, key, expected) {
  if (payload[key] !== expected) {
    throw new Error(`Expected LAN payload ${key}=${expected}, received ${payload[key]}`);
  }
}

async function writeEvidence(assertions, services) {
  await writeFile(
    evidencePath,
    `${JSON.stringify(
      {
        checkedAt: new Date().toISOString(),
        allowedOrigin,
        assertions,
        proofLimits: [
          'local-two-service-discovery-challenge-only',
          'physical-two-device-LAN-discovery-remains-manual-required',
        ],
        services: services.map((service) => ({
          label: service.label,
          port: service.port,
          childDeviceId: service.childDeviceId,
          registryPath: service.registryPath,
        })),
      },
      null,
      2
    )}\n`
  );
}

function collectOutput(child) {
  const chunks = [];
  child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
  return () => chunks.join('');
}
