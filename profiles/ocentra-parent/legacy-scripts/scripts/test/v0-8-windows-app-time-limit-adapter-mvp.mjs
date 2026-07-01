import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { basename, join, relative } from 'node:path';
import { mkdir, mkdtemp, readFile, stat, writeFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const evidenceDirectory = join(process.cwd(), 'test-results', 'v0-8-windows-app-time-limit-adapter-mvp');
const timeoutMs = envNumber('OCENTRA_PARENT_V08_APP_TIME_LIMIT_TIMEOUT_MS', 20_000);

const ids = {
  policyDecisionId: envText('OCENTRA_PARENT_V08_APP_TIME_LIMIT_POLICY_DECISION_ID', 'decision-v08-app-time-limit'),
  actionId: envText('OCENTRA_PARENT_V08_APP_TIME_LIMIT_ACTION_ID', 'action-v08-app-time-limit'),
  resultId: envText('OCENTRA_PARENT_V08_APP_TIME_LIMIT_RESULT_ID', 'result-v08-app-time-limit'),
  auditEventId: envText('OCENTRA_PARENT_V08_APP_TIME_LIMIT_AUDIT_EVENT_ID', 'audit-v08-app-time-limit'),
  timerEventId: envText('OCENTRA_PARENT_V08_APP_TIME_LIMIT_TIMER_EVENT_ID', 'timer-v08-app-time-limit'),
  parentActionReferenceId: envText(
    'OCENTRA_PARENT_V08_APP_TIME_LIMIT_PARENT_ACTION_REFERENCE_ID',
    'parent-action-v08-app-time-limit'
  ),
};
const commandRefs = {
  evidenceReferenceIds: envText(
    'OCENTRA_PARENT_V08_APP_TIME_LIMIT_EVIDENCE_REFERENCE_IDS',
    'evidence-v08-app-time-limit'
  ),
  ruleIds: envText('OCENTRA_PARENT_V08_APP_TIME_LIMIT_RULE_IDS', 'rule-v08-app-time-limit'),
  reasonCodes: envText('OCENTRA_PARENT_V08_APP_TIME_LIMIT_REASON_CODES', 'parent-time-limit'),
  enforcementIntentId: envText('OCENTRA_PARENT_V08_APP_TIME_LIMIT_INTENT_ID', 'intent-v08-app-time-limit'),
  targetId: envText('OCENTRA_PARENT_V08_APP_TIME_LIMIT_TARGET_ID', 'target-v08-app-time-limit'),
};

await main();

async function main() {
  await runCommand('cargo', ['test', '-p', 'ocentra-parent-agent-core', 'enforcement_app_time_limit']);
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  await mkdir(evidenceDirectory, { recursive: true });
  const runRoot = await mkdtemp(join(evidenceDirectory, 'run-'));
  const agentPort = await freePort();
  const child = spawnOwnedChildProcess();
  let dryRunChild;
  let staleChild;
  let expiryChild;
  let service = spawnAgentService(runRoot, agentPort);
  let serviceOutput = collectOutput(service);

  try {
    await waitForHealth(agentPort, serviceOutput);
    await waitForStartupCaptureSettled(runRoot);
    const executeEvent = await requestEvent(agentPort, commandEnvelope('execute', child));
    const executeAssertion = assertExecuteEvent(executeEvent, child);

    await stopProcessTreeAndWait(service);
    service = spawnAgentService(runRoot, agentPort);
    serviceOutput = collectOutput(service);
    await waitForHealth(agentPort, serviceOutput);
    await waitForStartupCaptureSettled(runRoot);

    const recoverEvent = await requestEvent(agentPort, commandEnvelope('recover', child));
    const recoverAssertion = assertRecoverEvent(recoverEvent);
    const cancelEvent = await requestEvent(agentPort, commandEnvelope('cancel', child));
    const cancelAssertion = assertCancelEvent(cancelEvent);
    const unavailableEvent = await requestEvent(agentPort, commandEnvelope('recover', child));
    const unavailableAssertion = assertUnavailableEvent(unavailableEvent);

    dryRunChild = spawnOwnedChildProcess();
    const dryRunEvent = await requestEvent(agentPort, commandEnvelope('execute-dry-run', dryRunChild));
    const dryRunAssertion = assertExecuteEvent(dryRunEvent, dryRunChild, {
      adapterResultCode: 'dry-run-no-action',
      dryRun: true,
      status: 'would-enforce',
    });
    const dryRunCancelEvent = await requestEvent(agentPort, commandEnvelope('cancel-dry-run', dryRunChild));
    const dryRunCancelAssertion = assertCancelEvent(dryRunCancelEvent);
    assertChildStillRunning(dryRunChild);

    staleChild = spawnOwnedChildProcess();
    const executeStaleEvent = await requestEvent(agentPort, commandEnvelope('execute-stale', staleChild));
    const executeStaleAssertion = assertExecuteEvent(executeStaleEvent, staleChild);
    const staleRejectEvent = await requestEvent(agentPort, commandEnvelope('expire-stale-mismatch', staleChild));
    const staleRejectAssertion = assertStaleRejectEvent(staleRejectEvent, staleChild);
    const staleRecoverEvent = await requestEvent(agentPort, commandEnvelope('recover-stale', staleChild));
    const staleRecoverAssertion = assertRecoverEvent(staleRecoverEvent);
    const staleCancelEvent = await requestEvent(agentPort, commandEnvelope('cancel-stale', staleChild));
    const staleCancelAssertion = assertCancelEvent(staleCancelEvent);

    expiryChild = spawnOwnedChildProcess();
    const executeExpiryEvent = await requestEvent(agentPort, commandEnvelope('execute-expire', expiryChild));
    const executeExpiryAssertion = assertExecuteEvent(executeExpiryEvent, expiryChild);
    const expireEvent = await requestEvent(agentPort, commandEnvelope('expire', expiryChild));
    const expireAssertion = await assertExpireEvent(expireEvent, expiryChild);

    const journalText = await readFile(join(runRoot, 'activity.ndjson'), 'utf8');
    if (journalText.includes(executeAssertion.policyDecisionId)) {
      throw new Error('Encrypted journal contains plaintext app time-limit policy decision id.');
    }

    const evidence = {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      platform: process.platform,
      agentEndpoint: 'loopback-redacted',
      runRoot: relative(process.cwd(), runRoot),
      childProcess: {
        pid: child.pid ?? null,
        expectedProcessName: executeAssertion.expectedProcessName,
      },
      inputRefs: {
        policyDecisionId: ids.policyDecisionId,
        actionId: ids.actionId,
        evidenceReferenceIds: commandRefs.evidenceReferenceIds,
        ruleIds: commandRefs.ruleIds,
        reasonCodes: commandRefs.reasonCodes,
        enforcementIntentId: commandRefs.enforcementIntentId,
        targetId: commandRefs.targetId,
      },
      assertions: {
        execute: executeAssertion,
        recover: recoverAssertion,
        cancel: cancelAssertion,
        unavailable: unavailableAssertion,
        dryRun: dryRunAssertion,
        dryRunCancel: dryRunCancelAssertion,
        executeStale: executeStaleAssertion,
        staleReject: staleRejectAssertion,
        staleRecover: staleRecoverAssertion,
        staleCancel: staleCancelAssertion,
        executeExpiry: executeExpiryAssertion,
        expire: expireAssertion,
      },
      serviceScope: {
        timeLimitCreateRecoverCancelExpireProven: true,
        dryRunNoTerminateProven: true,
        staleTimerMismatchRejectsBeforeAdapter: true,
        staleTimerSurvivesMismatchForRecoveryAndCancel: true,
        expiryAdapterReachedThroughService: true,
        broadPackageBlockClaimed: false,
      },
      events: {
        execute: eventSummary(executeEvent),
        recover: eventSummary(recoverEvent),
        cancel: eventSummary(cancelEvent),
        unavailable: eventSummary(unavailableEvent),
        dryRun: eventSummary(dryRunEvent),
        dryRunCancel: eventSummary(dryRunCancelEvent),
        executeStale: eventSummary(executeStaleEvent),
        staleReject: eventSummary(staleRejectEvent),
        staleRecover: eventSummary(staleRecoverEvent),
        staleCancel: eventSummary(staleCancelEvent),
        executeExpiry: eventSummary(executeExpiryEvent),
        expire: eventSummary(expireEvent),
      },
      artifacts: {
        activityJournal: relative(process.cwd(), join(runRoot, 'activity.ndjson')),
        activityStore: relative(process.cwd(), join(runRoot, 'activity.sqlite')),
        timerStatePath: relative(process.cwd(), join(runRoot, 'enforcement-timers.json')),
        devLogDirectory: relative(process.cwd(), join(runRoot, 'logs')),
      },
    };
    const evidencePath = join(
      evidenceDirectory,
      `${evidence.generatedAt.replaceAll(':', '-').replaceAll('.', '-')}.json`
    );
    await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
    printSummary(evidencePath, evidence.assertions);
  } finally {
    await stopProcessTreeAndWait(service);
    await stopProcessTreeAndWait(child);
    if (dryRunChild) {
      await stopProcessTreeAndWait(dryRunChild);
    }
    if (staleChild) {
      await stopProcessTreeAndWait(staleChild);
    }
    if (expiryChild) {
      await stopProcessTreeAndWait(expiryChild);
    }
  }
}

async function waitForStartupCaptureSettled(runRoot) {
  const journalPath = join(runRoot, 'activity.ndjson');
  const deadline = Date.now() + timeoutMs;
  let previousSize = null;
  let stableChecks = 0;
  while (Date.now() < deadline) {
    const size = await fileSize(journalPath);
    if (size !== undefined && size === previousSize) {
      stableChecks += 1;
      if (stableChecks >= 2) {
        return;
      }
    } else {
      stableChecks = 0;
    }
    previousSize = size;
    await delay(250);
  }
  throw new Error('Timed out waiting for startup activity capture to settle.');
}

async function fileSize(path) {
  try {
    return (await stat(path)).size;
  } catch {
    return null;
  }
}

function spawnOwnedChildProcess() {
  return spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000);'], {
    cwd: process.cwd(),
    stdio: 'ignore',
    windowsHide: true,
  });
}

