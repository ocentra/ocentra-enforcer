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

const outputDir = join(process.cwd(), 'test-results', 'v0-9-lan-pairing-control-mvp');
const evidencePath = join(outputDir, 'proof.json');
const allowedOrigin = createHttpOrigin(ParentDevHost.Loopback);
const wrongOrigin = createHttpOrigin(ParentDevHost.Loopback, 9478);
const issuedAt = '2026-05-26T18:20:00.000Z';
const expiresAt = '2099-05-26T18:25:00.000Z';
const staleExpiresAt = '2026-05-26T18:10:00.000Z';
const controllerLeaseId = 'controller-lease-v09-primary';
const secondControllerLeaseId = 'controller-lease-v09-secondary';
const controllerDeviceId = 'parent-device-v09';
const secondControllerDeviceId = 'parent-device-v09-secondary';
const parentActorId = 'parent-actor-v09';
const secondParentActorId = 'parent-actor-v09-secondary';
const controllerLeaseExpiresAt = '2099-05-26T18:24:00.000Z';
const controllerLeaseExpiredAt = '2026-05-26T18:09:00.000Z';
const parentAuthorityActiveController = 'active-controller';
const parentAuthorityObserver = 'observer';
const platform = 'windows';
const webSocketEventTimeoutMs = 20000;
const supportedWebSocketCommands = [
  'agent.lan-pairing.proof.submit',
  'agent.lan-pairing.route.select',
  'agent.lan-pairing.route.revoke',
  'agent.lan-pairing.status.get',
  'agent.lan-pairing.browser-discovery.scan',
  'agent.lan-pairing.add-device.request',
  'agent.lan-pairing.controller-lease.renew',
  'agent.lan-pairing.controller-lease.release',
  'agent.lan-pairing.controller-lease.takeover',
  'agent.lan-ai.provider.status.get',
  'agent.lan-ai.job.submit',
].join(',');
const sensitiveEvidenceMarkers = [
  'activity.sqlite',
  'activity.ndjson',
  'decryptedEvidence',
  'journalPath',
  'rawEvidence',
  'rawProofSecret',
  'rawToken',
  'registryPath',
  'sqlitePath',
];

