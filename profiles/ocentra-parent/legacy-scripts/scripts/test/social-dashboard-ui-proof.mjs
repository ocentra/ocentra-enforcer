import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

import {
  ParentDevEnv,
  ParentDevHost,
  ParentDevPort,
  createAgentAddress,
  createAgentHealthUrl,
  createAgentWebSocketUrl,
  createHttpOrigin,
  createPortalCommandsUrl,
  isLikelyParentAgentOccupant,
  isLikelyParentPortalOccupant,
  resolveParentDevPort,
} from '../dev/local-dev-config.mjs';
import { ensurePortFree } from '../dev/port-utils.mjs';
import { resolveDebugAgentServicePath, spawnVitePortal, stopProcessTreeAndWait } from './agent-service-process.mjs';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const portalRoot = path.join(repoRoot, 'apps', 'portal');
const proofRoot = path.join(repoRoot, 'output', 'browser-plan-proof', 'social-20-parent-social-dashboard-ux');
const screenshotDir = path.join(proofRoot, '06-ui-snapshots');
const proofResultDir = path.join(repoRoot, 'test-results', 'social-dashboard-ui-proof');
const proofPath = path.join(proofResultDir, 'proof.json');
const outputProofPath = path.join(proofRoot, '07-rendered-portal-ui-proof.json');
const validationLogPath = path.join(proofRoot, '08-validation-commands.log');
const securityLogPath = path.join(proofRoot, '09-security-negative-proof.log');
const playwrightLogPath = path.join(screenshotDir, 'social-dashboard-ui-playwright.log');
const desktopScreenshot = path.join(screenshotDir, 'social-dashboard-browser-route.png');
const mobileScreenshot = path.join(screenshotDir, 'social-dashboard-browser-route-mobile.png');
const accessibilitySummaryPath = path.join(proofResultDir, 'accessibility-summary.json');
const runRoot = await mkdtemp(path.join(tmpdir(), 'ocentra-parent-social-ui-'));
const devLogDir = path.join(runRoot, 'dev-log');
const activityDbPath = path.join(runRoot, 'activity.sqlite');
const commands = [];
const children = [];
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

let stopping = false;

try {
  await mkdir(devLogDir, { recursive: true });
  await mkdir(screenshotDir, { recursive: true });
  await mkdir(proofResultDir, { recursive: true });
  await runNpm(['run', 'build:contracts']);
  await runNpmWorkspace('@ocentra-parent/portal-domain', ['run', 'test', '--', 'social-dashboard-panel.test.ts']);
  await runNpmWorkspace('@ocentra-parent/portal', ['run', 'test', '--', 'social-dashboard-panel.test.ts']);
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  await ensurePortFree(agentPort, isLikelyParentAgentOccupant, console.log);
  await ensurePortFree(portalPort, isLikelyParentPortalOccupant, console.log);

  const agent = spawnAgent();
  trackChild(agent, 'agent');
  await waitForHttp(createAgentHealthUrl(agentPort));

  const portal = spawnVitePortal(portalPort, portalEnv(), repoRoot);
  trackChild(portal, 'portal');
  await waitForHttp(createPortalCommandsUrl(portalPort));

  const playwright = await runPlaywright();
  await writeProof(playwright);

  console.log('social-dashboard-ui-proof-ok');
  console.log(`evidence=${relativePath(proofPath)}`);
} finally {
  stopping = true;
  await Promise.all(children.map((child) => stopProcessTreeAndWait(child)));
  await rm(runRoot, { recursive: true, force: true });
}