function waitForChildExit(child) {
  if (child.exitCode !== undefined || child.signalCode !== undefined) {
    return Promise.resolve();
  }
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      child.off('exit', onExit);
      reject(new Error(`Timed out waiting for owned child process ${child.pid} to exit.`));
    }, timeoutMs);
    const onExit = () => {
      clearTimeout(timer);
      resolve();
    };
    child.once('exit', onExit);
  });
}

function assertChildStillRunning(child) {
  if (child.exitCode !== undefined || child.signalCode !== undefined) {
    throw new Error(`Expected owned child process ${child.pid} to still be running.`);
  }
}

function spawnAgentService(runRoot, agentPort) {
  return spawn(resolveDebugAgentServicePath(), [], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      OCENTRA_PARENT_AGENT_ADDR: `127.0.0.1:${agentPort}`,
      OCENTRA_PARENT_ACTIVITY_DB_PATH: join(runRoot, 'activity.sqlite'),
      OCENTRA_PARENT_ACTIVITY_JOURNAL_PATH: join(runRoot, 'activity.ndjson'),
      OCENTRA_PARENT_ACTIVITY_JOURNAL_KEY_PATH: join(runRoot, 'activity.key'),
      OCENTRA_PARENT_AGENT_ENFORCEMENT_TIMER_STATE_PATH: join(runRoot, 'enforcement-timers.json'),
      OCENTRA_PARENT_DEV_LOG_DIR: join(runRoot, 'logs'),
    },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
}

