import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { access, mkdtemp, mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
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
const workpack16 = path.join(repoRoot, 'output', 'tracking-plan-proof', '16-expected-place-schedule-engine');
const workpack17 = path.join(
  repoRoot,
  'output',
  'tracking-plan-proof',
  '17-parent-acknowledgement-and-exception-model'
);
const workpack29 = path.join(repoRoot, 'output', 'tracking-plan-proof', '29-missing-device-mode');
const workpack26 = path.join(repoRoot, 'output', 'tracking-plan-proof', '26-alert-severity-and-notification-model');
const workpack30 = path.join(repoRoot, 'output', 'tracking-plan-proof', '30-parent-and-child-ui-ux-surfaces');
const workpack31 = path.join(
  repoRoot,
  'output',
  'tracking-plan-proof',
  '31-platform-extension-checklists-and-proof-routing'
);
const workpack32 = path.join(repoRoot, 'output', 'tracking-plan-proof', '32-journal-sqlite-and-read-model-proof');
const workpack33 = path.join(repoRoot, 'output', 'tracking-plan-proof', '33-proof-gates-fixtures-rollout-and-pr-gate');
const proofResultDir = path.join(repoRoot, 'test-results', 'tracking-plan-hosted-ui-proof');
const proofPath = path.join(proofResultDir, 'proof.json');
const outputProofPath = path.join(workpack30, '17-hosted-ui-proof.json');
const gateProofPath = path.join(workpack33, '18-hosted-ui-accessibility-proof.json');
const playwrightLogPath = path.join(workpack30, '12-playwright-proof.log');
const securityLogPath = path.join(workpack30, '13-security-negative-proof.log');
const validationLogPath = path.join(workpack30, '16-validation-commands.log');
const desktopScreenshot = path.join(workpack30, '11-ui-snapshots', 'hosted-policy-tracking-live-summary.png');
const mobileScreenshot = path.join(workpack30, '11-ui-snapshots', 'hosted-policy-tracking-live-summary-mobile.png');
const familyDashboardScreenshot = path.join(
  workpack30,
  '11-ui-snapshots',
  'hosted-policy-tracking-family-dashboard-rollup.png'
);
const reportPolicyConsumerScreenshot = path.join(
  workpack30,
  '11-ui-snapshots',
  'hosted-policy-tracking-report-policy-consumer.png'
);
const reportExportScreenshot = path.join(workpack30, '11-ui-snapshots', 'hosted-policy-tracking-report-export.png');
const notificationParentSurfaceScreenshot = path.join(
  workpack30,
  '11-ui-snapshots',
  'hosted-policy-tracking-notification-parent-surface.png'
);
const parentActionReadinessScreenshot = path.join(
  workpack30,
  '11-ui-snapshots',
  'hosted-policy-tracking-parent-action-readiness.png'
);
const missingDeviceScreenshot = path.join(workpack30, '11-ui-snapshots', 'hosted-policy-tracking-missing-device.png');
const retentionSettingsScreenshot = path.join(
  workpack30,
  '11-ui-snapshots',
  'hosted-policy-tracking-retention-settings.png'
);
const evidenceDrawerScreenshot = path.join(workpack30, '11-ui-snapshots', 'hosted-policy-tracking-evidence-drawer.png');
const childCheckInScreenshot = path.join(workpack30, '11-ui-snapshots', 'hosted-policy-tracking-child-check-in.png');
const childRuntimeUiScreenshot = path.join(
  workpack30,
  '11-ui-snapshots',
  'hosted-policy-tracking-child-runtime-ui.png'
);
const parentOverviewShellScreenshot = path.join(workpack30, '11-ui-snapshots', 'hosted-parent-overview-shell.png');
const parentDevicesShellScreenshot = path.join(workpack30, '11-ui-snapshots', 'hosted-parent-devices-shell.png');
const childRuntimeUiProofPath = path.join(workpack30, '19-child-runtime-ui-proof.json');
const evidenceDrawerHostedUiProofPath = path.join(workpack30, '20-evidence-drawer-hosted-ui-proof.json');
const reportPolicyConsumerHostedUiProofPath = path.join(workpack30, '25-report-policy-consumer-hosted-ui-proof.json');
const reportPolicyConsumerWp32HostedUiProofPath = path.join(
  workpack32,
  '32-report-policy-consumer-hosted-ui-proof.json'
);
const reportPolicyConsumerWp33HostedUiProofPath = path.join(
  workpack33,
  '39-report-policy-consumer-hosted-ui-proof.json'
);
const reportExportHostedUiProofPath = path.join(workpack30, '21-report-export-hosted-ui-proof.json');
const reportExportWp32HostedUiProofPath = path.join(workpack32, '29-report-export-hosted-ui-proof.json');
const notificationParentSurfaceHostedUiProofPath = path.join(
  workpack30,
  '22-notification-parent-surface-hosted-ui-proof.json'
);
const notificationParentSurfaceWp26HostedUiProofPath = path.join(
  workpack26,
  '27-notification-parent-surface-hosted-ui-proof.json'
);
const notificationParentSurfaceWp33HostedUiProofPath = path.join(
  workpack33,
  '35-notification-parent-surface-hosted-ui-proof.json'
);
const parentActionReadinessHostedUiProofPath = path.join(workpack30, '23-parent-action-readiness-hosted-ui-proof.json');
const parentActionReadinessWp16HostedUiProofPath = path.join(
  workpack16,
  '30-expected-place-alert-policy-hosted-ui-proof.json'
);
const parentActionReadinessWp17HostedUiProofPath = path.join(
  workpack17,
  '31-parent-acknowledgement-action-hosted-ui-proof.json'
);
const parentActionReadinessWp33HostedUiProofPath = path.join(
  workpack33,
  '36-parent-action-readiness-hosted-ui-proof.json'
);
const missingDeviceHostedUiProofPath = path.join(workpack30, '24-missing-device-hosted-ui-proof.json');
const missingDeviceWp29HostedUiProofPath = path.join(workpack29, '20-missing-device-hosted-ui-proof.json');
const missingDeviceWp33HostedUiProofPath = path.join(workpack33, '37-missing-device-hosted-ui-proof.json');
const unsupportedManualScreenshot = path.join(workpack31, '19-unsupported-manual-hosted-ui.png');
const unsupportedManualHostedProofPath = path.join(workpack31, '19-unsupported-manual-hosted-ui-proof.json');
const accessibilitySummaryPath = path.join(proofResultDir, 'accessibility-summary.json');
const runRoot = await mkdtemp(path.join(tmpdir(), 'ocentra-parent-tracking-hosted-ui-'));
const devLogDir = path.join(runRoot, 'dev-log');
const activityDbPath = path.join(runRoot, 'activity.sqlite');
const sqlPath = path.join(runRoot, 'seed-tracking.sql');
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
  await mkdir(workpack26, { recursive: true });
  await mkdir(workpack29, { recursive: true });
  await mkdir(workpack30, { recursive: true });
  await mkdir(workpack31, { recursive: true });
  await mkdir(workpack32, { recursive: true });
  await mkdir(workpack33, { recursive: true });
  await mkdir(proofResultDir, { recursive: true });
  await seedActivityStore();
  await ensureWindowsOptionalNodeDependencies();
  await runNpm([
    'exec',
    '--workspace',
    '@ocentra-parent/portal',
    '--',
    'vitest',
    'run',
    'tests/tracking-status-panel.test.ts',
  ]);
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
  await assertPortalDevLogWritten();
  await writeProof(playwright);

  console.log('tracking-plan-hosted-ui-proof-ok');
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
    TRACKING_PLAN_HOSTED_UI_PROOF: '1',
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
    'tracking-hosted-ui-proof.spec.ts',
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
  const fields = {
    capabilityStatus: 'recent',
    evidenceReferenceIds: 'location-evidence-hosted-1,location-evidence-hosted-2',
  };
  const evidence = [
    {
      evidenceId: 'location-evidence-hosted-1',
      kind: 'local-db-row',
      digest: 'sha256:tracking-hosted-location-row',
      uri: null,
    },
    {
      evidenceId: 'location-evidence-hosted-2',
      kind: 'journal-entry',
      digest: 'sha256:tracking-hosted-journal-entry',
      uri: null,
    },
  ];
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
DELETE FROM activity_events;
INSERT INTO activity_events VALUES (
  'tracking-hosted-location-event',
  '2026-06-04T10:09:00.000Z',
  'child-android-hosted-proof',
  'android',
  'android-location',
  'activity.location.observed',
  'location',
  'school-location',
  'School location',
  ${sqlString(JSON.stringify(fields))},
  ${sqlString(JSON.stringify(evidence))}
);
INSERT INTO activity_events VALUES (
  'tracking-hosted-expected-place-event',
  '2026-06-04T10:10:00.000Z',
  'child-android-hosted-proof',
  'android',
  'tracking-engine',
  'activity.tracking.expected-place.evaluated',
  'tracking-rule',
  'expected-place-school',
  'School',
  ${sqlString(JSON.stringify(fields))},
  ${sqlString(JSON.stringify(evidence))}
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
    throw new Error('sqlite3 is required for tracking hosted UI proof.');
  }
  return result.stdout.split(/\r?\n/u).find(Boolean);
}

