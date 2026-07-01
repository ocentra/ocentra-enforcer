import { spawn, spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
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
const outputRoot = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'portal-chain');
const artifactScreenshotPath = join(outputRoot, 'parent-portal-screen-chain.png');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');
const failureScreenshotPath = join(outputRoot, 'failure.png');
const failureSummaryPath = join(outputRoot, 'failure.json');
const devLogDir = await mkdtemp(join(tmpdir(), 'ocentra-parent-screen-ai-portal-chain-'));
const activityDbPath = join(devLogDir, 'activity.sqlite');
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

await mkdir(outputRoot, { recursive: true });
await Promise.all([rm(failureScreenshotPath, { force: true }), rm(failureSummaryPath, { force: true })]);
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
  await page.goto(`http://127.0.0.1:${portalPort}/#/activity`, { waitUntil: 'domcontentloaded' });
  await page.getByText('ACTIVITY').first().waitFor({ timeout: 20000 });
  const serviceEvent = await requestScreenReadModel();
  await page.getByText('GAMEDEV').first().waitFor({ timeout: 15000 });
  await openScreenTab(page);
  await assertVisibleChain(page);
  await page.screenshot({ path: artifactScreenshotPath, fullPage: true });
  const summary = {
    status: 'ok',
    proofKind: 'real-service-to-portal-screen-chain',
    artifact: {
      screenshot: artifactScreenshotPath,
      summary: artifactSummaryPath,
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
      'Trigger nativeAppForegroundStart',
      'Capture scope activeWindow',
      'AI provider localVision',
      'Category productivity',
      'Confidence 91%',
      'Policy eligible Yes',
      'Raw image deleted',
      'Custody child-device-journal',
      'Queue job screen-queue-job-portal-proof',
      'Image digest sha256:portal-screen-image-d...',
    ],
    nonClaims: [
      'This proves parent portal rendering of the service-backed read-model chain.',
      'It does not claim live external-account/browser-plan trigger ownership.',
      'It does not claim product-complete background watcher or broad enforcement adapters.',
    ],
  };
  await writeFile(artifactSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`screen-ai-portal-chain-proof-ok ${artifactSummaryPath}`);
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
  const sqlPath = join(devLogDir, 'seed-screen-ai-portal-chain.sql');
  const fields = {
    screenAnalysisResultId: 'screen-analysis-result-portal-proof',
    queueJobId: 'screen-queue-job-portal-proof',
    summary: 'Native productivity window analyzed locally; raw image deleted.',
    primaryCategory: 'productivity',
    confidence: 0.91,
    imageDeletionState: 'deleted',
    policyEligible: true,
    modelRuntimeRef: 'local-vlm-runtime-portal-proof',
    modelId: 'qwen2-vl-local-proof',
    providerKind: 'localVision',
    promptOrTemplateVersion: 'screen-visible-activity-v1',
    captureReason: 'nativeAppForegroundStart',
    captureScope: 'activeWindow',
    capabilityStatus: 'ready',
    imageDigest: 'sha256:portal-screen-image-digest',
    custodyState: 'child-device-journal',
  };
  const evidence = [
    {
      evidenceId: 'screen-evidence-ref-portal-proof',
      kind: 'journal-entry',
      digest: 'sha256:portal-screen-image-digest',
      uri: null,
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
INSERT INTO activity_events (
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
  'screen-ai-portal-chain-event',
  '2026-06-03T19:45:00.000Z',
  'local-dev-agent',
  'windows',
  'local-ai',
  'activity.screen.analysis.summarized',
  'device',
  'local-dev-agent',
  NULL,
  ${sqlString(JSON.stringify(fields))},
  ${sqlString(JSON.stringify(evidence))}
);
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

function requestScreenReadModel() {
  return new Promise((resolvePromise, reject) => {
    const socket = new WebSocket(createAgentWebSocketUrl(agentPort));
    let settled = false;
    const timer = setTimeout(() => fail(new Error('screen read-model WebSocket proof timed out')), 20000);

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
        const row = readModel.rows?.[0];
        if (row?.captureReason !== 'nativeAppForegroundStart' || row?.imageDeletionState !== 'deleted') {
          fail(new Error(`Screen read-model chain fields missing: ${JSON.stringify(row)}`));
          return;
        }
        complete(parsed);
      } catch (error) {
        fail(error instanceof Error ? error : new Error(String(error)));
      }
    });

    socket.addEventListener('error', () => fail(new Error('screen read-model WebSocket failed')));
  });
}

async function openScreenTab(page) {
  await page.getByRole('button', { name: 'Select Per Device' }).first().click({ force: true });
  await page
    .getByRole('img', { name: /^Per Device:/u })
    .first()
    .waitFor({ timeout: 15000 });
  await page.getByRole('tab', { name: 'Show activity Screen' }).click({ force: true });
  await page.getByText('AI provider').waitFor({ timeout: 15000 });
}

async function assertVisibleChain(page) {
  const expected = [
    'Native productivity window analyzed locally; raw image deleted.',
    'nativeAppForegroundStart',
    'activeWindow',
    'ready',
    'localVision',
    'productivity',
    '91%',
    'Yes',
    'deleted',
    'child-device-journal',
    'screen-queue-job-portal-proof',
    'IMAGE DIGEST',
    'sha256:portal-screen-image-d',
  ];
  for (const text of expected) {
    await page.getByText(text, { exact: false }).first().waitFor({ timeout: 15000 });
  }
}

function commandEnvelope() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-screen-ai-portal-chain-proof',
    sentAt: new Date().toISOString(),
    source: AgentProtocolDefaults.Peer.PortalDev,
    target: AgentProtocolDefaults.Target.LocalhostWindowsAgent,
    command: AgentCommand.ActivityScreenReadModelGet,
    payload: {
      [AgentProtocolDefaults.Field.ScopeKind]: 'family',
      [AgentProtocolDefaults.Field.FamilyId]: 'family-local',
      [AgentProtocolDefaults.Field.RequestedAt]: new Date().toISOString(),
      [AgentProtocolDefaults.Field.RangeStart]: '2026-06-03T00:00:00.000Z',
      [AgentProtocolDefaults.Field.RangeEnd]: '2026-06-03T23:59:59.000Z',
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
    throw new Error('sqlite3 is required for screen AI portal chain proof.');
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
    message: error instanceof Error ? error.message : String(error),
    screenshot: failureScreenshotPath,
    pageText: pageText.slice(0, 8000),
    agentOutput: agentOutput.join('').slice(-8000),
    portalOutput: portalOutput.join('').slice(-8000),
  };
  await writeFile(failureSummaryPath, `${JSON.stringify(failure, null, 2)}\n`);
}