async function waitForHealth(agentPort, serviceOutput) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${agentPort}/health`);
      if (response.ok) {
        return;
      }
    } catch {
      await delay(250);
    }
  }
  throw new Error(`Timed out waiting for service health. ${serviceOutput()}`);
}

function requestEvent(agentPort, command) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(`ws://127.0.0.1:${agentPort}/api/dev/ws`);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error(`Timed out waiting for ${command.command}.`));
    }, timeoutMs);

    socket.addEventListener('open', () => {
      socket.send(JSON.stringify(command));
    });
    socket.addEventListener('message', (message) => {
      const event = JSON.parse(String(message.data));
      if (event.event === 'agent.connection.ready') {
        return;
      }
      clearTimeout(timer);
      socket.close();
      resolve(event);
    });
    socket.addEventListener('error', () => {
      clearTimeout(timer);
      reject(new Error(`WebSocket error while requesting ${command.command}.`));
    });
  });
}

function assertExecuteEvent(event, child, expected = {}) {
  const expectedStatus = expected.status ?? 'no-op';
  const expectedAdapterResultCode = expected.adapterResultCode ?? 'no-op';
  const expectedDryRun = expected.dryRun ?? false;

  assertEventName(event, 'agent.enforcement.audit.reported');
  assertPayloadValue(event.payload, 'enforcementTimerEventKind', 'created');
  assertPayloadValue(event.payload, 'enforcementStatus', expectedStatus);
  assertPayloadValue(event.payload, 'enforcementAdapterResultCode', expectedAdapterResultCode);
  assertPayloadValue(event.payload, 'enforcementRollbackState', 'not-required');
  assertStored(event.payload);
  const action = JSON.parse(event.payload.enforcementAction);
  const timer = JSON.parse(event.payload.enforcementTimerEvent);
  if (action.mode !== 'time-limit' || action.target.targetType !== 'app') {
    throw new Error(`Expected app time-limit action, received ${JSON.stringify(action)}`);
  }
  if (timer.actionId !== ids.actionId || timer.policyDecisionId !== ids.policyDecisionId) {
    throw new Error(`Timer did not preserve action identity: ${JSON.stringify(timer)}`);
  }
  if (action.dryRun !== expectedDryRun) {
    throw new Error(`Expected action.dryRun=${expectedDryRun}, received ${action.dryRun}`);
  }
  return {
    policyDecisionId: ids.policyDecisionId,
    expectedProcessName: basename(process.execPath),
    childPid: child.pid ?? null,
    actionMode: action.mode,
    dryRun: action.dryRun,
    targetType: action.target.targetType,
    timerEventKind: timer.timerEventKind,
    status: event.payload.enforcementStatus,
    adapterResultCode: event.payload.enforcementAdapterResultCode,
    statePersisted: typeof event.payload.enforcementTimerState === 'string',
    databaseReady: event.payload.databaseReady,
    eventsStored: event.payload.eventsStored,
  };
}

