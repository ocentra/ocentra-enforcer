import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import {
  AgentCommand,
  AgentEvent,
  AgentEventEnvelopeSchema,
} from '@ocentra-parent/schema-domain/agent-command-event-contracts';
import { AgentProtocolDefaults } from '@ocentra-parent/schema-domain/agent-protocol-defaults';
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

const LocalAiEnv = {
  RuntimeBinary: 'OCENTRA_PARENT_LOCAL_AI_RUNTIME_BINARY',
  RequestedModelId: 'OCENTRA_PARENT_LOCAL_AI_REQUESTED_MODEL_ID',
  ModelFile: 'OCENTRA_PARENT_LOCAL_AI_MODEL_FILE',
  RuntimeDevice: 'OCENTRA_PARENT_LOCAL_AI_RUNTIME_DEVICE',
  GpuLayers: 'OCENTRA_PARENT_LOCAL_AI_GPU_LAYERS',
  SplitMode: 'OCENTRA_PARENT_LOCAL_AI_SPLIT_MODE',
  TensorSplit: 'OCENTRA_PARENT_LOCAL_AI_TENSOR_SPLIT',
  MainGpu: 'OCENTRA_PARENT_LOCAL_AI_MAIN_GPU',
  Fit: 'OCENTRA_PARENT_LOCAL_AI_FIT',
  FitTarget: 'OCENTRA_PARENT_LOCAL_AI_FIT_TARGET',
  OpOffload: 'OCENTRA_PARENT_LOCAL_AI_OP_OFFLOAD',
  CpuMoe: 'OCENTRA_PARENT_LOCAL_AI_CPU_MOE',
  CpuMoeLayers: 'OCENTRA_PARENT_LOCAL_AI_CPU_MOE_LAYERS',
  ExecutionEnabled: 'OCENTRA_PARENT_LOCAL_AI_EXECUTION_ENABLED',
  MaxTokens: 'OCENTRA_PARENT_LOCAL_AI_GENERATION_MAX_TOKENS',
  TimeoutMs: 'OCENTRA_PARENT_LOCAL_AI_GENERATION_TIMEOUT_MS',
  ProofPrompt: 'OCENTRA_PARENT_LOCAL_AI_PROOF_PROMPT',
  ProofExpectedText: 'OCENTRA_PARENT_LOCAL_AI_PROOF_EXPECTED_TEXT',
  ProofExpectedResourceClass: 'OCENTRA_PARENT_LOCAL_AI_PROOF_EXPECTED_RESOURCE_CLASS',
  ProofTimeoutMs: 'OCENTRA_PARENT_LOCAL_AI_PROOF_TIMEOUT_MS',
};

const runtimeBinary = optionalExistingPath(LocalAiEnv.RuntimeBinary);
const modelFile = optionalExistingPath(LocalAiEnv.ModelFile);
const requestedModelId = optionalTrimmedEnv(LocalAiEnv.RequestedModelId);
const port = ParentDevPort.WebSocketSmokeAgent;
const healthUrl = createAgentHealthUrl(port);
const wsUrl = createAgentWebSocketUrl(port);
const devLogDir = await mkdtemp(join(tmpdir(), 'ocentra-parent-local-ai-proof-'));
const proofPrompt = process.env[LocalAiEnv.ProofPrompt] ?? 'Reply with the exact phrase local-ok.';
const proofExpectedText = process.env[LocalAiEnv.ProofExpectedText] ?? 'local-ok';
const proofExpectedResourceClass = process.env[LocalAiEnv.ProofExpectedResourceClass];
const proofTimeoutMs = positiveIntegerEnv(LocalAiEnv.ProofTimeoutMs, 120000);

await ensurePortFree(port, isLikelyParentAgentOccupant, console.log);

const service = spawn(resolveDebugAgentServicePath(), [], {
  cwd: process.cwd(),
  env: {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(port),
    [ParentDevEnv.ActivityDbPath]: join(devLogDir, 'activity.sqlite'),
    [ParentDevEnv.DevLogDir]: devLogDir,
    [LocalAiEnv.ExecutionEnabled]: 'true',
    [LocalAiEnv.MaxTokens]: process.env[LocalAiEnv.MaxTokens] ?? '32',
    [LocalAiEnv.TimeoutMs]: process.env[LocalAiEnv.TimeoutMs] ?? '180000',
    ...optionalPathEnv(LocalAiEnv.RuntimeBinary, runtimeBinary),
    ...optionalPathEnv(LocalAiEnv.ModelFile, modelFile),
    ...optionalProcessEnv(
      LocalAiEnv.RuntimeDevice,
      LocalAiEnv.GpuLayers,
      LocalAiEnv.SplitMode,
      LocalAiEnv.TensorSplit,
      LocalAiEnv.MainGpu,
      LocalAiEnv.Fit,
      LocalAiEnv.FitTarget,
      LocalAiEnv.OpOffload,
      LocalAiEnv.CpuMoe,
      LocalAiEnv.CpuMoeLayers
    ),
  },
  stdio: ['ignore', 'pipe', 'pipe'],
});

