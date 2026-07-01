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
const outputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'service-retention-sweeper');
const agentPort = resolveParentDevPort(
  process.env.OCENTRA_SCREEN_AI_RETENTION_SWEEPER_AGENT_PORT,
  4695,
  'OCENTRA_SCREEN_AI_RETENTION_SWEEPER_AGENT_PORT'
);
const buildRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-ai-retention-target-'));
const activityRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-ai-retention-'));
const queueDir = join(activityRoot, 'screen-queue');
const journalPath = join(activityRoot, 'activity.ndjson');
const keyPath = join(activityRoot, 'activity-journal.key');
const storePath = join(activityRoot, 'activity.sqlite');
const fixturePath = join(activityRoot, 'screen-ai-service-retention-fixture.html');
const queuePath = join(queueDir, 'screen-evidence-queue.ndjson');
const healthUrl = createAgentHealthUrl(agentPort);
const wsUrl = createAgentWebSocketUrl(agentPort);

if (process.platform !== 'win32') {
  throw new Error('screen-ai-service-retention-sweeper-proof requires a real Windows desktop capture surface.');
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

  service = startService(captureEnv());
  let serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  await waitForQueueRecordCount(1);
  const baselineReadModel = await waitForScreenReadModelRows(1);
  const baselineQueueRecords = readQueueRecordsAllowEmpty();
  assertBaselineCapture(baselineReadModel, baselineQueueRecords);
  await stopProcessTreeAndWait(service);
  service = undefined;

  await delayUntilExpired(baselineQueueRecords[0].expiresAt);
  await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
  service = startService(retentionSweeperEnv());
  serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  const sweptReadModel = await waitForExpiredDeletedReadModel(baselineQueueRecords[0].queueJobId);
  await waitForQueueRecordCount(0);
  const sweptQueueRecords = readQueueRecordsAllowEmpty();
  assertSweeperProof({
    baselineQueueRecords,
    baselineReadModel,
    sweptQueueRecords,
    sweptReadModel,
  });

  const sanitizedBaselineReadModel = sanitizeReadModel(baselineReadModel);
  const sanitizedSweptReadModel = sanitizeReadModel(sweptReadModel);
  const sanitizedBaselineQueueRecords = sanitizeQueueRecords(baselineQueueRecords);
  const sanitizedSweptQueueRecords = sanitizeQueueRecords(sweptQueueRecords);
  const expiredRow = expiredDeletedRow(sanitizedSweptReadModel, baselineQueueRecords[0].queueJobId);
  const summary = {
    proof: 'screen-ai-service-retention-sweeper-proof',
    proofTier: 'P3_LOCAL_DEV_MACHINE',
    platform: process.platform,
    agentPort,
    baselineQueueRecords: baselineQueueRecords.length,
    sweptQueueRecords: sweptQueueRecords.length,
    baselineScreenRows: baselineReadModel.rows.length,
    sweptScreenRows: sweptReadModel.rows.length,
    expiredDeletedRow: expiredRow,
    artifacts: {
      proofSummary: relative(repoRoot, join(outputDir, 'proof-summary.json')),
      baselineReadModel: relative(repoRoot, join(outputDir, 'baseline-screen-read-model.json')),
      sweptReadModel: relative(repoRoot, join(outputDir, 'swept-screen-read-model.json')),
      baselineQueueRecords: relative(repoRoot, join(outputDir, 'baseline-queue-records.json')),
      sweptQueueRecords: relative(repoRoot, join(outputDir, 'swept-queue-records.json')),
    },
    ephemeralPathsDeletedAfterProof: true,
    assertions: {
      realWindowsServiceCaptureRequired: true,
      capturePhaseCreatedEncryptedExpiringQueueRecord: baselineQueueRecords.length === 1,
      retentionSweeperRemovedExpiredQueueRecord: sweptQueueRecords.length === 0,
      expiredDeletionSurfacedInActivityReadModel: expiredRow?.imageDeletionState === 'expiredDeleted',
      expiredDeletionPreservedOriginalQueueJobId: expiredRow?.queueJobId === baselineQueueRecords[0].queueJobId,
      retentionSweeperDidNotRunLocalVision:
        sweptReadModel.rows.every((row) => row.providerKind !== 'localVision') &&
        sweptReadModel.rows.every((row) => row.providerKind !== 'localVisionUnavailable'),
      encryptedQueueDoesNotContainVisibleFixtureText:
        !readOptional(queuePath).includes('Ocentra Service Retention Sweeper Proof') &&
        !readOptional(queuePath).includes('Temporary screen evidence must expire and be deleted.'),
      activityReadModelReachedViaWebSocket: sweptReadModel.state === 'ready',
    },
    nonClaims: [
      'This proves the service-owned retention sweeper deletes expired encrypted screen queue entries and records an expiredDeleted Activity Screen row.',
      'Capture and analysis runtimes are disabled during the sweeper phase so this does not hide retention behavior behind new capture or VLM processing.',
      'This does not claim parent UI controls for retention duration or cloud retention policy.',
    ],
  };
  writeJson(join(outputDir, 'proof-summary.json'), summary);
  writeJson(join(outputDir, 'baseline-screen-read-model.json'), sanitizedBaselineReadModel);
  writeJson(join(outputDir, 'swept-screen-read-model.json'), sanitizedSweptReadModel);
  writeJson(join(outputDir, 'baseline-queue-records.json'), sanitizedBaselineQueueRecords);
  writeJson(join(outputDir, 'swept-queue-records.json'), sanitizedSweptQueueRecords);
  console.log(
    `screen-ai-service-retention-sweeper-proof-ok:${baselineQueueRecords.length}:${sweptQueueRecords.length}:${
      expiredRow === undefined ? 0 : 1
    }`
  );
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
  <title>Ocentra Service Retention Sweeper Proof</title>
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
    <h1>Ocentra Service Retention Sweeper Proof</h1>
    <p>Temporary screen evidence must expire and be deleted.</p>
    <p>The retention sweeper should remove the encrypted queue record.</p>
  </main>
</body>
</html>
`
  );
}

function startService(env) {
  return spawn(proofAgentServicePath(), [], {
    cwd: repoRoot,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
}

function captureEnv() {
  return {
    ...baseServiceEnv(),
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_RETENTION_SWEEPER_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_CAPTURES: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_TICKS: '4',
    OCENTRA_PARENT_SCREEN_SERVICE_TEMPORARY_IMAGE_TTL_SECONDS: '1',
  };
}

function retentionSweeperEnv() {
  return {
    ...baseServiceEnv(),
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_RETENTION_SWEEPER_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_RETENTION_SWEEPER_POLL_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_RETENTION_SWEEPER_MAX_SWEEPS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_RETENTION_SWEEPER_MAX_TICKS: '6',
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

async function delayUntilExpired(expiresAt) {
  const expiresAtMs = Date.parse(expiresAt);
  if (!Number.isFinite(expiresAtMs)) {
    throw new Error(`Queue record did not contain an RFC3339 expiry: ${expiresAt}`);
  }
  const waitMs = Math.max(0, expiresAtMs - Date.now() + 1000);
  await delay(waitMs);
}

async function waitForQueueRecordCount(count) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 30000) {
    const records = existsSync(queuePath) ? tryReadQueueRecords() : count === 0 ? [] : null;
    if (records !== undefined && records.length === count) {
      return;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for ${count} screen queue records.\n${readOptional(queuePath)}`);
}