function assertRecoverEvent(event) {
  assertEventName(event, 'agent.enforcement.timer.reported');
  assertPayloadValue(event.payload, 'enforcementTimerEventKind', 'restart-recovered');
  assertPayloadValue(event.payload, 'enforcementStatus', 'no-op');
  assertStored(event.payload);
  const timer = JSON.parse(event.payload.enforcementTimerEvent);
  if (timer.actionId !== ids.actionId || timer.policyDecisionId !== ids.policyDecisionId) {
    throw new Error(`Recovered timer did not preserve identity: ${JSON.stringify(timer)}`);
  }
  if (timer.recoveredAfterRestart !== true) {
    throw new Error('Recovered timer did not set recoveredAfterRestart=true.');
  }
  return {
    actionId: timer.actionId,
    policyDecisionId: timer.policyDecisionId,
    timerEventKind: timer.timerEventKind,
    recoveredAfterRestart: timer.recoveredAfterRestart,
    statePersisted: typeof event.payload.enforcementTimerState === 'string',
    databaseReady: event.payload.databaseReady,
    eventsStored: event.payload.eventsStored,
  };
}

function assertCancelEvent(event) {
  assertEventName(event, 'agent.enforcement.timer.reported');
  assertPayloadValue(event.payload, 'enforcementTimerEventKind', 'cancelled');
  assertPayloadValue(event.payload, 'enforcementStatus', 'superseded');
  assertStored(event.payload);
  const audit = JSON.parse(event.payload.enforcementAuditEvent);
  if (audit.auditEventKind !== 'cancelled') {
    throw new Error(`Expected cancelled audit, received ${audit.auditEventKind}`);
  }
  if (audit.parentOverride?.actionReferenceId !== ids.parentActionReferenceId) {
    throw new Error(`Expected parent override reference, received ${JSON.stringify(audit.parentOverride)}`);
  }
  if (event.payload.enforcementTimerState !== undefined) {
    throw new Error('Cancel event should clear persisted active timer state.');
  }
  return {
    actionId: audit.action.actionId,
    auditEventKind: audit.auditEventKind,
    parentOverrideId: audit.parentOverride.actionReferenceId,
    stateCleared: true,
    databaseReady: event.payload.databaseReady,
    eventsStored: event.payload.eventsStored,
  };
}

