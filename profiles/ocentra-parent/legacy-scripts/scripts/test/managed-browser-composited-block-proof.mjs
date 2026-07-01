import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, stat, writeFile } from 'node:fs/promises';
import { createServer as createNetServer } from 'node:net';
import { join, relative } from 'node:path';
import { tmpdir } from 'node:os';
import { setTimeout as delay } from 'node:timers/promises';

import {
  BrowserChildInterventionPageDefaults,
  renderBrowserChildInterventionPage,
} from '@ocentra-parent/portal-domain/browser-child-intervention-page';

import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const repoRoot = process.cwd();
const runId = new Date().toISOString().replaceAll(':', '-').replaceAll('.', '-');
const evidenceDirectory = join(repoRoot, 'test-results', 'managed-browser-composited-block-proof');
const screenshotDirectory = join(evidenceDirectory, `${runId}-screenshots`);
const probeRoot = join(tmpdir(), `ocentra-parent-managed-browser-composited-block-${process.pid}`);
const timeoutMs = envNumber('OCENTRA_PARENT_MANAGED_BROWSER_COMPOSITED_BLOCK_TIMEOUT_MS', 45_000);
const commandTimeoutMs = envNumber('OCENTRA_PARENT_MANAGED_BROWSER_COMPOSITED_BLOCK_COMMAND_TIMEOUT_MS', 20_000);
const requestedUrl =
  process.env.OCENTRA_PARENT_MANAGED_BROWSER_COMPOSITED_BLOCK_URL ?? 'https://www.youtube.com/watch?v=XzUB8_gj6xM';

async function main() {
  const browser = await installedChromiumBrowser();
  if (browser === null) {
    throw new Error('No installed Chrome or Edge executable found for managed composited block proof.');
  }

  await mkdir(evidenceDirectory, { recursive: true });
  await mkdir(screenshotDirectory, { recursive: true });
  await mkdir(probeRoot, { recursive: true });
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);

  let profileRun;
  let service;
  try {
    const runRoot = await mkdtemp(join(tmpdir(), 'ocentra-parent-managed-browser-composited-agent-'));
    const htmlPath = join(runRoot, 'browser-intervention-page.html');
    const agentPort = await freePort();
    service = spawnAgentService(runRoot, agentPort, htmlPath);
    const serviceOutput = collectOutput(service);
    await waitForHealth(agentPort, serviceOutput);
    profileRun = await launchChromiumProfile(browser);
    const version = await waitForJson(profileRun.port, '/json/version');
    const target = await waitForFirstPageTarget(profileRun.port);
    const client = await DevToolsClient.connect(target.webSocketDebuggerUrl);
    try {
      await client.command('Page.enable', {});
      await client.command('Runtime.enable', {});
      await client.command('Page.navigate', { url: requestedUrl });
      await waitForLivePage(client);
      await delay(3000);
      const capturedLocation = await evaluateChromiumValue(client, 'location.href');
      const capturedScreenshot = await client.command('Page.captureScreenshot', { format: 'png' });
      const backdropDataUrl = `data:image/png;base64,${capturedScreenshot.data}`;
      const blockPageHtml = renderBrowserChildInterventionPage({
        ...browserChildCompositedBlockModel(requestedUrl),
        backdrop: {
          imageUrl: backdropDataUrl,
          label: 'Captured page before block',
        },
      });
      await writeFile(htmlPath, blockPageHtml, 'utf8');
      const blockedPageUrl = `http://127.0.0.1:${agentPort}/api/browser/intervention/page?target=${encodeURIComponent(
        requestedUrl
      )}`;
      await client.command('Page.navigate', { url: blockedPageUrl });
      await waitForChromiumReady(client);
      await delay(750);
      const observed = await client.command('Runtime.evaluate', {
        expression: `({
          href: location.href,
          title: document.title,
          markerPresent: document.body.textContent.includes('${BrowserChildInterventionPageDefaults.BlockMarker}'),
          targetTextPresent: document.body.textContent.includes(${JSON.stringify(requestedUrl)}),
          backdropPresent: Boolean(document.querySelector('.ocentra-child-site-backdrop img')),
        })`,
        returnByValue: true,
      });
      const screenshotPath = await captureChromiumScreenshot(browser, client, 'composited-block-youtube');
      const evidence = {
        schemaVersion: 1,
        generatedAt: new Date().toISOString(),
        browser,
        browserVersion: version.Browser,
        requestedUrl,
        capturedLocation,
        blockedPageUrl,
        childAgentEndpoint: '/api/browser/intervention/page',
        htmlPath: relative(repoRoot, htmlPath),
        observed: observed.result?.value,
        screenshotPath,
        assertions: assertionsForCompositedBlock(observed.result?.value, capturedLocation),
      };
      const evidencePath = join(evidenceDirectory, `${runId}.json`);
      await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
      printSummary(evidence, evidencePath);
      if (!Object.values(evidence.assertions).every(Boolean)) {
        process.exitCode = 1;
      }
    } finally {
      client.close();
    }
  } finally {
    if (service !== undefined) {
      await stopProcessTreeAndWait(service);
    }
    if (profileRun !== undefined) {
      await cleanupProfileRun(profileRun);
    }
    await stopWindowsProcessesByCommandLineFragment(probeRoot);
  }
}

