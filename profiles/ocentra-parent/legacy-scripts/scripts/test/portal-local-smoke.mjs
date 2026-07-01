import { spawn } from 'node:child_process';
import { mkdtemp, readdir, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import {
  AgentCommand,
  AgentEvent,
  AgentEventEnvelopeSchema,
} from '@ocentra-parent/schema-domain/agent-command-event-contracts';
import { AgentLanBrowserAddDeviceReadModelSchema } from '@ocentra-parent/schema-domain/agent-lan-add-device';
import { AgentProtocolDefaults } from '@ocentra-parent/schema-domain/agent-protocol-defaults';

import {
  ParentDevEnv,
  ParentDevHost,
  ParentDevPort,
  createAgentAddress,
  createAgentHealthUrl,
  createAgentWebSocketUrl,
  createHttpOrigin,
  createPortalCommandsUrl,
  isLikelyParentAgentOccupant,
  isLikelyParentPortalOccupant,
  resolveParentDevPort,
} from '../dev/local-dev-config.mjs';
import { ensurePortFree } from '../dev/port-utils.mjs';
import {
  removeDirectoryWithRetry,
  resolveDebugAgentServicePath,
  spawnVitePortal,
  stopProcessTreeAndWait,
} from './agent-service-process.mjs';
import { PortalSmokeTargets, createPortalSmokeCommandEnvelope } from './websocket-command-envelope.mjs';

const agentPort = resolveParentDevPort(
  process.env[ParentDevEnv.AgentPort],
  ParentDevPort.PortalSmokeAgent,
  ParentDevEnv.AgentPort
);
const portalPort = resolveParentDevPort(
  process.env[ParentDevEnv.PortalPort],
  ParentDevPort.PortalSmokePortal,
  ParentDevEnv.PortalPort
);
const typedActivityAdapterSmokeTimeoutMs = positiveIntegerEnv(
  'OCENTRA_PARENT_PORTAL_ACTIVITY_SMOKE_TIMEOUT_MS',
  30_000
);
const lanBrowserDiscoverySmokeTimeoutMs = 30_000;
const devLogDir = await mkdtemp(join(tmpdir(), 'ocentra-parent-portal-log-'));

await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
await ensurePortFree(portalPort, isLikelyParentPortalOccupant, console.log);

const agent = spawn(resolveDebugAgentServicePath(), [], {
  cwd: process.cwd(),
  env: {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(agentPort),
    [ParentDevEnv.AgentAllowedOrigins]: createHttpOrigin(ParentDevHost.Loopback, portalPort),
    [ParentDevEnv.ActivityDbPath]: join(devLogDir, 'activity.sqlite'),
    [ParentDevEnv.DevLogDir]: devLogDir,
  },
  stdio: ['ignore', 'pipe', 'pipe'],
});

const portal = spawnVitePortal(portalPort, {
  ...process.env,
  [ParentDevEnv.PortalAgentWebSocketUrl]: createAgentWebSocketUrl(agentPort),
  [ParentDevEnv.DevLogDir]: devLogDir,
});

try {
  await waitForHttp(createAgentHealthUrl(agentPort));
  const portalResponse = await waitForHttp(createPortalCommandsUrl(portalPort));
  const html = await portalResponse.text();
  if (!html.includes('Ocentra Parent')) {
    throw new Error('Portal HTML shell did not include the expected title.');
  }
  await assertTypedActivityAdapterStates();
  await assertTypedLanBrowserDiscoveryReadModel();
  await assertDevServerLogWritten();
  console.log('portal-local-smoke-ok');
} finally {
  await Promise.all([stopProcess(portal), stopProcess(agent)]);
  await removeDirectoryWithRetry(devLogDir);
}

function assertTypedActivityAdapterStates() {
  const steps = [
    {
      messageId: 'cmd-portal-smoke-activity-report',
      command: AgentCommand.ActivityReportDailyGenerate,
      event: AgentEvent.ActivityReportGenerated,
      field: AgentProtocolDefaults.Field.ActivityReportDocument,
    },
    {
      messageId: 'cmd-portal-smoke-activity-report-history',
      command: AgentCommand.ActivityReportHistoryList,
      event: AgentEvent.ActivityReportHistoryReported,
      field: AgentProtocolDefaults.Field.ActivityReports,
    },
    {
      messageId: 'cmd-portal-smoke-activity-screen',
      command: AgentCommand.ActivityScreenReadModelGet,
      event: AgentEvent.ActivityScreenReadModelReported,
      field: AgentProtocolDefaults.Field.ActivityReadModel,
      readModelKind: 'screen',
    },
    {
      messageId: 'cmd-portal-smoke-activity-app-use',
      command: AgentCommand.ActivityAppUseReadModelGet,
      event: AgentEvent.ActivityAppUseReadModelReported,
      field: AgentProtocolDefaults.Field.ActivityReadModel,
      readModelKind: 'app-use',
    },
    {
      messageId: 'cmd-portal-smoke-activity-browser',
      command: AgentCommand.ActivityBrowserReadModelGet,
      event: AgentEvent.ActivityBrowserReadModelReported,
      field: AgentProtocolDefaults.Field.ActivityReadModel,
      readModelKind: 'browser',
    },
    {
      messageId: 'cmd-portal-smoke-activity-games',
      command: AgentCommand.ActivityGamesReadModelGet,
      event: AgentEvent.ActivityGamesReadModelReported,
      field: AgentProtocolDefaults.Field.ActivityReadModel,
      readModelKind: 'games',
    },
    {
      messageId: 'cmd-portal-smoke-activity-network',
      command: AgentCommand.ActivityNetworkReadModelGet,
      event: AgentEvent.ActivityNetworkReadModelReported,
      field: AgentProtocolDefaults.Field.ActivityReadModel,
      readModelKind: 'network',
    },
  ];

  return new Promise((resolve, reject) => {
    const socket = new WebSocket(createAgentWebSocketUrl(agentPort));
    let stepIndex = 0;
    let settled = false;
    const timer = setTimeout(
      () => fail(new Error(describeTypedActivityTimeout(steps, stepIndex))),
      typedActivityAdapterSmokeTimeoutMs
    );

    const fail = (error) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timer);
      socket.close();
      reject(error);
    };

    const complete = () => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timer);
      socket.close();
      resolve();
    };

    const sendCurrentStep = () => {
      const step = steps[stepIndex];
      socket.send(JSON.stringify(createPortalSmokeCommandEnvelope(step.messageId, step.command, activityPayload())));
    };

    socket.addEventListener('open', sendCurrentStep);

    socket.addEventListener('message', (message) => {
      try {
        const parsed = AgentEventEnvelopeSchema.parse(JSON.parse(String(message.data)));
        if (parsed.event === AgentEvent.ConnectionReady) {
          return;
        }

        const step = steps[stepIndex];
        if (parsed.event !== step.event) {
          fail(new Error(`Expected ${step.event}, received ${parsed.event}`));
          return;
        }
        assertSurfacePayload(parsed.payload, step.field, step.readModelKind);
        stepIndex += 1;
        if (stepIndex === steps.length) {
          complete();
          return;
        }
        sendCurrentStep();
      } catch (error) {
        fail(error instanceof Error ? error : new Error(String(error)));
      }
    });

    socket.addEventListener('error', () => fail(new Error('Typed Activity adapter smoke WebSocket failed')));
  });
}

