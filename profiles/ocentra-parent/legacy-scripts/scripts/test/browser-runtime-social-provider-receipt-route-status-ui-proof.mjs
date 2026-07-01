import { spawnSync } from 'node:child_process';
import { copyFile, mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

const repoRoot = process.cwd();
const proofName = 'browser-runtime-social-provider-receipt-route-status-ui-proof';
const resultDir = join(repoRoot, 'test-results', proofName);
const proofDir = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'browser-runtime-social-provider-receipt-route-status-ui'
);
const screenshotDir = join(proofDir, '06-ui-snapshots');
const accessibilitySummaryPath = join(resultDir, 'accessibility-summary.json');
const proofPath = join(resultDir, 'proof.json');
const markdownPath = join(proofDir, '01-browser-runtime-social-provider-receipt-route-status-ui-proof.md');
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
runNpm(['run', 'test:e2e', '--workspace', '@ocentra-parent/portal', '--', 'social-alert-report-ui-proof.spec.ts'], {
  SOCIAL_ALERT_REPORT_UI_PROOF: '1',
});
runNpm(['--workspace', '@ocentra-parent/portal', 'run', 'lint:exec']);

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
    'real-portal-e2e-harness',
    'browser-route-social-alert-report-region',
    'action-intent-stream-status-card-visible',
    'action-intent-zero-candidates-visible',
    'receipt-stream-status-card-visible',
    'receipt-ingestion-readiness-card-visible',
    'receipt-zero-provider-receipts-visible',
    'desktop-screenshot',
    'mobile-screenshot',
    'no-action-intent-dispatch-adapter-child-intervention-browser-mutation-or-enforcement-claim',
    'no-provider-delivery-or-enforcement-claim',
  ],
  sourceProof,
  accessibilitySummary,
  screenshotProof,
  noClaimBoundaries: [
    'provider delivery',
    'provider receipt ingestion runtime',
    'provider webhook runtime',
    'provider credentials',
    'observed provider receipts',
    'adapter dispatch',
    'report delivery execution',
    'final policy execution',
    'browser mutation',
    'child intervention execution',
    'unmanaged exact URL support',
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

console.log('browser-runtime-social-provider-receipt-route-status-ui-proof-ok=true');
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
  const parentRouteSource = await readFile(join(repoRoot, 'apps', 'portal', 'src', 'ParentPortalRoute.tsx'), 'utf8');
  const e2eSource = await readFile(
    join(repoRoot, 'apps', 'portal', 'e2e', 'social-alert-report-ui-proof.spec.ts'),
    'utf8'
  );
  assertIncludes(parentRouteSource, 'liveActivity={activityState}', 'parent route passes live activity');
  assertIncludes(routeSource, 'BrowserReceiptStatusCards', 'route renders receipt status cards');
  assertIncludes(
    routeSource,
    'createBrowserActionIntentStreamStatusIntent',
    'route consumes action intent stream status'
  );
  assertIncludes(
    routeSource,
    'browserSocialProviderReceiptIngestionReadinessStatusIntent',
    'route consumes readiness status intent'
  );
  assertIncludes(e2eSource, 'Browser action-intent stream status', 'e2e asserts action intent status card');
  assertIncludes(e2eSource, 'Social provider receipt stream status', 'e2e asserts receipt status card');
  assertIncludes(e2eSource, 'Social provider receipt ingestion readiness', 'e2e asserts readiness status card');
  return {
    parentRoutePassesLiveActivity: true,
    routeRendersReceiptStatusCards: true,
    routeRendersActionIntentStatusCard: true,
    e2eRequiresReceiptCards: true,
    e2eRequiresActionIntentCard: true,
  };
}

function assertIncludes(source, expected, label) {
  if (!source.includes(expected)) {
    throw new Error(`Missing source assertion: ${label}`);
  }
}

function assertProof(proof) {
  const summary = proof.accessibilitySummary.summary;
  if (!summary.headings.includes('Browser action-intent stream status')) {
    throw new Error('Action-intent stream status heading missing from accessibility summary.');
  }
  if (!summary.headings.includes('Social provider receipt stream status')) {
    throw new Error('Receipt stream status heading missing from accessibility summary.');
  }
  if (!summary.headings.includes('Social provider receipt ingestion readiness')) {
    throw new Error('Receipt ingestion readiness heading missing from accessibility summary.');
  }
  if (!summary.values.includes('0 provider receipts observed')) {
    throw new Error('Receipt zero-observed status missing from accessibility summary.');
  }
  if (!summary.values.includes('0 action candidates')) {
    throw new Error('Action-intent zero-candidate status missing from accessibility summary.');
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
    '# Browser Runtime Social Provider Receipt Route Status UI Proof',
    '',
    `- Branch: ${proof.branch}`,
    `- Commit: ${proof.commit}`,
    '- Scope: existing Browser social alert/report route renders live activity browser action-intent stream, social provider receipt stream, and receipt ingestion readiness statuses.',
    '- Real runtime proof: portal E2E harness starts the Rust agent service and Vite portal, requests the service-backed Browser route, and captures desktop/mobile screenshots.',
    `- Evidence: ${proof.proofPaths.proof}`,
    `- Accessibility summary: ${proof.proofPaths.accessibilitySummary}`,
    `- Screenshots: ${proof.proofPaths.screenshots.join(', ')}`,
    '- No-claim boundary: action adapter dispatch, provider delivery, receipt ingestion runtime, report delivery execution, final policy execution, browser mutation, child intervention execution, unmanaged exact URL support, and enforcement remain unclaimed.',
    '',
  ].join('\n');
}

function runNpm(args, ...rest) {
  const command = process.platform === 'win32' ? 'cmd' : 'npm';
  const commandArgs = process.platform === 'win32' ? ['/c', 'npm', ...args] : args;
  return run(command, commandArgs, ...rest);
}
