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

const outputDir = join(process.cwd(), 'test-results', 'platform-roles-lan-ai-provider-pool');
const evidencePath = join(outputDir, 'proof.json');
const allowedOrigin = createHttpOrigin(ParentDevHost.Loopback);
const issuedAt = '2026-05-27T06:35:00.000Z';
const expiresAt = '2099-05-27T06:40:00.000Z';
const controllerLeaseId = 'controller-lease-platform-roles';
const controllerDeviceId = 'parent-desktop-controller-platform-roles';
const parentActorId = 'parent-actor-platform-roles';
const controllerLeaseExpiresAt = '2099-05-27T06:39:00.000Z';
const parentAuthorityActiveController = 'active-controller';
const parentAuthorityObserver = 'observer';
const platform = 'windows';
const webSocketEventTimeoutMs = 20000;
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

const services = [
  {
    label: 'parent-desktop-controller-ai-provider',
    port: 4494,
    childDeviceId: 'child-device-platform-ai-provider',
    pairingId: 'pairing-platform-ai-provider',
    challengeId: 'challenge-platform-ai-provider',
    proofDigest: 'sha256:platform-ai-provider-proof',
    routeId: 'route-platform-ai-provider-local-network',
    evidenceReferenceIds: 'activity-event-platform-ai-provider',
    surface: 'parent-desktop',
    deviceRoles: 'parent-controller,child-agent,ai-provider',
    providerOptIn: true,
    providerCapabilities: 'chat-completion,summarization',
  },
  {
    label: 'parent-mobile-observer-scaffold',
    port: 4495,
    childDeviceId: 'child-device-platform-mobile-observer',
    pairingId: 'pairing-platform-mobile-observer',
    challengeId: 'challenge-platform-mobile-observer',
    proofDigest: 'sha256:platform-mobile-observer-proof',
    routeId: 'route-platform-mobile-observer-local-network',
    evidenceReferenceIds: 'activity-event-platform-mobile-observer',
    surface: 'parent-mobile',
    deviceRoles: 'parent-observer',
    providerOptIn: false,
    providerCapabilities: '',
  },
  {
    label: 'parent-desktop-busy-ai-provider',
    port: 4496,
    childDeviceId: 'child-device-platform-busy-provider',
    pairingId: 'pairing-platform-busy-provider',
    challengeId: 'challenge-platform-busy-provider',
    proofDigest: 'sha256:platform-busy-provider-proof',
    routeId: 'route-platform-busy-provider-local-network',
    evidenceReferenceIds: 'activity-event-platform-busy-provider',
    surface: 'parent-desktop',
    deviceRoles: 'parent-controller,child-agent,ai-provider',
    providerOptIn: true,
    providerBusy: true,
    providerCapabilities: 'chat-completion,summarization',
  },
  {
    label: 'parent-desktop-degraded-ai-provider',
    port: 4497,
    childDeviceId: 'child-device-platform-degraded-provider',
    pairingId: 'pairing-platform-degraded-provider',
    challengeId: 'challenge-platform-degraded-provider',
    proofDigest: 'sha256:platform-degraded-provider-proof',
    routeId: 'route-platform-degraded-provider-local-network',
    evidenceReferenceIds: 'activity-event-platform-degraded-provider',
    surface: 'parent-desktop',
    deviceRoles: 'parent-controller,child-agent,ai-provider',
    providerOptIn: false,
    providerCapabilities: 'chat-completion,summarization',
  },
];

await rm(outputDir, { recursive: true, force: true });
await mkdir(outputDir, { recursive: true });

for (const service of services) {
  await ensurePortFree(service.port, isLikelyParentAgentOccupant, console.log, ParentDevHost.Wildcard);
}

const runningServices = services.map(spawnAgentService);
const assertions = [];

try {
  await Promise.all(runningServices.map((service) => waitForHttp(service.healthUrl, service)));

  const providerProof = await runProviderPoolLifecycle(runningServices[0]);
  const mobileProof = await runMobileObserverScaffoldLifecycle(runningServices[1]);
  const busyProof = await runBusyProviderLifecycle(runningServices[2]);
  const degradedProof = await runDegradedProviderLifecycle(runningServices[3]);
  assertions.push(...providerProof, ...mobileProof, ...busyProof, ...degradedProof);

  await writeEvidence(assertions, runningServices);
  console.log(`platform-roles-lan-ai-provider-pool-ok:${assertions.join(',')}`);
} finally {
  await Promise.allSettled(runningServices.map((service) => stopProcessTreeAndWait(service.child)));
}

