import { createHash } from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright';
import {
  AgentCommand,
  AgentEvent,
  AgentEventEnvelopeSchema,
} from '@ocentra-parent/schema-domain/agent-command-event-contracts';
import { AgentProtocolDefaults } from '@ocentra-parent/schema-domain/agent-protocol-defaults';
import {
  ParentDevEnv,
  ParentDevHost,
  ParentDevPort,
  createAgentAddress,
  createAgentHealthUrl,
  createAgentWebSocketUrl,
  createHttpOrigin,
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

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'ai-plan-proof', 'activity-screen-ai-degraded-portal-proof');
const testRoot = resolve(repoRoot, 'test-results', 'activity-screen-ai-degraded-portal-proof');
const artifactScreenshotPath = join(outputRoot, 'activity-screen-ai-degraded-portal.png');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');
const testSummaryPath = join(testRoot, 'proof.json');
const failureScreenshotPath = join(outputRoot, 'failure.png');
const failureSummaryPath = join(outputRoot, 'failure.json');
const devLogDir = await mkdtemp(join(tmpdir(), 'ocentra-parent-screen-ai-degraded-portal-'));
const activityDbPath = join(devLogDir, 'activity.sqlite');
const portalFrames = [];
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

await Promise.all([
  mkdir(outputRoot, { recursive: true }),
  mkdir(testRoot, { recursive: true }),
  rm(failureScreenshotPath, { force: true }),
  rm(failureSummaryPath, { force: true }),
]);
await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
await ensurePortFree(portalPort, isLikelyParentPortalOccupant, console.log);
await seedActivityStore(activityDbPath);

const agent = spawn(resolveDebugAgentServicePath(), [], {
  cwd: repoRoot,
  env: {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(agentPort),
    [ParentDevEnv.AgentAllowedOrigins]: createHttpOrigin(ParentDevHost.Loopback, portalPort),
    [ParentDevEnv.ActivityDbPath]: activityDbPath,
    [ParentDevEnv.DevLogDir]: devLogDir,
  },
  stdio: ['ignore', 'pipe', 'pipe'],
});
const agentOutput = collectOutput(agent);
const portal = spawnVitePortal(portalPort, {
  ...process.env,
  [ParentDevEnv.PortalAgentWebSocketUrl]: createAgentWebSocketUrl(agentPort),
  [ParentDevEnv.DevLogDir]: devLogDir,
});
const portalOutput = collectOutput(portal);

let browser;
let page;

try {
  await waitForHttp(createAgentHealthUrl(agentPort));
  await waitForHttp(`http://127.0.0.1:${portalPort}/`);
  browser = await chromium.launch({ headless: true });
  page = await browser.newPage({ viewport: { width: 1920, height: 1080 } });
  page.on('websocket', (socket) => {
    socket.on('framesent', (frame) => capturePortalFrame('sent', frame.payload));
    socket.on('framereceived', (frame) => capturePortalFrame('received', frame.payload));
  });
  await page.goto(`http://127.0.0.1:${portalPort}/#/screen-analysis`, { waitUntil: 'domcontentloaded' });
  await page.getByText('SCREEN ANALYSIS').first().waitFor({ timeout: 20000 });
  const serviceEvent = await requestScreenReadModel();
  await openScreenAnalysisRoute(page);
  await assertVisibleDegradedRows(page);
  await page.screenshot({ path: artifactScreenshotPath, fullPage: true });
  const summary = {
    status: 'ok',
    proofKind: 'real-service-to-portal-degraded-activity-screen',
    generatedAt: new Date().toISOString(),
    artifact: {
      screenshot: artifactScreenshotPath,
      summary: artifactSummaryPath,
      testSummary: testSummaryPath,
    },
    ports: {
      agent: agentPort,
      portal: portalPort,
    },
    serviceEvent: {
      event: serviceEvent.event,
      activityReadModelKind: payloadDisplay(serviceEvent, AgentProtocolDefaults.Field.ActivityReadModelKind),
      activitySurfaceState: payloadDisplay(serviceEvent, AgentProtocolDefaults.Field.ActivitySurfaceState),
      returned: payloadDisplay(serviceEvent, AgentProtocolDefaults.Field.Returned),
    },
    renderedAssertions: [
      'OCR unavailable row renders localOcr, modelUnavailable, unavailableNoImage, unavailable custody, and not-reported policy handoff.',
      'VLM degraded row renders localVision, degraded, unknown category, deleted image state, query-store custody, and not-reported policy handoff.',
      'Both rows preserve runtime/model/template refs in the parent portal Screen Analysis route.',
    ],
    nonClaims: [
      'This proves service-backed portal rendering of degraded Activity Screen read-model rows.',
      'It does not execute OCR/VLM inference or prove production model quality.',
      'It does not capture new live pixels, grant policy authority, or dispatch enforcement.',
    ],
  };
  await Promise.all([
    writeFile(artifactSummaryPath, `${JSON.stringify(summary, null, 2)}\n`),
    writeFile(testSummaryPath, `${JSON.stringify(summary, null, 2)}\n`),
  ]);
  console.log(`screen-ai-degraded-portal-proof-ok ${artifactSummaryPath}`);
} catch (error) {
  await writeFailureLog(error);
  throw error;
} finally {
  if (browser !== undefined) {
    await browser.close();
  }
  await Promise.all([stopProcessTreeAndWait(portal), stopProcessTreeAndWait(agent)]);
  await removeDirectoryWithRetry(devLogDir);
}