function describeTypedActivityTimeout(steps, stepIndex) {
  const step = steps[Math.min(stepIndex, steps.length - 1)];
  return [
    `Typed Activity adapter smoke timed out after ${typedActivityAdapterSmokeTimeoutMs}ms`,
    `while waiting for ${step.event}`,
    `from ${step.command}`,
    `message ${step.messageId}.`,
  ].join(' ');
}

function assertTypedLanBrowserDiscoveryReadModel() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(createAgentWebSocketUrl(agentPort));
    let settled = false;
    const timer = setTimeout(
      () => fail(new Error('LAN browser discovery smoke timed out')),
      lanBrowserDiscoverySmokeTimeoutMs
    );

    const fail = (error) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timer);
      socket.close();
      reject(error);
    };

    const complete = () => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timer);
      socket.close();
      resolve();
    };

    socket.addEventListener('open', () => {
      socket.send(
        JSON.stringify(
          createPortalSmokeCommandEnvelope(
            'cmd-portal-smoke-lan-browser-discovery',
            AgentCommand.LanPairingBrowserDiscoveryScan,
            {
              schemaVersion: 1,
              requestedDiscoverySource: 'local-service',
            },
            PortalSmokeTargets.LocalNetworkWindowsAgent
          )
        )
      );
    });

    socket.addEventListener('message', (message) => {
      try {
        const parsed = AgentEventEnvelopeSchema.parse(JSON.parse(String(message.data)));
        if (parsed.event === AgentEvent.ConnectionReady) {
          return;
        }
        if (parsed.event !== AgentEvent.LanPairingBrowserDiscoveryReported) {
          fail(new Error(`Expected ${AgentEvent.LanPairingBrowserDiscoveryReported}, received ${parsed.event}`));
          return;
        }
        assertLanAddDeviceReadModel(parsed.payload);
        complete();
      } catch (error) {
        fail(error instanceof Error ? error : new Error(String(error)));
      }
    });

    socket.addEventListener('error', () => fail(new Error('LAN browser discovery smoke WebSocket failed')));
  });
}

