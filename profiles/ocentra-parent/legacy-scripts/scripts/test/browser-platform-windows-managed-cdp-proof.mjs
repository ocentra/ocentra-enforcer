import { createHash } from 'node:crypto';
import { existsSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'node:http';
import { spawn, execFileSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const proofRoot = join(repoRoot, 'output/browser-plan-proof/05-cross-platform-inventory-matrix');
const resultPath = join(repoRoot, 'test-results/browser-platform-windows-managed-cdp-proof/proof.json');
const outputProofPath = join(proofRoot, '14-windows-managed-cdp-proof.json');
const screenshotPath = join(proofRoot, '14-windows-managed-cdp-screenshot.png');
const generatedAt = new Date().toISOString();

mkdirSync(dirname(resultPath), { recursive: true });
mkdirSync(proofRoot, { recursive: true });

const browser = findManagedBrowser();
if (process.platform !== 'win32') {
  writeUnavailableProof('not-windows-host');
  process.exit(0);
}
if (!browser) {
  writeUnavailableProof('managed-chromium-browser-not-found');
  process.exit(0);
}

const profileRoot = mkdtempSync(join(tmpdir(), 'ocentra-managed-browser-proof-'));
const server = await startProofServer();
const cdpPort = await reservePort();
const proofUrl = `http://127.0.0.1:${server.port}/ocentra-managed-browser-proof`;
const browserProcess = launchBrowser(browser.path, profileRoot, cdpPort, proofUrl);
let proofInput = null;

try {
  const version = await waitForJson(`http://127.0.0.1:${cdpPort}/json/version`);
  const targets = await waitForTargets(cdpPort, proofUrl);
  const pageTarget = targets.find((target) => target.url === proofUrl);
  if (!pageTarget?.webSocketDebuggerUrl) {
    throw new Error('Managed CDP target did not expose the proof page websocket endpoint');
  }

  const capture = await captureManagedPage(pageTarget.webSocketDebuggerUrl);
  writeFileSync(screenshotPath, Buffer.from(capture.screenshotBase64, 'base64'));

  proofInput = {
    browser,
    cdpPort,
    profileRoot,
    proofUrl,
    version,
    targets,
    pageTarget,
    capture,
    resultState: 'windows-managed-cdp-exact-url-proof',
  };
} finally {
  const profileDeleted = await cleanupBrowserProfile(browserProcess, profileRoot);
  server.close();
  if (proofInput) {
    const proof = proofFor({ ...proofInput, managedProfileDeletedAfterProof: profileDeleted });
    writeJson(resultPath, proof);
    writeJson(outputProofPath, proof);

    console.log('browser-platform-windows-managed-cdp-proof-ok=true');
    console.log(`proof=${resultPath}`);
    console.log(`outputProof=${outputProofPath}`);
    console.log(`screenshot=${screenshotPath}`);
    console.log(`resultState=${proof.hostProofSummary.resultState}`);
  }
}

function writeUnavailableProof(resultState) {
  const proof = {
    schemaVersion: 1,
    proofId: 'browser-platform-windows-managed-cdp-proof',
    generatedAt,
    branch: git(['branch', '--show-current']),
    commit: git(['rev-parse', 'HEAD']),
    baseCommit: git(['rev-parse', 'origin/main']),
    hostProofSummary: {
      platform: process.platform,
      windowsHost: process.platform === 'win32',
      realManagedBrowserLaunched: false,
      loopbackCdpEndpointResponded: false,
      exactManagedUrlObserved: false,
      cdpScreenshotCaptured: false,
      managedProfileCreated: false,
      managedProfileDeletedAfterProof: false,
      rawExecutablePathPersisted: false,
      rawProfilePathPersisted: false,
      rawCdpPayloadPersisted: false,
      activeTabEnforcementClaimed: false,
      finalPolicyExecutionClaimed: false,
      enforcementClaimed: false,
      resultState,
    },
  };
  writeJson(resultPath, proof);
  writeJson(outputProofPath, proof);
  console.log('browser-platform-windows-managed-cdp-proof-ok=false');
  console.log(`resultState=${resultState}`);
}

function proofFor(input) {
  const exactUrlObserved = input.pageTarget.url === input.proofUrl && input.capture.locationHref === input.proofUrl;
  const targetRef = redactedRef('managed-cdp-target', input.pageTarget.id ?? input.pageTarget.webSocketDebuggerUrl);
  return {
    schemaVersion: 1,
    proofId: 'browser-platform-windows-managed-cdp-proof',
    generatedAt,
    branch: git(['branch', '--show-current']),
    commit: git(['rev-parse', 'HEAD']),
    baseCommit: git(['rev-parse', 'origin/main']),
    hostProofSummary: {
      platform: process.platform,
      windowsHost: true,
      browserFamily: input.browser.family,
      browserChannel: input.browser.channel,
      executableRef: redactedRef('windows-managed-browser-executable', input.browser.path),
      managedProfileRef: redactedRef('windows-managed-browser-profile', input.profileRoot),
      loopbackCdpEndpointRef: redactedRef('windows-managed-cdp-loopback', `127.0.0.1:${input.cdpPort}`),
      proofUrlRef: redactedRef('windows-managed-cdp-proof-url', input.proofUrl),
      targetRef,
      realManagedBrowserLaunched: true,
      loopbackCdpEndpointResponded: true,
      cdpVersionEndpointResponded: input.version.Browser !== undefined,
      cdpTabListEndpointResponded: input.targets.length > 0,
      exactManagedUrlObserved: exactUrlObserved,
      activeTabKnownByTargetSelection: exactUrlObserved,
      cdpScreenshotCaptured: existsSync(screenshotPath),
      managedProfileCreated: true,
      managedProfileDeletedAfterProof: input.managedProfileDeletedAfterProof,
      rawExecutablePathPersisted: false,
      rawProfilePathPersisted: false,
      rawCdpPayloadPersisted: false,
      rawPageContentPersisted: false,
      providerOrCloudUsed: false,
      activeTabEnforcementClaimed: false,
      finalPolicyExecutionClaimed: false,
      enforcementClaimed: false,
      resultState: input.resultState,
    },
    cdpEvidence: {
      browserVersionRef: redactedRef('windows-managed-cdp-browser-version', String(input.version.Browser ?? 'unknown')),
      protocolVersionRef: redactedRef(
        'windows-managed-cdp-protocol-version',
        String(input.version['Protocol-Version'] ?? 'unknown')
      ),
      targetCount: input.targets.length,
      pageTargetType: input.pageTarget.type,
      pageTargetUrlRef: redactedRef('windows-managed-cdp-page-url', input.pageTarget.url),
      runtimeLocationHrefRef: redactedRef('windows-managed-cdp-runtime-location', input.capture.locationHref),
      pageTitleRef: redactedRef('windows-managed-cdp-title', input.capture.title),
      screenshotRef: redactedRef('windows-managed-cdp-screenshot', input.capture.screenshotBase64),
      screenshotPath: relativePath(screenshotPath),
    },
    negativeChecks: [
      { claim: 'raw-executable-path-persisted', rejected: true },
      { claim: 'raw-profile-path-persisted', rejected: true },
      { claim: 'raw-cdp-payload-persisted', rejected: true },
      { claim: 'active-tab-enforcement', rejected: true },
      { claim: 'final-policy-execution', rejected: true },
      { claim: 'browser-enforcement', rejected: true },
      { claim: 'non-windows-managed-cdp-support', rejected: true },
    ],
  };
}

function launchBrowser(executablePath, profileRoot, cdpPort, proofUrl) {
  const args = [
    `--remote-debugging-address=127.0.0.1`,
    `--remote-debugging-port=${cdpPort}`,
    `--user-data-dir=${profileRoot}`,
    '--profile-directory=OcentraManagedChild',
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-background-networking',
    '--disable-sync',
    '--headless=new',
    '--disable-gpu',
    '--window-size=960,640',
    proofUrl,
  ];
  return spawn(executablePath, args, {
    cwd: repoRoot,
    stdio: ['ignore', 'ignore', 'ignore'],
    windowsHide: true,
  });
}

async function cleanupBrowserProfile(browserProcess, profileRoot) {
  browserProcess.kill();
  if (browserProcess.pid) {
    try {
      execFileSync('taskkill.exe', ['/PID', String(browserProcess.pid), '/T', '/F'], {
        stdio: ['ignore', 'ignore', 'ignore'],
      });
    } catch {
      // The process may have already exited.
    }
  }
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      rmSync(profileRoot, { recursive: true, force: true });
      return !existsSync(profileRoot);
    } catch {
      await delay(500);
    }
  }
  return false;
}