function browserChildCompositedBlockModel(url) {
  return {
    action: 'block',
    blockMarker: BrowserChildInterventionPageDefaults.BlockMarker,
    bridge: 'local-composited-block-page',
    deliveryState: 'local-composited-block-rendered',
    outcome: 'blocked',
    parentRequestEnabled: true,
    reason: 'Your family rule blocks this exact video URL.',
    requestedUrl: url,
    ruleId: 'blocked-youtube-video-url',
    ruleLabel: 'Disallowed YouTube video URL',
    ruleMarker: BrowserChildInterventionPageDefaults.BlockMarker,
    targetType: 'video',
    theme: 'dark',
  };
}

function assertionsForCompositedBlock(observed, capturedLocation) {
  return {
    backdropRendered: observed?.backdropPresent === true,
    blockMarkerPresent: observed?.markerPresent === true,
    capturedTargetBeforeBlock: isSameWatchedTarget(capturedLocation, requestedUrl),
    childAgentEndpointRendered:
      typeof observed?.href === 'string' && observed.href.includes('/api/browser/intervention/page?target='),
    targetUrlShown: observed?.targetTextPresent === true,
  };
}

function isSameWatchedTarget(observedUrl, expectedUrl) {
  if (typeof observedUrl !== 'string') {
    return false;
  }
  try {
    const observed = new URL(observedUrl);
    const expected = new URL(expectedUrl);
    return (
      observed.hostname === expected.hostname &&
      observed.pathname === expected.pathname &&
      observed.searchParams.get('v') === expected.searchParams.get('v')
    );
  } catch {
    return observedUrl.startsWith(expectedUrl);
  }
}

function printSummary(evidence, evidencePath) {
  const ok = Object.values(evidence.assertions).every(Boolean);
  console.log(`managed-browser-composited-block-proof-ok=${ok}`);
  console.log(`evidence=${evidencePath}`);
  console.log(`requested=${evidence.requestedUrl}`);
  console.log(`captured=${evidence.capturedLocation}`);
  console.log(`observed=${evidence.observed?.href ?? 'unknown'}`);
  console.log(`screenshot=${evidence.screenshotPath}`);
  for (const [name, passed] of Object.entries(evidence.assertions)) {
    console.log(`assertion.${name}=${passed}`);
  }
}