function spawnAgentService(service) {
  const registryPath = join(outputDir, `${service.label}-registry.json`);
  const child = spawn(resolveDebugAgentServicePath(), [], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      [ParentDevEnv.AgentAddress]: createAgentAddress(service.port, ParentDevHost.Wildcard),
      [ParentDevEnv.AgentAllowedOrigins]: allowedOrigin,
      [ParentDevEnv.AgentLocalNetworkEnabled]: ParentDevValue.True,
      OCENTRA_PARENT_AGENT_LAN_CHILD_DEVICE_ID: service.childDeviceId,
      OCENTRA_PARENT_AGENT_LAN_PAIRING_REGISTRY_PATH: registryPath,
      OCENTRA_PARENT_DEVICE_SURFACE: service.surface,
      OCENTRA_PARENT_DEVICE_ROLES: service.deviceRoles,
      OCENTRA_PARENT_LAN_AI_PROVIDER_OPT_IN: service.providerOptIn ? 'true' : 'false',
      OCENTRA_PARENT_LAN_AI_PROVIDER_BUSY: service.providerBusy ? 'true' : 'false',
      OCENTRA_PARENT_LAN_AI_PROVIDER_CAPABILITIES: service.providerCapabilities,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  return {
    ...service,
    child,
    healthUrl: createAgentHealthUrl(service.port),
    registryPath,
    serviceOutput: collectOutput(child),
    wsUrl: createAgentWebSocketUrl(service.port),
  };
}

async function runProviderPoolLifecycle(service) {
  const controllerSocket = await openWebSocket(service, allowedOrigin);
  const observerSocket = await openWebSocket(service, allowedOrigin);
  try {
    const labels = [];
    await pairAndSelect(controllerSocket, service);
    labels.push(`${service.label}:route-selected`);

    const providerStatus = await sendCommand(
      observerSocket,
      buildLanAiProviderStatusCommand(service, 'intent-platform-provider-status')
    );
    assertEvent(providerStatus, 'agent.lan-pairing.status.reported');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderStatus', 'lan-ai-provider-available');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderRoutingState', 'authorized-result');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderCustodyLabel', 'local-network-ai-provider');
    assertPayloadValue(providerStatus.payload, 'capabilityFlags', service.providerCapabilities);
    labels.push(`${service.label}:provider-advertised-available`);

    const [acceptedJob, observerJob] = await Promise.all([
      sendCommand(
        controllerSocket,
        buildLanAiJobCommand(
          service,
          'intent-platform-authorized-job',
          parentAuthorityActiveController,
          'chat-completion'
        )
      ),
      sendCommand(
        observerSocket,
        buildLanAiJobCommand(service, 'intent-platform-observer-job', parentAuthorityObserver, 'chat-completion')
      ),
    ]);
    assertEvent(acceptedJob, 'agent.lan-ai.job.reported');
    assertPayloadValue(acceptedJob.payload, 'auditEventType', 'lan-ai-job-completed');
    assertPayloadValue(acceptedJob.payload, 'lanAiProviderStatus', 'lan-ai-provider-available');
    assertPayloadValue(acceptedJob.payload, 'lanAiProviderRoutingState', 'authorized-result');
    assertPayloadValue(acceptedJob.payload, 'lanAiJobState', 'completed');
    assertPayloadValue(acceptedJob.payload, 'generationState', 'complete');
    assertPayloadValue(acceptedJob.payload, 'outputText', 'lan-ai-provider-result-redacted');
    assertEvent(observerJob, 'agent.command.rejected');
    assertPayloadValue(observerJob.payload, 'rejectionReason', 'observer-read-only');
    labels.push(`${service.label}:controller-job-completed-observer-job-rejected`);

    const unsupportedJob = await sendCommand(
      controllerSocket,
      buildLanAiJobCommand(
        service,
        'intent-platform-unsupported-capability',
        parentAuthorityActiveController,
        'classification'
      )
    );
    assertEvent(unsupportedJob, 'agent.command.rejected');
    assertPayloadValue(unsupportedJob.payload, 'rejectionReason', 'lan-ai-job-unauthorized');
    assertPayloadValue(unsupportedJob.payload, 'lanAiProviderRoutingState', 'unsupported-capability');
    assertPayloadValue(unsupportedJob.payload, 'lanAiJobStatus', 'rejected');
    labels.push(`${service.label}:unsupported-capability-rejected`);

    return labels;
  } finally {
    controllerSocket.close();
    observerSocket.close();
  }
}