async function captureManagedPage(webSocketUrl) {
  const client = await cdpClient(webSocketUrl);
  try {
    await client.send('Page.enable');
    await client.send('Runtime.enable');
    await client.send('Page.bringToFront');
    const location = await client.send('Runtime.evaluate', {
      expression: 'window.location.href',
      returnByValue: true,
    });
    const title = await client.send('Runtime.evaluate', {
      expression: 'document.title',
      returnByValue: true,
    });
    const screenshot = await client.send('Page.captureScreenshot', {
      format: 'png',
      captureBeyondViewport: false,
    });
    return {
      locationHref: location.result.value,
      title: title.result.value,
      screenshotBase64: screenshot.data,
    };
  } finally {
    client.close();
  }
}

async function cdpClient(webSocketUrl) {
  if (typeof WebSocket === 'undefined') {
    throw new Error('Global WebSocket is required for CDP screenshot capture');
  }
  const socket = new WebSocket(webSocketUrl);
  const pending = new Map();
  let nextId = 1;
  socket.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);
    const callbacks = pending.get(message.id);
    if (!callbacks) {
      return;
    }
    pending.delete(message.id);
    if (message.error) {
      callbacks.reject(new Error(JSON.stringify(message.error)));
      return;
    }
    callbacks.resolve(message.result);
  });
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', reject, { once: true });
  });
  return {
    send(method, params = {}) {
      const id = nextId;
      nextId += 1;
      socket.send(JSON.stringify({ id, method, params }));
      return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
    },
    close() {
      socket.close();
    },
  };
}

