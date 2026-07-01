import { spawn } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import { AgentEventEnvelopeSchema } from '@ocentra-parent/schema-domain/agent-command-event-contracts';
import {
  ParentDevEnv,
  ParentDevHost,
  ParentDevPort,
  ParentDevValue,
  createAgentAddress,
  createAgentHealthUrl,
  createAgentWebSocketUrl,
  createHttpOrigin,
  isLikelyParentAgentOccupant,
} from '../dev/local-dev-config.mjs';
import { ensurePortFree } from '../dev/port-utils.mjs';
import { resolveDebugAgentServicePath } from './agent-service-process.mjs';
import { sendAgentWebSocketCommand, withTimeout } from './websocket-smoke-client.mjs';

const port = ParentDevPort.LanWebSocketSmokeAgent;
const allowedOrigin = createHttpOrigin(ParentDevHost.Loopback);
const healthUrl = createAgentHealthUrl(port);
const wsUrl = createAgentWebSocketUrl(port);
const childDeviceId = 'child-device-integration-lan';
const parentDeviceId = 'parent-device-integration-lan';
const pairingId = 'pairing-integration-lan';
const proofDigest = 'sha256:integration-lan-proof';
const routeId = 'route-integration-lan';
const issuedAt = '2026-05-23T14:40:00.000Z';
const expiresAt = '2099-05-23T14:45:00.000Z';
const controllerLeaseId = 'controller-lease-integration-lan';
const controllerLeaseExpiresAt = '2099-05-23T15:45:00.000Z';
const parentActorId = 'parent-actor-integration-lan';
const parentAuthority = 'active-controller';
const smokeTimeoutMs = 120000;
const commandTimeoutMs = 30000;

await ensurePortFree(port, isLikelyParentAgentOccupant, console.log, ParentDevHost.Wildcard);

const service = spawn(resolveDebugAgentServicePath(), [], {
  cwd: process.cwd(),
  env: {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(port, ParentDevHost.Wildcard),
    [ParentDevEnv.AgentAllowedOrigins]: allowedOrigin,
    [ParentDevEnv.AgentLocalNetworkEnabled]: ParentDevValue.True,
    OCENTRA_PARENT_AGENT_LAN_CHILD_DEVICE_ID: childDeviceId,
  },
  stdio: ['ignore', 'pipe', 'pipe'],
});

const serviceOutput = collectOutput(service);

try {
  await waitForHttp(healthUrl);
  await assertCorsOrigin();
  const received = await runWebSocketSmoke();
  if (!received.includes('agent.health.reported')) {
    throw new Error(`Expected LAN health event, received ${received.join(',')}`);
  }
  console.log(`websocket-lan-smoke-ok:${received.join(',')}`);
} finally {
  stopProcess(service);
}

async function assertCorsOrigin() {
  const response = await fetch(healthUrl, { headers: { Origin: allowedOrigin } });
  const returnedOrigin = response.headers.get('access-control-allow-origin');
  if (returnedOrigin !== allowedOrigin) {
    throw new Error(`Expected LAN CORS origin ${allowedOrigin}, received ${returnedOrigin}`);
  }
}

async function waitForHttp(url) {
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
  throw new Error(`Timed out waiting for ${url}\n${serviceOutput()}`);
}

function runWebSocketSmoke() {
  const events = [];
  return withTimeout(runLanWebSocketSmoke(events), smokeTimeoutMs, () =>
    lanTimeoutMessage('LAN WebSocket smoke', events)
  );
}

async function runLanWebSocketSmoke(events) {
  const pairing = await sendLanCommand(buildPairingCommand(), events);
  if (pairing.event !== 'agent.lan-pairing.status.reported') {
    throw new Error(`Expected LAN pairing status after proof, received ${pairing.event}`);
  }
  assertLanSupportSurface(pairing.payload);

  const routeSelection = await sendLanCommand(buildRouteSelectCommand(), events);
  if (routeSelection.event !== 'agent.lan-pairing.status.reported') {
    throw new Error(`Expected LAN route selection status, received ${routeSelection.event}`);
  }
  assertLanSupportSurface(routeSelection.payload);

  const pairedHealth = await sendLanCommand(buildPairedHealthCommand(), events);
  if (pairedHealth.event !== 'agent.health.reported') {
    throw new Error(`Expected paired LAN health report, received ${pairedHealth.event}`);
  }
  assertPayloadValue(pairedHealth.payload, 'intentKind', 'rule-query');
  assertPairedControlAccepted(pairedHealth.payload);

  const anonymous = await sendLanCommand(buildUnpairedHealthCommand(), events);
  if (anonymous.event !== 'agent.command.rejected') {
    throw new Error(`Expected anonymous LAN command rejection, received ${anonymous.event}`);
  }
  assertUnpairedControlRejected(anonymous.payload);

  return events;
}

function sendLanCommand(command, events) {
  return sendAgentWebSocketCommand({
    wsUrl,
    headers: { Origin: allowedOrigin },
    command,
    events,
    parseMessage: (message) => AgentEventEnvelopeSchema.parse(JSON.parse(String(message.data))),
    timeoutMs: commandTimeoutMs,
    timeoutMessage: () => lanTimeoutMessage(`LAN WebSocket ${command.command}`, events),
    errorMessage: `LAN WebSocket ${command.command} failed`,
    closeMessage: `LAN WebSocket ${command.command} closed before a command response`,
  });
}

function lanTimeoutMessage(scope, events) {
  const output = serviceOutput().trim();
  const eventList = events.join(',') || '<none>';
  if (output.length === 0) {
    return `${scope} timed out after events=${eventList}`;
  }
  return `${scope} timed out after events=${eventList}\n${output}`;
}