async function runMobileObserverScaffoldLifecycle(service) {
  const socket = await openWebSocket(service, allowedOrigin);
  try {
    const labels = [];
    await pairAndSelect(socket, service);
    labels.push(`${service.label}:route-selected`);

    const providerStatus = await sendCommand(
      socket,
      buildLanAiProviderStatusCommand(service, 'intent-platform-mobile-provider-status')
    );
    assertEvent(providerStatus, 'agent.lan-pairing.status.reported');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderStatus', 'lan-ai-provider-unavailable');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderRoutingState', 'unavailable');
    labels.push(`${service.label}:provider-unavailable`);

    const degradedJob = await sendCommand(
      socket,
      buildLanAiJobCommand(
        service,
        'intent-platform-mobile-degraded-job',
        parentAuthorityActiveController,
        'chat-completion'
      )
    );
    assertEvent(degradedJob, 'agent.lan-ai.job.reported');
    assertPayloadValue(degradedJob.payload, 'auditEventType', 'lan-ai-job-degraded');
    assertPayloadValue(degradedJob.payload, 'lanAiJobState', 'degraded');
    assertPayloadValue(degradedJob.payload, 'lanAiProviderRoutingState', 'unavailable');
    assertPayloadValue(degradedJob.payload, 'unavailableReason', 'local-ai-provider-unconfigured');
    labels.push(`${service.label}:controller-job-degraded-with-provider-unavailable`);

    const observerWrite = await sendCommand(
      socket,
      buildLanAiJobCommand(service, 'intent-platform-mobile-observer-job', parentAuthorityObserver, 'chat-completion')
    );
    assertEvent(observerWrite, 'agent.command.rejected');
    assertPayloadValue(observerWrite.payload, 'rejectionReason', 'observer-read-only');
    labels.push(`${service.label}:observer-job-rejected`);

    return labels;
  } finally {
    socket.close();
  }
}

async function runBusyProviderLifecycle(service) {
  const socket = await openWebSocket(service, allowedOrigin);
  try {
    const labels = [];
    await pairAndSelect(socket, service);
    labels.push(`${service.label}:route-selected`);

    const providerStatus = await sendCommand(
      socket,
      buildLanAiProviderStatusCommand(service, 'intent-platform-busy-provider-status')
    );
    assertEvent(providerStatus, 'agent.lan-pairing.status.reported');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderStatus', 'lan-ai-provider-busy');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderRoutingState', 'busy');
    labels.push(`${service.label}:provider-busy`);

    const busyJob = await sendCommand(
      socket,
      buildLanAiJobCommand(
        service,
        'intent-platform-busy-provider-job',
        parentAuthorityActiveController,
        'chat-completion'
      )
    );
    assertEvent(busyJob, 'agent.lan-ai.job.reported');
    assertPayloadValue(busyJob.payload, 'auditEventType', 'lan-ai-job-degraded');
    assertPayloadValue(busyJob.payload, 'lanAiProviderStatus', 'lan-ai-provider-busy');
    assertPayloadValue(busyJob.payload, 'lanAiProviderRoutingState', 'busy');
    assertPayloadValue(busyJob.payload, 'lanAiJobState', 'degraded');
    assertPayloadValue(busyJob.payload, 'unavailableReason', 'overloaded');
    labels.push(`${service.label}:busy-job-degraded`);

    return labels;
  } finally {
    socket.close();
  }
}

