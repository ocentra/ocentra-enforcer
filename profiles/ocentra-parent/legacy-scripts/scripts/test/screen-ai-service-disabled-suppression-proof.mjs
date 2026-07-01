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
const outputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'service-disabled-suppression');
const agentPort = resolveParentDevPort(
  process.env.OCENTRA_SCREEN_AI_DISABLED_SUPPRESSION_AGENT_PORT,
  4691,
  'OCENTRA_SCREEN_AI_DISABLED_SUPPRESSION_AGENT_PORT'
);
const buildRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-ai-disabled-target-'));
const activityRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-ai-disabled-'));
const queueDir = join(activityRoot, 'screen-queue');
const journalPath = join(activityRoot, 'activity.ndjson');
const keyPath = join(activityRoot, 'activity-journal.key');
const storePath = join(activityRoot, 'activity.sqlite');
const fixturePath = join(activityRoot, 'screen-ai-service-disabled-fixture.html');
const queuePath = join(queueDir, 'screen-evidence-queue.ndjson');
const healthUrl = createAgentHealthUrl(agentPort);
const wsUrl = createAgentWebSocketUrl(agentPort);

if (process.platform !== 'win32') {
  throw new Error('screen-ai-service-disabled-suppression-proof requires a real Windows desktop capture surface.');
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

  service = startService(enabledCaptureEnv());
  let serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  await waitForQueueRecordCount(1);
  const baselineReadModel = await waitForScreenReadModelRows(1);
  const baselineQueueRecords = readQueueRecordsAllowEmpty();
  assertBaselineCapture(baselineReadModel, baselineQueueRecords);
  await stopProcessTreeAndWait(service);
  service = undefined;

  await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
  service = startService(disabledSuppressionEnv());
  serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  await delay(4500);
  const disabledReadModel = await requestScreenReadModel();
  const disabledQueueRecords = readQueueRecordsAllowEmpty();
  assertDisabledSuppression({
    baselineReadModel,
    baselineQueueRecords,
    disabledReadModel,
    disabledQueueRecords,
  });

  const sanitizedBaselineReadModel = sanitizeReadModel(baselineReadModel);
  const sanitizedDisabledReadModel = sanitizeReadModel(disabledReadModel);
  const sanitizedBaselineQueueRecords = sanitizeQueueRecords(baselineQueueRecords);
  const sanitizedDisabledQueueRecords = sanitizeQueueRecords(disabledQueueRecords);
  const summary = {
    proof: 'screen-ai-service-disabled-suppression-proof',
    proofTier: 'P3_LOCAL_DEV_MACHINE',
    platform: process.platform,
    agentPort,
    baselineQueueRecords: baselineQueueRecords.length,
    disabledQueueRecords: disabledQueueRecords.length,
    baselineScreenRows: baselineReadModel.rows.length,
    disabledScreenRows: disabledReadModel.rows.length,
    firstRow: sanitizedDisabledReadModel.rows[0],
    artifacts: {
      proofSummary: relative(repoRoot, join(outputDir, 'proof-summary.json')),
      baselineReadModel: relative(repoRoot, join(outputDir, 'baseline-screen-read-model.json')),
      disabledReadModel: relative(repoRoot, join(outputDir, 'disabled-screen-read-model.json')),
      baselineQueueRecords: relative(repoRoot, join(outputDir, 'baseline-queue-records.json')),
      disabledQueueRecords: relative(repoRoot, join(outputDir, 'disabled-queue-records.json')),
    },
    ephemeralPathsDeletedAfterProof: true,
    assertions: {
      realWindowsServiceCaptureRequired: true,
      enabledPhaseCreatedEncryptedQueueRecord: baselineQueueRecords.length === 1,
      disabledPhaseCreatedNoNewCaptureRows: disabledReadModel.rows.length === baselineReadModel.rows.length,
      disabledPhaseCreatedNoNewQueueRecords: disabledQueueRecords.length === baselineQueueRecords.length,
      disabledPhaseDidNotDrainPendingQueue: disabledQueueRecords[0]?.queueJobId === baselineQueueRecords[0]?.queueJobId,
      disabledPhaseCreatedNoLocalVisionRows: disabledReadModel.rows.every(
        (row) => row.providerKind !== 'localVision' && row.providerKind !== 'localVisionUnavailable'
      ),
      encryptedQueueDoesNotContainVisibleFixtureText:
        !readOptional(queuePath).includes('Ocentra Service Disabled Suppression Proof') &&
        !readOptional(queuePath).includes('Disabled parent setting must stop service capture and AI analysis'),
      activityReadModelReachedViaWebSocket: disabledReadModel.state === 'ready',
    },
    nonClaims: [
      'This proves the service-owned parent-disabled setting suppresses cadence capture, foreground capture, and queued analysis processing.',
      'The enabled phase intentionally leaves one encrypted queue record pending so the disabled phase can prove AI analysis does not consume it.',
      'This does not claim product UI controls for the setting, browser URL trigger ownership, or VLM quality.',
    ],
  };
  writeJson(join(outputDir, 'proof-summary.json'), summary);
  writeJson(join(outputDir, 'baseline-screen-read-model.json'), sanitizedBaselineReadModel);
  writeJson(join(outputDir, 'disabled-screen-read-model.json'), sanitizedDisabledReadModel);
  writeJson(join(outputDir, 'baseline-queue-records.json'), sanitizedBaselineQueueRecords);
  writeJson(join(outputDir, 'disabled-queue-records.json'), sanitizedDisabledQueueRecords);
  console.log(
    `screen-ai-service-disabled-suppression-proof-ok:${baselineQueueRecords.length}:${disabledQueueRecords.length}`
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
  <title>Ocentra Service Disabled Suppression Proof</title>
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
    <h1>Ocentra Service Disabled Suppression Proof</h1>
    <p>Disabled parent setting must stop service capture and AI analysis.</p>
    <p>This text must only appear inside encrypted screen evidence custody.</p>
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

function enabledCaptureEnv() {
  return {
    ...baseServiceEnv(),
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_CAPTURES: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_TICKS: '3',
  };
}

function disabledSuppressionEnv() {
  return {
    ...baseServiceEnv(),
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_CAPTURES: '2',
    OCENTRA_PARENT_SCREEN_SERVICE_CADENCE_MAX_TICKS: '4',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_POLL_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_MAX_JOBS: '2',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_MAX_TICKS: '4',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_POLL_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_MIN_GAP_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_MAX_CAPTURES: '2',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_MAX_TICKS: '4',
  };
}

function baseServiceEnv() {
  return {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(agentPort),
    [ParentDevEnv.ActivityDbPath]: storePath,
    OCENTRA_PARENT_ACTIVITY_JOURNAL_PATH: journalPath,
    OCENTRA_PARENT_ACTIVITY_JOURNAL_KEY_PATH: keyPath,
    OCENTRA_PARENT_SCREEN_SERVICE_QUEUE_MAX_PENDING: '3',
    OCENTRA_PARENT_SCREEN_SERVICE_QUEUE_DIR: queueDir,
  };
}

async function waitForQueueRecordCount(count) {
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

async function requestScreenReadModel() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error('Disabled suppression screen read-model WebSocket proof timed out.'));
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

    socket.addEventListener('error', (error) => {
      clearTimeout(timer);
      socket.close();
      reject(error);
    });
  });
}

