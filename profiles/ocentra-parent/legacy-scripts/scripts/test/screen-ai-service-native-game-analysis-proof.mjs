import { spawn } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
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
const outputDir = join(repoRoot, 'output', 'screen-ai-pipeline-proof', 'service-native-game-analysis');
const agentPort = resolveParentDevPort(
  process.env.OCENTRA_SCREEN_AI_NATIVE_GAME_PROOF_AGENT_PORT,
  4692,
  'OCENTRA_SCREEN_AI_NATIVE_GAME_PROOF_AGENT_PORT'
);
const buildRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-ai-native-game-target-'));
const activityRoot = await mkdtemp(join(tmpdir(), 'ocentra-screen-ai-native-game-'));
const queueDir = join(activityRoot, 'screen-queue');
const journalPath = join(activityRoot, 'activity.ndjson');
const keyPath = join(activityRoot, 'activity-journal.key');
const storePath = join(activityRoot, 'activity.sqlite');
const fixturePath = join(activityRoot, 'ocentra-native-game-proof.txt');
const fixtureTitle = 'ocentra-native-game-proof.txt';
const adapterCommandPath = join(
  activityRoot,
  process.platform === 'win32' ? 'screen-ai-native-game-adapter.cmd' : 'screen-ai-native-game-adapter.sh'
);
const adapterScriptPath = join(
  activityRoot,
  process.platform === 'win32' ? 'screen-ai-native-game-adapter.ps1' : 'screen-ai-native-game-adapter-node.mjs'
);
const queuePath = join(queueDir, 'screen-evidence-queue.ndjson');
const healthUrl = createAgentHealthUrl(agentPort);
const wsUrl = createAgentWebSocketUrl(agentPort);

if (process.platform !== 'win32') {
  throw new Error('screen-ai-service-native-game-analysis-proof requires a real Windows desktop capture surface.');
}

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });
writeGameFixture();
writeAdapterCommand();

await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);

let nativeWindow;
let service;
let serviceOutput = () => '';
let lastObservedReadModel = null;

