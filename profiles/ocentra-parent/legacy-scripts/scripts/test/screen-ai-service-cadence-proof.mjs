import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
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
const outputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'service-cadence');
const agentPort = resolveParentDevPort(
  process.env.OCENTRA_SCREEN_AI_SERVICE_PROOF_AGENT_PORT,
  4687,
  'OCENTRA_SCREEN_AI_SERVICE_PROOF_AGENT_PORT'
);
const buildRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-ai-service-target-'));
const activityRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-ai-service-cadence-'));
const queueDir = join(activityRoot, 'screen-queue');
const journalPath = join(activityRoot, 'activity.ndjson');
const keyPath = join(activityRoot, 'activity-journal.key');
const storePath = join(activityRoot, 'activity.sqlite');
const fixturePath = join(activityRoot, 'screen-ai-service-cadence-fixture.html');
const queuePath = join(queueDir, 'screen-evidence-queue.ndjson');
const healthUrl = createAgentHealthUrl(agentPort);
const wsUrl = createAgentWebSocketUrl(agentPort);

if (process.platform !== 'win32') {
  throw new Error('screen-ai-service-cadence-proof requires a real Windows desktop capture surface.');
}

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });
writeFixture();

await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);

let browser;
let service;

try {
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service'], {
    CARGO_TARGET_DIR: buildRoot,
  });
  browser = await chromium.launch({ headless: false });
  const page = await browser.newPage({ viewport: { width: 960, height: 620 } });
  await page.goto(pathToFileURL(fixturePath).href);
  await page.bringToFront();
  await page.waitForTimeout(750);

  service = spawn(proofAgentServicePath(), [], {
    cwd: repoRoot,
    env: serviceEnv(),
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
  const serviceOutput = collectOutput(service);

  await waitForHttp(healthUrl, serviceOutput);
  await waitForQueueRecords(3);
  const readModel = await waitForScreenReadModelRows(3);
  await delay(3500);
  const queueRecords = readQueueRecords();
  const backpressureReadModel = await requestScreenReadModel();
  assertProof(readModel, queueRecords);
  const sanitizedReadModel = sanitizeReadModel(readModel);
  const sanitizedQueueRecords = sanitizeQueueRecords(queueRecords);

  const summary = {
    proof: 'screen-ai-service-cadence-proof',
    proofTier: 'P3_LOCAL_DEV_MACHINE',
    platform: process.platform,
    agentPort,
    capturedQueueRecords: queueRecords.length,
    screenRows: readModel.rows.length,
    backpressureScreenRowsAfterSettling: backpressureReadModel.rows.length,
    firstRow: sanitizedReadModel.rows[0],
    artifacts: {
      proofSummary: relative(repoRoot, join(outputDir, 'proof-summary.json')),
      screenReadModel: relative(repoRoot, join(outputDir, 'screen-read-model.json')),
      queueRecordMetadata: relative(repoRoot, join(outputDir, 'queue-records.json')),
    },
    ephemeralPathsDeletedAfterProof: true,
    assertions: {
      realWindowsServiceCaptureRequired: true,
      threeTimedCadenceFramesCaptured: queueRecords.length === 3 && readModel.rows.length >= 3,
      queueBackpressureHeldAtThreePendingFrames:
        queueRecords.length === 3 && backpressureReadModel.rows.length === readModel.rows.length,
      encryptedQueueDoesNotContainVisibleFixtureText: !readFileSync(queuePath, 'utf8').includes(
        'Ocentra Service Cadence Proof'
      ),
      activityReadModelReachedViaWebSocket: readModel.state === 'ready',
      deterministicMetadataProviderDoesNotClaimVlm: readModel.rows.every(
        (row) => row.providerKind === 'serviceCaptureMetadata'
      ),
      policyEligibleFalseUntilRealVisionModel: readModel.rows.every((row) => row.policyEligible === false),
    },
    nonClaims: [
      'This proves service-owned timed cadence capture, encrypted queue write, and Activity Screen read-model surfacing.',
      'This does not claim broad browser URL trigger ownership or VLM classification quality.',
      'The provider is serviceCaptureMetadata, not localVision, until a real configured VLM worker analyzes the frames.',
    ],
  };
  writeJson(join(outputDir, 'proof-summary.json'), summary);
  writeJson(join(outputDir, 'screen-read-model.json'), sanitizedReadModel);
  writeJson(join(outputDir, 'queue-records.json'), sanitizedQueueRecords);
  console.log(`screen-ai-service-cadence-proof-ok:${queueRecords.length}:${readModel.rows.length}`);
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
  <title>Ocentra Service Cadence Proof</title>
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
    <h1>Ocentra Service Cadence Proof</h1>
    <p>Timed cadence frame visible for real Windows active-window capture.</p>
    <p>Homework notes and productivity text are visible on screen.</p>
  </main>
</body>
</html>
`
  );
}

function serviceEnv() {
  return {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(agentPort),
    [ParentDevEnv.ActivityDbPath]: storePath,
    OCENTRA_PARENT_ACTIVITY_JOURNAL_PATH: journalPath,
    OCENTRA_PARENT_ACTIVITY_JOURNAL_KEY_PATH: keyPath,
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_CAPTURES: '4',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_TICKS: '6',
    OCENTRA_PARENT_SCREEN_SERVICE_QUEUE_MAX_PENDING: '3',
    OCENTRA_PARENT_SCREEN_SERVICE_QUEUE_DIR: queueDir,
  };
}

async function waitForQueueRecords(count) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30000) {
    const records = existsSync(queuePath) ? tryReadQueueRecords() : null;
    if (records !== undefined && records.length >= count) {
      return;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for ${count} screen queue records.\n${readOptional(queuePath)}`);
}