async function runDegradedProviderLifecycle(service) {
  const socket = await openWebSocket(service, allowedOrigin);
  try {
    const labels = [];
    await pairAndSelect(socket, service);
    labels.push(`${service.label}:route-selected`);

    const providerStatus = await sendCommand(
      socket,
      buildLanAiProviderStatusCommand(service, 'intent-platform-degraded-provider-status')
    );
    assertEvent(providerStatus, 'agent.lan-pairing.status.reported');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderStatus', 'lan-ai-provider-degraded');
    assertPayloadValue(providerStatus.payload, 'lanAiProviderRoutingState', 'degraded');
    labels.push(`${service.label}:provider-degraded`);

    const degradedJob = await sendCommand(
      socket,
      buildLanAiJobCommand(
        service,
        'intent-platform-degraded-provider-job',
        parentAuthorityActiveController,
        'chat-completion'
      )
    );
    assertEvent(degradedJob, 'agent.lan-ai.job.reported');
    assertPayloadValue(degradedJob.payload, 'auditEventType', 'lan-ai-job-degraded');
    assertPayloadValue(degradedJob.payload, 'lanAiProviderStatus', 'lan-ai-provider-degraded');
    assertPayloadValue(degradedJob.payload, 'lanAiProviderRoutingState', 'degraded');
    assertPayloadValue(degradedJob.payload, 'lanAiJobState', 'degraded');
    assertPayloadValue(degradedJob.payload, 'unavailableReason', 'provider-unavailable');
    labels.push(`${service.label}:degraded-job-degraded`);

    return labels;
  } finally {
    socket.close();
  }
}

async function pairAndSelect(socket, service) {
  const paired = await sendCommand(socket, buildPairingCommand(service));
  assertEvent(paired, 'agent.lan-pairing.status.reported');
  assertPayloadValue(paired.payload, 'auditEventType', 'pairing-proof-accepted');

  const selected = await sendCommand(socket, buildRouteSelectCommand(service, `intent-${service.label}-route-select`));
  assertEvent(selected, 'agent.lan-pairing.status.reported');
  assertPayloadValue(selected.payload, 'auditEventType', 'route-selected');
  assertPayloadValue(selected.payload, 'selectedChildDeviceId', service.childDeviceId);
  assertPayloadValue(selected.payload, 'selectedRouteId', service.routeId);
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
  throw new Error(`No LAN event queue available for ${messageId}`);
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
        reject(new Error(`Timed out waiting for LAN AI provider event after ${messageId}`));
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
    parentDeviceId: controllerDeviceId,
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

function buildLanAiProviderStatusCommand(service, intentId) {
  return buildCommand(
    service,
    intentId,
    'agent.lan-ai.provider.status.get',
    withParentAuthority(intentPayload(service, intentId, 'lan-ai-provider-status'), parentAuthorityObserver)
  );
}

function buildLanAiJobCommand(service, intentId, parentAuthority, capability) {
  return buildCommand(service, intentId, 'agent.lan-ai.job.submit', {
    ...withParentAuthority(intentPayload(service, intentId, 'lan-ai-job-submit'), parentAuthority),
    lanAiJobId: `lan-ai-job-${intentId}`,
    capabilityFlags: capability,
  });
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
    proofDigest: service.proofDigest,
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

function withParentAuthority(payload, parentAuthority) {
  return {
    ...payload,
    parentAuthority,
  };
}

function assertEvent(event, expected) {
  if (event.event !== expected) {
    throw new Error(`Expected event ${expected}, received ${event.event}`);
  }
}

function assertPayloadValue(payload, key, expected) {
  if (payload[key] !== expected) {
    throw new Error(`Expected LAN AI provider payload ${key}=${expected}, received ${payload[key]}`);
  }
}

function assertNoSensitiveEvidenceMarkers(payload) {
  const serialized = JSON.stringify(payload);
  for (const marker of sensitiveEvidenceMarkers) {
    if (serialized.includes(marker)) {
      throw new Error(`LAN AI provider proof payload exposed sensitive marker ${marker}`);
    }
  }
}

async function writeEvidence(assertions, checkedServices) {
  await writeFile(
    evidencePath,
    `${JSON.stringify(
      {
        checkedAt: new Date().toISOString(),
        allowedOrigin,
        assertions,
        services: checkedServices.map((service) => ({
          label: service.label,
          port: service.port,
          surface: service.surface,
          deviceRoles: service.deviceRoles,
          providerOptIn: service.providerOptIn,
          providerBusy: service.providerBusy === true,
          providerCapabilities: service.providerCapabilities,
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