try {
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service'], {
    CARGO_TARGET_DIR: buildRoot,
  });

  nativeWindow = spawn('notepad.exe', [fixturePath], {
    cwd: repoRoot,
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: false,
  });
  await delay(1000);
  await focusNativeWindow(nativeWindow.pid, fixtureTitle);

  service = startService(serviceForegroundEnv());
  serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  await focusNativeWindow(nativeWindow.pid, fixtureTitle);
  const foregroundQueueRecords = await waitForQueueRecords(1);
  await delay(1500);
  await stopProcessTreeAndWait(service);
  service = undefined;

  await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
  service = startService(serviceAnalysisEnv());
  serviceOutput = collectOutput(service);
  await waitForHttp(healthUrl, serviceOutput);
  const analysisReadModel = await waitForGameAnalysisReadModel();
  const analyzedQueueJobId = localVisionGameRow(analysisReadModel)?.queueJobId;
  await waitForAnalyzedQueueRemoval(analyzedQueueJobId);
  const queueRecordsAfterAnalysis = readQueueRecordsAllowEmpty();
  assertProof(foregroundQueueRecords, analysisReadModel, queueRecordsAfterAnalysis);

  const sanitizedAnalysisReadModel = sanitizeReadModel(analysisReadModel);
  const sanitizedForegroundQueueRecords = sanitizeQueueRecords(foregroundQueueRecords);
  const sanitizedQueueRecordsAfterAnalysis = sanitizeQueueRecords(queueRecordsAfterAnalysis);
  const analysisRow = localVisionGameRow(sanitizedAnalysisReadModel);
  const foregroundQueueRecord = sanitizedForegroundQueueRecords[0];
  const summary = {
    proof: 'screen-ai-service-native-game-analysis-proof',
    proofTier: 'P3_LOCAL_DEV_MACHINE',
    platform: process.platform,
    agentPort,
    foregroundQueueRecord,
    analysisRow,
    foregroundQueueRecords: foregroundQueueRecords.length,
    queueRecordsAfterAnalysis: queueRecordsAfterAnalysis.length,
    artifacts: {
      proofSummary: relative(repoRoot, join(outputDir, 'proof-summary.json')),
      analysisReadModel: relative(repoRoot, join(outputDir, 'analysis-read-model.json')),
      foregroundQueueRecords: relative(repoRoot, join(outputDir, 'foreground-queue-records.json')),
      queueRecordsAfterAnalysis: relative(repoRoot, join(outputDir, 'queue-records-after-analysis.json')),
    },
    ephemeralPathsDeletedAfterProof: true,
    assertions: {
      realWindowsServiceCaptureRequired: true,
      controlledNativeWindowFocused: true,
      foregroundRuntimeCapturedNativeWindow: foregroundQueueRecords.length >= 1,
      foregroundQueueEncryptedImageStored:
        foregroundQueueRecord.nonceLength > 0 && foregroundQueueRecord.ciphertextLength > 0,
      foregroundReasonPreserved: analysisRow.captureReason === 'nativeAppForegroundStart',
      activeWindowScopePreserved: analysisRow.captureScope === 'activeWindow',
      localAdapterCommandExecutedThroughServiceRuntime: analysisRow.providerKind === 'localVision',
      serviceAnalysisClassifiedNativeGame: analysisRow.primaryCategory === 'game',
      adapterOutputAcceptedAbovePolicyThreshold: analysisRow.confidence >= 0.9 && analysisRow.policyEligible === true,
      encryptedQueueDrainedAfterAnalysis: queueRecordsAfterAnalysis.length === 0,
      rawFixtureTextNotRetainedInQueue:
        !readOptional(queuePath).includes('Ocentra Service Native Game Proof') &&
        !readOptional(queuePath).includes('Multiplayer lobby'),
      encryptedJournalDoesNotContainFixtureText:
        !readOptional(journalPath).includes('Ocentra Service Native Game Proof') &&
        !readOptional(journalPath).includes('Multiplayer lobby'),
      activityReadModelReachedViaWebSocket: analysisReadModel.state === 'ready',
    },
    nonClaims: [
      'This proves service-owned native foreground capture can feed service-owned local adapter analysis into a game-classified Activity Screen row.',
      'The controlled native surface is a Notepad-hosted game-lobby fixture, matching the existing native-game proof pattern; it is not a claim of installed commercial game detection.',
      'The service foreground trigger remains nativeAppForegroundStart until app/game identity evidence supplies a dedicated nativeGameForegroundStart producer.',
      'The adapter command is a local proof adapter for runtime plumbing; it is not a production VLM quality claim.',
    ],
  };
  writeJson(join(outputDir, 'proof-summary.json'), summary);
  writeJson(join(outputDir, 'analysis-read-model.json'), sanitizedAnalysisReadModel);
  writeJson(join(outputDir, 'foreground-queue-records.json'), sanitizedForegroundQueueRecords);
  writeJson(join(outputDir, 'queue-records-after-analysis.json'), sanitizedQueueRecordsAfterAnalysis);
  console.log(
    `screen-ai-service-native-game-analysis-proof-ok:${analysisRow.primaryCategory}:${queueRecordsAfterAnalysis.length}`
  );
} catch (error) {
  writeFailureArtifacts(error);
  throw error;
} finally {
  await Promise.allSettled([
    nativeWindow === undefined ? Promise.resolve() : stopProcessTreeAndWait(nativeWindow),
    service === undefined ? Promise.resolve() : stopProcessTreeAndWait(service),
  ]);
  await removeDirectoryWithRetry(activityRoot);
  await removeDirectoryWithRetry(buildRoot);
}

