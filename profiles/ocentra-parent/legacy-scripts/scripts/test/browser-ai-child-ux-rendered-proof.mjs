import { spawn } from 'node:child_process';
import { createServer as createNetServer } from 'node:net';
import { mkdir, mkdtemp, stat, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { tmpdir } from 'node:os';
import { setTimeout as delay } from 'node:timers/promises';

import {
  BrowserAiChildUxSchemaVersion,
  BrowserAiChildUxSnapshotSchema,
} from '@ocentra-parent/schema-domain/browser-ai-child-ux-schemas';
import { BrowserAiPolicyEvaluatorSchemaVersion } from '@ocentra-parent/schema-domain/browser-ai-policy-evaluator-schemas';
import { BrowserAiPostAnalysisActionSchemaVersion } from '@ocentra-parent/schema-domain/browser-ai-post-analysis-action-schemas';
import {
  BrowserChildInterventionPageDefaults,
  renderBrowserChildInterventionPage,
} from '@ocentra-parent/portal-domain/browser-child-intervention-page';
import { resolveBrowserChildUxText } from '@ocentra-parent/schema-domain/text-browser-ux';

import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const repoRoot = process.cwd();
const runId = new Date().toISOString().replaceAll(':', '-').replaceAll('.', '-');
const evidenceDirectory = join(repoRoot, 'test-results', 'browser-ai-child-ux-rendered-proof');
const screenshotDirectory = join(evidenceDirectory, `${runId}-screenshots`);
const probeRoot = join(tmpdir(), `ocentra-parent-browser-ai-child-ux-rendered-${process.pid}`);
const timeoutMs = envNumber('OCENTRA_PARENT_BROWSER_AI_CHILD_UX_RENDERED_TIMEOUT_MS', 45_000);
const commandTimeoutMs = envNumber('OCENTRA_PARENT_BROWSER_AI_CHILD_UX_RENDERED_COMMAND_TIMEOUT_MS', 20_000);
const requestedUrl =
  process.env.OCENTRA_PARENT_BROWSER_AI_CHILD_UX_RENDERED_URL ?? 'https://www.youtube.com/watch?v=XzUB8_gj6xM';

async function main() {
  const browser = await installedChromiumBrowser();
  if (browser === null) {
    throw new Error('No installed Chrome or Edge executable found for browser AI child UX rendered proof.');
  }

  await mkdir(evidenceDirectory, { recursive: true });
  await mkdir(screenshotDirectory, { recursive: true });
  await mkdir(probeRoot, { recursive: true });
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);

  const runRoot = await mkdtemp(join(tmpdir(), 'ocentra-parent-browser-ai-child-ux-agent-'));
  const htmlPath = join(runRoot, 'browser-intervention-page.html');
  const agentPort = await freePort();
  const service = spawnAgentService(runRoot, agentPort, htmlPath);
  const serviceOutput = collectOutput(service);
  let profileRun;
  try {
    await waitForHealth(agentPort, serviceOutput);
    profileRun = await launchChromiumProfile(browser);
    const target = await waitForFirstPageTarget(profileRun.port);
    const client = await DevToolsClient.connect(target.webSocketDebuggerUrl);
    try {
      await client.command('Page.enable', {});
      await client.command('Runtime.enable', {});
      await client.command('Page.navigate', { url: requestedUrl });
      await waitForLivePage(client);
      await delay(3000);
      const capturedLocation = await evaluateChromiumValue(client, 'location.href');
      const capturedTargetMatchedRequested = isSameWatchedTarget(capturedLocation, requestedUrl);
      const backdropDataUrl = localProofBackdropDataUrl();
      const cases = [];
      const caseChecks = [];
      for (const proofCase of childUxProofCases()) {
        const snapshot = BrowserAiChildUxSnapshotSchema.parse(childUxSnapshot(proofCase));
        const primaryText = String(resolveBrowserChildUxText(snapshot.primaryTextToken));
        const secondaryText =
          snapshot.secondaryTextToken === null ? null : String(resolveBrowserChildUxText(snapshot.secondaryTextToken));
        const html = renderBrowserChildInterventionPage({
          action: proofCase.action,
          backdrop: {
            imageUrl: backdropDataUrl,
            label: 'Local proof backdrop',
          },
          blockMarker: BrowserChildInterventionPageDefaults.BlockMarker,
          bridge: 'child-agent-browser-ai-child-ux-rendered-proof',
          deliveryState: snapshot.deliveryState,
          outcome: proofCase.outcome,
          parentRequestEnabled: proofCase.parentRequestEnabled,
          reason: secondaryText ?? primaryText,
          requestedUrl,
          ruleId: proofCase.ruleId,
          ruleLabel: proofCase.ruleLabel,
          ruleMarker: BrowserChildInterventionPageDefaults.BlockMarker,
          targetType: 'video',
          theme: 'dark',
        });
        await writeFile(htmlPath, html, 'utf8');
        const endpointUrl = `http://127.0.0.1:${agentPort}/api/browser/intervention/page?target=${encodeURIComponent(
          requestedUrl
        )}&state=${encodeURIComponent(snapshot.state)}`;
        await client.command('Page.navigate', { url: endpointUrl });
        await waitForChromiumReady(client);
        await delay(500);
        const observed = await client.command('Runtime.evaluate', {
          expression: `({
            href: location.href,
            title: document.title,
            markerPresent: document.body.textContent.includes('${BrowserChildInterventionPageDefaults.BlockMarker}'),
            primaryTextPresent: document.body.textContent.includes(${JSON.stringify(primaryText)}),
            targetTextPresent: document.body.textContent.includes(${JSON.stringify(requestedUrl)}),
            backdropPresent: Boolean(document.querySelector('.ocentra-child-site-backdrop img')),
            askParentButtonPresent: Boolean(document.querySelector('[data-ocentra-child-action="ask-parent"]')),
          })`,
          returnByValue: true,
        });
        const observedValue = observed.result?.value ?? {};
        const screenshotPath = await captureChromiumScreenshot(browser, client, `child-ux-${snapshot.state}`);
        caseChecks.push({
          backdropPresent: observedValue.backdropPresent === true,
          endpointRendered:
            typeof observedValue.href === 'string' &&
            observedValue.href.includes('/api/browser/intervention/page?target='),
          markerPresent: observedValue.markerPresent === true,
          primaryTextPresent: observedValue.primaryTextPresent === true,
          state: snapshot.state,
          targetTextPresent: observedValue.targetTextPresent === true,
        });
        cases.push({
          action: proofCase.action,
          adapterProofRef: snapshot.adapterProofRef,
          deliveryState: snapshot.deliveryState,
          endpointRoute: '/api/browser/intervention/page',
          outcome: proofCase.outcome,
          primaryText,
          ruleId: proofCase.ruleId,
          screenshotPath,
          snapshotId: snapshot.snapshotId,
          state: snapshot.state,
          surface: snapshot.surface,
        });
      }
      const assertions = assertionsForCases(caseChecks, capturedTargetMatchedRequested);
      const evidence = {
        schemaVersion: 1,
        generatedAt: new Date().toISOString(),
        browser,
        childAgentEndpoint: '/api/browser/intervention/page',
        htmlPath: relative(repoRoot, htmlPath),
        cases,
      };
      const evidencePath = join(evidenceDirectory, `${runId}.json`);
      await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`, 'utf8');
      printSummary({ assertions, capturedTargetMatchedRequested, cases, requestedUrl }, evidencePath);
      if (!Object.values(assertions).every(Boolean)) {
        process.exitCode = 1;
      }
    } finally {
      client.close();
    }
  } finally {
    if (profileRun !== undefined) {
      await cleanupProfileRun(profileRun);
    }
    await stopProcessTreeAndWait(service);
    await stopWindowsProcessesByCommandLineFragment(probeRoot);
  }
}

function childUxProofCases() {
  return [
    {
      action: 'checking-hold',
      deliveryState: 'checking-hold-rendered',
      outcome: 'checking',
      parentRequestEnabled: false,
      ruleId: 'browser-ai-checking-youtube-video',
      ruleLabel: 'Checking YouTube video',
      state: 'checking',
      surface: 'managed-browser-hold-page',
    },
    {
      action: 'warn',
      deliveryState: 'warn-page-rendered',
      outcome: 'warned',
      parentRequestEnabled: true,
      ruleId: 'browser-ai-warning-youtube-video',
      ruleLabel: 'Warn before continuing',
      state: 'warning',
      surface: 'managed-browser-warning-page',
    },
    {
      action: 'approval-hold',
      deliveryState: 'approval-hold-rendered',
      outcome: 'approval_required',
      parentRequestEnabled: true,
      ruleId: 'browser-ai-approval-youtube-video',
      ruleLabel: 'Parent approval required',
      state: 'approval_required',
      surface: 'parent-approval-hold-page',
    },
    {
      action: 'time-limit',
      deliveryState: 'warn-page-rendered',
      outcome: 'limited',
      parentRequestEnabled: true,
      ruleId: 'browser-ai-limited-youtube-video',
      ruleLabel: 'Time limit reached',
      state: 'limited',
      surface: 'managed-browser-warning-page',
    },
    {
      action: 'block',
      deliveryState: 'block-page-rendered',
      outcome: 'blocked',
      parentRequestEnabled: true,
      ruleId: 'browser-ai-block-youtube-video',
      ruleLabel: 'Blocked after review',
      state: 'blocked',
      surface: 'managed-browser-block-page',
    },
  ];
}

function childUxSnapshot(proofCase) {
  return {
    schemaVersion: BrowserAiChildUxSchemaVersion,
    snapshotId: `browser-ai-child-ux-${proofCase.state}`,
    createdAt: '2026-06-06T00:45:00.000Z',
    sourceEvidenceIds: ['browser-evidence-youtube-video-live-cdp-capture'],
    state: proofCase.state,
    tone: proofCase.state === 'blocked' ? 'neutral' : 'calm',
    surface: proofCase.surface,
    primaryTextToken: textTokenForState(proofCase.state),
    secondaryTextToken: null,
    deliveryState: proofCase.deliveryState,
    adapterProofRef: `child-agent-endpoint-proof-${proofCase.state}`,
    postAnalysisActionPlan: postAnalysisActionPlanForState(proofCase.state),
    rawCopyClaimed: false,
    visualRenderClaimed: false,
    surveillanceCopyClaimed: false,
    shamingCopyClaimed: false,
  };
}

function textTokenForState(state) {
  switch (state) {
    case 'checking':
      return 'browser.child.checking.title';
    case 'warning':
      return 'browser.child.warning.title';
    case 'approval_required':
      return 'browser.child.approval.title';
    case 'limited':
      return 'browser.child.limited.title';
    case 'blocked':
      return 'browser.child.blocked.title';
    default:
      return 'browser.child.unavailable.title';
  }
}

function postAnalysisActionPlanForState(state) {
  if (state === 'checking') {
    return null;
  }
  const outcome =
    state === 'approval_required'
      ? 'ask_parent'
      : state === 'blocked'
        ? 'block'
        : state === 'limited'
          ? 'time_limit'
          : 'warn';
  return {
    schemaVersion: BrowserAiPostAnalysisActionSchemaVersion,
    actionPlanId: `browser-post-analysis-action-plan-${state}`,
    createdAt: '2026-06-06T00:44:00.000Z',
    sourceEvidenceIds: ['browser-evidence-youtube-video-live-cdp-capture'],
    aiAnalysisId: 'browser-ai-analysis-result-youtube-video',
    policyDecision: policyDecision(outcome),
    policyDecisionAuditRefs: ['browser-policy-decision-audit-youtube-video'],
    parentRuleRefs: ['parent-rule-video-review'],
    actionLabels: actionLabelsForOutcome(outcome),
    trigger: 'policy_decision',
    timing: 'after_playback_started',
    childAlreadyEngaged: true,
    deliveryState: 'delivered',
    adapterProofRef: `child-agent-endpoint-proof-${state}`,
    rememberUntil: null,
    actionAuditRefs: ['browser-post-analysis-action-audit-youtube-video'],
    realtimeBlockClaimed: false,
    browserRuntimeMutationClaimed: false,
    directEnforcementClaimed: false,
  };
}

function actionLabelsForOutcome(outcome) {
  switch (outcome) {
    case 'ask_parent':
      return ['parent_approval_requested_after_review'];
    case 'block':
      return ['playback_stopped_after_review'];
    default:
      return ['warning_shown_after_review'];
  }
}

function policyDecision(outcome) {
  return {
    schemaVersion: BrowserAiPolicyEvaluatorSchemaVersion,
    decisionId: `browser-policy-decision-${outcome}`,
    requestId: 'browser-policy-evaluator-request-youtube-video',
    decidedAt: '2026-06-06T00:43:59.000Z',
    policyVersionRef: 'browser-policy-version-2026-06-06',
    sourceEvidenceIds: ['browser-evidence-youtube-video-live-cdp-capture'],
    aiAnalysisId: 'browser-ai-analysis-result-youtube-video',
    memoryHitIds: [],
    graphRefs: [],
    parentRuleRefs: ['parent-rule-video-review'],
    scheduleContextRefs: ['schedule-context-evening'],
    outcome,
    evaluatorMode: 'active',
    confidence: 'high',
    reasonCodes: ['explicit_parent_rule', 'ai_high_confidence'],
    auditRefs: ['browser-policy-decision-audit-youtube-video'],
    adapterProofRef: `child-agent-endpoint-policy-proof-${outcome}`,
    fallbackUsed: false,
    aiClaimedAsAuthority: false,
    portalEvaluatedClaimed: false,
    directEnforcementClaimed: false,
  };
}

function assertionsForCases(caseChecks, capturedTargetMatchedRequested) {
  const states = new Set(caseChecks.map((proofCase) => proofCase.state));
  return {
    approvalRendered: casePassed(caseChecks, 'approval_required'),
    blockRendered: casePassed(caseChecks, 'blocked'),
    checkingRendered: casePassed(caseChecks, 'checking'),
    childAgentEndpointRendered: caseChecks.every((proofCase) => proofCase.endpointRendered === true),
    liveTargetCapturedBeforeRender: capturedTargetMatchedRequested,
    timeLimitRendered: casePassed(caseChecks, 'limited'),
    warningRendered: casePassed(caseChecks, 'warning'),
    expectedStateCoverage:
      states.has('checking') &&
      states.has('warning') &&
      states.has('approval_required') &&
      states.has('limited') &&
      states.has('blocked'),
  };
}

function casePassed(caseChecks, state) {
  const proofCase = caseChecks.find((item) => item.state === state);
  return (
    proofCase !== undefined &&
    proofCase.markerPresent === true &&
    proofCase.primaryTextPresent === true &&
    proofCase.targetTextPresent === true &&
    proofCase.backdropPresent === true
  );
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

function printSummary(summary, evidencePath) {
  const ok = Object.values(summary.assertions).every(Boolean);
  console.log(`browser-ai-child-ux-rendered-proof-ok=${ok}`);
  console.log(`evidence=${evidencePath}`);
  console.log(`requested=${summary.requestedUrl}`);
  console.log(`capturedTargetMatchedRequested=${summary.capturedTargetMatchedRequested}`);
  console.log(`states=${summary.cases.map((item) => item.state).join(',')}`);
  for (const item of summary.cases) {
    console.log(`screenshot.${item.state}=${item.screenshotPath}`);
  }
  for (const [name, passed] of Object.entries(summary.assertions)) {
    console.log(`assertion.${name}=${passed}`);
  }
}

function localProofBackdropDataUrl() {
  const svg = [
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1280 720">',
    '<defs>',
    '<linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">',
    '<stop offset="0%" stop-color="#101820" />',
    '<stop offset="100%" stop-color="#213547" />',
    '</linearGradient>',
    '</defs>',
    '<rect width="1280" height="720" fill="url(#bg)" />',
    '<circle cx="1024" cy="160" r="180" fill="rgba(255,255,255,0.08)" />',
    '<circle cx="192" cy="560" r="220" fill="rgba(255,255,255,0.06)" />',
    '<text x="96" y="608" fill="#f5f7fa" font-family="Segoe UI, Arial, sans-serif" font-size="42">',
    'Ocentra Parent local child UX proof backdrop',
    '</text>',
    '</svg>',
  ].join('');
  return `data:image/svg+xml;base64,${Buffer.from(svg, 'utf8').toString('base64')}`;
}

async function launchChromiumProfile(browser) {
  const port = await freePort();
  const profileDir = join(probeRoot, browser.id, 'browser-ai-child-ux-rendered-proof');
  await mkdir(profileDir, { recursive: true });
  const child = spawn(
    browser.executablePath,
    [
      '--remote-debugging-address=127.0.0.1',
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${profileDir}`,
      '--profile-directory=OcentraBrowserAiChildUxRenderedProof',
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
  throw new Error('Timed out waiting for live page before child UX render proof.');
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

function safeFileName(value) {
  return value.replace(/[^a-zA-Z0-9_.-]/g, '-');
}

function envNumber(name, fallback) {
  const parsed = Number(process.env[name]);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
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