async function assertExpireEvent(event, child) {
  assertEventName(event, 'agent.enforcement.timer.reported');
  assertStored(event.payload);
  const timer = JSON.parse(event.payload.enforcementTimerEvent);
  const audit = JSON.parse(event.payload.enforcementAuditEvent);
  if (timer.actionId !== ids.actionId || timer.policyDecisionId !== ids.policyDecisionId) {
    throw new Error(`Expired timer did not preserve identity: ${JSON.stringify(timer)}`);
  }

  if (process.platform === 'win32') {
    assertPayloadValue(event.payload, 'enforcementTimerEventKind', 'expired');
    assertPayloadValue(event.payload, 'enforcementStatus', 'expired');
    if (!['process-terminated', 'process-already-exited'].includes(event.payload.enforcementAdapterResultCode)) {
      throw new Error(
        `Expected Windows process termination result, received ${event.payload.enforcementAdapterResultCode}`
      );
    }
    await waitForChildExit(child);
  } else {
    assertPayloadValue(event.payload, 'enforcementTimerEventKind', 'unavailable');
    assertPayloadValue(event.payload, 'enforcementStatus', 'unavailable');
    assertPayloadValue(event.payload, 'enforcementAdapterResultCode', 'unsupported-platform');
  }

  if (event.payload.enforcementTimerState !== undefined) {
    throw new Error('Expire event should clear persisted active timer state.');
  }

  return {
    actionId: timer.actionId,
    auditEventKind: audit.auditEventKind,
    timerEventKind: timer.timerEventKind,
    status: event.payload.enforcementStatus,
    adapterResultCode: event.payload.enforcementAdapterResultCode,
    stateCleared: true,
    databaseReady: event.payload.databaseReady,
    eventsStored: event.payload.eventsStored,
  };
}

function assertUnavailableEvent(event) {
  assertEventName(event, 'agent.enforcement.timer.reported');
  assertPayloadValue(event.payload, 'available', false);
  assertPayloadValue(event.payload, 'reason', 'enforcement-active-timer-state-required');
  assertPayloadValue(event.payload, 'enforcementStatus', 'unavailable');
  assertPayloadValue(event.payload, 'enforcementTimerEventKind', 'recovery-needed');
  return {
    available: event.payload.available,
    reason: event.payload.reason,
    status: event.payload.enforcementStatus,
    timerEventKind: event.payload.enforcementTimerEventKind,
  };
}

function assertStaleRejectEvent(event, child) {
  assertEventName(event, 'agent.command.rejected');
  assertPayloadValue(event.payload, 'reason', 'enforcement-active-timer-state-mismatch');
  assertChildStillRunning(child);
  return {
    event: event.event,
    reason: event.payload.reason,
    childPid: child.pid ?? null,
    adapterSkipped: true,
    timerStatePreservedForRecovery: true,
  };
}

function assertStored(payload) {
  if (payload.databaseReady !== true || Number(payload.eventsStored) < 1) {
    throw new Error(`Expected journal/store proof, payload=${JSON.stringify(payload)}`);
  }
}