async function seedActivityStore(dbPath) {
  const sqlite = resolveSqlite();
  const sqlPath = join(devLogDir, 'seed-screen-ai-degraded-portal.sql');
  const rows = [
    {
      eventId: 'screen-ai-degraded-portal-ocr-unavailable',
      observedAt: '2026-06-06T16:10:00.000Z',
      fields: {
        screenAnalysisResultId: 'screen-analysis-result-ocr-unavailable-portal-proof',
        queueJobId: 'screen-queue-job-ocr-unavailable-portal-proof',
        summary: 'OCR unavailable: no local OCR model output retained and no policy handoff.',
        primaryCategory: 'unknown',
        confidence: 0,
        imageDeletionState: 'unavailableNoImage',
        policyEligible: false,
        modelRuntimeRef: 'windows-winrt-ocr-local-runtime',
        modelId: 'windows-winrt-ocr',
        providerKind: 'localOcr',
        promptOrTemplateVersion: 'screen-ocr-worker-winrt-v1',
        captureReason: 'timedCadence',
        captureScope: 'activeWindow',
        capabilityStatus: 'modelUnavailable',
        imageDigest: 'sha256:screen-ocr-unavailable-portal-proof',
        custodyState: 'unavailable',
      },
      evidence: [],
    },
    {
      eventId: 'screen-ai-degraded-portal-vlm-degraded',
      observedAt: '2026-06-06T16:11:00.000Z',
      fields: {
        screenAnalysisResultId: 'screen-analysis-result-vlm-degraded-portal-proof',
        queueJobId: 'screen-queue-job-vlm-degraded-portal-proof',
        summary: 'VLM degraded: local vision result is visible but not policy eligible.',
        primaryCategory: 'unknown',
        confidence: 0.18,
        imageDeletionState: 'deleted',
        policyEligible: false,
        modelRuntimeRef: 'screen-vlm-worker-runtime',
        modelId: 'screen-vlm-worker-model',
        providerKind: 'localVision',
        promptOrTemplateVersion: 'screen-vlm-worker-v1',
        captureReason: 'nativeAppForegroundStart',
        captureScope: 'activeWindow',
        capabilityStatus: 'degraded',
        imageDigest: 'sha256:screen-vlm-degraded-portal-proof',
        custodyState: 'child-device-query-store',
      },
      evidence: [
        {
          evidenceId: 'screen-evidence-ref-vlm-degraded-portal-proof',
          kind: 'local-db-row',
          digest: 'sha256:screen-vlm-degraded-portal-proof',
          uri: null,
        },
      ],
    },
  ];
  const sql = `
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS activity_events (
  event_id TEXT PRIMARY KEY,
  observed_at TEXT NOT NULL,
  device_id TEXT NOT NULL,
  platform TEXT NOT NULL,
  observer TEXT NOT NULL,
  kind TEXT NOT NULL,
  subject_kind TEXT NOT NULL,
  subject_id TEXT NOT NULL,
  subject_display_name TEXT,
  fields_json TEXT NOT NULL,
  evidence_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS activity_events_recent_idx
  ON activity_events (observed_at DESC, event_id DESC);
CREATE TABLE IF NOT EXISTS parent_rule_contexts (
  parent_rule_ref_id TEXT PRIMARY KEY,
  updated_at TEXT NOT NULL,
  expires_at TEXT,
  context_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS parent_rule_contexts_recent_idx
  ON parent_rule_contexts (updated_at DESC, parent_rule_ref_id DESC);
DELETE FROM activity_events;
${rows.map(insertActivityEventSql).join('\n')}
`;
  await writeFile(sqlPath, sql);
  const seeded = spawnSync(sqlite, [dbPath, `.read ${sqlPath}`], {
    cwd: repoRoot,
    encoding: 'utf8',
  });
  if (seeded.status !== 0) {
    throw new Error(`sqlite seed failed: ${seeded.stderr || seeded.stdout}`);
  }
}