async function launchChromiumProfile(browser) {
  const port = await freePort();
  const profileDir = join(probeRoot, browser.id, 'managed-browser-composited-block-proof');
  await mkdir(profileDir, { recursive: true });
  const child = spawn(
    browser.executablePath,
    [
      '--remote-debugging-address=127.0.0.1',
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${profileDir}`,
      '--profile-directory=OcentraManagedCompositedBlock',
      '--no-first-run',
      '--no-default-browser-check',
      '--new-window',
      'about:blank',
    ],
    { stdio: 'ignore', windowsHide: true }
  );
  return { child, port, profileDir };
}

async function cleanupProfileRun(profileRun) {
  profileRun.child.kill();
  await new Promise((resolve) => {
    profileRun.child.once('exit', resolve);
    setTimeout(resolve, 3000).unref();
  });
}

async function installedChromiumBrowser() {
  for (const browser of browserCandidates()) {
    if (await fileExists(browser.executablePath)) {
      return browser;
    }
  }
  return null;
}

function browserCandidates() {
  if (process.platform !== 'win32') {
    return [];
  }
  return windowsRoots().flatMap((root) => [
    chromiumCandidate('chrome-stable', 'chrome', join(root, 'Google', 'Chrome', 'Application', 'chrome.exe')),
    chromiumCandidate('edge-stable', 'edge', join(root, 'Microsoft', 'Edge', 'Application', 'msedge.exe')),
  ]);
}

function chromiumCandidate(id, family, executablePath) {
  return {
    bridge: 'chromium-devtools-protocol',
    channel: 'stable',
    executablePath,
    family,
    id,
  };
}

function windowsRoots() {
  return [process.env.ProgramFiles, process.env['ProgramFiles(x86)'], process.env.LOCALAPPDATA].filter(Boolean);
}

async function waitForFirstPageTarget(port) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const targets = await waitForJson(port, '/json/list');
    const pageTarget = targets.find((target) => target.type === 'page' && target.webSocketDebuggerUrl !== undefined);
    if (pageTarget !== undefined) {
      return pageTarget;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for page target on ${port}`);
}

async function waitForLivePage(client) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const result = await client
      .command('Runtime.evaluate', {
        expression:
          "Boolean(document.body) && document.body.children.length > 0 && ['interactive', 'complete'].includes(document.readyState)",
        returnByValue: true,
      })
      .catch(() => ({ result: { value: false } }));
    if (result.result?.value === true) {
      return;
    }
    await delay(500);
  }
  throw new Error('Timed out waiting for live page before capture.');
}

async function waitForChromiumReady(client) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const state = await evaluateChromiumValue(client, 'document.readyState').catch(() => '');
    if (state === 'complete') {
      return;
    }
    await delay(250);
  }
}

async function evaluateChromiumValue(client, expression) {
  const result = await client.command('Runtime.evaluate', {
    expression,
    returnByValue: true,
  });
  return result.result?.value;
}

async function captureChromiumScreenshot(browser, client, label) {
  const screenshot = await client.command('Page.captureScreenshot', { format: 'png' });
  const path = join(screenshotDirectory, `${safeFileName(`${browser.id}-${label}`)}.png`);
  await writeFile(path, Buffer.from(screenshot.data, 'base64'));
  return path;
}

async function fileExists(pathValue) {
  if (pathValue.length === 0) {
    return false;
  }
  try {
    return (await stat(pathValue)).isFile();
  } catch {
    return false;
  }
}

async function freePort() {
  const server = createNetServer();
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  await new Promise((resolve) => server.close(resolve));
  return address.port;
}

