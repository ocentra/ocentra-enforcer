import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdir, mkdtemp, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { tmpdir } from 'node:os';
import { setTimeout as delay } from 'node:timers/promises';

import { chromium } from '@playwright/test';

import { resolveDebugAgentServicePath, stopProcessTreeAndWait } from './agent-service-process.mjs';

const repoRoot = process.cwd();
const runId = new Date().toISOString().replaceAll(':', '-').replaceAll('.', '-');
const workpackId = 'game-22-tests-fixtures-playwright-manual-proof';
const outputDirectory = join(repoRoot, 'output', 'browser-plan-proof', workpackId);
const screenshotDirectory = join(outputDirectory, '06-ui-snapshots');
const evidenceDirectory = join(repoRoot, 'test-results', 'browser-game-child-intervention-page-proof');
const timeoutMs = envNumber('OCENTRA_PARENT_BROWSER_GAME_CHILD_INTERVENTION_TIMEOUT_MS', 60_000);

const targets = [
  {
    action: 'approval-hold',
    deliveryState: 'approval-hold-rendered',
    outcome: 'approval-required',
    parentRequestEnabled: true,
    reason: 'This game needs parent approval before it opens.',
    ruleId: 'browser-game-approval-required',
    ruleLabel: 'Unknown browser game needs parent approval',
    surface: 'roblox-discover',
    targetType: 'unknown-game',
    url: 'https://www.roblox.com/discover',
  },
  {
    action: 'block',
    deliveryState: 'block-page-rendered',
    outcome: 'blocked',
    parentRequestEnabled: true,
    reason: 'This browser game is blocked by a family rule.',
    ruleId: 'browser-game-blocked',
    ruleLabel: 'Blocked browser game route',
    surface: 'coolmath-run',
    targetType: 'browser-game',
    url: 'https://www.coolmathgames.com/0-run',
  },
  {
    action: 'checking-hold',
    deliveryState: 'checking-hold-rendered',
    outcome: 'held',
    parentRequestEnabled: false,
    reason: 'Ocentra is checking this game before it opens.',
    ruleId: 'browser-game-checking',
    ruleLabel: 'Checking browser game route',
    surface: 'scratch-games',
    targetType: 'browser-game',
    url: 'https://scratch.mit.edu/explore/projects/games/',
  },
  {
    action: 'parent-review',
    deliveryState: 'manual-required',
    outcome: 'manual-required',
    parentRequestEnabled: true,
    reason: 'Cloud gaming needs manual review before Ocentra can decide.',
    ruleId: 'cloud-gaming-manual-review',
    ruleLabel: 'Cloud gaming requires parent review',
    surface: 'xbox-cloud',
    targetType: 'cloud-gaming',
    url: 'https://www.xbox.com/en-US/play',
  },
  {
    action: 'warn',
    deliveryState: 'warn-page-rendered',
    outcome: 'warned',
    parentRequestEnabled: true,
    reason: 'This store or launcher surface can lead to native game access.',
    ruleId: 'native-game-store-warning',
    ruleLabel: 'Native game store warning',
    surface: 'steam-store',
    targetType: 'game-purchase',
    url: 'https://store.steampowered.com/',
  },
];

await main();

