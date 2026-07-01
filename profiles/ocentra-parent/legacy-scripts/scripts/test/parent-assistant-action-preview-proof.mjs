import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import {
  ParentDevEnv,
  ParentDevPort,
  createAgentAddress,
  createAgentHealthUrl,
  createAgentWebSocketUrl,
  isLikelyParentAgentOccupant,
} from '../dev/local-dev-config.mjs';
import { ensurePortFree } from '../dev/port-utils.mjs';
import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const proofPort = ParentDevPort.WebSocketSmokeAgent;
const healthUrl = createAgentHealthUrl(proofPort);
const wsUrl = createAgentWebSocketUrl(proofPort);
const devLogDir = await mkdtemp(join(tmpdir(), 'ocentra-parent-assistant-action-preview-proof-'));
let AgentCommand;
let AgentEvent;
let AgentEventEnvelopeSchema;
let AgentProtocolDefaults;

await runPackageCommand(['run', 'build:contracts']);
({ AgentCommand, AgentEvent, AgentEventEnvelopeSchema } =
  await import('@ocentra-parent/schema-domain/agent-command-event-contracts'));
({ AgentProtocolDefaults } = await import('@ocentra-parent/schema-domain/agent-protocol-defaults'));
await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
await ensurePortFree(proofPort, isLikelyParentAgentOccupant, console.log);

const service = spawn(resolveDebugAgentServicePath(), [], {
  cwd: process.cwd(),
  env: {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(proofPort),
    [ParentDevEnv.DevLogDir]: devLogDir,
    OCENTRA_PARENT_LOCAL_AI_EXECUTION_ENABLED: 'false',
  },
  stdio: ['ignore', 'pipe', 'pipe'],
});

const serviceOutput = collectOutput(service);

try {
  await waitForHttp(healthUrl);
  await runActionPreviewProof();
  console.log('parent-assistant-action-preview-proof-ok');
} finally {
  await stopProcessTreeAndWait(service);
  await rm(devLogDir, { recursive: true, force: true });
}

function runActionPreviewProof() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    let settled = false;
    let phase = 'preview';
    const timer = setTimeout(() => fail(new Error('Parent Assistant action-preview proof timed out')), 30000);

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
      socket.send(JSON.stringify(actionPreviewCommand()));
    });

    socket.addEventListener('message', (message) => {
      try {
        const parsed = AgentEventEnvelopeSchema.parse(JSON.parse(String(message.data)));
        if (parsed.event === AgentEvent.ConnectionReady) {
          return;
        }
        if (phase === 'preview') {
          if (parsed.event !== AgentEvent.ParentAssistantActionPreviewed) {
            fail(new Error(`Expected ${AgentEvent.ParentAssistantActionPreviewed}, received ${parsed.event}`));
            return;
          }
          const result = assertActionPreviewResult(parsed.payload);
          phase = 'confirm';
          socket.send(JSON.stringify(actionConfirmCommand(result.preview.previewId)));
          return;
        }

        if (phase === 'confirm') {
          if (parsed.event !== AgentEvent.ParentAssistantActionConfirmed) {
            fail(new Error(`Expected ${AgentEvent.ParentAssistantActionConfirmed}, received ${parsed.event}`));
            return;
          }
          assertActionConfirmResult(parsed.payload);
          phase = 'raw-prose';
          socket.send(JSON.stringify(rawProseActionConfirmCommand()));
          return;
        }

        if (parsed.event !== AgentEvent.ParentAssistantActionConfirmed) {
          fail(new Error(`Expected raw-prose ${AgentEvent.ParentAssistantActionConfirmed}, received ${parsed.event}`));
          return;
        }
        assertRawProseConfirmRejected(parsed.payload);
        complete();
      } catch (error) {
        fail(error instanceof Error ? error : new Error(String(error)));
      }
    });

    socket.addEventListener('error', () => fail(new Error('Parent Assistant action-preview proof WebSocket failed')));
  });
}