function assertSurfacePayload(payload, jsonField, readModelKind) {
  const state = payload[AgentProtocolDefaults.Field.ActivitySurfaceState];
  if (!allowedActivityStates().has(state)) {
    throw new Error(`Activity adapter state was not typed: ${JSON.stringify(payload)}`);
  }
  if (readModelKind !== undefined && payload[AgentProtocolDefaults.Field.ActivityReadModelKind] !== readModelKind) {
    throw new Error(`Activity adapter returned wrong read-model kind: ${JSON.stringify(payload)}`);
  }
  const jsonValue = payload[jsonField];
  if (typeof jsonValue !== 'string') {
    throw new Error(`Activity adapter did not include JSON field ${jsonField}: ${JSON.stringify(payload)}`);
  }
  const parsed = JSON.parse(jsonValue);
  if (parsed.schemaVersion !== 1) {
    throw new Error(`Activity adapter returned unexpected schema version: ${jsonValue}`);
  }
  if (typeof parsed.state === 'string' && parsed.state !== state) {
    throw new Error(`Activity adapter payload state did not match event state: ${jsonValue}`);
  }
}

function assertLanAddDeviceReadModel(payload) {
  if (!allowedLanDiscoverySources().has(payload[AgentProtocolDefaults.Field.LanDiscoverySource])) {
    throw new Error(`LAN browser discovery source was not typed: ${JSON.stringify(payload)}`);
  }
  const jsonValue = payload[AgentProtocolDefaults.Field.LanAddDeviceReadModel];
  if (typeof jsonValue !== 'string') {
    throw new Error(`LAN browser discovery did not include addDeviceReadModel: ${JSON.stringify(payload)}`);
  }

  const readModel = AgentLanBrowserAddDeviceReadModelSchema.parse(JSON.parse(jsonValue));
  if (!allowedLanDiscoverySources().has(readModel.discoverySource)) {
    throw new Error(`LAN read model discovery source was not typed: ${jsonValue}`);
  }
  if (readModel.addDeviceState !== 'discovered' || readModel.localServiceDiscoveryState !== 'discovered') {
    throw new Error(`LAN local-service discovery was not discovered: ${jsonValue}`);
  }
  if (!allowedPhysicalLanStates().has(readModel.physicalHouseholdLanState)) {
    throw new Error(`LAN physical household state was not explicit: ${jsonValue}`);
  }
  if (readModel.cloudRelayState !== 'unavailable') {
    throw new Error(`LAN cloud relay state was not unavailable: ${jsonValue}`);
  }
  assertLanScanSummary(readModel, jsonValue);
  assertDiscoveredLanDevices(readModel, jsonValue);
  if (!readModel.honestNonClaims.includes('remote-desktop-not-implemented')) {
    throw new Error(`LAN read model claimed remote desktop support: ${jsonValue}`);
  }
  if (!readModel.honestNonClaims.includes('cloud-relay-not-implemented')) {
    throw new Error(`LAN read model missed cloud relay non-claim: ${jsonValue}`);
  }
}