function writeGameFixture() {
  writeFileSync(
    fixturePath,
    [
      'Ocentra Service Native Game Proof',
      'Native game window',
      'Multiplayer lobby visible',
      'Start match, chat, and store buttons visible',
      'Game budget should apply after local screen analysis.',
    ].join('\r\n')
  );
}

function writeAdapterCommand() {
  writeFileSync(
    adapterScriptPath,
    [
      '$inputPayload = [Console]::In.ReadToEnd()',
      '$null = $inputPayload | ConvertFrom-Json',
      '$output = @{',
      "  summary = 'Local adapter classified a native game lobby from the queued capture.'",
      "  primaryCategory = 'game'",
      '  confidence = 0.93',
      '  policyEligible = $true',
      '} | ConvertTo-Json -Compress',
      '[Console]::Out.WriteLine($output)',
      '',
    ].join('\r\n')
  );
  writeFileSync(
    adapterCommandPath,
    [
      '@echo off',
      'powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0screen-ai-native-game-adapter.ps1"',
      '',
    ].join('\r\n')
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

function serviceForegroundEnv() {
  return {
    ...baseServiceEnv(),
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_RUNTIME_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_RUNTIME_ENABLED: 'false',
    OCENTRA_PARENT_SCREEN_SERVICE_ANALYSIS_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_ENABLED: 'true',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_POLL_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_MIN_GAP_SECONDS: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_MAX_CAPTURES: '1',
    OCENTRA_PARENT_SCREEN_SERVICE_FOREGROUND_MAX_TICKS: '8',
    OCENTRA_PARENT_SCREEN_SERVICE_QUEUE_MAX_PENDING: '1',
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
    OCENTRA_PARENT_SCREEN_SERVICE_QUEUE_MAX_PENDING: '3',
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
          return records;
        }
      } catch {
        await delay(250);
        continue;
      }
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for ${count} native game queue records.\n${queueDebugForError()}`);
}

async function waitForGameAnalysisReadModel() {
  const startedAt = Date.now();
  let lastReadModel;
  while (Date.now() - startedAt < 45000) {
    lastReadModel = await requestScreenReadModel();
    lastObservedReadModel = lastReadModel;
    if (lastReadModel.state === 'ready' && Array.isArray(lastReadModel.rows) && localVisionGameRow(lastReadModel)) {
      return lastReadModel;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for native game localVision row: ${JSON.stringify(lastReadModel)}`);
}

async function waitForAnalyzedQueueRemoval(queueJobId) {
  if (typeof queueJobId !== 'string' || queueJobId.length === 0) {
    throw new Error('Cannot wait for native game queue removal without analyzed queue job id.');
  }
  const startedAt = Date.now();
  while (Date.now() - startedAt < 10000) {
    const records = readQueueRecordsAllowEmpty();
    if (!records.some((record) => record.queueJobId === queueJobId)) {
      return;
    }
    await delay(100);
  }
  throw new Error(`Timed out waiting for analyzed native game queue job removal: ${queueJobId}`);
}

async function requestScreenReadModel() {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(wsUrl);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error('Native game screen read-model WebSocket proof timed out.'));
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
      reject(new Error('Native game screen read-model WebSocket failed.'));
    });
  });
}

function commandEnvelope() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-screen-ai-service-native-game-read-model',
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

