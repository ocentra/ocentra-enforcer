import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

import { stopProcessTreeAndWait } from './agent-service-process.mjs';

const proofUrl =
  process.env.OCENTRA_PARENT_MANAGED_BROWSER_SERVICE_PROOF_URL ?? 'https://example.com/?ocentra_service_proof=1';
const expectedHost = new URL(proofUrl).hostname;
const evidenceDirectory = join(process.cwd(), 'test-results', 'managed-browser-service-proof');
const timeoutMs = envNumber('OCENTRA_PARENT_MANAGED_BROWSER_SERVICE_PROOF_TIMEOUT_MS', 20_000);
const commandTimeoutMs = envNumber('OCENTRA_PARENT_MANAGED_BROWSER_SERVICE_PROOF_COMMAND_TIMEOUT_MS', 15_000);

await main();

async function main() {
  const browser = await firstInstalledChromiumBrowser();
  if (browser === null) {
    throw new Error('No installed Chrome or Edge executable found for managed browser service proof.');
  }

  await mkdir(evidenceDirectory, { recursive: true });
  const runRoot = await mkdtemp(join(tmpdir(), 'ocentra-managed-browser-service-proof-'));
  const profileDir = join(runRoot, 'managed-browser-profile-dev');
  await mkdir(profileDir, { recursive: true });

  const bridgePort = await freePort();
  const agentPort = await freePort();
  const browserProcess = spawnManagedBrowser(browser.executablePath, profileDir, bridgePort);
  let serviceProcess = null;

  try {
    const externalTargets = await observedTargetsFromBridge(bridgePort);
    serviceProcess = spawnAgentService(runRoot, agentPort, bridgePort);
    await waitForHealth(agentPort, serviceProcess);
    const proofEvents = await requestBrowserEvidence(agentPort);
    const assertion = assertServiceProof(proofEvents);
    const evidence = {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      browser,
      proofUrl,
      expectedHost,
      bridgeEndpoint: 'loopback-redacted',
      agentEndpoint: 'loopback-redacted',
      externalTargets,
      serviceEvents: proofEvents,
      assertion,
    };
    const evidencePath = join(
      evidenceDirectory,
      `${evidence.generatedAt.replaceAll(':', '-').replaceAll('.', '-')}.json`
    );
    await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
    printSummary(evidencePath, assertion);
  } finally {
    if (serviceProcess !== undefined) {
      await stopProcessTreeAndWait(serviceProcess);
    }
    await stopProcessTreeAndWait(browserProcess);
    await rm(runRoot, { recursive: true, force: true });
  }
}

async function firstInstalledChromiumBrowser() {
  const candidates = browserCandidates();
  for (const candidate of candidates) {
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
    {
      id: 'edge-stable',
      family: 'edge',
      executablePath: join(root, 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
    },
    {
      id: 'chrome-stable',
      family: 'chrome',
      executablePath: join(root, 'Google', 'Chrome', 'Application', 'chrome.exe'),
    },
  ]);
}

function windowsRoots() {
  return [process.env.ProgramFiles, process.env['ProgramFiles(x86)'], process.env.LOCALAPPDATA].filter(Boolean);
}

async function fileExists(pathValue) {
  try {
    const { stat } = await import('node:fs/promises');
    return (await stat(pathValue)).isFile();
  } catch {
    return false;
  }
}

function spawnManagedBrowser(executablePath, profileDir, bridgePort) {
  return spawn(
    executablePath,
    [
      '--remote-debugging-address=127.0.0.1',
      `--remote-debugging-port=${bridgePort}`,
      `--user-data-dir=${profileDir}`,
      '--profile-directory=OcentraManagedChild',
      '--no-first-run',
      '--no-default-browser-check',
      '--new-window',
      proofUrl,
    ],
    { stdio: 'ignore', windowsHide: true }
  );
}