const serviceOutput = collectOutput(service);

try {
  await waitForHttp(healthUrl);
  const result = await runLocalAiProof();
  console.log(`local-ai-chat-proof-ok:${result.slice(0, 200)}`);
} finally {
  await stopProcessTreeAndWait(service);
  await rm(devLogDir, { recursive: true, force: true });
}

function runLocalAiProof() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    let settled = false;
    const timer = setTimeout(() => {
      fail(new Error('Local AI chat proof timed out'));
    }, proofTimeoutMs);

    const fail = (error) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timer);
      socket.close();
      reject(error);
    };

    const complete = (outputText) => {
      if (settled) {
        return;
      }
      settled = true;
      clearTimeout(timer);
      socket.close();
      resolve(outputText);
    };

    socket.addEventListener('open', () => {
      socket.send(
        JSON.stringify(
          commandEnvelope('cmd-local-ai-runtime-status', AgentCommand.LocalAiRuntimeStatusGet, modelRequestPayload())
        )
      );
    });

    socket.addEventListener('message', (message) => {
      try {
        const parsed = AgentEventEnvelopeSchema.parse(JSON.parse(String(message.data)));
        if (parsed.event === AgentEvent.LocalAiRuntimeStatusReported) {
          assertRuntimeReady(parsed.payload);
          socket.send(
            JSON.stringify(
              commandEnvelope('cmd-local-ai-chat-proof', AgentCommand.LocalAiChatGenerate, {
                ...modelRequestPayload(),
                [AgentProtocolDefaults.Field.LocalAiPrompt]: proofPrompt,
                [AgentProtocolDefaults.Field.LocalAiMaxOutputTokens]: 32,
                [AgentProtocolDefaults.Field.LocalAiTimeoutMs]: proofTimeoutMs,
              })
            )
          );
        }

        if (parsed.event === AgentEvent.LocalAiChatGenerationReported) {
          const outputText = parsed.payload[AgentProtocolDefaults.Field.LocalAiOutputText];
          if (
            parsed.payload[AgentProtocolDefaults.Field.LocalAiGenerationState] !== 'complete' ||
            typeof outputText !== 'string' ||
            outputText.trim() === ''
          ) {
            fail(new Error(`Local AI chat did not complete: ${JSON.stringify(parsed.payload)}`));
            return;
          }
          if (!outputText.includes(proofExpectedText)) {
            fail(new Error(`Local AI chat did not include expected text ${JSON.stringify(proofExpectedText)}`));
            return;
          }
          complete(outputText.trim());
        }
      } catch (error) {
        fail(error instanceof Error ? error : new Error(String(error)));
      }
    });

    socket.addEventListener('error', () => {
      fail(new Error('Local AI chat proof WebSocket failed'));
    });
  });
}

function commandEnvelope(messageId, command, payload) {
  return {
    schemaVersion: 1,
    messageId,
    sentAt: new Date().toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command,
    payload,
  };
}

function modelRequestPayload() {
  return requestedModelId === undefined ? {} : { [AgentProtocolDefaults.Field.LocalAiModelId]: requestedModelId };
}

function assertRuntimeReady(payload) {
  if (
    payload[AgentProtocolDefaults.Field.LocalAiExecutionAllowed] !== true ||
    payload[AgentProtocolDefaults.Field.LocalAiExecutionState] !== 'dry-run-ready'
  ) {
    throw new Error(`Local AI runtime is not execution-ready: ${JSON.stringify(payload)}`);
  }
  if (
    proofExpectedResourceClass !== undefined &&
    payload[AgentProtocolDefaults.Field.LocalAiResourceClass] !== proofExpectedResourceClass
  ) {
    throw new Error(`Local AI runtime did not report expected resource class: ${JSON.stringify(payload)}`);
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

function optionalExistingPath(envName) {
  const value = process.env[envName]?.trim();
  if (value === undefined || value.length === 0) {
    return undefined;
  }
  if (!existsSync(value)) {
    throw new Error(`${envName} does not exist: ${value}`);
  }
  return value;
}

function optionalTrimmedEnv(envName) {
  const value = process.env[envName]?.trim();
  return value === undefined || value.length === 0 ? undefined : value;
}

function positiveIntegerEnv(envName, fallback) {
  const value = Number.parseInt(process.env[envName] ?? '', 10);
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

function optionalProcessEnv(...envNames) {
  return Object.fromEntries(
    envNames.filter((envName) => process.env[envName] !== undefined).map((envName) => [envName, process.env[envName]])
  );
}

function optionalPathEnv(envName, value) {
  return value === undefined ? {} : { [envName]: value };
}

function collectOutput(child) {
  const chunks = [];
  child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
  return () => chunks.join('');
}
