import { spawn } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';
import {
  AgentCommand,
  AgentEvent,
  AgentEventEnvelopeSchema,
} from '@ocentra-parent/schema-domain/agent-command-event-contracts';
import { AgentProtocolDefaults } from '@ocentra-parent/schema-domain/agent-protocol-defaults';

import {
  ParentDevEnv,
  createAgentAddress,
  createAgentHealthUrl,
  createAgentWebSocketUrl,
  isLikelyParentAgentOccupant,
  resolveParentDevPort,
} from '../dev/local-dev-config.mjs';
import { ensurePortFree } from '../dev/port-utils.mjs';
import { removeDirectoryWithRetry, stopProcessTreeAndWait } from './agent-service-process.mjs';

const repoRoot = process.cwd();
const outputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'service-analysis');
const agentPort = resolveParentDevPort(
  process.env.OCENTRA_SCREEN_AI_ANALYSIS_PROOF_AGENT_PORT,
  4689,
  'OCENTRA_SCREEN_AI_ANALYSIS_PROOF_AGENT_PORT'
);
const buildRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-ai-analysis-target-'));
const activityRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-ai-analysis-'));
const queueDir = join(activityRoot, 'screen-queue');
const journalPath = join(activityRoot, 'activity.ndjson');
const keyPath = join(activityRoot, 'activity-journal.key');
const storePath = join(activityRoot, 'activity.sqlite');
const fixturePath = join(activityRoot, 'screen-ai-service-analysis-fixture.html');
const adapterCommandPath = join(
  activityRoot,
  process.platform === 'win32' ? 'screen-ai-adapter.cmd' : 'screen-ai-adapter.sh'
);
const adapterScriptPath = join(
  activityRoot,
  process.platform === 'win32' ? 'screen-ai-adapter.ps1' : 'screen-ai-adapter-node.mjs'
);
const queuePath = join(queueDir, 'screen-evidence-queue.ndjson');
const healthUrl = createAgentHealthUrl(agentPort);
const wsUrl = createAgentWebSocketUrl(agentPort);

if (process.platform !== 'win32') {
  throw new Error('screen-ai-service-analysis-proof requires a real Windows desktop capture surface.');
}

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });
writeFixture();
writeAdapterCommand();

await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);

let browser;
let service;
let serviceOutput = () => '';
let lastObservedReadModel = null;