function spawnAgentService(runRoot, agentPort, bridgePort) {
  const binaryName = process.platform === 'win32' ? 'ocentra-parent-agent-service.exe' : 'ocentra-parent-agent-service';
  return spawn(join(process.cwd(), 'target', 'debug', binaryName), [], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      OCENTRA_PARENT_AGENT_ADDR: `127.0.0.1:${agentPort}`,
      OCENTRA_PARENT_ACTIVITY_DB_PATH: join(runRoot, 'activity.sqlite'),
      OCENTRA_PARENT_ACTIVITY_JOURNAL_PATH: join(runRoot, 'activity.ndjson.enc'),
      OCENTRA_PARENT_ACTIVITY_JOURNAL_KEY_PATH: join(runRoot, 'activity.key'),
      OCENTRA_PARENT_DEV_LOG_DIR: join(runRoot, 'logs'),
      OCENTRA_PARENT_MANAGED_BROWSER_BRIDGE_PORT: String(bridgePort),
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

async function observedTargetsFromBridge(port) {
  const version = await waitForJson(port, '/json/version');
  const targets = await waitForJson(port, '/json/list');
  return {
    browserVersion: String(version.Browser ?? ''),
    pageTargets: targets
      .filter((target) => target.type === 'page')
      .map((target) => ({
        url: String(target.url ?? ''),
        title: String(target.title ?? ''),
        type: String(target.type ?? ''),
      })),
  };
}

async function waitForHealth(port, child) {
  const output = collectOutput(child);
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/health`);
      if (response.ok) {
        return;
      }
    } catch {
      await delay(250);
    }
  }
  throw new Error(`Timed out waiting for service health. ${output()}`);
}

function requestBrowserEvidence(agentPort) {
  return new Promise((resolve, reject) => {
    const events = [];
    const socket = new WebSocket(`ws://127.0.0.1:${agentPort}/api/dev/ws`);
    const timer = setTimeout(() => {
      socket.close();
      reject(new Error(`Timed out waiting for browser evidence. events=${JSON.stringify(events)}`));
    }, commandTimeoutMs);

    socket.addEventListener('open', () => {
      socket.send(JSON.stringify(commandEnvelope('cmd-managed-browser-poll', 'agent.browser.managed.bridge.poll')));
    });
    socket.addEventListener('message', (message) => {
      const event = JSON.parse(String(message.data));
      events.push(event);
      if (event.event === 'agent.browser.managed.status.reported') {
        socket.send(JSON.stringify(commandEnvelope('cmd-managed-browser-recent', 'agent.browser.evidence.recent.get')));
      }
      if (event.event === 'agent.browser.evidence.recent.reported') {
        clearTimeout(timer);
        socket.close();
        resolve(events.filter((item) => item.event.includes('browser')));
      }
    });
    socket.addEventListener('error', () => {
      clearTimeout(timer);
      reject(new Error('WebSocket error while requesting browser evidence.'));
    });
  });
}

function commandEnvelope(messageId, command) {
  return {
    schemaVersion: 1,
    messageId,
    sentAt: new Date().toISOString(),
    source: { peerId: 'portal-dev', role: 'portal' },
    target: { deviceId: 'local-dev-agent', platform: 'windows', route: 'localhost' },
    command,
    payload: {},
  };
}

function assertServiceProof(events) {
  const status = events.find((event) => event.event === 'agent.browser.managed.status.reported')?.payload;
  const evidence = events.find((event) => event.event === 'agent.browser.evidence.recent.reported')?.payload;
  const failures = [];
  if (status?.managedState !== 'bridge-connected') {
    failures.push(`managedState=${status?.managedState ?? 'missing'}`);
  }
  if (status?.capabilityStatus !== 'tab-list-only') {
    failures.push(`statusCapability=${status?.capabilityStatus ?? 'missing'}`);
  }
  if (evidence?.returned !== 1) {
    failures.push(`returned=${evidence?.returned ?? 'missing'}`);
  }
  if (evidence?.url !== proofUrl) {
    failures.push(`url=${evidence?.url ?? 'missing'}`);
  }
  if (evidence?.domain !== expectedHost) {
    failures.push(`domain=${evidence?.domain ?? 'missing'}`);
  }
  if (typeof evidence?.title !== 'string' || evidence.title.length === 0) {
    failures.push('title=missing');
  }
  if (evidence?.managedBrowserSessionId !== 'managed-browser-session-dev') {
    failures.push(`managedBrowserSessionId=${evidence?.managedBrowserSessionId ?? 'missing'}`);
  }
  if (failures.length > 0) {
    throw new Error(`Managed browser service proof failed: ${failures.join(', ')}`);
  }
  return {
    servicePathProven: true,
    managedState: status.managedState,
    capabilityStatus: status.capabilityStatus,
    url: evidence.url,
    domain: evidence.domain,
    title: evidence.title,
    activeState: evidence.activeState,
    queryVisibility: evidence.queryVisibility,
  };
}

function printSummary(evidencePath, assertion) {
  console.log('managed-browser-service-proof-ok=true');
  console.log(`evidence=${evidencePath}`);
  console.log(
    `url=${assertion.url} title=${assertion.title} domain=${assertion.domain} activeState=${assertion.activeState} capability=${assertion.capabilityStatus}`
  );
}

async function waitForJson(port, path) {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}${path}`);
      if (response.ok) {
        return await response.json();
      }
    } catch (error) {
      lastError = error;
    }
    await delay(250);
  }
  throw lastError ?? new Error(`Timed out waiting for ${path}`);
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