async function waitForScreenReadModelRows(rowCount) {
  const startedAt = Date.now();
  let lastReadModel;
  while (Date.now() - startedAt < 30000) {
    lastReadModel = await requestScreenReadModel();
    if (lastReadModel.state === 'ready' && Array.isArray(lastReadModel.rows) && lastReadModel.rows.length >= rowCount) {
      return lastReadModel;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for ${rowCount} screen rows: ${JSON.stringify(lastReadModel)}`);
}

async function waitForExpiredDeletedReadModel(queueJobId) {
  const startedAt = Date.now();
  let lastReadModel;
  while (Date.now() - startedAt < 30000) {
    lastReadModel = await requestScreenReadModel();
    if (lastReadModel.state === 'ready' && expiredDeletedRow(lastReadModel, queueJobId) !== undefined) {
      return lastReadModel;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for expiredDeleted screen row: ${JSON.stringify(lastReadModel)}`);
}

async function requestScreenReadModel() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error('Retention sweeper screen read-model WebSocket proof timed out.'));
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
      reject(new Error('Retention sweeper screen read-model WebSocket failed.'));
    });
  });
}

function commandEnvelope() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-screen-ai-service-retention-sweeper-read-model',
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

function assertBaselineCapture(readModel, queueRecords) {
  if (readModel.state !== 'ready' || readModel.rows.length !== 1) {
    throw new Error(`Capture phase did not expose one service-capture row: ${JSON.stringify(readModel)}`);
  }
  if (queueRecords.length !== 1) {
    throw new Error(`Capture phase did not leave one encrypted queue record: ${JSON.stringify(queueRecords)}`);
  }
  const row = readModel.rows[0];
  const record = queueRecords[0];
  if (row.providerKind !== 'serviceCaptureMetadata' || row.policyEligible !== false) {
    throw new Error(`Capture row was not service metadata only: ${JSON.stringify(row)}`);
  }
  if (row.queueJobId !== record.queueJobId) {
    throw new Error(`Read model and queue job IDs diverged: ${row.queueJobId} !== ${record.queueJobId}`);
  }
  if (typeof record.createdAt !== 'string' || typeof record.expiresAt !== 'string') {
    throw new Error(`Queue record did not expose created/expires metadata: ${JSON.stringify(record)}`);
  }
  if (Date.parse(record.expiresAt) <= Date.parse(record.createdAt)) {
    throw new Error(`Queue expiry was not later than creation: ${JSON.stringify(record)}`);
  }
  if (readOptional(queuePath).includes('Ocentra Service Retention Sweeper Proof')) {
    throw new Error('Encrypted screen queue retained visible fixture text.');
  }
}