try {
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service'], {
    CARGO_TARGET_DIR: buildRoot,
  });
  browser = await chromium.launch({ headless: false });
  const page = await browser.newPage({ viewport: { width: 960, height: 620 } });
  await page.goto(pathToFileURL(fixturePath).href);
  await page.bringToFront();
  await page.waitForTimeout(750);

  service = startService(serviceCaptureEnv());
  serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  await waitForQueueRecords(1);
  await delay(1500);
  await stopProcessTreeAndWait(service);
  service = undefined;

  await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
  service = startService(serviceAnalysisEnv());
  serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  const readModel = await waitForAnalyzedScreenReadModel();
  const analyzedQueueJobId = localVisionRow(readModel)?.queueJobId;
  await waitForAnalyzedQueueRemoval(analyzedQueueJobId);
  const queueRecords = readQueueRecordsAllowEmpty();
  assertProof(readModel, queueRecords);

  const sanitizedReadModel = sanitizeReadModel(readModel);
  const sanitizedQueueRecords = sanitizeQueueRecords(queueRecords);
  const analysisRow = localVisionRow(sanitizedReadModel);
  const summary = {
    proof: 'screen-ai-service-analysis-proof',
    proofTier: 'P3_LOCAL_DEV_MACHINE',
    platform: process.platform,
    agentPort,
    analysisRow,
    queueRecordsAfterAnalysis: queueRecords.length,
    artifacts: {
      proofSummary: relative(repoRoot, join(outputDir, 'proof-summary.json')),
      screenReadModel: relative(repoRoot, join(outputDir, 'screen-read-model.json')),
      queueRecordMetadataAfterAnalysis: relative(repoRoot, join(outputDir, 'queue-records-after-analysis.json')),
    },
    ephemeralPathsDeletedAfterProof: true,
    assertions: {
      realWindowsServiceCaptureRequired: true,
      localAdapterCommandExecutedThroughServiceRuntime: analysisRow.providerKind === 'localVision',
      captureReasonPreserved: analysisRow.captureReason === 'timedCadence',
      activeWindowScopePreserved: analysisRow.captureScope === 'activeWindow',
      adapterOutputAcceptedAbovePolicyThreshold: analysisRow.confidence >= 0.88 && analysisRow.policyEligible === true,
      encryptedQueueDrainedAfterAnalysis: queueRecords.length === 0,
      rawFixtureTextNotRetainedInQueue:
        !readOptional(queuePath).includes('Ocentra Service Analysis Proof') &&
        !readOptional(queuePath).includes('Expected visible category'),
      activityReadModelReachedViaWebSocket: readModel.state === 'ready',
    },
    nonClaims: [
      'This proves the service-owned capture-to-analysis handoff, local adapter process boundary, Activity Screen read-model surfacing, and queue deletion after analysis.',
      'The adapter command is a local proof adapter for runtime plumbing; it is not a claim of production VLM quality.',
      'Browser URL-change triggers remain browser-plan scope; this proof uses service timed cadence active-window capture.',
    ],
  };
  writeJson(join(outputDir, 'proof-summary.json'), summary);
  writeJson(join(outputDir, 'screen-read-model.json'), sanitizedReadModel);
  writeJson(join(outputDir, 'queue-records-after-analysis.json'), sanitizedQueueRecords);
  console.log(`screen-ai-service-analysis-proof-ok:${analysisRow.providerKind}:${queueRecords.length}`);
} catch (error) {
  writeFailureArtifacts(error);
  throw error;
} finally {
  await Promise.allSettled([
    browser === undefined ? Promise.resolve() : browser.close(),
    service === undefined ? Promise.resolve() : stopProcessTreeAndWait(service),
  ]);
  await removeDirectoryWithRetry(activityRoot);
  await removeDirectoryWithRetry(buildRoot);
}

function writeFixture() {
  writeFileSync(
    fixturePath,
    `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Ocentra Service Analysis Proof</title>
  <style>
    body {
      margin: 0;
      font-family: Arial, sans-serif;
      background: #f5fbff;
      color: #07172f;
    }
    main {
      border: 8px solid #087d77;
      margin: 42px;
      padding: 42px;
      min-height: 420px;
      background: white;
    }
    h1 {
      font-size: 44px;
      margin: 0 0 24px;
    }
    p {
      font-size: 28px;
      margin: 16px 0;
    }
  </style>
</head>
<body>
  <main>
    <h1>Ocentra Service Analysis Proof</h1>
    <p>Expected visible category: school</p>
    <p>Algebra worksheet and teacher notes are visible for real Windows capture.</p>
  </main>
</body>
</html>
`
  );
}

function writeAdapterCommand() {
  writeFileSync(
    adapterScriptPath,
    [
      '$inputPayload = [Console]::In.ReadToEnd()',
      '$null = $inputPayload | ConvertFrom-Json',
      '$output = @{',
      "  summary = 'Local adapter classified a school worksheet from the queued capture.'",
      "  primaryCategory = 'school'",
      '  confidence = 0.91',
      '  policyEligible = $true',
      '} | ConvertTo-Json -Compress',
      '[Console]::Out.WriteLine($output)',
      '',
    ].join('\r\n')
  );
  writeFileSync(
    adapterCommandPath,
    ['@echo off', 'powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0screen-ai-adapter.ps1"', ''].join('\r\n')
  );
  chmodSync(adapterCommandPath, 0o755);
}