function commandEnvelope() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-screen-ai-service-disabled-suppression-read-model',
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
    throw new Error(`Enabled phase did not expose one service-capture row: ${JSON.stringify(readModel)}`);
  }
  if (queueRecords.length !== 1) {
    throw new Error(`Enabled phase did not leave one encrypted queue record: ${JSON.stringify(queueRecords)}`);
  }
  const row = readModel.rows[0];
  if (row.providerKind !== 'serviceCaptureMetadata' || row.policyEligible !== false) {
    throw new Error(`Enabled phase row was not service metadata only: ${JSON.stringify(row)}`);
  }
  if (row.queueJobId !== queueRecords[0].queueJobId) {
    throw new Error(`Read model and queue job IDs diverged: ${row.queueJobId} !== ${queueRecords[0].queueJobId}`);
  }
  if (readOptional(queuePath).includes('Ocentra Service Disabled Suppression Proof')) {
    throw new Error('Encrypted screen queue retained visible fixture text.');
  }
}

function assertDisabledSuppression({
  baselineReadModel,
  baselineQueueRecords,
  disabledReadModel,
  disabledQueueRecords,
}) {
  if (disabledReadModel.state !== 'ready') {
    throw new Error(`Disabled phase read model was not ready: ${JSON.stringify(disabledReadModel)}`);
  }
  if (disabledReadModel.rows.length !== baselineReadModel.rows.length) {
    throw new Error(`Disabled phase created new screen rows: ${JSON.stringify(disabledReadModel)}`);
  }
  if (disabledQueueRecords.length !== baselineQueueRecords.length) {
    throw new Error(`Disabled phase changed queue length: ${JSON.stringify(disabledQueueRecords)}`);
  }
  if (disabledQueueRecords[0]?.queueJobId !== baselineQueueRecords[0]?.queueJobId) {
    throw new Error(
      `Disabled phase consumed or replaced the pending queue job: ${JSON.stringify(disabledQueueRecords)}`
    );
  }
  if (
    disabledReadModel.rows.some(
      (row) => row.providerKind === 'localVision' || row.providerKind === 'localVisionUnavailable'
    )
  ) {
    throw new Error(`Disabled phase ran local vision analysis: ${JSON.stringify(disabledReadModel)}`);
  }
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