function assertProof(foregroundQueueRecords, analysisReadModel, queueRecordsAfterAnalysis) {
  if (foregroundQueueRecords.length < 1) {
    throw new Error('Service foreground runtime did not queue a native game capture.');
  }
  if (foregroundQueueRecords.length !== 1) {
    throw new Error(
      `Service foreground proof must leave exactly one pending native game capture: ${JSON.stringify(
        sanitizeQueueRecords(foregroundQueueRecords)
      )}`
    );
  }
  const foregroundQueueRecord = foregroundQueueRecords[0];
  if (typeof foregroundQueueRecord.nonce !== 'string' || typeof foregroundQueueRecord.ciphertext !== 'string') {
    throw new Error(
      `Foreground queue record did not store encrypted image bytes: ${JSON.stringify(foregroundQueueRecord)}`
    );
  }
  const analysisRow = localVisionGameRow(analysisReadModel);
  if (!analysisRow) {
    throw new Error(`Analysis read model did not include a game localVision row: ${JSON.stringify(analysisReadModel)}`);
  }
  if (analysisRow.queueJobId !== foregroundQueueRecord.queueJobId) {
    throw new Error(
      `Analysis row did not consume the foreground queue job: ${JSON.stringify({
        foregroundQueueRecords: sanitizeQueueRecords(foregroundQueueRecords),
        analysisRow,
      })}`
    );
  }
  if (analysisRow.imageDigest !== foregroundQueueRecord.imageDigest) {
    throw new Error(
      `Analysis row did not preserve the foreground image digest: ${JSON.stringify({
        foregroundQueueRecord: sanitizeQueueRecords([foregroundQueueRecord])[0],
        analysisRow,
      })}`
    );
  }
  if (analysisRow.captureReason !== 'nativeAppForegroundStart') {
    throw new Error(`Analysis row lost native foreground reason: ${JSON.stringify(analysisRow)}`);
  }
  if (analysisRow.captureScope !== 'activeWindow') {
    throw new Error(`Analysis row lost active-window scope: ${JSON.stringify(analysisRow)}`);
  }
  if (analysisRow.confidence < 0.9 || analysisRow.policyEligible !== true) {
    throw new Error(`Analysis row was not policy eligible after threshold: ${JSON.stringify(analysisRow)}`);
  }
  if (queueRecordsAfterAnalysis.length !== 0) {
    throw new Error(`Queue was not drained after native game analysis: ${JSON.stringify(queueRecordsAfterAnalysis)}`);
  }
}

function localVisionGameRow(readModel) {
  if (!Array.isArray(readModel.rows)) {
    return undefined;
  }
  return readModel.rows.find((row) => row.providerKind === 'localVision' && row.primaryCategory === 'game');
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

function runQuietCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: repoRoot,
      env: process.env,
      stdio: ['ignore', 'pipe', 'pipe'],
      shell: false,
      windowsHide: true,
    });
    let stderr = '';
    child.stderr.on('data', (chunk) => {
      stderr += String(chunk);
    });
    child.once('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(' ')} exited with ${code}: ${stderr}`));
    });
    child.once('error', reject);
  });
}

async function focusNativeWindow(pid, titleContains) {
  const expectedTitle = String(titleContains ?? '').replace(/'/g, "''");
  const command = [
    '$shell = New-Object -ComObject WScript.Shell;',
    `$expectedTitle = '${expectedTitle}';`,
    'for ($i = 0; $i -lt 20; $i++) {',
    '$process = Get-Process -Name notepad -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -like "*$expectedTitle*" } | Select-Object -First 1;',
    'if ($process -and $shell.AppActivate($process.Id)) { Start-Sleep -Milliseconds 750; exit 0 };',
    Number.isInteger(pid) ? `if ($shell.AppActivate(${pid})) { Start-Sleep -Milliseconds 750; exit 0 };` : '',
    'Start-Sleep -Milliseconds 250;',
    '}',
    'exit 1;',
  ].join(' ');
  await runQuietCommand('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', command]);
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

function queueDebugForError() {
  if (!existsSync(queuePath)) {
    return 'queue file was not created';
  }
  try {
    return JSON.stringify(sanitizeQueueRecords(readQueueRecordsAllowEmpty()), null, 2);
  } catch {
    return 'queue file contained incomplete JSON records';
  }
}

function writeFailureArtifacts(error) {
  let queueRecords = [];
  try {
    queueRecords = sanitizeQueueRecords(readQueueRecordsAllowEmpty());
  } catch {
    queueRecords = [];
  }
  writeJson(join(outputDir, 'failure-summary.json'), {
    proof: 'screen-ai-service-native-game-analysis-proof',
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