function startService(env) {
  return spawn(proofAgentServicePath(), [], {
    cwd: repoRoot,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
}

function serviceCaptureEnv() {
  return {
    ...baseServiceEnv(),
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_CAPTURES: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_TICKS: '4',
  };
}

function serviceAnalysisEnv() {
  return {
    ...baseServiceEnv(),
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_RETENTION_SWEEPER_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_POLL_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_MAX_JOBS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_MAX_TICKS: '30',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ADAPTER_TIMEOUT_MS: '10000',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ADAPTER_COMMAND: adapterCommandPath,
  };
}

function baseServiceEnv() {
  return {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(agentPort),
    [ParentDevEnv.ActivityDbPath]: storePath,
    OCENTRA_PARENT_ACTIVITY_JOURNAL_PATH: journalPath,
    OCENTRA_PARENT_ACTIVITY_JOURNAL_KEY_PATH: keyPath,
    OCENTRA_PARENT_SCREEN_SERVICE_QUEUE_MAX_PENDING: '2',
    OCENTRA_PARENT_SCREEN_SERVICE_QUEUE_DIR: queueDir,
  };
}

async function waitForQueueRecords(count) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30000) {
    if (existsSync(queuePath)) {
      try {
        const records = readQueueRecordsAllowEmpty();
        if (records.length >= count) {
          return;
        }
      } catch {
        await delay(250);
        continue;
      }
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for ${count} screen queue records.\n${readOptional(queuePath)}`);
}

async function waitForAnalyzedScreenReadModel() {
  const startedAt = Date.now();
  let lastReadModel;
  while (Date.now() - startedAt < 45000) {
    lastReadModel = await requestScreenReadModel();
    lastObservedReadModel = lastReadModel;
    if (
      lastReadModel.state === 'ready' &&
      Array.isArray(lastReadModel.rows) &&
      lastReadModel.rows.some((row) => row.providerKind === 'localVision')
    ) {
      return lastReadModel;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for localVision analysis row: ${JSON.stringify(lastReadModel)}`);
}

async function waitForAnalyzedQueueRemoval(queueJobId) {
  if (typeof queueJobId !== 'string' || queueJobId.length === 0) {
    throw new Error('Cannot wait for queue removal without analyzed queue job id.');
  }
  const startedAt = Date.now();
  while (Date.now() - startedAt < 10000) {
    const records = readQueueRecordsAllowEmpty();
    if (!records.some((record) => record.queueJobId === queueJobId)) {
      return;
    }
    await delay(100);
  }
  throw new Error(`Timed out waiting for analyzed queue job removal: ${queueJobId}`);
}

async function requestScreenReadModel() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error('Screen analysis read-model WebSocket proof timed out.'));
    }, 10000);

    socket.addEventListener('open', () => {
      socket.send(JSON.stringify(commandEnvelope()));
    });

    socket.addEventListener('message', (message) => {
      try {
        const parsed = AgentEventEnvelopeSchema.parse(JSON.parse(String(message.data)));
        if (parsed.event === AgentEvent.ConnectionReady) {
          return;
        }
        if (parsed.event !== AgentEvent.ActivityScreenReadModelReported) {
          throw new Error(`Expected screen read model, received ${parsed.event}`);
        }
        const readModelJson = parsed.payload[AgentProtocolDefaults.Field.ActivityReadModel];
        if (typeof readModelJson !== 'string') {
          throw new Error(`Screen read model payload was missing JSON: ${JSON.stringify(parsed.payload)}`);
        }
        clearTimeout(timer);
        socket.close();
        resolve(JSON.parse(readModelJson));
      } catch (error) {
        clearTimeout(timer);
        socket.close();
        reject(error);
      }
    });

    socket.addEventListener('error', () => {
      clearTimeout(timer);
      reject(new Error('Screen analysis read-model WebSocket failed.'));
    });
  });
}

function commandEnvelope() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-screen-ai-service-analysis-read-model',
    sentAt: new Date().toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command: AgentCommand.ActivityScreenReadModelGet,
    payload: {
      [AgentProtocolDefaults.Field.ScopeKind]: 'family',
      [AgentProtocolDefaults.Field.FamilyId]: 'family-local',
      [AgentProtocolDefaults.Field.RangeStart]: '1970-01-01T00:00:00Z',
      [AgentProtocolDefaults.Field.RangeEnd]: new Date().toISOString(),
    },
  };
}

