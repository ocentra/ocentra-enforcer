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
const outputRoot = resolve(repoRoot, 'output', 'screen-plan-proof', 'settings-writable-controls');
const artifactScreenshotPath = join(outputRoot, 'parent-settings-writable-controls.png');
const artifactSummaryPath = join(outputRoot, 'proof-summary.json');
const failureScreenshotPath = join(outputRoot, 'failure.png');
const failureSummaryPath = join(outputRoot, 'failure.json');
const devLogDir = await mkdtemp(join(tmpdir(), 'ocentra-parent-screen-settings-controls-'));
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
  page = await browser.newPage({ viewport: { width: 1600, height: 1200 } });
  await page.goto(`http://127.0.0.1:${portalPort}/#/settings-rules`, { waitUntil: 'domcontentloaded' });
  await page.getByText('Writable screen settings proof').waitFor({ timeout: 20000 });
  await expectText('Keep screen analysis disabled');
  await expectText('schema-valid local parent intent');
  await expectText('disabled');

  await page.getByRole('button', { name: 'Enable observe-only summaries' }).click();
  await expectText('Five-minute local summaries can be reviewed by the parent');
  await expectText('observeOnly');
  await expectText('foregroundAppChange | policyAmbiguity');

  await page.getByRole('button', { name: 'Enable strict dry-run review' }).click();
  const strictAssertions = [
    'One-minute cadence, selected triggers, local OCR, redaction, and policy dry-run become explicit parent intent.',
    'policyDryRun',
    'foregroundAppChange | managedBrowserUrlChange | appGameForegroundStart | policyAmbiguity',
    'localSensitiveText',
    'disabled',
    'unavailable',
  ];
  for (const assertion of strictAssertions) {
    await expectText(assertion);
  }

  await page.screenshot({ path: artifactScreenshotPath, fullPage: true });
  const summary = {
    status: 'ok',
    proof: 'screen-settings-writable-controls-proof',
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
    renderedAssertions: [
      'Writable screen settings proof',
      'Keep screen analysis disabled',
      'Enable observe-only summaries',
      'Enable strict dry-run review',
      'observeOnly',
      'policyDryRun',
      'foregroundAppChange | managedBrowserUrlChange | appGameForegroundStart | policyAmbiguity',
      'schema-valid local parent intent',
    ],
    nonClaims: [
      'This proves real parent Settings route controls can build schema-valid local screen-summary setting intents.',
      'It does not persist settings to the Rust service or claim child-agent runtime application of the draft.',
      'It does not enable raw screenshot retention, live view, or raw remote screenshot upload.',
    ],
  };
  await writeFile(artifactSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
  console.log(`screen-settings-writable-controls-proof-ok ${artifactSummaryPath}`);
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

async function expectText(assertion) {
  const pageText = await page.locator('body').innerText();
  if (!pageText.includes(assertion)) {
    throw new Error(`Settings writable controls did not render expected text: ${assertion}`);
  }
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