const agents = [
  {
    label: 'first-child-agent',
    port: 4492,
    childDeviceId: 'child-device-v09-first',
    pairingId: 'pairing-v09-first',
    challengeId: 'challenge-v09-first',
    proofDigest: 'sha256:v09-first-proof',
    routeId: 'route-v09-first-local-network',
    evidenceReferenceIds: 'activity-event-v09-first',
  },
  {
    label: 'second-child-agent',
    port: 4493,
    childDeviceId: 'child-device-v09-second',
    pairingId: 'pairing-v09-second',
    challengeId: 'challenge-v09-second',
    proofDigest: 'sha256:v09-second-proof',
    routeId: 'route-v09-second-local-network',
    evidenceReferenceIds: 'activity-event-v09-second',
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

  const firstLifecycle = await runLanLifecycle(services[0], { revokeAtEnd: true });
  const secondLifecycle = await runLanLifecycle(services[1], { revokeAtEnd: false });
  assertions.push(...firstLifecycle, ...secondLifecycle);

  await assertWrongAgentPortRejected(services[0], services[1]);
  assertions.push('wrong-agent-port-rejected-as-wrong-device');

  await stopProcessTreeAndWait(services[1].child);
  services[1] = spawnAgentService(agents[1]);
  await waitForHttp(services[1].healthUrl, services[1]);
  const restartLifecycle = await runPersistentRestartLifecycle(services[1]);
  assertions.push(...restartLifecycle);

  await writeEvidence(assertions, services);
  console.log(`v0-9-lan-pairing-control-mvp-ok:${assertions.join(',')}`);
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

async function runLanLifecycle(service, { revokeAtEnd }) {
  const socket = await openWebSocket(service, allowedOrigin);
  try {
    const labels = [];
    const unpaired = await sendCommand(socket, buildHealthCommand(service, 'unpaired-health', {}));
    assertEvent(unpaired, 'agent.command.rejected');
    assertPayloadValue(unpaired.payload, 'rejectionReason', 'anonymous');
    labels.push(`${service.label}:anonymous-rejected`);

    const paired = await sendCommand(socket, buildPairingCommand(service));
    assertEvent(paired, 'agent.lan-pairing.status.reported');
    assertPayloadValue(paired.payload, 'auditEventType', 'pairing-proof-accepted');
    assertPayloadValue(paired.payload, 'trustedDeviceIds', service.childDeviceId);
    assertPayloadValue(paired.payload, 'selectedChildDeviceId', '');
    assertLanSupportSurface(paired.payload);
    labels.push(`${service.label}:pairing-proof-accepted-unselected`);

    const beforeSelection = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'before-selection-health',
        intentPayload(service, 'intent-before-selection', 'rule-query')
      )
    );
    assertEvent(beforeSelection, 'agent.command.rejected');
    assertPayloadValue(beforeSelection.payload, 'rejectionReason', 'unselected-device');
    labels.push(`${service.label}:unselected-control-rejected`);

    const selected = await sendCommand(socket, buildRouteSelectCommand(service, 'intent-route-select'));
    assertEvent(selected, 'agent.lan-pairing.status.reported');
    assertPayloadValue(selected.payload, 'auditEventType', 'route-selected');
    assertPayloadValue(selected.payload, 'authenticationState', 'paired');
    assertPayloadValue(selected.payload, 'selectedChildDeviceId', service.childDeviceId);
    assertPayloadValue(selected.payload, 'selectedPairingId', service.pairingId);
    assertPayloadValue(selected.payload, 'selectedRouteId', service.routeId);
    assertPayloadValue(selected.payload, 'selectedRouteTrustState', 'paired');
    assertPayloadValue(selected.payload, 'selectedRouteStaleAt', expiresAt);
    assertPayloadValue(selected.payload, 'selectedRouteOfflineAt', '');
    assertPayloadValue(selected.payload, 'discoveryState', 'paired');
    labels.push(`${service.label}:route-selected`);
    labels.push(`${service.label}:selected-route-trust-state-paired`);

    const accepted = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'accepted-rule-query',
        intentPayload(service, 'intent-accepted-rule-query', 'rule-query')
      )
    );
    assertEvent(accepted, 'agent.health.reported');
    assertAcceptedControl(accepted.payload, 'rule-query', service);
    labels.push(`${service.label}:rule-query-accepted`);

    const observerRead = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'observer-rule-query',
        withParentAuthority(intentPayload(service, 'intent-observer-rule-query', 'rule-query'), parentAuthorityObserver)
      )
    );
    assertEvent(observerRead, 'agent.health.reported');
    assertPayloadValue(observerRead.payload, 'parentAuthority', parentAuthorityObserver);
    labels.push(`${service.label}:observer-rule-query-accepted`);

    const observerWrite = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'observer-rule-update',
        withParentAuthority(
          intentPayload(service, 'intent-observer-rule-update', 'rule-update'),
          parentAuthorityObserver
        )
      )
    );
    assertEvent(observerWrite, 'agent.command.rejected');
    assertPayloadValue(observerWrite.payload, 'rejectionReason', 'observer-read-only');
    labels.push(`${service.label}:observer-write-rejected`);

    const leaseRenewed = await sendCommand(
      socket,
      buildControllerLeaseCommand(
        service,
        'intent-controller-lease-renew',
        'agent.lan-pairing.controller-lease.renew',
        'controller-lease-renew'
      )
    );
    assertEvent(leaseRenewed, 'agent.lan-pairing.status.reported');
    assertPayloadValue(leaseRenewed.payload, 'auditEventType', 'controller-lease-renewed');
    labels.push(`${service.label}:controller-lease-renewed`);

    const leaseReleased = await sendCommand(
      socket,
      buildControllerLeaseCommand(
        service,
        'intent-controller-lease-release',
        'agent.lan-pairing.controller-lease.release',
        'controller-lease-release'
      )
    );
    assertEvent(leaseReleased, 'agent.lan-pairing.status.reported');
    assertPayloadValue(leaseReleased.payload, 'auditEventType', 'controller-lease-released');
    labels.push(`${service.label}:controller-lease-released`);

    const reacquired = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'post-release-rule-query',
        intentPayload(service, 'intent-post-release-rule-query', 'rule-query')
      )
    );
    assertEvent(reacquired, 'agent.health.reported');
    assertAcceptedControl(reacquired.payload, 'rule-query', service);
    labels.push(`${service.label}:controller-lease-reacquired`);

    const replayed = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'replayed-rule-query',
        intentPayload(service, 'intent-accepted-rule-query', 'rule-query')
      )
    );
    assertEvent(replayed, 'agent.command.rejected');
    assertPayloadValue(replayed.payload, 'rejectionReason', 'replayed');
    labels.push(`${service.label}:replay-rejected`);

    const stale = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'stale-rule-update',
        intentPayload(service, 'intent-stale-rule-update', 'rule-update', staleExpiresAt)
      )
    );
    assertEvent(stale, 'agent.command.rejected');
    assertPayloadValue(stale.payload, 'rejectionReason', 'stale');
    labels.push(`${service.label}:stale-control-rejected`);

    const malformedPayload = intentPayload(service, 'intent-malformed-control', 'approval-decision');
    delete malformedPayload.intentKind;
    const malformed = await sendCommand(
      socket,
      buildHealthCommand(service, 'malformed-approval-decision', malformedPayload)
    );
    assertEvent(malformed, 'agent.command.rejected');
    assertPayloadValue(malformed.payload, 'rejectionReason', 'malformed');
    labels.push(`${service.label}:malformed-control-rejected`);

    const missingLease = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'missing-controller-lease',
        withoutControllerLease(intentPayload(service, 'intent-missing-controller-lease', 'rule-query'))
      )
    );
    assertEvent(missingLease, 'agent.command.rejected');
    assertPayloadValue(missingLease.payload, 'rejectionReason', 'controller-lease-missing');
    labels.push(`${service.label}:missing-controller-lease-rejected`);

    const expiredLease = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'expired-controller-lease',
        withExpiredControllerLease(intentPayload(service, 'intent-expired-controller-lease', 'rule-update'))
      )
    );
    assertEvent(expiredLease, 'agent.command.rejected');
    assertPayloadValue(expiredLease.payload, 'rejectionReason', 'controller-lease-expired');
    labels.push(`${service.label}:expired-controller-lease-rejected`);

    const wrongController = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'wrong-controller',
        withSecondController(intentPayload(service, 'intent-wrong-controller', 'approval-decision'))
      )
    );
    assertEvent(wrongController, 'agent.command.rejected');
    assertPayloadValue(wrongController.payload, 'rejectionReason', 'wrong-controller');
    labels.push(`${service.label}:wrong-controller-rejected`);

    const takeoverDenied = await sendCommand(
      socket,
      buildControllerLeaseCommand(
        service,
        'intent-controller-lease-takeover-denied',
        'agent.lan-pairing.controller-lease.takeover',
        'controller-lease-takeover',
        withSecondController
      )
    );
    assertEvent(takeoverDenied, 'agent.command.rejected');
    assertPayloadValue(takeoverDenied.payload, 'rejectionReason', 'takeover-denied');
    assertPayloadValue(takeoverDenied.payload, 'auditEventType', 'controller-lease-takeover-rejected');
    labels.push(`${service.label}:controller-lease-takeover-denied`);

    const providerStatus = await sendCommand(
      socket,
      buildLanAiProviderStatusCommand(service, 'intent-lan-ai-provider-status')
    );
    assertEvent(providerStatus, 'agent.lan-pairing.status.reported');
    assertPayloadValue(providerStatus.payload, 'auditEventType', 'lan-ai-provider-advertised');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderStatus', 'lan-ai-provider-unavailable');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderRoutingState', 'unavailable');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderCustodyLabel', 'local-network-ai-provider');
    labels.push(`${service.label}:lan-ai-provider-advertised`);

    const lanAiJob = await sendCommand(socket, buildLanAiJobCommand(service, 'intent-lan-ai-job'));
    assertEvent(lanAiJob, 'agent.lan-ai.job.reported');
    assertPayloadValue(lanAiJob.payload, 'auditEventType', 'lan-ai-job-degraded');
    assertPayloadValue(lanAiJob.payload, 'lanAiJobState', 'degraded');
    assertPayloadValue(lanAiJob.payload, 'lanAiProviderRoutingState', 'unavailable');
    assertPayloadValue(lanAiJob.payload, 'unavailableReason', 'local-ai-provider-unconfigured');
    labels.push(`${service.label}:lan-ai-job-degraded`);

    const observerLanAiJob = await sendCommand(
      socket,
      buildLanAiJobCommand(service, 'intent-observer-lan-ai-job', parentAuthorityObserver)
    );
    assertEvent(observerLanAiJob, 'agent.command.rejected');
    assertPayloadValue(observerLanAiJob.payload, 'rejectionReason', 'observer-read-only');
    labels.push(`${service.label}:observer-lan-ai-job-rejected`);

    if (!revokeAtEnd) {
      const finalRelease = await sendCommand(
        socket,
        buildControllerLeaseCommand(
          service,
          'intent-controller-lease-final-release',
          'agent.lan-pairing.controller-lease.release',
          'controller-lease-release'
        )
      );
      assertEvent(finalRelease, 'agent.lan-pairing.status.reported');

      const takeoverAccepted = await sendCommand(
        socket,
        buildControllerLeaseCommand(
          service,
          'intent-controller-lease-takeover-accepted',
          'agent.lan-pairing.controller-lease.takeover',
          'controller-lease-takeover',
          withSecondController
        )
      );
      assertEvent(takeoverAccepted, 'agent.lan-pairing.status.reported');
      assertPayloadValue(takeoverAccepted.payload, 'auditEventType', 'controller-lease-takeover-accepted');
      labels.push(`${service.label}:controller-lease-takeover-accepted`);
    }

    if (revokeAtEnd) {
      const revoked = await sendCommand(socket, buildRouteRevokeCommand(service, 'intent-route-revoke'));
      assertEvent(revoked, 'agent.lan-pairing.status.reported');
      assertPayloadValue(revoked.payload, 'auditEventType', 'pairing-revoked');
      assertPayloadValue(revoked.payload, 'pairingState', 'revoked');
      assertPayloadValue(revoked.payload, 'discoveryState', 'revoked');
      labels.push(`${service.label}:route-revoked`);

      const afterRevoke = await sendCommand(
        socket,
        buildHealthCommand(service, 'after-revoke-health', intentPayload(service, 'intent-after-revoke', 'rule-update'))
      );
      assertEvent(afterRevoke, 'agent.command.rejected');
      assertPayloadValue(afterRevoke.payload, 'rejectionReason', 'revoked');
      labels.push(`${service.label}:revoked-control-rejected`);
    }

    return labels;
  } finally {
    socket.close();
  }
}