function spawnAgent() {
  return spawn(resolveDebugAgentServicePath(repoRoot), [], {
    cwd: repoRoot,
    detached: process.platform !== 'win32',
    env: {
      ...process.env,
      [ParentDevEnv.ActivityDbPath]: activityDbPath,
      [ParentDevEnv.AgentAddress]: createAgentAddress(agentPort),
      [ParentDevEnv.AgentAllowedOrigins]: createHttpOrigin(ParentDevHost.Loopback, portalPort),
      [ParentDevEnv.DevLogDir]: devLogDir,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function portalEnv() {
  return {
    ...process.env,
    [ParentDevEnv.ActivityDbPath]: activityDbPath,
    [ParentDevEnv.DevLogDir]: devLogDir,
    [ParentDevEnv.PortalAgentWebSocketUrl]: createAgentWebSocketUrl(agentPort),
    [ParentDevEnv.PortalPort]: String(portalPort),
    SOCIAL_DASHBOARD_UI_PROOF: '1',
  };
}

function trackChild(child, label) {
  children.push(child);
  child.stdout?.on('data', (chunk) => process.stdout.write(chunk));
  child.stderr?.on('data', (chunk) => process.stderr.write(chunk));
  child.once('exit', (code, signal) => {
    if (!stopping && code !== 0) {
      console.error(`${label} exited early: code=${code ?? 'null'} signal=${signal ?? 'null'}`);
    }
  });
}

async function runPlaywright() {
  const cliPath = path.join(repoRoot, 'node_modules', '@playwright', 'test', 'cli.js');
  const args = [
    cliPath,
    'test',
    '--config',
    path.join(portalRoot, 'playwright.config.ts'),
    'social-dashboard-ui-proof.spec.ts',
    '--workers=1',
  ];
  const result = await runCommand(process.execPath, args, { cwd: portalRoot, env: portalEnv(), capture: true });
  await writeFile(playwrightLogPath, `${result.output.trimEnd()}\n`);
  return {
    command: [process.execPath, ...args].join(' '),
    exitCode: result.exitCode,
    log: relativePath(playwrightLogPath),
  };
}

async function runNpmWorkspace(workspaceName, args) {
  await runNpm(['--workspace', workspaceName, ...args]);
}

async function runNpm(args, ...rest) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  await runCommand(command, commandArgs, ...rest);
}

async function runCommand(command, args, options = {}) {
  const commandLine = [command, ...args].join(' ');
  const chunks = [];
  const child = spawn(command, args, {
    cwd: options.cwd ?? repoRoot,
    env: options.env ?? process.env,
    stdio: ['ignore', options.capture ? 'pipe' : 'inherit', options.capture ? 'pipe' : 'inherit'],
    windowsHide: true,
  });
  if (options.capture) {
    child.stdout?.on('data', (chunk) => {
      chunks.push(String(chunk));
      process.stdout.write(chunk);
    });
    child.stderr?.on('data', (chunk) => {
      chunks.push(String(chunk));
      process.stderr.write(chunk);
    });
  }
  const [code, signal] = await once(child, 'exit');
  const exitCode = signal === null ? (code ?? 1) : 1;
  commands.push({ command: commandLine, exitCode });
  if (exitCode !== 0) {
    throw new Error(`${commandLine} exited with ${exitCode}`);
  }
  return { exitCode, output: chunks.join('') };
}

async function waitForHttp(url) {
  const deadline = Date.now() + 90_000;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
      lastError = new Error(`${url} returned ${response.status}`);
    } catch (error) {
      lastError = error;
    }
    await delay(500);
  }
  throw lastError instanceof Error ? lastError : new Error(`Timed out waiting for ${url}`);
}

async function writeProof(playwright) {
  const checkedAt = new Date().toISOString();
  const accessibilitySummary = JSON.parse(await readFile(accessibilitySummaryPath, 'utf8'));
  const proof = {
    schemaVersion: 1,
    checkedAt,
    commit: await gitHead(),
    workpackIds: ['social-20-parent-social-dashboard-ux', 'social-23-tests-fixtures-playwright-manual-proof'],
    proofMode: 'real-portal-browser-route-service-backed-social-dashboard',
    route: '#/browser',
    currentStatus: 'service-backed-social-dashboard-rendered',
    productClaimReady: false,
    artifacts: {
      proof: relativePath(proofPath),
      outputProof: relativePath(outputProofPath),
      playwrightLog: playwright.log,
      securityNegativeLog: relativePath(securityLogPath),
      validationCommands: relativePath(validationLogPath),
      desktopScreenshot: relativePath(desktopScreenshot),
      mobileScreenshot: relativePath(mobileScreenshot),
      accessibilitySummary: relativePath(accessibilitySummaryPath),
    },
    serviceBoundary: {
      route: '#/browser',
      source: 'real Vite portal route with real Rust service WebSocket command',
      command: 'agent.browser.social-dashboard.read-model.get',
      event: 'agent.browser.social-dashboard.read-model.reported',
      socialSnapshot: 'reported',
      renderedState: 'seven service-backed social rows',
    },
    assertions: [
      'Portal browser route renders the social dashboard region from the real app shell.',
      'The route starts with zero rows, then requests the service-backed social dashboard read model over the real WebSocket path.',
      'The reported service snapshot renders account approval, feed/video gate, native capability, connector boundary, decision memory, settings/custody, and manual-required gap rows.',
      'The rendered copy explicitly keeps runtime social data, connector authorization, native app control, policy execution, and enforcement unclaimed.',
      'Desktop and mobile screenshots were captured from the real portal route.',
    ],
    nonClaims: [
      'This proof does not claim social connector authorization, account login, feed scraping, notification delivery, or policy enforcement.',
      'This proof does not claim native app control or child-device execution.',
      'This proof does not seed browser-local portal-only rows; rows are returned by the real local Rust service command.',
      'This proof does not claim product readiness for social/video control.',
    ],
    remainingGapsBeforeProductReady: [
      'Runtime settings custody mutation remains pending.',
      'Connector-native runtime and final enforcement proof remain pending.',
    ],
    accessibilitySummary,
    commands,
  };
  const proofContent = `${JSON.stringify(proof, null, 2)}\n`;
  await writeFile(proofPath, proofContent);
  await writeFile(outputProofPath, proofContent);
  await writeFile(
    validationLogPath,
    commands.map((entry) => `${entry.command} # exit ${entry.exitCode}`).join('\n') + '\n'
  );
  await writeFile(
    securityLogPath,
    [
      `checkedAt=${checkedAt}`,
      'asserted=no product-ready social/video copy',
      'asserted=no connector authorized claim',
      'asserted=no native app control claim',
      'asserted=no policy execution or enforcement-active claim',
      'asserted=productClaimReady=false',
    ].join('\n') + '\n'
  );
}

async function gitHead() {
  const result = await runCommand('git', ['rev-parse', 'HEAD'], { capture: true });
  return result.output.trim();
}

function relativePath(value) {
  return path.relative(repoRoot, value).replace(/\\/gu, '/');
}