async function runNpm(args, ...rest) {
  const command = process.platform === 'win32' ? 'npm.cmd' : 'npm';
  await runCommand(command, args, ...rest);
}

async function ensureWindowsOptionalNodeDependencies() {
  if (process.platform !== 'win32') {
    return;
  }

  const bindingPackage = '@rolldown/binding-win32-x64-msvc';
  const bindingPackageJson = path.join(repoRoot, 'node_modules', '@rolldown', 'binding-win32-x64-msvc', 'package.json');

  try {
    await access(bindingPackageJson);
    return;
  } catch {}

  const rolldownPackageJson = JSON.parse(
    await readFile(path.join(repoRoot, 'node_modules', 'rolldown', 'package.json'), 'utf8')
  );
  const bindingVersion = rolldownPackageJson.optionalDependencies?.[bindingPackage] ?? rolldownPackageJson.version;
  await runNpm(['install', '--no-save', '--ignore-scripts', `${bindingPackage}@${bindingVersion}`]);
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
  return {
    exitCode,
    output: chunks.join(''),
  };
}

async function waitForHttp(url, timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;
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
    await delay(250);
  }
  throw lastError instanceof Error ? lastError : new Error(`Timed out waiting for ${url}`);
}

async function assertPortalDevLogWritten() {
  const content = await waitForDevLogContent('portal-', 'Portal command sent.');
  if (!content.includes('Portal WebSocket event received.')) {
    throw new Error(`Portal dev log did not include WebSocket event entry:\n${content}`);
  }
}

