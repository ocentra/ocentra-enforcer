import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright';

import {
  ParentDevEnv,
  ParentDevHost,
  ParentDevPort,
  createAgentAddress,
  createAgentHealthUrl,
  createAgentWebSocketUrl,
  createHttpOrigin,
  isLikelyParentAgentOccupant,
  isLikelyParentPortalOccupant,
  resolveParentDevPort,
} from '../dev/local-dev-config.mjs';
import { ensurePortFree } from '../dev/port-utils.mjs';
import {
  removeDirectoryWithRetry,
  resolveDebugAgentServicePath,
  spawnVitePortal,
  stopProcessTreeAndWait,
} from './agent-service-process.mjs';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-plan-proof', 'settings-ui');
const artifactScreenshotPath = join(outputRoot, 'parent-settings-screen-catalog.png');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');
const failureScreenshotPath = join(outputRoot, 'failure.png');
const failureSummaryPath = join(outputRoot, 'failure.json');
const devLogDir = await mkdtemp(join(tmpdir(), 'ocentra-parent-screen-settings-ui-'));
const agentPort = resolveParentDevPort(
  process.env[ParentDevEnv.AgentPort],
  ParentDevPort.PortalSmokeAgent,
  ParentDevEnv.AgentPort
);
const portalPort = resolveParentDevPort(
  process.env[ParentDevEnv.PortalPort],
  ParentDevPort.PortalSmokePortal,
  ParentDevEnv.PortalPort
);

await mkdir(outputRoot, { recursive: true });
await Promise.all([rm(failureScreenshotPath, { force: true }), rm(failureSummaryPath, { force: true })]);
await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
await ensurePortFree(portalPort, isLikelyParentPortalOccupant, console.log);

const agent = spawn(resolveDebugAgentServicePath(), [], {
  cwd: repoRoot,
  env: {
    ...process.env,
    [ParentDevEnv.AgentAddress]: createAgentAddress(agentPort),
    [ParentDevEnv.AgentAllowedOrigins]: createHttpOrigin(ParentDevHost.Loopback, portalPort),
    [ParentDevEnv.ActivityDbPath]: join(devLogDir, 'activity.sqlite'),
    [ParentDevEnv.DevLogDir]: devLogDir,
  },
  stdio: ['ignore', 'pipe', 'pipe'],
});
const agentOutput = collectOutput(agent);
const portal = spawnVitePortal(portalPort, {
  ...process.env,
  [ParentDevEnv.PortalAgentWebSocketUrl]: createAgentWebSocketUrl(agentPort),
  [ParentDevEnv.DevLogDir]: devLogDir,
});
const portalOutput = collectOutput(portal);

let browser;
let page;

try {
  await waitForHttp(createAgentHealthUrl(agentPort));
  await waitForHttp(`http://127.0.0.1:${portalPort}/`);
  browser = await chromium.launch({ headless: true });
  page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  await page.goto(`http://127.0.0.1:${portalPort}/#/settings-rules`, { waitUntil: 'domcontentloaded' });
  await page.getByText('Screen settings and capability proof').waitFor({ timeout: 20000 });
  const pageText = await page.locator('body').innerText();
  const renderedAssertions = [
    'Screen settings and capability proof',
    'Catalog settings',
    '474',
    'Catalog tabs',
    '11',
    'Proof-required controls',
    '68',
    'Unavailable sensitive modes',
    '9',
    'unavailable / unavailable',
    'proof-required / available',
    'needs-effect-wiring / available',
  ];
  for (const assertion of renderedAssertions) {
    if (!pageText.includes(assertion)) {
      throw new Error(`Settings route did not render expected Screen proof text: ${assertion}`);
    }
  }
  await page.screenshot({ path: artifactScreenshotPath, fullPage: true });
  const summary = {
    status: 'ok',
    proof: 'screen-settings-portal-proof',
    proofTier: 'P3_LOCAL_DEV_PORTAL',
    route: '#/settings-rules',
    artifacts: {
      screenshot: artifactScreenshotPath,
      summary: artifactSummaryPath,
    },
    ports: {
      agent: agentPort,
      portal: portalPort,
    },
    renderedAssertions,
    nonClaims: [
      'This proves the real portal Settings route renders the read-only Screen control catalog proof.',
      'It does not write parent settings or claim production opt-in/retention controls.',
      'It does not claim browser, network, mobile, or broad enforcement adapter support.',
    ],
  };
  await writeFile(artifactSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`screen-settings-portal-proof-ok ${artifactSummaryPath}`);
} catch (error) {
  await writeFailureLog(error);
  throw error;
} finally {
  if (browser !== undefined) {
    await browser.close();
  }
  await Promise.all([stopProcessTreeAndWait(portal), stopProcessTreeAndWait(agent)]);
  await removeDirectoryWithRetry(devLogDir);
}

function collectOutput(child) {
  const chunks = [];
  child.stdout?.on('data', (chunk) => chunks.push(String(chunk)));
  child.stderr?.on('data', (chunk) => chunks.push(String(chunk)));
  return chunks;
}

async function waitForHttp(url) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return response;
      }
    } catch {
      await delay(250);
    }
  }
  throw new Error(`Timed out waiting for ${url}`);
}

async function writeFailureLog(error) {
  if (page !== undefined) {
    await page.screenshot({ path: failureScreenshotPath, fullPage: true }).catch(() => undefined);
  }
  await writeFile(
    failureSummaryPath,
    `${JSON.stringify(
      {
        status: 'failed',
        error: error instanceof Error ? error.message : String(error),
        agentOutput: agentOutput.slice(-80),
        portalOutput: portalOutput.slice(-80),
      },
      null,
      2
    )}\n`
  );
}