function insertActivityEventSql(row) {
  return `INSERT INTO activity_events (
  event_id,
  observed_at,
  device_id,
  platform,
  observer,
  kind,
  subject_kind,
  subject_id,
  subject_display_name,
  fields_json,
  evidence_json
) VALUES (
  ${sqlString(row.eventId)},
  ${sqlString(row.observedAt)},
  'local-dev-agent',
  'windows',
  'local-ai',
  'activity.screen.analysis.summarized',
  'device',
  'local-dev-agent',
  NULL,
  ${sqlString(JSON.stringify(row.fields))},
  ${sqlString(JSON.stringify(row.evidence))}
);`;
}

function requestScreenReadModel() {
  return new Promise((resolvePromise, reject) => {
    const socket = new WebSocket(createAgentWebSocketUrl(agentPort));
    let settled = false;
    const timer = setTimeout(() => fail(new Error('degraded screen read-model WebSocket proof timed out')), 20000);

    const fail = (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      socket.close();
      reject(error);
    };

    const complete = (event) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      socket.close();
      resolvePromise(event);
    };

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
          fail(new Error(`Expected ${AgentEvent.ActivityScreenReadModelReported}, received ${parsed.event}`));
          return;
        }
        const readModel = JSON.parse(payloadText(parsed, AgentProtocolDefaults.Field.ActivityReadModel));
        console.log(
          `screen-ai-degraded-portal-read-model ${JSON.stringify({
            state: readModel.state,
            summary: readModel.summary,
            rows: readModel.rows?.length ?? 0,
            providers: (readModel.rows ?? []).map((row) => row.providerKind),
          })}`
        );
        assertServiceRows(readModel.rows ?? []);
        complete(parsed);
      } catch (error) {
        fail(error instanceof Error ? error : new Error(String(error)));
      }
    });

    socket.addEventListener('error', () => fail(new Error('degraded screen read-model WebSocket failed')));
  });
}

function assertServiceRows(rows) {
  const ocrRow = rows.find((row) => row.providerKind === 'localOcr');
  const vlmRow = rows.find((row) => row.providerKind === 'localVision');
  if (
    ocrRow?.capabilityStatus !== 'modelUnavailable' ||
    ocrRow?.imageDeletionState !== 'unavailableNoImage' ||
    ocrRow?.policyEligible !== false ||
    ocrRow?.custodyState !== 'unavailable'
  ) {
    throw new Error(`OCR unavailable service row missing degraded fields: ${JSON.stringify(ocrRow)}`);
  }
  if (
    vlmRow?.capabilityStatus !== 'degraded' ||
    vlmRow?.primaryCategory !== 'unknown' ||
    vlmRow?.imageDeletionState !== 'deleted' ||
    vlmRow?.policyEligible !== false ||
    vlmRow?.custodyState !== 'child-device-query-store'
  ) {
    throw new Error(`VLM degraded service row missing degraded fields: ${JSON.stringify(vlmRow)}`);
  }
}

async function openScreenAnalysisRoute(page) {
  await page.waitForURL(`**/#/screen-analysis`, { timeout: 15000 });
  await page.getByRole('heading', { name: 'Screen analysis' }).first().waitFor({ timeout: 15000 });
}

async function assertVisibleDegradedRows(page) {
  const expected = [
    'OCR unavailable: no local OCR model output retained and no policy handoff.',
    'localOcr',
    'modelUnavailable',
    'unavailableNoImage',
    'Unavailable',
    'windows-winrt-ocr-local-runtime',
    'windows-winrt-ocr | screen-ocr-worker-winrt-v1 | screen-queue-job-ocr-unavailable-portal-proof',
    'screen-ocr-worker-winrt-v1',
    'VLM degraded: local vision result is visible but not policy eligible.',
    'localVision',
    'degraded',
    'unknown',
    '0.18',
    'deleted',
    'Child device',
    'screen-vlm-worker-runtime',
    'screen-vlm-worker-model | screen-vlm-worker-v1 | screen-queue-job-vlm-degraded-portal-proof',
    'screen-vlm-worker-v1',
    'screen-evidence-ref-vlm-degraded-portal-proof',
    'Not reported',
    'Not claimed',
    'No family setting is configured for this area yet.',
  ];
  for (const text of expected) {
    await page.getByText(text, { exact: false }).first().waitFor({ timeout: 15000 });
  }
}

