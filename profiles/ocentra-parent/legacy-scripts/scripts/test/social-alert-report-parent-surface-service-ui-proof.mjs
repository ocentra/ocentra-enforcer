import { spawnSync } from 'node:child_process';
import { copyFile, mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofName = 'social-alert-report-parent-surface-service-ui-proof';
const resultDir = join(repoRoot, 'test-results', proofName);
const proofDir = join(repoRoot, 'output', 'browser-plan-proof', proofName);
const screenshotDir = join(proofDir, '06-ui-snapshots');
const accessibilitySummaryPath = join(resultDir, 'accessibility-summary.json');
const proofPath = join(resultDir, 'proof.json');
const markdownPath = join(proofDir, '01-social-alert-report-parent-surface-service-ui-proof.md');
const socialIntentProofDir = join(repoRoot, 'output', 'browser-plan-proof', 'social-alert-report-intent-ui-proof');
const socialIntentScreenshotDir = join(socialIntentProofDir, '06-ui-snapshots');
const socialIntentAccessibilitySummaryPath = join(
  repoRoot,
  'test-results',
  'social-alert-report-intent-ui-proof',
  'accessibility-summary.json'
);

const commands = [];

await mkdir(resultDir, { recursive: true });
await mkdir(proofDir, { recursive: true });

runNpm(['run', 'build:contracts']);
run('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
run('cargo', ['test', '-p', 'ocentra-parent-agent-service', 'social_alert_report_parent_surface', '--quiet']);
runNpm([
  '--workspace',
  '@ocentra-parent/agent-protocol-domain',
  'run',
  'test',
  '--',
  'social-alert-report-parent-surface-read-model.test.ts',
]);
runNpm([
  '--workspace',
  '@ocentra-parent/portal-domain',
  'run',
  'test',
  '--',
  'social-alert-report-parent-surface-panel.test.ts',
]);
runNpm(['run', 'test:e2e', '--workspace', '@ocentra-parent/portal', '--', 'social-alert-report-ui-proof.spec.ts'], {
  SOCIAL_ALERT_REPORT_UI_PROOF: '1',
});

await copyProofArtifacts();
const accessibilitySummary = await readJson(accessibilitySummaryPath);
const screenshotProof = await screenshotArtifacts();
const sourceProof = await sourceAssertions();

const proof = {
  schemaVersion: 1,
  proofName,
  generatedAt: new Date().toISOString(),
  branch: gitOutput(['rev-parse', '--abbrev-ref', 'HEAD']),
  commit: gitOutput(['rev-parse', 'HEAD']),
  commands,
  assertions: [
    'real-rust-service-backed-websocket-command',
    'real-portal-e2e-harness',
    'browser-route-social-parent-surface-command',
    'parent-surface-manual-action-required-row-visible',
    'parent-surface-unavailable-row-visible',
    'desktop-screenshot',
    'mobile-screenshot',
    'internal-ocentra-eventing-request-response-boundary',
    'provider-status-and-preference-status-handoff-inputs',
    'no-parent-notification-ui-delivery-claim',
    'no-parent-preference-ui-delivery-claim',
    'no-provider-delivery-or-receipt-ingestion-claim',
    'no-final-policy-or-enforcement-claim',
  ],
  sourceProof,
  accessibilitySummary,
  screenshotProof,
  noClaimBoundaries: [
    'parent notification UI delivery',
    'parent notification preference UI delivery',
    'parent notification history UI delivery',
    'provider delivery',
    'provider receipt ingestion',
    'child delivery',
    'quiet-hours timer runtime',
    'report delivery execution',
    'final policy execution',
    'enforcement',
  ],
  proofPaths: {
    proof: toRepoPath(proofPath),
    markdown: toRepoPath(markdownPath),
    accessibilitySummary: toRepoPath(accessibilitySummaryPath),
    screenshots: screenshotProof.map((artifact) => artifact.path),
  },
};

assertProof(proof);
await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`, 'utf8');
await writeFile(markdownPath, markdown(proof), 'utf8');

console.log('social-alert-report-parent-surface-service-ui-proof-ok=true');
console.log(`proof=${toRepoPath(proofPath)}`);

function run(command, args, env = {}) {
  const rendered = [command, ...args].join(' ');
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    env: {
      ...process.env,
      ...env,
    },
    encoding: 'utf8',
    stdio: 'pipe',
  });
  commands.push({
    command: rendered,
    exitCode: result.status,
    stdout: result.stdout.slice(-4000),
    stderr: result.stderr.slice(-4000),
  });
  if (result.status !== 0) {
    throw new Error(`Command failed (${result.status}): ${rendered}\n${result.stdout}\n${result.stderr}`);
  }
}

function gitOutput(args) {
  const result = spawnSync('git', args, {
    cwd: repoRoot,
    encoding: 'utf8',
    stdio: 'pipe',
  });
  if (result.status !== 0) {
    throw new Error(`git ${args.join(' ')} failed: ${result.stderr}`);
  }
  return result.stdout.trim();
}

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'));
}

async function copyProofArtifacts() {
  await mkdir(resultDir, { recursive: true });
  await mkdir(screenshotDir, { recursive: true });
  await copyFile(socialIntentAccessibilitySummaryPath, accessibilitySummaryPath);
  await Promise.all(
    ['social-alert-report-browser-route.png', 'social-alert-report-browser-route-mobile.png'].map(async (file) => {
      const target = join(screenshotDir, file);
      await mkdir(dirname(target), { recursive: true });
      await copyFile(join(socialIntentScreenshotDir, file), target);
    })
  );
}

async function screenshotArtifacts() {
  return Promise.all(
    ['social-alert-report-browser-route.png', 'social-alert-report-browser-route-mobile.png'].map(async (file) => {
      const path = join(screenshotDir, file);
      const bytes = await readFile(path);
      const stats = await stat(path);
      if (bytes.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a') {
        throw new Error(`Screenshot is not a PNG: ${file}`);
      }
      return {
        path: toRepoPath(path),
        bytes: stats.size,
        width: bytes.readUInt32BE(16),
        height: bytes.readUInt32BE(20),
      };
    })
  );
}

async function sourceAssertions() {
  const routeSource = await readFile(
    join(repoRoot, 'apps', 'portal', 'src', 'SocialAlertReportRoutePanel.tsx'),
    'utf8'
  );
  const serviceSource = await readFile(
    join(
      repoRoot,
      'crates',
      'agent-service',
      'src',
      'activity_api',
      'social_alert_report_parent_surface_read_model_payload.rs'
    ),
    'utf8'
  );
  const protocolSource = await readFile(
    join(repoRoot, 'crates', 'agent-protocol', 'src', 'social_alert_report_parent_surface_read_model.rs'),
    'utf8'
  );
  const e2eSource = await readFile(
    join(repoRoot, 'apps', 'portal', 'e2e', 'social-alert-report-ui-proof.spec.ts'),
    'utf8'
  );
  assertIncludes(
    routeSource,
    'BrowserSocialAlertReportParentSurfaceReadModelGet',
    'route sends parent-surface read-model command'
  );
  assertIncludes(routeSource, 'SocialAlertReportParentSurfaceCards', 'route renders parent-surface cards');
  assertIncludes(
    serviceSource,
    'request_social_alert_report_parent_surface_read_model_from_service',
    'service uses evented parent-surface request'
  );
  assertIncludes(
    serviceSource,
    'request_social_provider_status_handoff_from_service',
    'service asks provider-status handoff before parent-surface projection'
  );
  assertIncludes(
    serviceSource,
    'request_social_preference_status_handoff_from_service',
    'service asks preference-status handoff before parent-surface projection'
  );
  assertIncludes(
    serviceSource,
    'EVENT_BROWSER_SOCIAL_ALERT_REPORT_PARENT_SURFACE_STATUS_REQUESTED',
    'service publishes named social parent-surface status request'
  );
  assertIncludes(
    serviceSource,
    'SUBSCRIBER_BROWSER_SOCIAL_ALERT_REPORT_PARENT_SURFACE_STATUS',
    'service registers social parent-surface status subscriber'
  );
  assertIncludes(
    serviceSource,
    'source_preference_status_handoff_id',
    'service keeps preference status handoff source'
  );
  assertIncludes(protocolSource, 'parent-surface-status-ref-only', 'protocol names minimal payload boundary');
  assertIncludes(e2eSource, 'Parent surface manual action required', 'e2e asserts manual parent-surface row');
  assertIncludes(e2eSource, 'Parent surface unavailable', 'e2e asserts unavailable parent-surface row');
  assertIncludes(
    serviceSource,
    'parent_notification_ui_rendered: false',
    'service keeps parent notification UI unclaimed'
  );
  assertIncludes(
    serviceSource,
    'provider_receipt_claimed: false',
    'service keeps provider receipt ingestion unclaimed'
  );
  return {
    routeSendsParentSurfaceCommand: true,
    routeRendersParentSurfaceCards: true,
    serviceUsesNamedLocalEventingRequest: true,
    servicePreservesProviderAndPreferenceStatusSources: true,
    servicePreservesNoClaimBoundaries: true,
    e2eRequiresParentSurfaceRows: true,
  };
}

function assertIncludes(source, expected, label) {
  if (!source.includes(expected)) {
    throw new Error(`Missing source assertion: ${label}`);
  }
}

function assertProof(proof) {
  const summary = proof.accessibilitySummary.summary;
  for (const heading of [
    '3 parent surface rows',
    'Parent surface manual action required',
    'Parent surface unavailable',
  ]) {
    if (!summary.headings.includes(heading)) {
      throw new Error(`Missing accessibility heading: ${heading}`);
    }
  }
  for (const value of ['manual-action-required', 'unavailable-visible', 'preference-setup-required']) {
    if (!summary.values.includes(value)) {
      throw new Error(`Missing accessibility value: ${value}`);
    }
  }
  const weakScreenshot = proof.screenshotProof.find((artifact) => artifact.bytes <= 1024 || artifact.width <= 0);
  if (weakScreenshot !== undefined) {
    throw new Error(`Invalid screenshot artifact: ${JSON.stringify(weakScreenshot)}`);
  }
}

function toRepoPath(path) {
  return relative(repoRoot, path).replace(/\\/gu, '/');
}

function markdown(proof) {
  return [
    '# Social Alert Report Parent Surface Service UI Proof',
    '',
    `- Branch: ${proof.branch}`,
    `- Commit: ${proof.commit}`,
    '- Scope: Browser social route requests the service-backed parent-surface status read model and renders provider/preference-derived manual-action-required plus unavailable rows.',
    '- Eventing: the Rust service publishes the local `browser.social-alert-report.parent-surface.status.requested` request and completes it through the reusable `ocentra-eventing` request/response path before reporting the portal read model.',
    '- Handoff boundary: rows preserve provider status and preference/quiet-hours handoff refs while rendering only minimal parent-surface status payloads.',
    '- Real runtime proof: portal E2E harness starts the Rust agent service and Vite portal, requests the service-backed Browser route, and captures desktop/mobile screenshots.',
    `- Evidence: ${proof.proofPaths.proof}`,
    `- Accessibility summary: ${proof.proofPaths.accessibilitySummary}`,
    `- Screenshots: ${proof.proofPaths.screenshots.join(', ')}`,
    '- No-claim boundary: parent notification UI delivery, preference UI delivery, history UI delivery, provider delivery, provider receipt ingestion, child delivery, quiet-hours timer runtime, report delivery execution, final policy execution, and enforcement remain unclaimed.',
    '',
  ].join('\n');
}

function runNpm(args, ...rest) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return run(command, commandArgs, ...rest);
}