async function runPersistentRestartLifecycle(service) {
  const socket = await openWebSocket(service, allowedOrigin);
  try {
    const labels = [];
    const restartStatus = await sendCommand(socket, buildLoopbackStatusCommand(service, 'restart-status'));
    assertEvent(restartStatus, 'agent.lan-pairing.status.reported');
    assertPayloadValue(restartStatus.payload, 'pairingState', 'paired');
    assertPayloadValue(restartStatus.payload, 'authenticationState', 'paired');
    assertPayloadValue(restartStatus.payload, 'trustedDeviceIds', service.childDeviceId);
    assertPayloadValue(restartStatus.payload, 'selectedChildDeviceId', service.childDeviceId);
    assertPayloadValue(restartStatus.payload, 'selectedPairingId', service.pairingId);
    assertPayloadValue(restartStatus.payload, 'selectedRouteId', service.routeId);
    assertPayloadValue(restartStatus.payload, 'selectedRouteTrustState', 'paired');
    assertPayloadValue(restartStatus.payload, 'selectedRouteStaleAt', expiresAt);
    assertPayloadValue(restartStatus.payload, 'selectedRouteOfflineAt', '');
    assertPayloadValue(restartStatus.payload, 'discoveryState', 'paired');
    assertPayloadValue(restartStatus.payload, 'persistenceMode', 'local-json-registry');
    assertPayloadValue(restartStatus.payload, 'restartBehavior', 'restore-trusted-registry-selected-route');
    labels.push(`${service.label}:restart-restores-selected-route`);
    labels.push(`${service.label}:restart-restores-selected-route-trust-state`);

    const acceptedAfterRestart = await sendCommand(
      socket,
      buildHealthCommand(
        service,
        'restart-recovered-approval',
        intentPayload(service, 'intent-after-restart-approval', 'approval-decision')
      )
    );
    assertEvent(acceptedAfterRestart, 'agent.health.reported');
    assertAcceptedControl(acceptedAfterRestart.payload, 'approval-decision', service);
    labels.push(`${service.label}:restart-recovered-approval-accepted`);

    return labels;
  } finally {
    socket.close();
  }
}

