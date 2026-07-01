import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
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
import {
  NetworkEvidenceDrawerProofFixture,
  networkActivityEvidence,
  networkActivityFields,
  networkActivityObservedAt,
  networkEvidenceReferenceIds,
} from './network-evidence-drawer-proof-fixture.mjs';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const portalRoot = path.join(repoRoot, 'apps', 'portal');
const proofRoot = path.join(repoRoot, 'output', 'network-plan-proof', '36-parent-ui-network-evidence-drawer');
const screenshotDir = path.join(proofRoot, '08-ui-snapshots');
const proofResultDir = path.join(repoRoot, 'test-results', 'network-parent-ui-evidence-drawer-proof');
const proofPath = path.join(proofResultDir, 'proof.json');
const outputProofPath = path.join(proofRoot, 'proof-summary.json');
const validationLogPath = path.join(proofRoot, '12-validation-commands.log');
const securityLogPath = path.join(proofRoot, '09-security-negative-proof.log');
const playwrightLogPath = path.join(screenshotDir, 'network-evidence-drawer-playwright.log');
const screenshotPath = path.join(screenshotDir, 'network-evidence-drawer.png');
const runRoot = await mkdtemp(path.join(tmpdir(), 'ocentra-parent-network-ui-'));
const activityDbPath = path.join(runRoot, 'activity.sqlite');
const sqlPath = path.join(runRoot, 'seed-network-ui.sql');
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
  await mkdir(screenshotDir, { recursive: true });
  await mkdir(proofResultDir, { recursive: true });
  await seedActivityStore();
  await runNpm(['run', 'build:contracts']);
  await runNpmWorkspace('@ocentra-parent/portal', ['run', 'test', '--', 'live-activity-network-flow']);
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

  console.log('network-parent-ui-evidence-drawer-proof-ok');
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
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function portalEnv() {
  return {
    ...process.env,
    [ParentDevEnv.ActivityDbPath]: activityDbPath,
    [ParentDevEnv.PortalAgentWebSocketUrl]: createAgentWebSocketUrl(agentPort),
    [ParentDevEnv.PortalPort]: String(portalPort),
    NETWORK_EVIDENCE_DRAWER_SCREENSHOT: screenshotPath,
  };
}