function assertSweeperProof({ baselineQueueRecords, baselineReadModel, sweptQueueRecords, sweptReadModel }) {
  if (sweptReadModel.state !== 'ready') {
    throw new Error(`Sweeper phase read model was not ready: ${JSON.stringify(sweptReadModel)}`);
  }
  if (sweptQueueRecords.length !== 0) {
    throw new Error(`Sweeper phase did not empty expired queue: ${JSON.stringify(sweptQueueRecords)}`);
  }
  if (sweptReadModel.rows.length <= baselineReadModel.rows.length) {
    throw new Error(`Sweeper phase did not add an Activity Screen row: ${JSON.stringify(sweptReadModel)}`);
  }
  const row = expiredDeletedRow(sweptReadModel, baselineQueueRecords[0].queueJobId);
  if (row === undefined) {
    throw new Error(`Sweeper phase did not surface expiredDeleted row: ${JSON.stringify(sweptReadModel)}`);
  }
  if (row.providerKind !== 'serviceCaptureMetadata' || row.policyEligible !== false) {
    throw new Error(`Sweeper row claimed the wrong provider or eligibility: ${JSON.stringify(row)}`);
  }
}

function expiredDeletedRow(readModel, queueJobId) {
  return readModel.rows.find((row) => row.queueJobId === queueJobId && row.imageDeletionState === 'expiredDeleted');
}

function readQueueRecordsAllowEmpty() {
  if (!existsSync(queuePath)) {
    return [];
  }
  const contents = readFileSync(queuePath, 'utf8').trim();
  if (contents.length === 0) {
    return [];
  }
  return contents.split('\n').map((line) => JSON.parse(line));
}

function tryReadQueueRecords() {
  const raw = readOptional(queuePath).trim();
  if (raw.length === 0) {
    return [];
  }
  return raw
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
    createdAt: record.createdAt,
    expiresAt: record.expiresAt,
    status: record.status,
    deletionRequired: record.deletionRequired,
    deletionStatus: record.deletionStatus,
    deletionProofRef: record.deletionProofRef,
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
