import { spawn, spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
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
  resolveParentDevPort,
} from '../dev/local-dev-config.mjs';
import { ensurePortFree } from '../dev/port-utils.mjs';
import {
  removeDirectoryWithRetry,
  resolveDebugAgentServicePath,
  stopProcessTreeAndWait,
} from './agent-service-process.mjs';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'ai-plan-proof', 'screen-summary-parent-explanation-service-read-model');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');
const sourceSnapshotPath = join(outputRoot, '00-source-snapshot.md');
const validationLogPath = join(outputRoot, '14-validation-commands.log');
const failureSummaryPath = join(outputRoot, 'failure.json');
const devLogDir = await mkdtemp(join(tmpdir(), 'ocentra-parent-screen-summary-explanation-service-'));
const activityDbPath = join(devLogDir, 'activity.sqlite');
const agentPort = resolveParentDevPort(
  process.env[ParentDevEnv.AgentPort],
  ParentDevPort.PortalSmokeAgent,
  ParentDevEnv.AgentPort
);

const expected = {
  rowId: 'screen-summary-parent-explanation-service-row',
  queueJobId: 'screen-summary-parent-explanation-service-queue',
  policyDecisionRef: 'screen-summary-parent-explanation-service-policy-decision',
  policyAction: 'allow',
  parentRuleRef: 'screen-summary-parent-explanation-service-parent-rule',
  localModelRuntimeRef: 'screen-summary-parent-explanation-service-local-runtime',
  parentExplanationRef: 'screen-summary-parent-explanation-service-explanation',
  explanationReason: 'screen-summary-cited',
  deletionReason: 'screen-image-deleted',
};

await mkdir(outputRoot, { recursive: true });
await rm(failureSummaryPath, { force: true });
await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
await seedActivityStore(activityDbPath);

const agent = spawn(resolveDebugAgentServicePath(), [], {
  cwd: repoRoot,
  env: {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(agentPort),
    [ParentDevEnv.AgentAllowedOrigins]: createHttpOrigin(ParentDevHost.Loopback, ParentDevPort.PortalSmokePortal),
    [ParentDevEnv.ActivityDbPath]: activityDbPath,
    [ParentDevEnv.DevLogDir]: devLogDir,
  },
  stdio: ['ignore', 'pipe', 'pipe'],
});
const agentOutput = collectOutput(agent);

try {
  await waitForHttp(createAgentHealthUrl(agentPort));
  const serviceEvent = await requestScreenReadModel();
  const readModel = JSON.parse(payloadText(serviceEvent, AgentProtocolDefaults.Field.ActivityReadModel));
  const row = readModel.rows?.[0];
  assert(row !== undefined, 'service read model returned no screen rows');
  assert(row.rowId === expected.rowId, `unexpected rowId ${row.rowId}`);
  assert(row.queueJobId === expected.queueJobId, `unexpected queueJobId ${row.queueJobId}`);
  assert(row.policyDecisionRef === expected.policyDecisionRef, `policy decision ref missing: ${row.policyDecisionRef}`);
  assert(row.policyAction === expected.policyAction, `policy action missing: ${row.policyAction}`);
  assert(includes(row.policyReasonCodes, 'screen-summary-linked'), 'policy reason code missing');
  assert(includes(row.parentRuleRefs, expected.parentRuleRef), 'parent rule ref missing');
  assert(includes(row.localModelRuntimeRefs, expected.localModelRuntimeRef), 'runtime ref missing');
  assert(includes(row.parentExplanationRefs, expected.parentExplanationRef), 'parent explanation ref missing');
  assert(includes(row.explanationReasons, expected.explanationReason), 'explanation reason missing');
  assert(includes(row.deletionReasons, expected.deletionReason), 'deletion reason missing');
  assert(row.imageDeletionState === 'deleted', `raw image deletion state missing: ${row.imageDeletionState}`);
  assert(row.custodyState === 'child-device-journal', `custody state missing: ${row.custodyState}`);
  assert(row.policyEligible === true, 'policy eligibility not preserved');

  const summary = {
    status: 'ok',
    proofKind: 'screen-summary-parent-explanation-service-read-model',
    artifacts: {
      summary: artifactSummaryPath,
      sourceSnapshot: sourceSnapshotPath,
      validationLog: validationLogPath,
    },
    serviceEvent: {
      event: serviceEvent.event,
      activityReadModelKind: payloadDisplay(serviceEvent, AgentProtocolDefaults.Field.ActivityReadModelKind),
      activitySurfaceState: payloadDisplay(serviceEvent, AgentProtocolDefaults.Field.ActivitySurfaceState),
      returned: payloadDisplay(serviceEvent, AgentProtocolDefaults.Field.Returned),
    },
    row: {
      rowId: row.rowId,
      queueJobId: row.queueJobId,
      policyDecisionRef: row.policyDecisionRef,
      policyAction: row.policyAction,
      policyReasonCodes: row.policyReasonCodes,
      parentRuleRefs: row.parentRuleRefs,
      localModelRuntimeRefs: row.localModelRuntimeRefs,
      parentExplanationRefs: row.parentExplanationRefs,
      explanationReasons: row.explanationReasons,
      deletionReasons: row.deletionReasons,
      imageDeletionState: row.imageDeletionState,
      custodyState: row.custodyState,
      evidenceCount: Array.isArray(row.evidence) ? row.evidence.length : 0,
    },
    closure: {
      serviceBackedWebSocketReadModel: true,
      queryStoreIngestPreservedExplanationRefs: true,
      rawScreenshotsRetainedByDefault: false,
      remoteAiUsedForChildSafety: false,
      portalUiRenderingClaimed: false,
    },
    nonClaims: [
      'This proof starts the real Rust service and requests the Activity Screen read model over WebSocket.',
      'It proves query-store/service read-model custody for parent explanation refs, not production portal rendering.',
      'It does not create new captures, rerun model inference, upload raw screenshots, or claim remote/API AI.',
    ],
  };
  await writeFile(artifactSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  await writeFile(sourceSnapshotPath, sourceSnapshot(summary));
  await writeFile(
    validationLogPath,
    [
      'node --check scripts/test/screen-summary-parent-explanation-service-read-model-proof.mjs',
      'node scripts/test/screen-summary-parent-explanation-service-read-model-proof.mjs',
    ].join('\n') + '\n'
  );
  console.log(`screen-summary-parent-explanation-service-read-model-proof-ok ${artifactSummaryPath}`);
} catch (error) {
  await writeFailureLog(error);
  throw error;
} finally {
  await stopProcessTreeAndWait(agent);
  await removeDirectoryWithRetry(devLogDir);
}

async function seedActivityStore(dbPath) {
  const sqlite = resolveSqlite();
  const sqlPath = join(devLogDir, 'seed-screen-summary-parent-explanation-service.sql');
  const fields = {
    screenAnalysisResultId: expected.rowId,
    queueJobId: expected.queueJobId,
    summary: 'Screen summary parent explanation is ready for parent audit.',
    primaryCategory: 'school',
    confidence: 0.94,
    imageDeletionState: 'deleted',
    policyEligible: true,
    modelRuntimeRef: expected.localModelRuntimeRef,
    localModelRuntimeRefs: expected.localModelRuntimeRef,
    modelId: 'windows-winrt-ocr-local-proof',
    providerKind: 'localOcr',
    promptOrTemplateVersion: 'screen-summary-parent-explanation-service-v1',
    captureReason: 'managedBrowserUrlChange',
    captureScope: 'selectedWindow',
    capabilityStatus: 'ready',
    imageDigest: 'sha256:screen-summary-parent-explanation-service-digest',
    custodyState: 'child-device-journal',
    policyDecisionId: expected.policyDecisionRef,
    policyAction: expected.policyAction,
    reasonCodes: 'screen-summary-linked,parent-rule-linked,deleted-image-linked',
    ruleIds: expected.parentRuleRef,
    parentExplanationRefs: expected.parentExplanationRef,
    explanationReasons: `${expected.explanationReason},policy-decision-cited,parent-rule-cited`,
    deletionReasons: expected.deletionReason,
  };
  const evidence = [
    {
      evidenceId: 'screen-summary-parent-explanation-service-evidence',
      kind: 'journal-entry',
      digest: 'sha256:screen-summary-parent-explanation-service-digest',
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
  'screen-summary-parent-explanation-service-event',
  '2026-06-05T12:00:00.000Z',
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
    const timer = setTimeout(() => fail(new Error('screen explanation service read-model timed out')), 20000);

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
        complete(parsed);
      } catch (error) {
        fail(error instanceof Error ? error : new Error(String(error)));
      }
    });

    socket.addEventListener('error', () => fail(new Error('screen explanation read-model WebSocket failed')));
  });
}

