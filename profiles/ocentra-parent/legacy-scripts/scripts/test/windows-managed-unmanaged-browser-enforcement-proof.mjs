import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { basename, join, relative } from 'node:path';
import { mkdir, mkdtemp, readFile, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { setTimeout as delay } from 'node:timers/promises';

import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const repoRoot = process.cwd();
const evidenceDirectory = join(repoRoot, 'test-results', 'windows-managed-unmanaged-browser-enforcement-proof');
const timeoutMs = envNumber('OCENTRA_PARENT_BROWSER_ENFORCEMENT_TIMEOUT_MS', 30_000);

await main();

async function main() {
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  await mkdir(evidenceDirectory, { recursive: true });
  const runRoot = await mkdtemp(join(evidenceDirectory, 'run-'));
  const agentPort = await freePort();
  const service = spawnAgentService(runRoot, agentPort);
  const serviceOutput = collectOutput(service);
  const ownedProcessProbe = await launchOwnedProcessProbe();
  const launchedBrowser = await launchUnmanagedBrowser(runRoot);
  const assertions = [];

  try {
    await waitForHealth(agentPort, serviceOutput);
    await waitForStartupCapture(runRoot);
    assertions.push(await assertMissingProcessIdRejected(agentPort));
    assertions.push(await assertOwnedProcessMismatchRejected(agentPort, ownedProcessProbe));
    assertions.push(await assertOwnedProcessTerminate(agentPort, ownedProcessProbe));
    if (launchedBrowser === null) {
      assertions.push({
        id: 'unmanaged-browser-terminate',
        state: 'manual-required',
        reason: 'no-supported-browser-executable-found',
        exactUrlClaimState: 'not-claimed',
      });
    } else {
      assertions.push(await assertUnmanagedTerminate(agentPort, launchedBrowser));
    }
    assertions.push(await assertUnmanagedWarn(agentPort));
    assertions.push(assertUnmanagedReportOnlyState());
    assertions.push(assertUnmanagedParentReviewState());
    assertions.push(assertUnmanagedRelaunchManagedManualRequired());
    assertions.push(assertUnmanagedDegradedState());
    assertions.push(assertUnmanagedUnavailableState(launchedBrowser));
    assertions.push(await assertBroadAppBlockingManualRequired(agentPort));
    assertions.push(await assertManagedBrowserManualRequired(agentPort));
    const evidence = await writeEvidence(runRoot, assertions);
    printSummary(evidence);
    if (assertions.some((assertion) => assertion.state === 'failed')) {
      process.exitCode = 1;
    }
  } finally {
    if (launchedBrowser !== undefined) {
      await stopProcessTreeAndWait(launchedBrowser.child);
    }
    await stopProcessTreeAndWait(ownedProcessProbe.child);
    await stopProcessTreeAndWait(service);
  }
}

async function assertMissingProcessIdRejected(agentPort) {
  const event = await requestEvent(
    agentPort,
    enforcementCommand({
      id: 'owned-process-id-required',
      policyAction: 'block',
      targetType: 'process',
      targetValue: basename(process.execPath),
      processId: null,
    })
  );
  assertEqual(event.event, 'agent.command.rejected', 'missing process id event');
  assertEqual(event.payload.reason, 'enforcement-process-id-required', 'missing process id reason');
  return {
    id: 'owned-process-id-required',
    state: 'rejected',
    adapterRuntimeState: 'process-id-required',
    exactUrlClaimState: 'not-claimed',
    status: 'rejected',
    reason: event.payload.reason,
  };
}

async function assertOwnedProcessMismatchRejected(agentPort, ownedProcessProbe) {
  await assertProcessExists(ownedProcessProbe.child.pid, true, 'owned process mismatch precondition');
  const mismatchedTarget = `not-${ownedProcessProbe.processName}`;
  const event = await requestEvent(
    agentPort,
    enforcementCommand({
      id: 'owned-process-name-mismatch',
      policyAction: 'block',
      targetType: 'process',
      targetValue: mismatchedTarget,
      processId: ownedProcessProbe.child.pid,
    })
  );
  const action = JSON.parse(event.payload.enforcementAction);
  const result = JSON.parse(event.payload.enforcementResult);
  const audit = JSON.parse(event.payload.enforcementAuditEvent);
  assertEqual(event.event, 'agent.enforcement.audit.reported', 'owned process mismatch event');
  assertEqual(action.target.targetType, 'process', 'owned process mismatch target type');
  assertEqual(action.mode, 'terminate-process', 'owned process mismatch mode');
  assertEqual(result.status, 'failed', 'owned process mismatch status');
  assertEqual(result.adapterResultCode, 'adapter-failed', 'owned process mismatch adapter result');
  assertEqual(result.failedReason, 'policy-target-mismatch', 'owned process mismatch failed reason');
  assertEqual(result.rollbackState, 'failed', 'owned process mismatch rollback state');
  assertEqual(audit.auditEventKind, 'failed', 'owned process mismatch audit');
  await assertProcessExists(ownedProcessProbe.child.pid, true, 'owned process mismatch leaves process running');
  return {
    id: 'owned-process-name-mismatch',
    state: 'rejected-without-termination',
    adapterRuntimeState: 'pid-name-match-required',
    exactUrlClaimState: 'not-claimed',
    processName: ownedProcessProbe.processName,
    mismatchedTarget,
    status: result.status,
    adapterResultCode: result.adapterResultCode,
    failedReason: result.failedReason,
    rollbackState: result.rollbackState,
    auditEventKind: audit.auditEventKind,
  };
}

async function assertOwnedProcessTerminate(agentPort, ownedProcessProbe) {
  const event = await requestEvent(
    agentPort,
    enforcementCommand({
      id: 'owned-process-runtime-terminate',
      policyAction: 'block',
      targetType: 'process',
      targetValue: ownedProcessProbe.processName,
      processId: ownedProcessProbe.child.pid,
    })
  );
  const action = JSON.parse(event.payload.enforcementAction);
  const result = JSON.parse(event.payload.enforcementResult);
  const audit = JSON.parse(event.payload.enforcementAuditEvent);
  assertEqual(event.event, 'agent.enforcement.audit.reported', 'owned process terminate event');
  assertEqual(action.target.targetType, 'process', 'owned process terminate target type');
  assertEqual(action.mode, 'terminate-process', 'owned process terminate mode');
  assertEqual(result.status, 'actually-enforced', 'owned process terminate status');
  assertOneOf(
    result.adapterResultCode,
    ['process-terminated', 'process-already-exited'],
    'owned process terminate adapter result'
  );
  assertEqual(audit.auditEventKind, 'succeeded', 'owned process terminate audit');
  if (result.adapterResultCode === 'process-terminated') {
    await waitForChildExit(ownedProcessProbe.child, 5_000);
  }
  return {
    id: 'owned-process-runtime-terminate',
    state: result.adapterResultCode === 'process-terminated' ? 'terminated' : 'already-exited',
    adapterRuntimeState: 'pid-name-match-enforced',
    exactUrlClaimState: 'not-claimed',
    processName: ownedProcessProbe.processName,
    status: result.status,
    adapterResultCode: result.adapterResultCode,
    auditEventKind: audit.auditEventKind,
  };
}

async function assertUnmanagedTerminate(agentPort, launchedBrowser) {
  const event = await requestEvent(
    agentPort,
    enforcementCommand({
      id: 'unmanaged-browser-terminate',
      policyAction: 'block',
      targetType: 'process',
      targetValue: basename(launchedBrowser.executablePath),
      processId: launchedBrowser.child.pid,
    })
  );
  const action = JSON.parse(event.payload.enforcementAction);
  const result = JSON.parse(event.payload.enforcementResult);
  const audit = JSON.parse(event.payload.enforcementAuditEvent);
  assertEqual(event.event, 'agent.enforcement.audit.reported', 'unmanaged terminate event');
  assertEqual(action.target.targetType, 'process', 'unmanaged terminate target type');
  assertEqual(action.mode, 'terminate-process', 'unmanaged terminate mode');
  assertEqual(result.status, 'actually-enforced', 'unmanaged terminate status');
  assertOneOf(
    result.adapterResultCode,
    ['process-terminated', 'process-already-exited'],
    'unmanaged terminate adapter result'
  );
  assertEqual(audit.auditEventKind, 'succeeded', 'unmanaged terminate audit');
  assertEqual(action.target.targetValue.includes('://'), false, 'unmanaged terminate target is not URL');
  return {
    id: 'unmanaged-browser-terminate',
    state: 'terminated',
    browserBoundaryState: 'unmanaged-browser-process',
    exactUrlClaimState: 'not-claimed',
    unmanagedDetectionState: 'terminated',
    processName: basename(launchedBrowser.executablePath),
    status: result.status,
    adapterResultCode: result.adapterResultCode,
    auditEventKind: audit.auditEventKind,
  };
}

async function assertUnmanagedWarn(agentPort) {
  const event = await requestEvent(
    agentPort,
    enforcementCommand({
      id: 'unmanaged-browser-warn',
      policyAction: 'warn',
      targetType: 'process',
      targetValue: 'browser-like-process',
      processId: null,
    })
  );
  const action = JSON.parse(event.payload.enforcementAction);
  const result = JSON.parse(event.payload.enforcementResult);
  const audit = JSON.parse(event.payload.enforcementAuditEvent);
  assertEqual(action.mode, 'observe-only', 'unmanaged warn mode');
  assertEqual(result.status, 'no-op', 'unmanaged warn status');
  assertEqual(result.adapterResultCode, 'no-op', 'unmanaged warn adapter result');
  assertEqual(audit.auditEventKind, 'attempted', 'unmanaged warn audit');
  return {
    id: 'unmanaged-browser-warn',
    state: 'warned',
    browserBoundaryState: 'unmanaged-browser-process',
    exactUrlClaimState: 'not-claimed',
    unmanagedDetectionState: 'warned',
    status: result.status,
    adapterResultCode: result.adapterResultCode,
    auditEventKind: audit.auditEventKind,
  };
}

function assertUnmanagedReportOnlyState() {
  return {
    id: 'unmanaged-browser-report-only',
    state: 'report-only',
    browserBoundaryState: 'unmanaged-browser-process',
    exactUrlClaimState: 'not-claimed',
    activeTabClaimState: 'not-claimed',
    titleClaimState: 'not-claimed',
    contentClaimState: 'not-claimed',
    unmanagedDetectionState: 'report-only',
    fallbackState: 'report-only',
    status: 'monitor-only',
    processIdentityRequired: false,
    boundary:
      'Report-only unmanaged browser fallback records process suspicion without warning delivery, termination, relaunch, or exact content claims.',
  };
}

function assertUnmanagedParentReviewState() {
  return {
    id: 'unmanaged-browser-parent-review',
    state: 'parent-review',
    browserBoundaryState: 'unmanaged-browser-process',
    exactUrlClaimState: 'not-claimed',
    activeTabClaimState: 'not-claimed',
    titleClaimState: 'not-claimed',
    contentClaimState: 'not-claimed',
    unmanagedDetectionState: 'parent-review',
    fallbackState: 'parent-review',
    status: 'manual-required',
    processIdentityRequired: false,
    boundary:
      'Parent-review unmanaged browser fallback stays manual and does not claim a browser block, warning delivery, relaunch, or exact URL result.',
  };
}

function assertUnmanagedRelaunchManagedManualRequired() {
  return {
    id: 'unmanaged-browser-relaunch-managed-manual-required',
    state: 'manual-required',
    browserBoundaryState: 'unmanaged-browser-process',
    exactUrlClaimState: 'not-claimed',
    activeTabClaimState: 'not-claimed',
    titleClaimState: 'not-claimed',
    contentClaimState: 'not-claimed',
    unmanagedDetectionState: 'relaunch-managed-browser',
    fallbackState: 'relaunch-managed-browser',
    status: 'manual-required',
    processIdentityRequired: true,
    processIdentityState: 'pid-name-required',
    boundary:
      'Relaunch-managed fallback requires managed launch and custody proof before it can execute beyond a manual-required state.',
  };
}

function assertUnmanagedDegradedState() {
  return {
    id: 'unmanaged-browser-degraded',
    state: 'degraded',
    browserBoundaryState: 'unmanaged-browser-process',
    exactUrlClaimState: 'not-claimed',
    activeTabClaimState: 'not-claimed',
    titleClaimState: 'not-claimed',
    contentClaimState: 'not-claimed',
    unmanagedDetectionState: 'degraded',
    fallbackState: 'degraded',
    status: 'no-op',
    processIdentityRequired: false,
    reason: 'notification-and-browser-integration-not-proved',
    boundary:
      'Degraded unmanaged browser fallback keeps warn/report behavior visible as no-op proof until delivery and browser integration are proved.',
  };
}

function assertUnmanagedUnavailableState(launchedBrowser) {
  return {
    id: 'unmanaged-browser-unavailable',
    state: 'unavailable',
    browserBoundaryState: 'unmanaged-browser-process',
    exactUrlClaimState: 'not-claimed',
    activeTabClaimState: 'not-claimed',
    titleClaimState: 'not-claimed',
    contentClaimState: 'not-claimed',
    unmanagedDetectionState: 'unavailable',
    fallbackState: 'unavailable',
    status: launchedBrowser === null ? 'unavailable' : 'manual-required',
    processIdentityRequired: false,
    reason: launchedBrowser === null ? 'no-supported-browser-executable-found' : 'exact-browser-content-not-available',
    boundary:
      'Unavailable unmanaged browser fallback is separate from report-only, warn, review, terminate, relaunch, manual-required, and degraded states.',
  };
}

async function assertBroadAppBlockingManualRequired(agentPort) {
  const event = await requestEvent(
    agentPort,
    enforcementCommand({
      id: 'broad-app-blocking-manual-required',
      policyAction: 'block',
      targetType: 'app',
      targetValue: 'browser-like-app',
      processId: null,
    })
  );
  const action = JSON.parse(event.payload.enforcementAction);
  const result = JSON.parse(event.payload.enforcementResult);
  const audit = JSON.parse(event.payload.enforcementAuditEvent);
  assertEqual(action.adapterKind, 'process-control', 'broad app block adapter kind');
  assertEqual(action.mode, 'block-process', 'broad app block mode');
  assertEqual(result.status, 'unavailable', 'broad app block status');
  assertEqual(result.unavailableStatus.unavailableReason, 'manual-required', 'broad app block unavailable reason');
  assertEqual(audit.auditEventKind, 'unavailable', 'broad app block audit');
  return {
    id: 'broad-app-blocking-manual-required',
    state: 'manual-required',
    adapterRuntimeState: 'broad-app-blocking-not-claimed',
    exactUrlClaimState: 'not-claimed',
    status: result.status,
    adapterResultCode: result.adapterResultCode,
    unavailableReason: result.unavailableStatus.unavailableReason,
    auditEventKind: audit.auditEventKind,
  };
}

async function assertManagedBrowserManualRequired(agentPort) {
  const event = await requestEvent(
    agentPort,
    enforcementCommand({
      id: 'managed-browser-manual-required',
      policyAction: 'block',
      targetType: 'site',
      targetValue: 'https://example.invalid/watch',
      processId: null,
    })
  );
  const action = JSON.parse(event.payload.enforcementAction);
  const result = JSON.parse(event.payload.enforcementResult);
  const audit = JSON.parse(event.payload.enforcementAuditEvent);
  assertEqual(action.adapterKind, 'managed-browser-control', 'managed browser adapter kind');
  assertEqual(action.mode, 'temporary-block', 'managed browser mode');
  assertEqual(result.status, 'unavailable', 'managed browser status');
  assertEqual(result.unavailableStatus.unavailableReason, 'manual-required', 'managed browser unavailable reason');
  assertEqual(audit.auditEventKind, 'unavailable', 'managed browser audit');
  return {
    id: 'managed-browser-manual-required',
    state: 'manual-required',
    browserBoundaryState: 'managed-session',
    exactUrlClaimState: 'not-claimed-service-command-manual-required',
    unmanagedDetectionState: 'none',
    status: result.status,
    adapterResultCode: result.adapterResultCode,
    unavailableReason: result.unavailableStatus.unavailableReason,
    auditEventKind: audit.auditEventKind,
  };
}

function enforcementCommand({ id, policyAction, targetType, targetValue, processId }) {
  const now = new Date();
  const payload = {
    policyDecisionId: `decision-browser-boundary-${id}`,
    policyVersion: 'policy-browser-boundary',
    policyAction,
    targetType,
    targetId: `target-browser-boundary-${id}`,
    targetValue,
    dryRun: false,
    reasonCodes: 'parent-explicit-block',
    ruleIds: 'rule-browser-boundary',
    evidenceReferenceIds: `evidence-browser-boundary-${id}`,
    requestedAt: now.toISOString(),
    expiresAt: new Date(now.getTime() + 300_000).toISOString(),
    enforcementActionId: `action-browser-boundary-${id}`,
    enforcementResultId: `result-browser-boundary-${id}`,
    enforcementAuditEventId: `audit-browser-boundary-${id}`,
    enforcementTimerEventId: `timer-browser-boundary-${id}`,
  };
  if (processId !== undefined && processId !== undefined) {
    payload.processId = processId;
  }
  return {
    schemaVersion: 1,
    messageId: `cmd-browser-boundary-${id}`,
    sentAt: now.toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command: 'agent.enforcement.execute',
    payload,
  };
}

async function launchOwnedProcessProbe() {
  const child = spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000);'], {
    stdio: 'ignore',
    windowsHide: true,
  });
  await delay(250);
  if (child.pid === undefined) {
    throw new Error('Owned process probe did not expose a process id.');
  }
  await assertProcessExists(child.pid, true, 'owned process probe started');
  return {
    executablePath: process.execPath,
    processName: basename(process.execPath),
    child,
  };
}