function assertUnpairedControlRejected(payload) {
  assertPayloadValue(payload, 'controlState', 'rejected');
  assertPayloadValue(payload, 'auditEventType', 'control-rejected');
  assertPayloadValue(payload, 'authenticationState', 'unauthenticated');
  assertPayloadValue(payload, 'rejectionReason', 'anonymous');
}

function assertPairedControlAccepted(payload) {
  assertPayloadValue(payload, 'controlState', 'accepted');
  assertPayloadValue(payload, 'auditEventType', 'control-accepted');
  assertPayloadValue(payload, 'authenticationState', 'paired');
  assertPayloadValue(payload, 'evidenceReferenceCount', 1);
  assertPayloadValue(payload, 'evidenceReferenceIds', 'activity-event-lan-control-1');
  assertPayloadValue(payload, 'controllerLeaseId', controllerLeaseId);
  assertPayloadValue(payload, 'controllerDeviceId', parentDeviceId);
  assertPayloadValue(payload, 'parentActorId', parentActorId);
}

function assertLanSupportSurface(payload) {
  assertPayloadValue(payload, 'transport', 'websocket');
  assertPayloadValue(
    payload,
    'supportedWebSocketCommands',
    'agent.lan-pairing.proof.submit,agent.lan-pairing.route.select,agent.lan-pairing.route.revoke,agent.lan-pairing.status.get,agent.lan-pairing.browser-discovery.scan,agent.lan-pairing.add-device.request,agent.lan-pairing.controller-lease.renew,agent.lan-pairing.controller-lease.release,agent.lan-pairing.controller-lease.takeover,agent.lan-ai.provider.status.get,agent.lan-ai.job.submit'
  );
  assertPayloadValue(
    payload,
    'unsupportedHttpEndpoints',
    '/api/lan-pairing/discovery,/api/lan-pairing/challenge,/api/lan-pairing/proof,/api/lan-pairing/control,/api/lan-pairing/registry'
  );
  assertPayloadValue(payload, 'discoveryStatus', 'websocket-direct');
  assertPayloadValue(payload, 'challengeStatus', 'websocket-direct');
  assertPayloadValue(payload, 'proofPreviewStatus', 'websocket-direct');
  assertPayloadValue(payload, 'lanAiProviderStatus', 'websocket-direct');
  assertPayloadValue(payload, 'lanAiJobStatus', 'websocket-direct');
  assertPayloadValue(payload, 'persistenceMode', 'local-json-registry');
  assertPayloadValue(payload, 'proofMode', 'direct-proof-submit');
  assertPayloadValue(
    payload,
    'routeRequirements',
    'paired-device,allowed-origin,target-device-match,route-id-match,unexpired-intent,non-replayed-intent,unrevoked-pairing,active-controller-lease,selected-device-reachable,parent-write-authority,lan-ai-job-authorized,discovery-state-explicit,route-recovery-persisted'
  );
  assertPayloadValue(
    payload,
    'manualProofGaps',
    'manual-lan-bind-proof,manual-firewall-proof,manual-physical-device-proof'
  );
}

function assertPayloadValue(payload, key, expected) {
  if (payload[key] !== expected) {
    throw new Error(`Expected LAN payload ${key}=${expected}, received ${payload[key]}`);
  }
}

function buildPairingCommand() {
  return buildCommand('cmd-integration-lan-pairing', 'agent.lan-pairing.proof.submit', {
    pairingId,
    challengeId: 'challenge-integration-lan',
    childDeviceId,
    parentDeviceId,
    routeId,
    origin: allowedOrigin,
    proofDigest,
    evidenceReferenceIds: 'activity-event-lan-control-1',
    startedAt: issuedAt,
    staleAt: expiresAt,
  });
}

function buildUnpairedHealthCommand() {
  return buildCommand('cmd-integration-lan-unpaired-health', 'agent.health.check', {});
}

function buildPairedHealthCommand() {
  return buildCommand('cmd-integration-lan-health', 'agent.health.check', {
    intentId: 'intent-integration-lan-health',
    intentKind: 'rule-query',
    pairingId,
    childDeviceId,
    routeId,
    origin: allowedOrigin,
    proofDigest,
    evidenceReferenceIds: 'activity-event-lan-control-1',
    startedAt: issuedAt,
    staleAt: expiresAt,
    controllerLeaseId,
    controllerDeviceId: parentDeviceId,
    parentActorId,
    parentAuthority,
    controllerLeaseIssuedAt: issuedAt,
    controllerLeaseExpiresAt,
  });
}

function buildRouteSelectCommand() {
  return buildCommand('cmd-integration-lan-route-select', 'agent.lan-pairing.route.select', {
    intentId: 'intent-integration-lan-route-select',
    intentKind: 'configuration-update',
    pairingId,
    childDeviceId,
    routeId,
    origin: allowedOrigin,
    proofDigest,
    startedAt: issuedAt,
    staleAt: expiresAt,
    controllerLeaseId,
    controllerDeviceId: parentDeviceId,
    parentActorId,
    parentAuthority,
    controllerLeaseIssuedAt: issuedAt,
    controllerLeaseExpiresAt,
  });
}

function buildCommand(messageId, command, payload) {
  return {
    schemaVersion: 1,
    messageId,
    sentAt: new Date().toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: childDeviceId, platform: 'windows', route: 'local-network' },
    command,
    payload,
  };
}

function stopProcess(child) {
  if (process.platform === 'win32' && child.pid !== undefined) {
    spawn('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
    return;
  }
  child.kill();
}

function collectOutput(child) {
  const chunks = [];
  child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
  return () => chunks.join('');
}