function commandEnvelope() {
  return {
    schemaVersion: 1,
    messageId: 'cmd-screen-summary-parent-explanation-service-read-model-proof',
    sentAt: new Date().toISOString(),
    source: AgentProtocolDefaults.Peer.PortalDev,
    target: AgentProtocolDefaults.Target.LocalhostWindowsAgent,
    command: AgentCommand.ActivityScreenReadModelGet,
    payload: {
      [AgentProtocolDefaults.Field.ScopeKind]: 'family',
      [AgentProtocolDefaults.Field.FamilyId]: 'family-local',
      [AgentProtocolDefaults.Field.RequestedAt]: new Date().toISOString(),
      [AgentProtocolDefaults.Field.RangeStart]: '2026-06-05T00:00:00.000Z',
      [AgentProtocolDefaults.Field.RangeEnd]: '2026-06-05T23:59:59.000Z',
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
    throw new Error('sqlite3 is required for screen summary parent explanation service read-model proof.');
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

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function includes(value, expectedValue) {
  return Array.isArray(value) && value.includes(expectedValue);
}

function sourceSnapshot(summary) {
  return [
    '# Screen Summary Parent Explanation Service Read Model Proof',
    '',
    `- Status: ${summary.status}`,
    `- Proof kind: ${summary.proofKind}`,
    `- Service event: ${summary.serviceEvent.event}`,
    `- Activity read-model kind: ${summary.serviceEvent.activityReadModelKind}`,
    `- Activity surface state: ${summary.serviceEvent.activitySurfaceState}`,
    `- Row id: ${summary.row.rowId}`,
    `- Policy decision ref: ${summary.row.policyDecisionRef}`,
    `- Parent rule refs: ${summary.row.parentRuleRefs.join(', ')}`,
    `- Parent explanation refs: ${summary.row.parentExplanationRefs.join(', ')}`,
    `- Local model runtime refs: ${summary.row.localModelRuntimeRefs.join(', ')}`,
    `- Image deletion state: ${summary.row.imageDeletionState}`,
    `- Custody state: ${summary.row.custodyState}`,
    '',
    'Non-claims:',
    ...summary.nonClaims.map((claim) => `- ${claim}`),
    '',
  ].join('\n');
}

async function writeFailureLog(error) {
  const failure = {
    status: 'failed',
    message: error instanceof Error ? error.message : String(error),
    agentOutput: agentOutput.join('').slice(-8000),
  };
  await writeFile(failureSummaryPath, `${JSON.stringify(failure, null, 2)}\n`);
}
