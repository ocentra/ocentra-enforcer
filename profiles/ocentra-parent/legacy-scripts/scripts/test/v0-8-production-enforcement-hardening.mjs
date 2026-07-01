import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { basename, join, relative } from 'node:path';
import { mkdir, mkdtemp, readFile, stat, writeFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const evidenceDirectory = join(process.cwd(), 'test-results', 'v0-8-production-enforcement-hardening');
const timeoutMs = envNumber('OCENTRA_PARENT_V08_PRODUCTION_HARDENING_TIMEOUT_MS', 20_000);

await main();

async function main() {
  await runCommand('cargo', [
    'test',
    '-p',
    'ocentra-parent-agent-service',
    'enforcement_execute_reports_manual_required_service_states_for_unwired_adapters',
  ]);
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  await mkdir(evidenceDirectory, { recursive: true });
  const runRoot = await mkdtemp(join(evidenceDirectory, 'run-'));
  const agentPort = await freePort();
  const service = spawnAgentService(runRoot, agentPort);
  const serviceOutput = collectOutput(service);

  try {
    await waitForHealth(agentPort, serviceOutput);
    await waitForStartupActivityCaptureIdle(runRoot);
    const assertions = [];
    const processChild = spawnOwnedChildProcess();
    try {
      const processEvent = await requestEvent(agentPort, processTerminateCommand(processChild));
      assertions.push(await assertProcessTerminateEvent(processEvent, processChild));
    } finally {
      await stopProcessTreeAndWait(processChild);
    }
    for (const scenario of scenarios()) {
      const event = await requestEvent(agentPort, commandEnvelope(scenario));
      assertions.push(assertManualRequiredEvent(event, scenario));
    }
    const journalText = await readFile(join(runRoot, 'activity.ndjson'), 'utf8');
    if (journalText.includes('decision-v08-hardening')) {
      throw new Error('Encrypted journal contains plaintext V0.8 hardening decision identifiers.');
    }
    const evidence = {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      platform: process.platform,
      agentEndpoint: 'loopback-redacted',
      runRoot: relative(process.cwd(), runRoot),
      serviceScope: {
        manualRequiredStatesProvenThroughService: true,
        unsupportedBlockingClaimsRejected: true,
        auditStoragePathProven: true,
        processTerminateServiceProof: process.platform === 'win32' ? 'actually-enforced' : 'unsupported-platform',
      },
      assertions,
      artifacts: {
        activityJournal: relative(process.cwd(), join(runRoot, 'activity.ndjson')),
        activityStore: relative(process.cwd(), join(runRoot, 'activity.sqlite')),
        devLogDirectory: relative(process.cwd(), join(runRoot, 'logs')),
      },
    };
    const evidencePath = join(
      evidenceDirectory,
      `${evidence.generatedAt.replaceAll(':', '-').replaceAll('.', '-')}.json`
    );
    await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
    printSummary(evidencePath, assertions);
  } finally {
    await stopProcessTreeAndWait(service);
  }
}

async function waitForStartupActivityCaptureIdle(runRoot) {
  if (process.platform !== 'win32') {
    return;
  }
  const journalPath = join(runRoot, 'activity.ndjson');
  const deadline = Date.now() + timeoutMs;
  let lastSize = -1;
  let stableSamples = 0;
  while (Date.now() < deadline) {
    try {
      const journal = await stat(journalPath);
      if (journal.size > 0 && journal.size === lastSize) {
        stableSamples += 1;
        if (stableSamples >= 3) {
          return;
        }
      } else {
        stableSamples = 0;
        lastSize = journal.size;
      }
    } catch (error) {
      if (error.code !== 'ENOENT') {
        throw error;
      }
    }
    await delay(150);
  }
  throw new Error('Timed out waiting for startup activity capture journal to settle.');
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

async function assertProcessTerminateEvent(event, child) {
  if (event.event !== 'agent.enforcement.audit.reported') {
    throw new Error(
      `process-terminate-owned-process event: expected agent.enforcement.audit.reported, received ${JSON.stringify(event)}`
    );
  }
  assertEqual(event.payload.databaseReady, true, 'process-terminate-owned-process databaseReady');
  const action = JSON.parse(event.payload.enforcementAction);
  const result = JSON.parse(event.payload.enforcementResult);
  const audit = JSON.parse(event.payload.enforcementAuditEvent);
  assertEqual(action.adapterKind, 'process-control', 'process-terminate-owned-process adapterKind');
  assertEqual(action.mode, 'terminate-process', 'process-terminate-owned-process mode');
  assertEqual(action.target.targetType, 'process', 'process-terminate-owned-process targetType');
  if (process.platform === 'win32') {
    assertEqual(event.payload.enforcementStatus, 'actually-enforced', 'process-terminate-owned-process status');
    if (!['process-terminated', 'process-already-exited'].includes(event.payload.enforcementAdapterResultCode)) {
      throw new Error(
        `process-terminate-owned-process adapterResultCode: expected process termination, received ${event.payload.enforcementAdapterResultCode}`
      );
    }
    assertEqual(result.capability.capabilityState, 'supported', 'process-terminate-owned-process capabilityState');
    assertEqual(audit.auditEventKind, 'succeeded', 'process-terminate-owned-process auditEventKind');
    await waitForChildExit(child);
  } else {
    assertEqual(event.payload.enforcementStatus, 'unavailable', 'process-terminate-owned-process status');
    assertEqual(
      event.payload.enforcementAdapterResultCode,
      'unsupported-platform',
      'process-terminate-owned-process adapterResultCode'
    );
    assertEqual(result.capability.capabilityState, 'unavailable', 'process-terminate-owned-process capabilityState');
    assertEqual(audit.auditEventKind, 'unavailable', 'process-terminate-owned-process auditEventKind');
  }
  return {
    id: 'process-terminate-owned-process',
    targetType: action.target.targetType,
    adapterKind: action.adapterKind,
    mode: action.mode,
    status: event.payload.enforcementStatus,
    adapterResultCode: event.payload.enforcementAdapterResultCode,
    capabilityState: result.capability.capabilityState,
    auditEventKind: audit.auditEventKind,
    eventsStored: event.payload.eventsStored,
  };
}

function scenarios() {
  return [
    {
      id: 'app-block-process-control',
      targetType: 'app',
      targetValue: 'child-game.exe',
      expectedAdapterKind: 'process-control',
      expectedMode: 'block-process',
    },
    {
      id: 'domain-block-network-control',
      targetType: 'domain',
      targetValue: 'example.invalid',
      expectedAdapterKind: 'network-control',
      expectedMode: 'temporary-block',
    },
    {
      id: 'site-block-managed-browser-control',
      targetType: 'site',
      targetValue: 'https://example.invalid/watch',
      expectedAdapterKind: 'managed-browser-control',
      expectedMode: 'temporary-block',
    },
  ];
}

function processTerminateCommand(child) {
  const now = new Date();
  return {
    schemaVersion: 1,
    messageId: 'cmd-v08-hardening-process-terminate-owned-process',
    sentAt: now.toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command: 'agent.enforcement.execute',
    payload: {
      policyDecisionId: 'decision-v08-hardening-process-terminate-owned-process',
      policyVersion: 'policy-v08-hardening',
      policyAction: 'block',
      targetType: 'process',
      targetId: 'target-v08-hardening-process-terminate-owned-process',
      targetValue: basename(process.execPath),
      dryRun: false,
      reasonCodes: 'parent-explicit-block',
      ruleIds: 'rule-v08-hardening',
      evidenceReferenceIds: 'evidence-v08-hardening-process-terminate-owned-process',
      requestedAt: now.toISOString(),
      expiresAt: new Date(now.getTime() + 300_000).toISOString(),
      enforcementActionId: 'action-v08-hardening-process-terminate-owned-process',
      enforcementResultId: 'result-v08-hardening-process-terminate-owned-process',
      enforcementAuditEventId: 'audit-v08-hardening-process-terminate-owned-process',
      enforcementTimerEventId: 'timer-v08-hardening-process-terminate-owned-process',
      processId: child.pid,
    },
  };
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
      reject(new Error(`Timed out waiting for ${command.messageId}.`));
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
      reject(new Error(`WebSocket error while requesting ${command.messageId}.`));
    });
  });
}