function spawnAgentService(runRoot, agentPort, htmlPath) {
  return spawn(resolveDebugAgentServicePath(), [], {
    cwd: repoRoot,
    env: {
      ...process.env,
      OCENTRA_PARENT_ACTIVITY_DB_PATH: join(runRoot, 'activity.sqlite'),
      OCENTRA_PARENT_ACTIVITY_JOURNAL_KEY_PATH: join(runRoot, 'activity.key'),
      OCENTRA_PARENT_ACTIVITY_JOURNAL_PATH: join(runRoot, 'activity.ndjson'),
      OCENTRA_PARENT_AGENT_ADDR: `127.0.0.1:${agentPort}`,
      OCENTRA_PARENT_AGENT_ENFORCEMENT_TIMER_STATE_PATH: join(runRoot, 'enforcement-timers.json'),
      OCENTRA_PARENT_DEV_LOG_DIR: join(runRoot, 'logs'),
      OCENTRA_PARENT_MANAGED_BROWSER_INTERVENTION_HTML_PATH: htmlPath,
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
  throw new Error(`Timed out waiting for child-agent health. ${serviceOutput()}`);
}

async function runCommand(command, args) {
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${command} exited with ${code}`))));
    child.once('error', reject);
  });
}

function collectOutput(child) {
  const chunks = [];
  child.stdout.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr.on('data', (chunk) => chunks.push(String(chunk)));
  return () => chunks.join('');
}

function safeFileName(value) {
  return value.replace(/[^a-zA-Z0-9_.-]/g, '-');
}

function envNumber(name, fallback) {
  const parsed = Number(process.env[name]);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
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

async function stopWindowsProcessesByCommandLineFragment(fragment) {
  if (process.platform !== 'win32') {
    return;
  }
  const script = [
    "$fragment = [Environment]::GetEnvironmentVariable('OCENTRA_PARENT_STOP_FRAGMENT')",
    'Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -and $_.CommandLine.Contains($fragment) } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }',
  ].join('; ');
  await new Promise((resolve) => {
    const child = spawn('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], {
      env: { ...process.env, OCENTRA_PARENT_STOP_FRAGMENT: fragment },
      stdio: 'ignore',
      windowsHide: true,
    });
    child.once('exit', resolve);
  });
}

class DevToolsClient {
  static async connect(url) {
    const webSocket = await connectWebSocket(url, timeoutMs);
    return new DevToolsClient(webSocket);
  }

  constructor(webSocket) {
    this.webSocket = webSocket;
    this.commandId = 0;
    this.pending = new Map();
    this.webSocket.addEventListener('message', (message) => this.handleMessage(message));
  }

  command(method, params) {
    const id = ++this.commandId;
    const promise = new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      setTimeout(() => {
        if (this.pending.delete(id)) {
          reject(new Error(`Timed out waiting for ${method}`));
        }
      }, commandTimeoutMs).unref();
    });
    this.webSocket.send(JSON.stringify({ id, method, params }));
    return promise;
  }

  close() {
    this.webSocket.close();
  }

  handleMessage(message) {
    const data = JSON.parse(message.data);
    if (data.id === undefined) {
      return;
    }
    const pending = this.pending.get(data.id);
    if (pending === undefined) {
      return;
    }
    this.pending.delete(data.id);
    if (data.error !== undefined) {
      pending.reject(new Error(JSON.stringify(data.error)));
      return;
    }
    pending.resolve(data.result ?? {});
  }
}

async function connectWebSocket(url, timeout) {
  const deadline = Date.now() + timeout;
  let lastError;
  while (Date.now() < deadline) {
    try {
      return await openWebSocket(url);
    } catch (error) {
      lastError = error;
      await delay(250);
    }
  }
  throw lastError;
}

function openWebSocket(url) {
  return new Promise((resolve, reject) => {
    const webSocket = new WebSocket(url);
    const timer = setTimeout(() => {
      webSocket.close();
      reject(new Error(`Timed out connecting to ${url}`));
    }, 3000);
    webSocket.addEventListener(
      'open',
      () => {
        clearTimeout(timer);
        resolve(webSocket);
      },
      { once: true }
    );
    webSocket.addEventListener(
      'error',
      () => {
        clearTimeout(timer);
        reject(new Error(`WebSocket error for ${url}`));
      },
      { once: true }
    );
  });
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