function readQueueRecords() {
  const records = tryReadQueueRecords();
  if (records === null) {
    throw new Error(`Screen queue contained incomplete JSON records:\n${readOptional(queuePath)}`);
  }
  return records;
}

function tryReadQueueRecords() {
  return readFileSync(queuePath, 'utf8')
    .trim()
    .split('\n')
    .filter((line) => line.length > 0)
    .map((line) => {
      try {
        return JSON.parse(line);
      } catch {
        return null;
      }
    })
    .reduce((records, record) => {
      if (records === null || record === null) {
        return null;
      }
      records.push(record);
      return records;
    }, []);
}

async function requestScreenReadModel() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error('Screen read-model WebSocket proof timed out.'));
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
      reject(new Error('Screen read-model WebSocket failed.'));
    });
  });
}

async function waitForScreenReadModelRows(count) {
  const startedAt = Date.now();
  let lastReadModel;
  while (Date.now() - startedAt < 10000) {
    lastReadModel = await requestScreenReadModel();
    if (lastReadModel.state === 'ready' && Array.isArray(lastReadModel.rows) && lastReadModel.rows.length >= count) {
      return lastReadModel;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for ${count} screen read-model rows: ${JSON.stringify(lastReadModel)}`);
}

function commandEnvelope() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-screen-ai-service-cadence-read-model',
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
  if (readModel.state !== 'ready' || readModel.rows.length < 3) {
    throw new Error(`Screen read model did not expose repeated cadence rows: ${JSON.stringify(readModel)}`);
  }
  if (queueRecords.length !== 3) {
    throw new Error(`Queue did not contain repeated cadence captures: ${JSON.stringify(queueRecords)}`);
  }
  for (const row of readModel.rows.slice(0, 3)) {
    if (row.captureReason !== 'timedCadence') {
      throw new Error(`Screen row did not preserve timed cadence reason: ${JSON.stringify(row)}`);
    }
    if (row.captureScope !== 'activeWindow') {
      throw new Error(`Screen row did not preserve active-window scope: ${JSON.stringify(row)}`);
    }
    if (row.providerKind !== 'serviceCaptureMetadata') {
      throw new Error(`Screen row claimed the wrong provider kind: ${JSON.stringify(row)}`);
    }
    if (row.policyEligible !== false) {
      throw new Error(`Service metadata row must not be policy eligible without VLM analysis: ${JSON.stringify(row)}`);
    }
    if (row.imageDeletionState !== 'deleted') {
      throw new Error(`Screen row did not mark raw image deleted after queue handoff: ${JSON.stringify(row)}`);
    }
  }
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

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}