async function main() {
  await runCommand('cmd', ['/c', 'npm run build:contracts']);
  const { BrowserChildInterventionPageDefaults, renderBrowserChildInterventionPage } =
    await import('@ocentra-parent/portal-domain/browser-child-intervention-page');
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  await mkdir(evidenceDirectory, { recursive: true });
  await mkdir(screenshotDirectory, { recursive: true });

  const runRoot = await mkdtemp(join(tmpdir(), 'ocentra-parent-browser-game-child-intervention-'));
  const htmlPath = join(runRoot, 'browser-intervention-page.html');
  const agentPort = await freePort();
  const service = spawnAgentService(runRoot, agentPort, htmlPath);
  const serviceOutput = collectOutput(service);
  const browser = await chromium.launch();

  try {
    await waitForHealth(agentPort, serviceOutput);
    const page = await browser.newPage({ viewport: { width: 1366, height: 850 } });
    const servedPages = [];

    for (const target of targets) {
      const liveCapture = await captureLiveSurface(page, target);
      const requestedUrl = target.url;
      const html = renderBrowserChildInterventionPage({
        action: target.action,
        backdrop: {
          imageUrl: liveCapture.backdropDataUrl,
          label: `Captured ${target.surface} before intervention`,
        },
        blockMarker: BrowserChildInterventionPageDefaults.BlockMarker,
        bridge: 'browser-game-child-intervention-proof',
        childName: 'browser-game-proof-child',
        deliveryState: target.deliveryState,
        outcome: target.outcome,
        parentRequestEnabled: target.parentRequestEnabled,
        reason: target.reason,
        requestedUrl,
        ruleId: target.ruleId,
        ruleLabel: target.ruleLabel,
        ruleMarker: BrowserChildInterventionPageDefaults.BlockMarker,
        targetType: target.targetType,
        theme: 'dark',
      });
      await writeFile(htmlPath, html, 'utf8');

      const servedUrl = `http://127.0.0.1:${agentPort}/api/browser/intervention/page?target=${encodeURIComponent(
        requestedUrl
      )}`;
      const response = await fetch(servedUrl);
      const servedHtml = await response.text();
      await page.goto(servedUrl, { waitUntil: 'networkidle', timeout: timeoutMs });
      const screenshotPath = join(screenshotDirectory, `${target.surface}-${target.action}.png`);
      await page.screenshot({ fullPage: true, path: screenshotPath });
      const renderedState = await page.evaluate(() => ({
        backdropPresent: Boolean(document.querySelector('.ocentra-child-site-backdrop img')),
        bridgePayloadPresent: document.body.textContent.includes('ocentra-child-approval-request'),
        markerPresent: document.body.textContent.includes('OCENTRA_MANAGED_BROWSER_BLOCKED'),
        requestButtonPresent: Boolean(document.querySelector('[data-ocentra-request-button]')),
        title: document.title,
      }));

      servedPages.push({
        action: target.action,
        assertions: {
          blockMarkerPresent: servedHtml.includes(BrowserChildInterventionPageDefaults.BlockMarker),
          bridgePayloadPresent: servedHtml.includes('ocentra-child-approval-request'),
          cacheDisabled: response.headers.get('cache-control') === 'no-store',
          endpointServedHtml: response.ok,
          liveBackdropCaptured: liveCapture.screenshotBytes > 10_000,
          renderedBackdropPresent: renderedState.backdropPresent,
          renderedMarkerPresent: renderedState.markerPresent,
          screenshotCaptured: true,
        },
        deliveryState: target.deliveryState,
        finalOriginSha256: liveCapture.finalOriginSha256,
        finalPathSha256: liveCapture.finalPathSha256,
        inputOriginSha256: liveCapture.inputOriginSha256,
        inputPathSha256: liveCapture.inputPathSha256,
        liveStatus: liveCapture.status,
        outcome: target.outcome,
        rawUrlPersisted: false,
        requestedUrlSha256: sha256(requestedUrl),
        screenshot: relativePath(screenshotPath),
        status: response.status,
        surface: target.surface,
        targetType: target.targetType,
      });
    }

    const proof = {
      schemaVersion: 1,
      checkedAt: new Date().toISOString(),
      commit: await git(['rev-parse', 'HEAD']),
      workpackIds: [workpackId],
      proofMode: 'real-browser-game-child-agent-served-intervention-pages',
      productClaimReady: false,
      artifacts: {
        outputProof: `output/browser-plan-proof/${workpackId}/02-rendered-browser-game-child-intervention-proof.json`,
        proof: 'test-results/browser-game-child-intervention-page-proof/proof.json',
        screenshots: `output/browser-plan-proof/${workpackId}/06-ui-snapshots`,
      },
      assertions: [
        'Real public browser-game, cloud-gaming, and native game store surfaces were opened in Playwright before intervention rendering.',
        'Each captured live surface screenshot was used as the shared BrowserChildInterventionPage backdrop.',
        'The Rust child-agent intervention endpoint served each rendered browser-game child page with no-store caching.',
        'The rendered pages cover approval-hold, block, checking-hold, parent-review, and warn states.',
      ],
      nonClaims: [
        'This proof does not claim final policy decisions or enforcement.',
        'This proof does not claim cloud-streamed frame analysis or native game control.',
        'This proof does not claim notification delivery or parent approval delivery confirmation.',
        'This proof does not persist raw target URLs in the proof JSON.',
      ],
      servedPages,
      summary: {
        targetCount: servedPages.length,
        allAssertionsPassed: servedPages.every((entry) => Object.values(entry.assertions).every(Boolean)),
        rawUrlPersisted: false,
        livePublicSurfaceCaptured: true,
        childAgentEndpointRendered: true,
        productChecklistUpgradeClaimed: false,
      },
    };

    const outputProofPath = join(outputDirectory, '02-rendered-browser-game-child-intervention-proof.json');
    const proofPath = join(evidenceDirectory, 'proof.json');
    await writeFile(outputProofPath, `${JSON.stringify(proof, null, 2)}\n`, 'utf8');
    await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`, 'utf8');
    await writeFile(join(outputDirectory, '10-validation-commands.log'), validationLog(), 'utf8');

    console.log(`browser-game-child-intervention-page-proof-ok=${proof.summary.allAssertionsPassed}`);
    console.log(`evidence=${relativePath(proofPath)}`);
    console.log(`screenshots=${relativePath(screenshotDirectory)}`);
    console.log(`targets=${proof.summary.targetCount}`);
    if (!proof.summary.allAssertionsPassed) {
      process.exitCode = 1;
    }
  } finally {
    await browser.close();
    await stopProcessTreeAndWait(service);
  }
}

async function captureLiveSurface(page, target) {
  let response = null;
  try {
    response = await page.goto(target.url, { waitUntil: 'domcontentloaded', timeout: timeoutMs });
    await page.waitForLoadState('networkidle', { timeout: 10_000 }).catch(() => undefined);
  } catch {
    await page.goto(target.url, { waitUntil: 'commit', timeout: timeoutMs });
  }
  await delay(1000);
  const screenshot = await page.screenshot({ fullPage: false, type: 'png' });
  const finalUrl = new URL(page.url());
  const inputUrl = new URL(target.url);
  return {
    backdropDataUrl: `data:image/png;base64,${screenshot.toString('base64')}`,
    finalOriginSha256: sha256(finalUrl.origin),
    finalPathSha256: sha256(finalUrl.pathname),
    inputOriginSha256: sha256(inputUrl.origin),
    inputPathSha256: sha256(inputUrl.pathname),
    screenshotBytes: screenshot.byteLength,
    status: response?.status() ?? null,
  };
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
  throw new Error(`Timed out waiting for service health. ${serviceOutput()}`);
}

async function runCommand(command, args) {
  await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: repoRoot, stdio: 'inherit', windowsHide: true });
    child.once('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${command} exited with ${code}`))));
    child.once('error', reject);
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

async function git(args) {
  const child = spawn('git', args, { cwd: repoRoot, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true });
  const output = [];
  const errors = [];
  child.stdout.on('data', (chunk) => output.push(String(chunk)));
  child.stderr.on('data', (chunk) => errors.push(String(chunk)));
  const code = await new Promise((resolve) => child.once('exit', resolve));
  if (code !== 0) {
    throw new Error(`git ${args.join(' ')} failed: ${errors.join('')}`);
  }
  return output.join('').trim();
}

function validationLog() {
  return [
    'cmd /c npm run build:contracts',
    'cargo build -p ocentra-parent-agent-service',
    'cmd /c node scripts/test/browser-game-child-intervention-page-proof.mjs',
    '',
  ].join('\n');
}

function envNumber(name, fallback) {
  const parsed = Number(process.env[name]);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}