async function launchUnmanagedBrowser(runRoot) {
  const browser = await firstInstalledBrowser();
  if (browser === null) {
    return null;
  }
  const profile = join(runRoot, 'unmanaged-browser-profile');
  await mkdir(profile, { recursive: true });
  const child = spawn(
    browser.executablePath,
    [`--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--new-window', 'about:blank'],
    { stdio: 'ignore', windowsHide: true }
  );
  await delay(750);
  return { ...browser, child };
}

async function firstInstalledBrowser() {
  for (const candidate of browserCandidates()) {
    if (await fileExists(candidate.executablePath)) {
      return candidate;
    }
  }
  return null;
}

function browserCandidates() {
  if (process.platform !== 'win32') {
    return [];
  }
  return windowsRoots().flatMap((root) => [
    { id: 'edge-stable', executablePath: join(root, 'Microsoft', 'Edge', 'Application', 'msedge.exe') },
    { id: 'chrome-stable', executablePath: join(root, 'Google', 'Chrome', 'Application', 'chrome.exe') },
  ]);
}

function windowsRoots() {
  return [process.env.ProgramFiles, process.env['ProgramFiles(x86)'], process.env.LOCALAPPDATA].filter(Boolean);
}

function spawnAgentService(runRoot, agentPort) {
  return spawn(resolveDebugAgentServicePath(), [], {
    cwd: repoRoot,
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

async function waitForStartupCapture(runRoot) {
  if (process.platform !== 'win32') {
    return;
  }
  const journalPath = join(runRoot, 'activity.ndjson');
  const deadline = Date.now() + 5_000;
  let lastSize = -1;
  let stableTicks = 0;
  while (Date.now() < deadline) {
    const currentSize = await fileSize(journalPath);
    if (currentSize > 0 && currentSize === lastSize) {
      stableTicks += 1;
      if (stableTicks >= 4) {
        return;
      }
    } else {
      stableTicks = 0;
    }
    lastSize = currentSize;
    await delay(250);
  }
}

async function fileSize(pathValue) {
  try {
    return (await stat(pathValue)).size;
  } catch {
    return -1;
  }
}

function requestEvent(agentPort, command) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(`ws://127.0.0.1:${agentPort}/api/dev/ws`);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error(`Timed out waiting for ${command.messageId}.`));
    }, timeoutMs);
    socket.addEventListener('open', () => socket.send(JSON.stringify(command)));
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

async function writeEvidence(runRoot, assertions) {
  const generatedAt = new Date().toISOString();
  const evidence = {
    schemaVersion: 1,
    generatedAt,
    platform: process.platform,
    agentEndpoint: 'loopback-redacted',
    runRoot: relative(repoRoot, runRoot),
    states: {
      managedBrowserInterventionCapability: 'manual-required-with-live-managed-proof-script',
      windowsProcessAdapterRuntime: assertions.find((assertion) => assertion.id === 'owned-process-runtime-terminate')
        ?.state,
      windowsProcessAdapterGuard: assertions.find((assertion) => assertion.id === 'owned-process-name-mismatch')?.state,
      processIdRequiredRejection: assertions.find((assertion) => assertion.id === 'owned-process-id-required')?.state,
      broadAppBlockingCapability: assertions.find((assertion) => assertion.id === 'broad-app-blocking-manual-required')
        ?.state,
      managedBrowserServiceCommand: assertions.find((assertion) => assertion.id === 'managed-browser-manual-required')
        ?.state,
      unmanagedBrowserBoundary: assertions.find((assertion) => assertion.id === 'unmanaged-browser-terminate')?.state,
      exactUnmanagedUrlClaim: 'not-claimed',
      exactUnmanagedActiveTabClaim: 'not-claimed',
      exactUnmanagedTitleClaim: 'not-claimed',
      exactUnmanagedContentClaim: 'not-claimed',
      exactManagedBrowserServiceCommandUrlClaim: assertions.find(
        (assertion) => assertion.id === 'managed-browser-manual-required'
      )?.exactUrlClaimState,
      unmanagedFallbackStates: assertions
        .filter((assertion) => assertion.id.startsWith('unmanaged-browser-'))
        .map((assertion) => ({
          id: assertion.id,
          state: assertion.state,
          fallbackState: assertion.fallbackState ?? assertion.unmanagedDetectionState,
          exactUrlClaimState: assertion.exactUrlClaimState,
        })),
    },
    assertions,
    artifacts: {
      activityJournal: relative(repoRoot, join(runRoot, 'activity.ndjson')),
      activityStore: relative(repoRoot, join(runRoot, 'activity.sqlite')),
      devLogDirectory: relative(repoRoot, join(runRoot, 'logs')),
    },
  };
  const path = join(evidenceDirectory, `${generatedAt.replaceAll(':', '-').replaceAll('.', '-')}.json`);
  await writeFile(path, `${JSON.stringify(evidence, null, 2)}\n`);
  evidence.path = path;
  await assertNoPlaintextUrlInJournal(evidence.artifacts.activityJournal);
  return evidence;
}

async function assertNoPlaintextUrlInJournal(relativeJournalPath) {
  const path = join(repoRoot, relativeJournalPath);
  const text = await readFile(path, 'utf8').catch(() => '');
  if (text.includes('https://example.invalid/watch')) {
    throw new Error('Activity journal leaked plaintext managed browser URL from enforcement proof.');
  }
}

function printSummary(evidence) {
  console.log('windows-managed-unmanaged-browser-enforcement-proof-ok=true');
  console.log(`evidence=${evidence.path}`);
  for (const assertion of evidence.assertions) {
    console.log(`${assertion.id}:${assertion.state}:${assertion.exactUrlClaimState}`);
  }
}

async function fileExists(pathValue) {
  try {
    return (await stat(pathValue)).isFile();
  } catch {
    return false;
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

async function assertProcessExists(pid, expected, label) {
  await delay(100);
  assertEqual(processExists(pid), expected, label);
}

function processExists(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error?.code !== 'ESRCH';
  }
}

function waitForChildExit(child, deadlineMs) {
  if (child.exitCode !== undefined || child.signalCode !== undefined) {
    return Promise.resolve();
  }
  return new Promise((resolve, reject) => {
    const onExit = () => {
      clearTimeout(timer);
      resolve();
    };
    const timer = setTimeout(() => {
      child.off('exit', onExit);
      reject(new Error(`Timed out waiting for child process ${child.pid} to exit.`));
    }, deadlineMs);
    child.once('exit', onExit);
  });
}

function envNumber(name, fallback) {
  const parsed = Number(process.env[name]);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function runCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
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

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${expected}, received ${actual}`);
  }
}

function assertOneOf(actual, expected, label) {
  if (!expected.includes(actual)) {
    throw new Error(`${label}: expected one of ${expected.join(', ')}, received ${actual}`);
  }
}