function assertEventName(event, expected) {
  if (event.event !== expected) {
    throw new Error(`Expected ${expected}, received ${event.event}: ${JSON.stringify(event.payload)}`);
  }
}

function assertPayloadValue(payload, key, expected) {
  if (payload[key] !== expected) {
    throw new Error(`Expected ${key}=${expected}, received ${payload[key]}`);
  }
}

function commandEnvelope(kind, child) {
  const now = new Date();
  const expiresAt = new Date(now.getTime() + 300_000);
  const base = {
    schemaVersion: 1,
    messageId: `cmd-v08-app-time-limit-${kind}`,
    sentAt: now.toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
  };
  if (kind.startsWith('execute')) {
    return {
      ...base,
      command: 'agent.enforcement.execute',
      payload: {
        ...commonPayload(now, kind),
        policyAction: 'time-limit',
        targetType: 'app',
        targetId: commandRefs.targetId,
        targetValue: basename(process.execPath),
        dryRun: kind === 'execute-dry-run',
        reasonCodes: commandRefs.reasonCodes,
        ruleIds: commandRefs.ruleIds,
        evidenceReferenceIds: commandRefs.evidenceReferenceIds,
        expiresAt: expiresAt.toISOString(),
        enforcementIntentId: commandRefs.enforcementIntentId,
        processId: child.pid,
      },
    };
  }
  if (kind === 'expire' || kind === 'expire-stale-mismatch') {
    return {
      ...base,
      command: 'agent.enforcement.timer.expire',
      payload: {
        ...commonPayload(now, kind),
        processId: child.pid,
      },
    };
  }
  if (kind.startsWith('cancel')) {
    return {
      ...base,
      command: 'agent.enforcement.override.cancel',
      payload: {
        ...commonPayload(now, kind),
        parentActionReferenceId: ids.parentActionReferenceId,
        parentActorId: 'parent-v08-app-time-limit',
        parentActorRole: 'parent',
        parentActionCreatedAt: now.toISOString(),
      },
    };
  }
  return {
    ...base,
    command: 'agent.enforcement.timer.recover',
    payload: commonPayload(now, kind),
  };
}

function commonPayload(now, kind) {
  return {
    policyDecisionId: ids.policyDecisionId,
    policyVersion: 'policy-v08-app-time-limit',
    requestedAt: now.toISOString(),
    enforcementActionId: kind === 'expire-stale-mismatch' ? 'action-v08-app-time-limit-stale-mismatch' : ids.actionId,
    enforcementResultId: `${ids.resultId}-${kind}`,
    enforcementAuditEventId: `${ids.auditEventId}-${kind}`,
    enforcementTimerEventId: `${ids.timerEventId}-${kind}`,
  };
}

async function freePort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const port = server.address().port;
  await new Promise((resolve) => server.close(resolve));
  return port;
}

function collectOutput(child) {
  const chunks = [];
  child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
  return () => chunks.join('');
}

function envNumber(name, fallback) {
  const parsed = Number(process.env[name]);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function envText(name, fallback) {
  const value = process.env[name]?.trim();
  return value && value.length > 0 ? value : fallback;
}

function runCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: process.cwd(), stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} exited with ${code}`));
    });
    child.once('error', reject);
  });
}

function eventSummary(event) {
  return {
    event: event.event,
    severity: event.severity,
    payload: event.payload,
  };
}

function printSummary(evidencePath, assertions) {
  console.log('v0-8-windows-app-time-limit-adapter-mvp-ok=true');
  console.log(`evidence=${evidencePath}`);
  console.log(
    `execute=${assertions.execute.timerEventKind}/${assertions.execute.status} dryRun=${assertions.dryRun.status}/${assertions.dryRun.adapterResultCode} stale=${assertions.staleReject.reason} recover=${assertions.recover.timerEventKind} cancel=${assertions.cancel.auditEventKind} expire=${assertions.expire.timerEventKind}/${assertions.expire.adapterResultCode} unavailable=${assertions.unavailable.reason}`
  );
}