function assertActionPreviewResult(payload) {
  const result = parseJsonField(payload, AgentProtocolDefaults.Field.ParentAssistantActionPreview);
  if (
    result.previewState !== 'draft' ||
    result.preview?.actionKind !== 'policy-suggestion' ||
    result.requiresControllerLease !== true ||
    result.childAgentContractRequired !== true ||
    result.previewRequired !== true ||
    result.previewSatisfied !== true ||
    result.rawAssistantProseAccepted !== false ||
    result.parentConfirmationRequired !== true ||
    result.parentConfirmationRecorded !== false ||
    result.childAgentValidationState !== 'child-agent-contract-required' ||
    !Array.isArray(result.evidenceContext) ||
    result.evidenceContext.length < 2 ||
    !Array.isArray(result.sourceRefs) ||
    result.sourceRefs.length < 2
  ) {
    throw new Error(`Parent Assistant action preview was not a policy draft: ${JSON.stringify(result)}`);
  }
  const reportContext = result.evidenceContext.find((context) => context.citationLabel === 'Activity report');
  if (
    reportContext?.evidence?.evidenceReferenceId !== 'activity-report-daily-local' ||
    !String(reportContext.allowedSummary).includes('fileName=activity-report-daily-local.json') ||
    !String(reportContext.allowedSummary).includes('savedAt=2026-05-28T14:54:02Z') ||
    !String(reportContext.allowedSummary).includes(
      'storageReason=Activity report is saved in local parent report storage.'
    ) ||
    !String(reportContext.allowedSummary).includes('savedState=saved') ||
    !String(reportContext.allowedSummary).includes('offlineSources=1') ||
    !String(reportContext.allowedSummary).includes('staleSources=1') ||
    !String(reportContext.allowedSummary).includes('unreachableSources=1')
  ) {
    throw new Error(
      `Parent Assistant action preview did not cite the saved Activity report: ${JSON.stringify(result)}`
    );
  }
  if (
    result.enforcementApplied !== false ||
    result.policyWritten !== false ||
    result.preview.enforcementApplied !== false
  ) {
    throw new Error(`Parent Assistant action preview applied enforcement or wrote policy: ${JSON.stringify(result)}`);
  }
  return result;
}

function assertActionConfirmResult(payload) {
  const result = parseJsonField(payload, AgentProtocolDefaults.Field.ParentAssistantActionConfirmResult);
  if (
    result.confirmState !== 'contract-required' ||
    result.previewId !== 'parent-assistant-action-preview-local' ||
    result.previewRequired !== true ||
    result.previewSatisfied !== true ||
    result.rawAssistantProseAccepted !== false ||
    result.parentConfirmationRequired !== true ||
    result.parentConfirmationRecorded !== false ||
    result.childAgentValidationState !== 'child-agent-contract-required' ||
    !Array.isArray(result.sourceRefs) ||
    result.sourceRefs.length < 2
  ) {
    throw new Error(`Parent Assistant action confirm did not require child contract: ${JSON.stringify(result)}`);
  }
  if (result.enforcementApplied !== false || result.policyWritten !== false) {
    throw new Error(`Parent Assistant action confirm applied enforcement or wrote policy: ${JSON.stringify(result)}`);
  }
}

function assertRawProseConfirmRejected(payload) {
  const result = parseJsonField(payload, AgentProtocolDefaults.Field.ParentAssistantActionConfirmResult);
  if (
    result.confirmState !== 'rejected' ||
    result.previewSatisfied !== false ||
    result.rawAssistantProseAccepted !== false ||
    result.reason !== 'Action confirmation rejected because raw assistant prose is not an executable action intent.'
  ) {
    throw new Error(`Parent Assistant raw-prose confirm was not rejected: ${JSON.stringify(result)}`);
  }
  if (result.enforcementApplied !== false || result.policyWritten !== false) {
    throw new Error(
      `Parent Assistant raw-prose confirm applied enforcement or wrote policy: ${JSON.stringify(result)}`
    );
  }
}

function actionPreviewCommand() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-parent-assistant-action-preview-proof',
    sentAt: new Date().toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command: AgentCommand.ParentAssistantActionPreview,
    payload: {
      [AgentProtocolDefaults.Field.ParentAssistantActionIntentId]: 'parent-assistant-action-intent-proof',
      [AgentProtocolDefaults.Field.ParentAssistantQuestion]: 'Suggest a policy rule from recent activity.',
      [AgentProtocolDefaults.Field.ActivityReportDocument]: JSON.stringify(activityReport()),
    },
  };
}