function commandEnvelope() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-screen-ai-degraded-portal-proof',
    sentAt: new Date().toISOString(),
    source: AgentProtocolDefaults.Peer.PortalDev,
    target: AgentProtocolDefaults.Target.LocalhostWindowsAgent,
    command: AgentCommand.ActivityScreenReadModelGet,
    payload: {
      [AgentProtocolDefaults.Field.ScopeKind]: 'family',
      [AgentProtocolDefaults.Field.FamilyId]: 'family-local',
      [AgentProtocolDefaults.Field.RequestedAt]: new Date().toISOString(),
      [AgentProtocolDefaults.Field.RangeStart]: '2026-06-06T00:00:00.000Z',
      [AgentProtocolDefaults.Field.RangeEnd]: '2026-06-06T23:59:59.000Z',
    },
  };
}

function payloadText(event, field) {
  const value = event.payload[field];
  if (typeof value !== 'string') {
    throw new Error(`Expected payload field ${field} to be string.`);
  }
  return value;
}

function payloadDisplay(event, field) {
  const value = event.payload[field];
  if (value === undefined) {
    throw new Error(`Expected payload field ${field} to be present.`);
  }
  return typeof value === 'string' ? value : JSON.stringify(value);
}

async function waitForHttp(url, timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return response;
      }
      lastError = new Error(`${url} returned ${response.status}`);
    } catch (error) {
      lastError = error;
    }
    await delay(250);
  }
  throw lastError instanceof Error ? lastError : new Error(`Timed out waiting for ${url}`);
}

function resolveSqlite() {
  const command = process.platform === 'win32' ? 'where' : 'which';
  const result = spawnSync(command, ['sqlite3'], { encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error('sqlite3 is required for degraded Activity Screen portal proof.');
  }
  return result.stdout.split(/\r?\n/u).find(Boolean);
}

function sqlString(value) {
  return `'${value.replaceAll("'", "''")}'`;
}

function collectOutput(child) {
  const chunks = [];
  child.stdout?.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr?.on('data', (chunk) => chunks.push(String(chunk)));
  return chunks;
}

function capturePortalFrame(direction, payload) {
  const text = typeof payload === 'string' ? payload : payload.toString('utf8');
  if (
    text.includes('activity.screen.read-model') ||
    text.includes('activityReadModel') ||
    text.includes('activityReadModelKind')
  ) {
    portalFrames.push({
      direction,
      payload: text.slice(0, 4000),
    });
  }
}

async function writeFailureLog(error) {
  let pageText = '';
  if (page !== undefined) {
    try {
      await page.screenshot({ path: failureScreenshotPath, fullPage: true });
      pageText = await page.locator('body').innerText({ timeout: 2000 });
    } catch {
      pageText = '';
    }
  }
  const failure = {
    status: 'failed',
    error: summarizeTextEvidence(error instanceof Error ? error.message : String(error)),
    screenshot: failureScreenshotPath,
    pageText: summarizeTextEvidence(pageText),
    portalFrames: portalFrames.map(summarizePortalFrame),
    agentOutput: summarizeTextEvidence(agentOutput.join('')),
    portalOutput: summarizeTextEvidence(portalOutput.join('')),
  };
  await writeFile(failureSummaryPath, `${JSON.stringify(failure, null, 2)}\n`);
}

function summarizePortalFrame(frame) {
  return {
    direction: sanitizeProofToken(frame.direction, 'portal frame direction'),
    payloadSha256: createHash('sha256').update(frame.payload).digest('hex'),
    payloadBytes: Buffer.byteLength(frame.payload),
  };
}

function sanitizeProofToken(value, label) {
  const text = String(value ?? '');
  if (!/^[A-Za-z0-9._:-]+$/u.test(text)) {
    throw new Error(`Unexpected ${label} token shape.`);
  }
  return text;
}

function summarizeTextEvidence(value) {
  const text = String(value ?? '');
  return {
    sha256: createHash('sha256').update(text).digest('hex'),
    bytes: Buffer.byteLength(text),
  };
}