function assertProof(readModel, queueRecords) {
  if (readModel.state !== 'ready' || !Array.isArray(readModel.rows) || readModel.rows.length === 0) {
    throw new Error(`Screen read model did not expose analysis rows: ${JSON.stringify(readModel)}`);
  }
  const analysisRow = localVisionRow(readModel);
  if (!analysisRow) {
    throw new Error(`Read model did not include local adapter analysis: ${JSON.stringify(readModel)}`);
  }
  if (
    analysisRow.queueJobId === undefined ||
    analysisRow.captureReason === undefined ||
    analysisRow.imageDigest === undefined
  ) {
    throw new Error(`Adapter analysis did not surface capture metadata: ${JSON.stringify(analysisRow)}`);
  }
  if (analysisRow.primaryCategory !== 'school') {
    throw new Error(`Adapter category was not surfaced: ${JSON.stringify(analysisRow)}`);
  }
  if (analysisRow.policyEligible !== true) {
    throw new Error(`Adapter analysis was not policy eligible after threshold: ${JSON.stringify(analysisRow)}`);
  }
  if (queueRecords.length !== 0) {
    throw new Error(`Queue was not drained after analysis: ${JSON.stringify(queueRecords)}`);
  }
}

function localVisionRow(readModel) {
  return readModel.rows.find((row) => row.providerKind === 'localVision');
}

function readQueueRecordsAllowEmpty() {
  const raw = readOptional(queuePath).trim();
  if (raw.length === 0) {
    return [];
  }
  return raw
    .split('\n')
    .filter((line) => line.length > 0)
    .map((line) => JSON.parse(line));
}

function sanitizeReadModel(readModel) {
  return {
    ...readModel,
    rows: readModel.rows.map((row) => ({
      ...row,
      evidence: row.evidence.map((evidence) => ({
        ...evidence,
        uri: '<ephemeral-screen-queue>',
      })),
    })),
  };
}

function sanitizeQueueRecords(queueRecords) {
  return queueRecords.map((record) => ({
    schemaVersion: record.schemaVersion,
    queueJobId: record.queueJobId,
    custodyState: record.custodyState,
    imageDigest: record.imageDigest,
    nonceLength: typeof record.nonce === 'string' ? record.nonce.length : 0,
    ciphertextLength: typeof record.ciphertext === 'string' ? record.ciphertext.length : 0,
  }));
}

async function waitForHttp(url, output) {
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
  throw new Error(`Timed out waiting for ${url}\n${output()}`);
}

function proofAgentServicePath() {
  const binaryName = process.platform === 'win32' ? 'ocentra-parent-agent-service.exe' : 'ocentra-parent-agent-service';
  return join(buildRoot, 'debug', binaryName);
}

function runCommand(command, args, env = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      env: {
        ...process.env,
        ...env,
      },
      stdio: 'inherit',
      shell: process.platform === 'win32',
      windowsHide: true,
    });
    child.once('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} exited with ${code}`));
    });
    child.once('error', reject);
  });
}

function collectOutput(child) {
  const chunks = [];
  child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
  return () => chunks.join('');
}

function readOptional(path) {
  return existsSync(path) ? readFileSync(path, 'utf8') : '';
}

function writeFailureArtifacts(error) {
  let queueRecords = [];
  try {
    queueRecords = sanitizeQueueRecords(readQueueRecordsAllowEmpty());
  } catch {
    queueRecords = [];
  }
  writeJson(join(outputDir, 'failure-summary.json'), {
    proof: 'screen-ai-service-analysis-proof',
    error: error instanceof Error ? error.message : String(error),
    queueRecordCount: queueRecords.length,
    queueRecords,
    lastObservedReadModel,
    journalBytes: readOptional(journalPath).length,
    keyBytes: readOptional(keyPath).length,
    storePresent: existsSync(storePath),
    serviceOutputTail: serviceOutput().slice(-6000),
  });
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}