function actionConfirmCommand(previewId) {
  return {
    schemaVersion: 1,
    messageId: 'cmd-parent-assistant-action-confirm-proof',
    sentAt: new Date().toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command: AgentCommand.ParentAssistantActionConfirm,
    payload: {
      [AgentProtocolDefaults.Field.ParentAssistantActionIntentId]: 'parent-assistant-action-intent-proof',
      [AgentProtocolDefaults.Field.ParentAssistantActionPreviewId]: previewId,
      [AgentProtocolDefaults.Field.ParentAssistantActionAuditReason]:
        'Parent confirmed a preview-only action boundary.',
      [AgentProtocolDefaults.Field.ActivityReportDocument]: JSON.stringify(activityReport()),
    },
  };
}

function rawProseActionConfirmCommand() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-parent-assistant-action-raw-prose-proof',
    sentAt: new Date().toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command: AgentCommand.ParentAssistantActionConfirm,
    payload: {
      [AgentProtocolDefaults.Field.ParentAssistantActionIntentId]: 'parent-assistant-action-intent-proof',
      [AgentProtocolDefaults.Field.ParentAssistantActionPreviewId]: 'parent-assistant-action-preview-local',
      [AgentProtocolDefaults.Field.ParentAssistantActionRawProse]: 'Please just block games tonight.',
      [AgentProtocolDefaults.Field.ActivityReportDocument]: JSON.stringify(activityReport()),
    },
  };
}

function activityReport() {
  return {
    schemaVersion: 1,
    reportId: 'activity-report-daily-local',
    frequency: 'daily',
    scope: {
      scopeKind: 'family',
      familyId: 'family-local',
      deviceId: null,
    },
    requestedAt: '2026-05-28T14:54:00Z',
    rangeStart: '2026-05-28T00:00:00Z',
    rangeEnd: '2026-05-28T14:54:00Z',
    generatedAt: '2026-05-28T14:54:01Z',
    savedMetadata: {
      reportId: 'activity-report-daily-local',
      fileName: 'activity-report-daily-local.json',
      savedState: 'saved',
      savedAt: '2026-05-28T14:54:02Z',
      storageReason: 'Activity report is saved in local parent report storage.',
      custodyLabel: 'parent-device-local-report-json',
      sourceLabel: 'saved-report-json',
      rawChildEvidenceIncluded: false,
    },
    sourceStates: [
      {
        deviceId: 'local-dev-agent',
        reachabilityState: 'reachable',
        state: 'ready',
        reason: 'Family scope is backed by the reachable local child-device query store.',
        lastUpdatedAt: '2026-05-28T14:53:00Z',
        custodyLabel: 'child-device-local-summary',
        sourceLabel: 'activity-query-store-summary',
        rawChildEvidenceIncluded: false,
      },
      {
        deviceId: 'family-child-offline',
        reachabilityState: 'offline',
        state: 'offline',
        reason: 'Child-device source is registered but not reachable for this report request.',
        lastUpdatedAt: null,
        custodyLabel: 'child-device-local-summary',
        sourceLabel: 'family-fanout-source-state',
        rawChildEvidenceIncluded: false,
      },
      {
        deviceId: 'family-child-stale',
        reachabilityState: 'unreachable',
        state: 'stale',
        reason: 'Child-device source has stale report material and needs a fresh activity sync.',
        lastUpdatedAt: '2026-05-28T13:54:00Z',
        custodyLabel: 'child-device-local-summary',
        sourceLabel: 'family-fanout-source-state',
        rawChildEvidenceIncluded: false,
      },
    ],
    sections: [
      {
        sectionKind: 'summary',
        title: 'Summary',
        state: 'ready',
        summary: 'Activity data is available from the local query store.',
        itemCount: 1,
        evidence: [],
      },
    ],
  };
}

function parseJsonField(payload, field) {
  const value = payload[field];
  if (typeof value !== 'string') {
    throw new Error(`Expected string JSON field ${field}: ${JSON.stringify(payload)}`);
  }
  return JSON.parse(value);
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

function runCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: process.cwd(),
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const output = collectOutput(child);
    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} failed with ${code}\n${output()}`));
    });
  });
}

function runPackageCommand(args) {
  if (process.platform === 'win32') {
    return runCommand(...npmCommand([...args]));
  }

  return runCommand('npm', args);
}

function collectOutput(child) {
  const chunks = [];
  child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
  return () => chunks.join('');
}

function npmCommand(args) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return [command, commandArgs];
}