async function waitForDevLogContent(prefix, expectedText) {
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    const files = await readdir(devLogDir);
    const logFile = files.find((file) => file.startsWith(prefix) && file.endsWith('.ndjson'));
    if (logFile !== undefined) {
      const content = await readFile(path.join(devLogDir, logFile), 'utf8');
      if (content.includes(expectedText)) {
        return content;
      }
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for dev log ${prefix} in ${devLogDir}`);
}

async function writeProof(playwright) {
  const checkedAt = new Date().toISOString();
  const accessibilitySummary = JSON.parse(await readFile(accessibilitySummaryPath, 'utf8'));
  const proof = {
    schemaVersion: 1,
    checkedAt,
    commit: await gitHead(),
    workpackIds: [
      '30-parent-and-child-ui-ux-surfaces',
      '31-platform-extension-checklists-and-proof-routing',
      '32-journal-sqlite-and-read-model-proof',
      '33-proof-gates-fixtures-rollout-and-pr-gate',
    ],
    proofMode: 'tracking-hosted-portal-screenshot-accessibility-proof',
    requiredProofTier: 'P2_HOSTED_CI',
    currentProofTier: 'P2_HOSTED_CI',
    currentStatus: 'proved',
    productClaimReady: false,
    activityStoreSeed: {
      rowsSeeded: 2,
      latestEventId: 'tracking-hosted-expected-place-event',
      latestObservedAt: '2026-06-04T10:10:00.000Z',
      evidenceReferenceIds: ['location-evidence-hosted-1', 'location-evidence-hosted-2'],
    },
    serviceBoundary: {
      command: 'agent.activity.tracking.read-model.get',
      event: 'agent.activity.tracking.read-model.reported',
      payloadField: 'trackingReadModel',
      sourceStore: 'temporary ActivityStore SQLite activity_events',
      route: '#/policy-tracking',
    },
    artifacts: {
      proof: relativePath(proofPath),
      workpack30Proof: relativePath(outputProofPath),
      workpack33Proof: relativePath(gateProofPath),
      playwrightLog: playwright.log,
      securityNegativeLog: relativePath(securityLogPath),
      validationCommands: relativePath(validationLogPath),
      desktopScreenshot: relativePath(desktopScreenshot),
      mobileScreenshot: relativePath(mobileScreenshot),
      familyDashboardScreenshot: relativePath(familyDashboardScreenshot),
      reportPolicyConsumerScreenshot: relativePath(reportPolicyConsumerScreenshot),
      reportPolicyConsumerHostedUiProof: relativePath(reportPolicyConsumerHostedUiProofPath),
      reportPolicyConsumerWp32HostedUiProof: relativePath(reportPolicyConsumerWp32HostedUiProofPath),
      reportPolicyConsumerWp33HostedUiProof: relativePath(reportPolicyConsumerWp33HostedUiProofPath),
      reportExportScreenshot: relativePath(reportExportScreenshot),
      reportExportHostedUiProof: relativePath(reportExportHostedUiProofPath),
      reportExportWp32HostedUiProof: relativePath(reportExportWp32HostedUiProofPath),
      notificationParentSurfaceScreenshot: relativePath(notificationParentSurfaceScreenshot),
      notificationParentSurfaceHostedUiProof: relativePath(notificationParentSurfaceHostedUiProofPath),
      notificationParentSurfaceWp26HostedUiProof: relativePath(notificationParentSurfaceWp26HostedUiProofPath),
      notificationParentSurfaceWp33HostedUiProof: relativePath(notificationParentSurfaceWp33HostedUiProofPath),
      parentActionReadinessScreenshot: relativePath(parentActionReadinessScreenshot),
      parentActionReadinessHostedUiProof: relativePath(parentActionReadinessHostedUiProofPath),
      parentActionReadinessWp16HostedUiProof: relativePath(parentActionReadinessWp16HostedUiProofPath),
      parentActionReadinessWp17HostedUiProof: relativePath(parentActionReadinessWp17HostedUiProofPath),
      parentActionReadinessWp33HostedUiProof: relativePath(parentActionReadinessWp33HostedUiProofPath),
      missingDeviceScreenshot: relativePath(missingDeviceScreenshot),
      missingDeviceHostedUiProof: relativePath(missingDeviceHostedUiProofPath),
      missingDeviceWp29HostedUiProof: relativePath(missingDeviceWp29HostedUiProofPath),
      missingDeviceWp33HostedUiProof: relativePath(missingDeviceWp33HostedUiProofPath),
      retentionSettingsScreenshot: relativePath(retentionSettingsScreenshot),
      retentionLocalServiceStateProof:
        'output/tracking-plan-proof/07-retention-and-custody-model/22-retention-local-service-state-proof.json',
      evidenceDrawerScreenshot: relativePath(evidenceDrawerScreenshot),
      evidenceDrawerHostedUiProof: relativePath(evidenceDrawerHostedUiProofPath),
      childCheckInScreenshot: relativePath(childCheckInScreenshot),
      childRuntimeUiScreenshot: relativePath(childRuntimeUiScreenshot),
      parentOverviewShellScreenshot: relativePath(parentOverviewShellScreenshot),
      parentDevicesShellScreenshot: relativePath(parentDevicesShellScreenshot),
      childRuntimeUiProof: relativePath(childRuntimeUiProofPath),
      unsupportedManualPlatformScreenshot: relativePath(unsupportedManualScreenshot),
      unsupportedManualPlatformProof: relativePath(unsupportedManualHostedProofPath),
      accessibilitySummary: relativePath(accessibilitySummaryPath),
    },
    accessibilitySummary,
    parentPortalShellProof: {
      routeScope: ['#/overview', '#/devices'],
      screenshots: {
        overview: relativePath(parentOverviewShellScreenshot),
        devices: relativePath(parentDevicesShellScreenshot),
      },
      assertions: [
        'parent-overview-shell-visible',
        'parent-overview-custody-copy-visible',
        'parent-devices-shell-visible',
        'parent-devices-context-copy-visible',
      ],
      boundary:
        'Parent portal shell screenshot proof only; it does not claim child-device runtime delivery, authority enrollment, provider delivery, physical-device proof, production proof, or product-ready tracking.',
      productClaimReady: false,
    },
    commands,
    childRuntimeUiProof: {
      screenshot: relativePath(childRuntimeUiScreenshot),
      assertions: [
        'tracking-request-disclosure-visible',
        'safe-response-visible',
        'help-response-visible',
        'location-share-consent-visible',
        'hosted-proof-only-boundary-visible',
        'child-device-delivery-not-claimed',
        'no-product-claim-visible',
      ],
      productClaimReady: false,
    },
    familyDashboardHostedRollupProof: {
      sourceProof:
        'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/23-family-dashboard-rollup-proof.json',
      screenshot: relativePath(familyDashboardScreenshot),
      renderedRows: ['family-active-summary', 'child-attention-summary', 'retention-audit-summary'],
      childDeviceDeliveryClaimedRows: 0,
      providerDeliveryClaimedRows: 0,
      notificationReceiptClaimedRows: 0,
      physicalDeviceClaimedRows: 0,
      authorityClaimedRows: 0,
      productClaimReadyRows: 0,
      productClaimReady: false,
    },
    reportPolicyConsumerHostedUiProof: {
      sourceProofs: [
        'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/22-report-policy-consumer-proof.json',
        'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/21-product-surface-summary-proof.json',
      ],
      screenshot: relativePath(reportPolicyConsumerScreenshot),
      renderedRows: ['parent-report-summary', 'policy-evidence-drill-in', 'retention-audit-export'],
      storedJournalRefsRequired: true,
      storedReadModelRefsRequired: true,
      aiExecutionClaimedRows: 0,
      policyMutationClaimedRows: 0,
      platformRuntimeClaimedRows: 0,
      childDeviceDeliveryClaimedRows: 0,
      providerDeliveryClaimedRows: 0,
      notificationReceiptClaimedRows: 0,
      physicalDeviceClaimedRows: 0,
      authorityClaimedRows: 0,
      productClaimReadyRows: 0,
      productClaimReady: false,
    },
    reportExportHostedUiProof: {
      sourceProof:
        'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/28-report-export-read-model-proof.json',
      screenshot: relativePath(reportExportScreenshot),
      renderedRows: [
        'redacted-report-packet',
        'retention-audit-export-packet',
        'family-dashboard-summary-packet',
        'policy-drill-in-export-packet',
      ],
      rawLocationPayloadClaimedRows: 0,
      serviceMutationClaimedRows: 0,
      platformRuntimeClaimedRows: 0,
      childDeviceDeliveryClaimedRows: 0,
      providerDeliveryClaimedRows: 0,
      notificationReceiptClaimedRows: 0,
      physicalDeviceClaimedRows: 0,
      authorityClaimedRows: 0,
      productClaimReadyRows: 0,
      productClaimReady: false,
    },
    notificationParentSurfaceHostedUiProof: {
      sourceProof:
        'output/tracking-plan-proof/26-alert-severity-and-notification-model/26-notification-parent-surface-history-proof.json',
      screenshot: relativePath(notificationParentSurfaceScreenshot),
      renderedRows: ['history-intent-ready', 'manual-action-required', 'provider-unavailable'],
      parentPreferenceMutationRows: 0,
      providerDeliveryClaimedRows: 0,
      receiptIngestionClaimedRows: 0,
      childDeviceDeliveryClaimedRows: 0,
      physicalDeviceClaimedRows: 0,
      authorityClaimedRows: 0,
      productionStorageClaimedRows: 0,
      adapterDispatchClaimedRows: 0,
      productClaimReadyRows: 0,
      productClaimReady: false,
    },
    parentActionReadinessHostedUiProof: {
      expectedPlaceSourceProof:
        'output/tracking-plan-proof/16-expected-place-schedule-engine/29-expected-place-alert-policy-proof.json',
      acknowledgementSourceProof:
        'output/tracking-plan-proof/17-parent-acknowledgement-and-exception-model/30-parent-acknowledgement-action-readiness-proof.json',
      screenshot: relativePath(parentActionReadinessScreenshot),
      renderedRows: [
        'alert-policy-ready',
        'check-in-policy-ready',
        'suppressed-no-action',
        'manual-required',
        'acknowledgement-recorded',
        'exception-active',
        'false-alarm-recorded',
        'child-check-in-request-ready',
        'escalation-review-ready',
      ],
      expectedPlaceRows: 4,
      acknowledgementActionRows: 5,
      liveServiceMutationRows: 0,
      alertDeliveryClaimedRows: 0,
      providerDeliveryClaimedRows: 0,
      notificationReceiptClaimedRows: 0,
      childDeviceRuntimeClaimedRows: 0,
      physicalDeviceClaimedRows: 0,
      authorityClaimedRows: 0,
      productionWorkerClaimedRows: 0,
      adapterDispatchClaimedRows: 0,
      productClaimReady: false,
    },
    missingDeviceHostedUiProof: {
      sourceProof: 'output/tracking-plan-proof/29-missing-device-mode/proof.json',
      screenshot: relativePath(missingDeviceScreenshot),
      renderedRows: ['last-known-only', 'offline', 'contact-requested', 'manual-required'],
      lastKnownOnlyRows: 1,
      offlineRows: 1,
      contactRequestedRows: 1,
      manualRequiredRows: 1,
      currentLocationRuntimeClaimedRows: 0,
      poweredOffTrackingClaimedRows: 0,
      remoteSyncRuntimeClaimedRows: 0,
      providerDeliveryClaimedRows: 0,
      physicalDeviceProofClaimedRows: 0,
      osLostModeApiClaimedRows: 0,
      productClaimReady: false,
    },
    retentionSettingsHostedUiProof: {
      sourceProof:
        'output/tracking-plan-proof/32-journal-sqlite-and-read-model-proof/24-retention-settings-read-model-proof.json',
      writeCommandProof:
        'output/tracking-plan-proof/07-retention-and-custody-model/21-retention-settings-write-command-proof.json',
      localServiceStateProof:
        'output/tracking-plan-proof/07-retention-and-custody-model/22-retention-local-service-state-proof.json',
      mutationProof:
        'output/tracking-plan-proof/07-retention-and-custody-model/20-retention-settings-mutation-proof.json',
      screenshot: relativePath(retentionSettingsScreenshot),
      renderedRows: [
        'retention-window-setting',
        'delete-after-alert-setting',
        'parent-export-setting',
        'remote-sync-disabled-setting',
        'remote-ai-disabled-setting',
      ],
      writePreflight: {
        command: 'agent.activity.tracking.retention-settings.write',
        event: 'agent.activity.tracking.retention-settings.write.reported',
        settingsKind: 'retention-window-setting',
        writeState: 'service-write-command-accepted',
        commandTransportClaimedRows: 1,
        serviceWritePreflightClaimedRows: 1,
        serviceMutationExecutedRows: 1,
        localServiceStateReadbackClaimedRows: 1,
        localServiceStateRevision: 1,
        localServiceStateSnapshotRef: 'agent-service-local-retention-settings-state',
        durableSettingsPersistedRows: 1,
        appliedRetentionWindowHours: 168,
        remoteSyncEnabledRows: 0,
        remoteAiEnabledRows: 0,
        portalResultRendered: true,
        productClaimReady: false,
      },
      serviceMutationClaimedRows: 0,
      platformRuntimeClaimedRows: 0,
      childDeviceDeliveryClaimedRows: 0,
      providerDeliveryClaimedRows: 0,
      physicalDeviceClaimedRows: 0,
      authorityClaimedRows: 0,
      productClaimReadyRows: 0,
      productClaimReady: false,
    },
    evidenceDrawerHostedUiProof: {
      sourceEventId: 'tracking-hosted-expected-place-event',
      sourceProof:
        'output/tracking-plan-proof/30-parent-and-child-ui-ux-surfaces/20-evidence-drawer-hosted-ui-proof.json',
      screenshot: relativePath(evidenceDrawerScreenshot),
      evidenceReferenceIds: ['location-evidence-hosted-1', 'location-evidence-hosted-2'],
      drawerMode: 'read-only evidence drawer',
      policyEvaluatorClaimedRows: 0,
      actionDispatchClaimedRows: 0,
      childDeviceDeliveryClaimedRows: 0,
      providerDeliveryClaimedRows: 0,
      physicalDeviceClaimedRows: 0,
      authorityClaimedRows: 0,
      productClaimReady: false,
    },
    unsupportedManualPlatformProof: {
      sourceProof: 'output/tracking-plan-proof/unsupported-platform-manual-proof/proof.json',
      screenshot: relativePath(unsupportedManualScreenshot),
      rowCount: 7,
      renderedStates: {
        manualRequired: 5,
        unavailable: 1,
        authorityRequired: 1,
      },
      fakeCapabilityRows: 0,
      productClaimReadyRows: 0,
      physicalDeviceClaimedRows: 0,
      authorityClaimedRows: 0,
      productClaimReady: false,
    },
    nonClaims: [
      'This proof does not claim Android or iOS physical background tracking behavior.',
      'This proof does not claim real physical-device location, geofence, provider, or notification delivery.',
      'This proof uses a seeded temporary ActivityStore SQLite database to prove hosted portal rendering against the real Rust service command.',
      'This proof renders a read-only evidence drawer from the selected service-backed citation but does not claim policy evaluation, action dispatch, child-device delivery, provider delivery, physical-device proof, authority, or product readiness.',
      'This proof renders child runtime UI disclosure, safe/help responses, and location-share consent copy but does not claim child-device delivery or physical-device execution.',
      'This proof renders parent report summary, policy drill-in, and retention audit consumer rows from stored journal/read-model refs but does not claim AI execution, product policy mutation, platform runtime, child-device delivery, provider delivery, notification receipt ingestion, physical-device proof, authority, production, or product readiness.',
      'This proof renders report/export read-model packet rows but does not claim raw location payload export, service mutation, platform runtime, child-device delivery, provider delivery, notification receipt ingestion, physical-device proof, authority, or product-ready export behavior.',
      'This proof renders notification parent-surface history/preference rows but does not claim preference mutation, quiet-hours runtime, provider delivery, receipt ingestion, child-device delivery, physical-device proof, authority, production storage, adapter dispatch, or product readiness.',
      'This proof renders parent action readiness rows for expected-place alert policy and parent acknowledgement actions but does not claim live service mutation, alert delivery, provider delivery, receipt ingestion, child-device runtime, physical-device proof, authority, production workers, adapter dispatch, or product readiness.',
      'This proof renders missing-device last-known, offline, contact-requested, and manual-required rows but does not claim current-location runtime, powered-off tracking, remote sync, provider delivery, physical-device proof, OS lost-mode API execution, authority, production workers, or product readiness.',
      'This proof renders retention settings read-model rows and local service execution proof with a local service state revision and local durable settings persistence but does not claim product-ready writable settings, durable production persistence, or platform runtime execution.',
      'This proof sends and renders a typed retention settings write result with local service mutation execution and local state snapshot ref but does not claim product-ready service behavior, platform runtime execution, child-device delivery, provider delivery, physical-device proof, authority, or production behavior.',
      'This proof renders unsupported/manual platform rows in the hosted portal but does not claim physical-device execution, authority enrollment, provider delivery, or product-ready tracking.',
      'This proof does not claim full child-device UI or authority-enrolled hard-control readiness.',
    ],
    remainingGapsBeforeProductReady: [
      'Full child/parent tracking UI beyond the first tracking proof route remains pending.',
      'Android/iOS physical-device foreground/background proof remains pending.',
      'Authority-enrolled hard-control and production pilot proof remain absent.',
    ],
  };
  const proofContent = `${JSON.stringify(proof, null, 2)}\n`;
  await writeFile(proofPath, proofContent);
  await writeFile(outputProofPath, proofContent);
  await writeFile(gateProofPath, proofContent);
  await writeFile(childRuntimeUiProofPath, proofContent);
  await writeFile(evidenceDrawerHostedUiProofPath, proofContent);
  await writeFile(reportPolicyConsumerHostedUiProofPath, proofContent);
  await writeFile(reportPolicyConsumerWp32HostedUiProofPath, proofContent);
  await writeFile(reportPolicyConsumerWp33HostedUiProofPath, proofContent);
  await writeFile(reportExportHostedUiProofPath, proofContent);
  await writeFile(reportExportWp32HostedUiProofPath, proofContent);
  await writeFile(notificationParentSurfaceHostedUiProofPath, proofContent);
  await writeFile(notificationParentSurfaceWp26HostedUiProofPath, proofContent);
  await writeFile(notificationParentSurfaceWp33HostedUiProofPath, proofContent);
  await writeFile(parentActionReadinessHostedUiProofPath, proofContent);
  await writeFile(parentActionReadinessWp16HostedUiProofPath, proofContent);
  await writeFile(parentActionReadinessWp17HostedUiProofPath, proofContent);
  await writeFile(parentActionReadinessWp33HostedUiProofPath, proofContent);
  await writeFile(missingDeviceHostedUiProofPath, proofContent);
  await writeFile(missingDeviceWp29HostedUiProofPath, proofContent);
  await writeFile(missingDeviceWp33HostedUiProofPath, proofContent);
  await writeFile(unsupportedManualHostedProofPath, proofContent);
  await writeFile(
    securityLogPath,
    [
      `checkedAt=${checkedAt}`,
      'asserted=no browser console or page errors',
      'asserted=no product-ready or physical-device-proved route copy',
      'asserted=family dashboard rollup rows render from existing proof refs without product-ready claim',
      'asserted=family dashboard rollup screenshot captured',
      'asserted=report policy consumer rows render stored journal/read-model refs without product-ready claim',
      'asserted=report policy consumer screenshot captured',
      'asserted=report export packet rows render from existing read-model proof refs without product-ready claim',
      'asserted=report export packet screenshot captured',
      'asserted=notification parent-surface history rows render from existing notification proof refs without provider delivery or receipt runtime claims',
      'asserted=notification parent-surface history screenshot captured',
      'asserted=parent action readiness rows render expected-place alert policy and parent acknowledgement actions without live mutation, delivery, receipt, child-device runtime, authority, or product claims',
      'asserted=parent action readiness screenshot captured',
      'asserted=missing-device rows render last-known, offline, contact-requested, and manual-required states without current-location runtime, powered-off tracking, remote sync, provider delivery, physical-device proof, OS lost-mode API execution, authority, production worker, or product claims',
      'asserted=missing-device screenshot captured',
      'asserted=evidence drawer renders selected service-backed citation without evaluator or dispatch claims',
      'asserted=evidence drawer screenshot captured',
      'asserted=retention settings local write command button clicked',
      'asserted=retention settings local write service result rendered',
      'asserted=retention settings write result keeps productClaimReady=false while serviceMutationExecutedRows=1 and durableSettingsPersistedRows=1',
      'asserted=manual proof required and physical device proof required labels visible',
      'asserted=child check-in copy and actions visible without child-device delivery claim',
      'asserted=child runtime UI disclosure, safe/help response, and location-share consent copy visible',
      'asserted=hosted proof only boundary visible for child runtime UI',
      'asserted=unsupported/manual platform rows render manual-required, unavailable, and authority-required states',
      'asserted=unsupported/manual platform rows keep fakeCapabilityRows=0 and productClaimReady=false',
      'asserted=productClaimReady=false',
    ].join('\n') + '\n'
  );
  await writeFile(
    validationLogPath,
    `${commands.map(({ command, exitCode }) => `${command} # exit ${exitCode}`).join('\n')}\n`
  );
}

async function gitHead() {
  const result = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error('git rev-parse HEAD failed');
  }
  return result.stdout.trim();
}

function sqlString(value) {
  return `'${value.replaceAll("'", "''")}'`;
}

function relativePath(value) {
  return path.relative(repoRoot, value).replace(/\\/gu, '/');
}