function assertManualRequiredEvent(event, scenario) {
  assertEqual(event.event, 'agent.enforcement.audit.reported', `${scenario.id} event`);
  assertEqual(event.payload.enforcementStatus, 'unavailable', `${scenario.id} status`);
  assertEqual(event.payload.databaseReady, true, `${scenario.id} databaseReady`);
  const action = JSON.parse(event.payload.enforcementAction);
  const result = JSON.parse(event.payload.enforcementResult);
  const audit = JSON.parse(event.payload.enforcementAuditEvent);
  assertEqual(action.adapterKind, scenario.expectedAdapterKind, `${scenario.id} adapterKind`);
  assertEqual(action.mode, scenario.expectedMode, `${scenario.id} mode`);
  assertEqual(audit.auditEventKind, 'unavailable', `${scenario.id} auditEventKind`);

  if (process.platform === 'win32') {
    assertEqual(result.capability.capabilityState, 'manual-required', `${scenario.id} capabilityState`);
    assertEqual(result.unavailableStatus.unavailableReason, 'manual-required', `${scenario.id} unavailableReason`);
  } else {
    assertEqual(result.capability.capabilityState, 'unavailable', `${scenario.id} capabilityState`);
    assertEqual(result.unavailableStatus.unavailableReason, 'unsupported-platform', `${scenario.id} unavailableReason`);
  }

  return {
    id: scenario.id,
    targetType: action.target.targetType,
    adapterKind: action.adapterKind,
    mode: action.mode,
    status: event.payload.enforcementStatus,
    capabilityState: result.capability.capabilityState,
    unavailableReason: result.unavailableStatus.unavailableReason,
    auditEventKind: audit.auditEventKind,
    eventsStored: event.payload.eventsStored,
  };
}