function trackChild(child, label) {
  children.push(child);
  child.stdout?.on('data', (chunk) => process.stdout.write(chunk));
  child.stderr?.on('data', (chunk) => process.stderr.write(chunk));
  child.once('exit', (code, signal) => {
    if (!stopping && code !== 0) {
      console.error(`${label} process exited early: code=${code ?? 'null'} signal=${signal ?? 'null'}`);
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
    'network-evidence-drawer-proof.spec.ts',
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

async function seedActivityStore() {
  const fields = networkActivityFields();
  const evidence = networkActivityEvidence();
  const sql = `
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS activity_events (
  event_id TEXT PRIMARY KEY,
  observed_at TEXT NOT NULL,
  device_id TEXT NOT NULL,
  platform TEXT NOT NULL,
  observer TEXT NOT NULL,
  kind TEXT NOT NULL,
  subject_kind TEXT NOT NULL,
  subject_id TEXT NOT NULL,
  subject_display_name TEXT,
  fields_json TEXT NOT NULL,
  evidence_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS activity_events_recent_idx
  ON activity_events (observed_at DESC, event_id DESC);
INSERT INTO activity_events (
  event_id,
  observed_at,
  device_id,
  platform,
  observer,
  kind,
  subject_kind,
  subject_id,
  subject_display_name,
  fields_json,
  evidence_json
) VALUES (
  '${sqlString(NetworkEvidenceDrawerProofFixture.eventId)}',
  '${sqlString(networkActivityObservedAt())}',
  '${sqlString(NetworkEvidenceDrawerProofFixture.deviceId)}',
  '${sqlString(NetworkEvidenceDrawerProofFixture.platform)}',
  '${sqlString(NetworkEvidenceDrawerProofFixture.observer)}',
  '${sqlString(NetworkEvidenceDrawerProofFixture.kind)}',
  '${sqlString(NetworkEvidenceDrawerProofFixture.subjectKind)}',
  '${sqlString(NetworkEvidenceDrawerProofFixture.subjectId)}',
  '${sqlString(NetworkEvidenceDrawerProofFixture.subjectDisplayName)}',
  '${sqlString(JSON.stringify(fields))}',
  '${sqlString(JSON.stringify(evidence))}'
);
`;
  await writeFile(sqlPath, sql);
  const sqlite = resolveSqlite();
  const result = spawnSync(sqlite, [activityDbPath, `.read ${sqlPath}`], { cwd: repoRoot, encoding: 'utf8' });
  commands.push({ command: `${sqlite} ${activityDbPath} .read ${sqlPath}`, exitCode: result.status ?? 1 });
  if (result.status !== 0) {
    throw new Error(`sqlite seed failed: ${result.stderr || result.stdout}`);
  }
}

function resolveSqlite() {
  const result = spawnSync(process.platform === 'win32' ? 'where' : 'which', ['sqlite3'], { encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error('sqlite3 is required for network parent UI evidence drawer proof.');
  }
  return result.stdout.split(/\r?\n/u).find(Boolean);
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
    throw new Error(`${commandLine} failed with exit code ${exitCode}`);
  }
  return { exitCode, output: chunks.join('') };
}

async function waitForHttp(url) {
  const deadline = Date.now() + 90_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // keep waiting
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`Timed out waiting for ${url}`);
}

async function writeProof(playwright) {
  const checkedAt = new Date().toISOString();
  const proof = {
    checkedAt,
    planRow: 36,
    branch: 'codex/eventing-network-runtime-implementation',
    proofMode: 'service-backed-parent-network-evidence-drawer',
    artifacts: {
      proof: relativePath(proofPath),
      outputProof: relativePath(outputProofPath),
      playwrightLog: playwright.log,
      screenshot: relativePath(screenshotPath),
      validationCommands: relativePath(validationLogPath),
      securityNegativeLog: relativePath(securityLogPath),
    },
    serviceBoundary: {
      command: 'agent.network.flow.read-model.get',
      event: 'agent.network.flow.read-model.reported',
      sourceStore: 'temporary ActivityStore SQLite activity_events',
      route: '#/activity',
      evidenceReferenceIds: networkEvidenceReferenceIds(),
    },
    assertions: [
      'Portal renders the network evidence drawer from the real Rust service read model.',
      'Drawer cites service ActivityStore evidence refs from network activity digest.',
      'Drawer keeps exact URL, AI audit, policy, intervention, and retention facets not reported when the service does not provide those refs.',
      'Portal remains a renderer and does not publish policy or adapter commands.',
    ],
    nonClaims: [
      'No decrypted HTTPS/page/message/search content is shown or claimed.',
      'No exact URL is claimed from network-only evidence.',
      'No notification provider delivery, AI model execution, policy decision, or adapter execution is claimed.',
      'The temporary seeded ActivityStore proves the UI/read-model path, not live packet capture.',
    ],
    commands,
  };
  const proofContent = `${JSON.stringify(proof, null, 2)}\n`;
  await writeFile(proofPath, proofContent);
  await writeFile(outputProofPath, proofContent);
  await writeFile(
    validationLogPath,
    commands.map((entry) => `${entry.command} -> ${entry.exitCode}`).join('\n') + '\n'
  );
  await writeFile(
    securityLogPath,
    [
      `checkedAt=${checkedAt}`,
      'asserted=no exact URL claim from network-only evidence',
      'asserted=no decrypted payload, page content, message content, or search query rendering',
      'asserted=no UI-owned policy or adapter command path',
    ].join('\n') + '\n'
  );
}

function sqlString(value) {
  return value.replaceAll("'", "''");
}

function relativePath(filePath) {
  return path.relative(repoRoot, filePath).replaceAll(path.sep, '/');
}
