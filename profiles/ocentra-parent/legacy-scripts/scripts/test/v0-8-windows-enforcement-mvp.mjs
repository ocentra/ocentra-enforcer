import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { basename, join, relative } from 'node:path';
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const evidenceDirectory = join(process.cwd(), 'test-results', 'v0-8-windows-enforcement-mvp');
const timeoutMs = envNumber('OCENTRA_PARENT_V08_ENFORCEMENT_TIMEOUT_MS', 20_000);

await main();

async function main() {
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  await mkdir(evidenceDirectory, { recursive: true });
  const runRoot = await mkdtemp(join(evidenceDirectory, 'run-'));
  const agentPort = await freePort();
  const child = spawnOwnedChildProcess();
  const service = spawnAgentService(runRoot, agentPort);
  const serviceOutput = collectOutput(service);

  try {
    await waitForHealth(agentPort, serviceOutput);
    const event = await requestEnforcement(agentPort, child);
    const assertion = await assertEnforcementResult(event, child);
    const journalText = await readFile(join(runRoot, 'activity.ndjson'), 'utf8');
    if (journalText.includes(assertion.policyDecisionId) || journalText.includes(assertion.expectedProcessName)) {
      throw new Error('Encrypted journal contains plaintext enforcement identifiers.');
    }
    const evidence = {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      platform: process.platform,
      agentEndpoint: 'loopback-redacted',
      runRoot: relative(process.cwd(), runRoot),
      childProcess: {
        pid: child.pid ?? null,
        expectedProcessName: assertion.expectedProcessName,
      },
      assertion,
      event: {
        event: event.event,
        severity: event.severity,
        payload: event.payload,
      },
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
    printSummary(evidencePath, assertion);
  } finally {
    await stopProcessTreeAndWait(service);
    await stopProcessTreeAndWait(child);
  }
}

function spawnOwnedChildProcess() {
  return spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000);'], {
    cwd: process.cwd(),
    stdio: 'ignore',
    windowsHide: true,
  });
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

function requestEnforcement(agentPort, child) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(`ws://127.0.0.1:${agentPort}/api/dev/ws`);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error('Timed out waiting for enforcement audit event.'));
    }, timeoutMs);

    socket.addEventListener('open', () => {
      socket.send(JSON.stringify(commandEnvelope(child)));
    });
    socket.addEventListener('message', (message) => {
      const event = JSON.parse(String(message.data));
      if (event.event === 'agent.enforcement.audit.reported' || event.event === 'agent.command.rejected') {
        clearTimeout(timer);
        socket.close();
        resolve(event);
      }
    });
    socket.addEventListener('error', () => {
      clearTimeout(timer);
      reject(new Error('WebSocket error while requesting enforcement.'));
    });
  });
}

async function assertEnforcementResult(event, child) {
  const expectedProcessName = basename(process.execPath);
  const policyDecisionId = 'decision-v08-owned-process';
  if (process.platform === 'win32') {
    if (event.event !== 'agent.enforcement.audit.reported') {
      throw new Error(`Expected enforcement audit event, received ${event.event}`);
    }
    if (event.payload.enforcementStatus !== 'actually-enforced') {
      throw new Error(`Expected actually-enforced, received ${event.payload.enforcementStatus}`);
    }
    if (event.payload.enforcementAdapterResultCode !== 'process-terminated') {
      throw new Error(`Expected process-terminated, received ${event.payload.enforcementAdapterResultCode}`);
    }
    await waitForExit(child);
  } else if (event.payload.enforcementStatus !== 'unavailable') {
    throw new Error(`Expected unavailable on non-Windows, received ${event.payload.enforcementStatus}`);
  }

  if (event.payload.databaseReady !== true || Number(event.payload.eventsStored) < 1) {
    throw new Error(`Expected journal/store proof, payload=${JSON.stringify(event.payload)}`);
  }

  return {
    servicePathProven: true,
    policyDecisionId,
    expectedProcessName,
    status: event.payload.enforcementStatus,
    adapterResultCode: event.payload.enforcementAdapterResultCode,
    rollbackState: event.payload.enforcementRollbackState,
    journalEventId: event.payload.enforcementJournalEventId,
    timerEventKind: event.payload.enforcementTimerEventKind ?? null,
    databaseReady: event.payload.databaseReady,
    eventsStored: event.payload.eventsStored,
  };
}

function commandEnvelope(child) {
  const now = new Date();
  const expiresAt = new Date(now.getTime() + 300_000);
  return {
    schemaVersion: 1,
    messageId: 'cmd-v08-enforcement-owned-process',
    sentAt: now.toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command: 'agent.enforcement.execute',
    payload: {
      policyDecisionId: 'decision-v08-owned-process',
      policyVersion: 'policy-v08',
      policyAction: 'block',
      targetType: 'process',
      targetId: 'target-v08-owned-process',
      targetValue: basename(process.execPath),
      dryRun: false,
      reasonCodes: 'parent-explicit-block',
      ruleIds: 'rule-v08-owned-process',
      evidenceReferenceIds: 'evidence-v08-owned-process',
      requestedAt: now.toISOString(),
      expiresAt: expiresAt.toISOString(),
      enforcementActionId: 'action-v08-owned-process',
      enforcementResultId: 'result-v08-owned-process',
      enforcementAuditEventId: 'audit-v08-owned-process',
      enforcementTimerEventId: 'timer-v08-owned-process',
      processId: child.pid,
    },
  };
}

function waitForExit(child) {
  if (child.exitCode !== undefined || child.signalCode !== undefined) {
    return Promise.resolve();
  }
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Owned child process was not terminated.')), timeoutMs);
    child.once('exit', () => {
      clearTimeout(timer);
      resolve();
    });
  });
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
      reject(new Error(`${command} exited with ${code}`));
    });
    child.once('error', reject);
  });
}

function printSummary(evidencePath, assertion) {
  console.log('v0-8-windows-enforcement-mvp-ok=true');
  console.log(`evidence=${evidencePath}`);
  console.log(
    `status=${assertion.status} adapter=${assertion.adapterResultCode} rollback=${assertion.rollbackState} journal=${assertion.journalEventId}`
  );
}