async function waitForTargets(cdpPort, expectedUrl) {
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline) {
    const targets = await waitForJson(`http://127.0.0.1:${cdpPort}/json/list`);
    if (targets.some((target) => target.url === expectedUrl)) {
      return targets;
    }
    await delay(250);
  }
  throw new Error('Timed out waiting for managed browser proof URL in CDP target list');
}

async function waitForJson(url) {
  const deadline = Date.now() + 20_000;
  let lastError = null;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return await response.json();
      }
    } catch (error) {
      lastError = error;
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for ${url}: ${lastError?.message ?? 'no response'}`);
}

async function startProofServer() {
  const server = createServer((request, response) => {
    response.writeHead(200, {
      'Content-Type': 'text/html; charset=utf-8',
      'Cache-Control': 'no-store',
    });
    response.end(proofHtml(request.url ?? '/'));
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  return {
    port: server.address().port,
    close() {
      server.close();
    },
  };
}

function proofHtml(pathname) {
  return [
    '<!doctype html>',
    '<html>',
    '<head><meta charset="utf-8"><title>Ocentra Managed Browser CDP Proof</title></head>',
    '<body>',
    '<main id="ocentra-managed-browser-cdp-proof">',
    '<h1>Ocentra Managed Browser CDP Proof</h1>',
    `<p data-path="${escapeHtml(pathname)}">Loopback managed profile proof page.</p>`,
    '</main>',
    '</body>',
    '</html>',
  ].join('');
}

async function reservePort() {
  const server = createServer();
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const port = server.address().port;
  await new Promise((resolve) => server.close(resolve));
  return port;
}

function findManagedBrowser() {
  return managedBrowserCandidates().find((candidate) => existsSync(candidate.path)) ?? null;
}

function managedBrowserCandidates() {
  const programFiles = process.env.ProgramFiles ?? '';
  const programFilesX86 = process.env['ProgramFiles(x86)'] ?? '';
  const localAppData = process.env.LOCALAPPDATA ?? '';
  return [
    {
      family: 'edge',
      channel: 'stable',
      path: join(programFiles, 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
    },
    {
      family: 'edge',
      channel: 'stable',
      path: join(programFilesX86, 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
    },
    {
      family: 'chrome',
      channel: 'stable',
      path: join(programFiles, 'Google', 'Chrome', 'Application', 'chrome.exe'),
    },
    {
      family: 'chrome',
      channel: 'stable',
      path: join(programFilesX86, 'Google', 'Chrome', 'Application', 'chrome.exe'),
    },
    {
      family: 'chrome',
      channel: 'stable',
      path: join(localAppData, 'Google', 'Chrome', 'Application', 'chrome.exe'),
    },
  ].filter((candidate) => candidate.path.trim().length > 0);
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function redactedRef(prefix, value) {
  return `${prefix}-${sha256(value).slice(0, 16)}`;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function relativePath(path) {
  return path
    .replace(repoRoot, '')
    .replace(/^[/\\]/u, '')
    .replaceAll('\\', '/');
}

function escapeHtml(value) {
  return value.replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}