function assertLanScanSummary(readModel, jsonValue) {
  if (readModel.scanSummary.scannedDeviceCount < 1) {
    throw new Error(`LAN scan summary did not include the local agent: ${jsonValue}`);
  }
  if (readModel.scanSummary.agentDeviceCount < 1) {
    throw new Error(`LAN scan summary did not count the connected child agent: ${jsonValue}`);
  }
  if (!readModel.scanSummary.sourceLabels.includes('local-service')) {
    throw new Error(`LAN scan summary missed local-service source evidence: ${jsonValue}`);
  }
}

function assertDiscoveredLanDevices(readModel, jsonValue) {
  const localAgent = readModel.discoveredDevices.find((device) => device.childDevice.deviceId === 'local-dev-agent');
  if (localAgent === undefined || localAgent.childDevice.agentStatus !== 'ocentra-local-service') {
    throw new Error(`LAN read model did not expose the connected local agent: ${jsonValue}`);
  }
  if (localAgent.childDevice.hardwareProfile === null) {
    throw new Error(`LAN connected agent did not expose typed inventory: ${jsonValue}`);
  }

  const router = readModel.discoveredDevices.find((device) => device.childDevice.platform === 'router');
  if (router !== undefined && router.childDevice.agentStatus !== undefined) {
    throw new Error(`LAN router row incorrectly claimed an agent: ${jsonValue}`);
  }
  if (router !== undefined && router.childDevice.hardwareProfile !== undefined) {
    throw new Error(`LAN router row incorrectly exposed agent inventory: ${jsonValue}`);
  }
}

function allowedLanDiscoverySources() {
  return new Set(['local-service', 'physical-household-lan']);
}

function allowedPhysicalLanStates() {
  return new Set(['manual-required', 'discovered']);
}

function allowedActivityStates() {
  return new Set(['ready', 'empty', 'unavailable', 'offline', 'stale', 'permission-required', 'scaffold-only']);
}

function activityPayload() {
  return {
    [AgentProtocolDefaults.Field.ScopeKind]: 'family',
    [AgentProtocolDefaults.Field.FamilyId]: 'family-local',
    [AgentProtocolDefaults.Field.RangeStart]: '1970-01-01T00:00:00Z',
    [AgentProtocolDefaults.Field.RangeEnd]: new Date().toISOString(),
  };
}

async function waitForHttp(url) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30000) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return response;
      }
    } catch {
      await delay(250);
    }
  }
  throw new Error(`Timed out waiting for ${url}`);
}

async function stopProcess(child) {
  await stopProcessTreeAndWait(child);
}

function positiveIntegerEnv(envName, fallback) {
  const value = Number.parseInt(process.env[envName] ?? '', 10);
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

async function assertDevServerLogWritten() {
  const files = await readdir(devLogDir);
  const devServerLog = files.find((file) => file.startsWith('dev-server-') && file.endsWith('.ndjson'));
  if (devServerLog === undefined) {
    throw new Error(`Vite dev server log was not written in ${devLogDir}`);
  }

  const content = await readFile(join(devLogDir, devServerLog), 'utf8');
  if (!content.includes('Vite dev server started.')) {
    throw new Error(`Vite dev server log did not include startup entry:\n${content}`);
  }
}