async function assertWrongAgentPortRejected(firstService, secondService) {
  const socket = await openWebSocket(firstService, allowedOrigin);
  try {
    const wrongPort = await sendCommand(
      socket,
      buildHealthCommand(
        secondService,
        'wrong-agent-port-health',
        intentPayload(secondService, 'intent-wrong-agent-port', 'health-query')
      )
    );
    assertEvent(wrongPort, 'agent.command.rejected');
    assertPayloadValue(wrongPort.payload, 'rejectionReason', 'wrong-device');
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
      assertNoSensitiveEvidenceMarkers(event.payload);
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
      reject(new Error(`Timed out waiting for LAN pairing event after ${messageId}`));
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
        reject(new Error(`Timed out waiting for LAN pairing event after ${messageId}`));
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

function buildPairingCommand(service) {
  return buildCommand(service, 'pairing-proof-submit', 'agent.lan-pairing.proof.submit', {
    pairingId: service.pairingId,
    challengeId: service.challengeId,
    childDeviceId: service.childDeviceId,
    parentDeviceId: 'parent-device-v09',
    routeId: service.routeId,
    origin: allowedOrigin,
    proofDigest: service.proofDigest,
    evidenceReferenceIds: service.evidenceReferenceIds,
    startedAt: issuedAt,
    staleAt: expiresAt,
  });
}

function buildRouteSelectCommand(service, intentId) {
  return buildCommand(
    service,
    intentId,
    'agent.lan-pairing.route.select',
    intentPayload(service, intentId, 'configuration-update')
  );
}

function buildRouteRevokeCommand(service, intentId) {
  return buildCommand(
    service,
    intentId,
    'agent.lan-pairing.route.revoke',
    intentPayload(service, intentId, 'configuration-update')
  );
}

function buildHealthCommand(service, messageSuffix, payload) {
  return buildCommand(service, messageSuffix, 'agent.health.check', payload);
}

function buildLoopbackStatusCommand(service, messageSuffix) {
  return {
    ...buildCommand(service, messageSuffix, 'agent.lan-pairing.status.get', {}),
    target: { deviceId: service.childDeviceId, platform, route: 'localhost' },
  };
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

function intentPayload(service, intentId, intentKind, payloadExpiresAt = expiresAt) {
  return {
    intentId,
    intentKind,
    pairingId: service.pairingId,
    childDeviceId: service.childDeviceId,
    routeId: service.routeId,
    origin: allowedOrigin,
    proofDigest: service.proofDigest,
    evidenceReferenceIds: service.evidenceReferenceIds,
    startedAt: issuedAt,
    staleAt: payloadExpiresAt,
    controllerLeaseId,
    controllerDeviceId,
    parentActorId,
    parentAuthority: parentAuthorityActiveController,
    controllerLeaseIssuedAt: issuedAt,
    controllerLeaseExpiresAt,
  };
}

function withoutControllerLease(payload) {
  const next = { ...payload };
  delete next.controllerLeaseId;
  delete next.controllerDeviceId;
  delete next.parentActorId;
  delete next.controllerLeaseIssuedAt;
  delete next.controllerLeaseExpiresAt;
  return next;
}

function withExpiredControllerLease(payload) {
  return {
    ...payload,
    controllerLeaseExpiresAt: controllerLeaseExpiredAt,
  };
}

function withSecondController(payload) {
  return {
    ...payload,
    controllerLeaseId: secondControllerLeaseId,
    controllerDeviceId: secondControllerDeviceId,
    parentActorId: secondParentActorId,
  };
}

function withParentAuthority(payload, parentAuthority) {
  return {
    ...payload,
    parentAuthority,
  };
}

function buildControllerLeaseCommand(service, intentId, command, intentKind, mutate = (payload) => payload) {
  return buildCommand(service, intentId, command, mutate(intentPayload(service, intentId, intentKind)));
}

function buildLanAiProviderStatusCommand(service, intentId) {
  return buildCommand(
    service,
    intentId,
    'agent.lan-ai.provider.status.get',
    withParentAuthority(intentPayload(service, intentId, 'lan-ai-provider-status'), parentAuthorityObserver)
  );
}

function buildLanAiJobCommand(service, intentId, parentAuthority = parentAuthorityActiveController) {
  return buildCommand(service, intentId, 'agent.lan-ai.job.submit', {
    ...withParentAuthority(intentPayload(service, intentId, 'lan-ai-job-submit'), parentAuthority),
    lanAiJobId: `lan-ai-job-${intentId}`,
  });
}

function assertLanSupportSurface(payload) {
  assertPayloadValue(payload, 'transport', 'websocket');
  assertPayloadValue(payload, 'supportedWebSocketCommands', supportedWebSocketCommands);
  assertPayloadValue(payload, 'discoveryStatus', 'websocket-direct');
  assertPayloadValue(payload, 'challengeStatus', 'websocket-direct');
  assertPayloadValue(payload, 'proofPreviewStatus', 'websocket-direct');
  assertPayloadValue(payload, 'persistenceMode', 'local-json-registry');
  assertPayloadValue(payload, 'proofMode', 'direct-proof-submit');
  assertPayloadValue(payload, 'lanAiProviderStatus', 'websocket-direct');
  assertPayloadValue(payload, 'lanAiJobStatus', 'websocket-direct');
  if (!['discovered', 'paired', 'revoked', 'stale', 'offline'].includes(payload.discoveryState)) {
    throw new Error(`Expected explicit discoveryState, received ${payload.discoveryState}`);
  }
  assertPayloadValue(payload, 'lanAiProviderRoutingState', 'unavailable');
  assertPayloadValue(payload, 'lanAiProviderCustodyLabel', 'local-network-ai-provider');
}

function assertAcceptedControl(payload, intentKind, service) {
  assertPayloadValue(payload, 'controlState', 'accepted');
  assertPayloadValue(payload, 'auditEventType', 'control-accepted');
  assertPayloadValue(payload, 'authenticationState', 'paired');
  assertPayloadValue(payload, 'intentKind', intentKind);
  assertPayloadValue(payload, 'routeId', service.routeId);
  assertPayloadValue(payload, 'controllerLeaseId', controllerLeaseId);
  assertPayloadValue(payload, 'controllerDeviceId', controllerDeviceId);
  assertPayloadValue(payload, 'parentActorId', parentActorId);
  assertPayloadValue(payload, 'parentAuthority', parentAuthorityActiveController);
  assertPayloadValue(payload, 'evidenceReferenceIds', service.evidenceReferenceIds);
}

function assertEvent(event, expected) {
  if (event.event !== expected) {
    throw new Error(`Expected event ${expected}, received ${event.event}`);
  }
}

function assertPayloadValue(payload, key, expected) {
  if (payload[key] !== expected) {
    throw new Error(`Expected LAN payload ${key}=${expected}, received ${payload[key]}`);
  }
}

function assertNoSensitiveEvidenceMarkers(payload) {
  const serialized = JSON.stringify(payload);
  for (const marker of sensitiveEvidenceMarkers) {
    if (serialized.includes(marker)) {
      throw new Error(`LAN proof payload exposed sensitive marker ${marker}`);
    }
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
          'local-two-service-route-control-and-restart-recovery-only',
          'physical-two-device-selected-route-offline-stale-proof-remains-manual-required',
        ],
        services: services.map((service) => ({
          label: service.label,
          port: service.port,
          childDeviceId: service.childDeviceId,
          registryPersistence: 'local-json-registry',
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
