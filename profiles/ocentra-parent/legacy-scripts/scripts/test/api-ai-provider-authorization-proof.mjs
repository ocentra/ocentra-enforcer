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
const devLogDir = await mkdtemp(join(tmpdir(), 'ocentra-api-ai-provider-proof-'));
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
    OCENTRA_PARENT_PARENT_ASSISTANT_API_AI_AUTHORIZED: 'true',
    OCENTRA_PARENT_PARENT_ASSISTANT_API_AI_DEGRADED: 'true',
  },
  stdio: ['ignore', 'pipe', 'pipe'],
});

const serviceOutput = collectOutput(service);

try {
  await waitForHttp(healthUrl);
  await runApiProviderAuthorizationProof();
  console.log('api-ai-provider-authorization-proof-ok');
} finally {
  await stopProcessTreeAndWait(service);
  await rm(devLogDir, { recursive: true, force: true });
}

function runApiProviderAuthorizationProof() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    let settled = false;
    let stepIndex = 0;
    const steps = [
      {
        name: 'env-only-fails-closed',
        authorizationTerms: false,
        assertPayload: assertEnvOnlyFailsClosedAnswerResult,
      },
      {
        name: 'explicit-parent-authorized-degraded',
        authorizationTerms: true,
        assertPayload: assertAuthorizedDegradedAnswerResult,
      },
    ];
    const timer = setTimeout(() => fail(new Error('API AI provider authorization proof timed out')), 30000);

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
      socket.send(JSON.stringify(answerGenerateCommand(steps[stepIndex])));
    });

    socket.addEventListener('message', (message) => {
      try {
        const parsed = AgentEventEnvelopeSchema.parse(JSON.parse(String(message.data)));
        if (parsed.event === AgentEvent.ConnectionReady) {
          return;
        }
        if (parsed.event !== AgentEvent.ParentAssistantAnswerReported) {
          fail(new Error(`Expected ${AgentEvent.ParentAssistantAnswerReported}, received ${parsed.event}`));
          return;
        }

        steps[stepIndex].assertPayload(parsed.payload);
        stepIndex += 1;
        if (stepIndex >= steps.length) {
          complete();
          return;
        }
        socket.send(JSON.stringify(answerGenerateCommand(steps[stepIndex])));
      } catch (error) {
        fail(error instanceof Error ? error : new Error(String(error)));
      }
    });

    socket.addEventListener('error', () => fail(new Error('API AI provider authorization proof WebSocket failed')));
  });
}

function assertEnvOnlyFailsClosedAnswerResult(payload) {
  const answer = parseJsonField(payload, AgentProtocolDefaults.Field.ParentAssistantAnswer);
  const boundary = answer.apiProviderBoundary;
  if (
    boundary.authorizationState !== 'not-authorized' ||
    boundary.accessState !== 'not-authorized' ||
    boundary.providerState !== 'unavailable' ||
    boundary.retentionState !== 'no-retention-without-parent-authorization'
  ) {
    throw new Error(`API AI provider did not fail closed without explicit parent terms: ${JSON.stringify(boundary)}`);
  }
  if (boundary.childSafetyOrEnforcementUseAllowed !== false || answer.actionPreview.enforcementApplied !== false) {
    throw new Error(
      `API AI provider crossed into child safety or enforcement without explicit terms: ${JSON.stringify(answer)}`
    );
  }
}

function assertAuthorizedDegradedAnswerResult(payload) {
  const answer = parseJsonField(payload, AgentProtocolDefaults.Field.ParentAssistantAnswer);
  const boundary = answer.apiProviderBoundary;
  if (
    boundary.authorizationState !== 'authorized' ||
    boundary.accessState !== 'authorized-degraded' ||
    boundary.providerState !== 'degraded' ||
    boundary.parentAuthorizationRequired !== true ||
    boundary.evidenceCitationRequired !== true
  ) {
    throw new Error(`API AI provider boundary did not expose authorized degraded state: ${JSON.stringify(boundary)}`);
  }
  if (
    boundary.custodyState !== 'parent-owned-citations-only' ||
    boundary.retentionState !== 'parent-authorized-no-default-retention' ||
    boundary.deletionState !== 'delete-provider-cache-on-parent-request' ||
    boundary.citations.length < 1
  ) {
    throw new Error(
      `API AI provider boundary missed custody, retention, deletion, or citation proof: ${JSON.stringify(boundary)}`
    );
  }
  if (boundary.childSafetyOrEnforcementUseAllowed !== false) {
    throw new Error(`API AI provider was allowed to drive child safety or enforcement: ${JSON.stringify(boundary)}`);
  }
  if (
    answer.providerState !== 'unavailable' ||
    answer.answerState !== 'unavailable' ||
    answer.answerText !== undefined ||
    answer.actionPreview.enforcementApplied !== false
  ) {
    throw new Error(
      `Parent Assistant answer used API AI as a local policy/enforcement decision path: ${JSON.stringify(answer)}`
    );
  }
}

function answerGenerateCommand(step) {
  return {
    schemaVersion: 1,
    messageId: `cmd-api-ai-provider-authorization-proof-${step.name}`,
    sentAt: new Date().toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command: AgentCommand.ParentAssistantAnswerGenerate,
    payload: {
      [AgentProtocolDefaults.Field.ParentAssistantRequestId]: `parent-assistant-request-api-proof-${step.name}`,
      [AgentProtocolDefaults.Field.ParentAssistantQuestion]: 'Suggest a policy rule from recent activity.',
      [AgentProtocolDefaults.Field.ParentAssistantEvidenceSummary]:
        'Recent Activity evidence is cited for parent review.',
      ...(step.authorizationTerms ? parentAuthorizedApiContextPayload() : {}),
    },
  };
}

function parentAuthorizedApiContextPayload() {
  return {
    [AgentProtocolDefaults.Field.ParentAssistantApiAuthorizationState]: 'authorized',
    [AgentProtocolDefaults.Field.ParentAssistantApiCustodyLabel]: 'parent-authorized-api-ai',
    [AgentProtocolDefaults.Field.ParentAssistantApiRetentionState]: 'parent-authorized-no-default-retention',
    [AgentProtocolDefaults.Field.ParentAssistantApiDeletionState]: 'delete-provider-cache-on-parent-request',
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