function commandEnvelope(scenario) {
  const now = new Date();
  return {
    schemaVersion: 1,
    messageId: `cmd-v08-hardening-${scenario.id}`,
    sentAt: now.toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command: 'agent.enforcement.execute',
    payload: {
      policyDecisionId: `decision-v08-hardening-${scenario.id}`,
      policyVersion: 'policy-v08-hardening',
      policyAction: 'block',
      targetType: scenario.targetType,
      targetId: `target-v08-hardening-${scenario.id}`,
      targetValue: scenario.targetValue,
      dryRun: false,
      reasonCodes: 'parent-explicit-block',
      ruleIds: 'rule-v08-hardening',
      evidenceReferenceIds: `evidence-v08-hardening-${scenario.id}`,
      requestedAt: now.toISOString(),
      expiresAt: new Date(now.getTime() + 300_000).toISOString(),
      enforcementActionId: `action-v08-hardening-${scenario.id}`,
      enforcementResultId: `result-v08-hardening-${scenario.id}`,
      enforcementAuditEventId: `audit-v08-hardening-${scenario.id}`,
      enforcementTimerEventId: `timer-v08-hardening-${scenario.id}`,
    },
  };
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
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

function runCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: process.cwd(), stdio: 'inherit', windowsHide: true });
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

function printSummary(evidencePath, assertions) {
  console.log('v0-8-production-enforcement-hardening-ok=true');
  console.log(`evidence=${evidencePath}`);
  console.log(
    assertions.map((assertion) => `${assertion.id}:${assertion.adapterKind}/${assertion.capabilityState}`).join(' ')
  );
}
