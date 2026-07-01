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
const workpackId = 'social-21-child-approval-block-ux';
const outputDirectory = join(repoRoot, 'output', 'browser-plan-proof', workpackId);
const screenshotDirectory = join(outputDirectory, '06-ui-snapshots');
const evidenceDirectory = join(repoRoot, 'test-results', 'social-child-intervention-page-proof');
const timeoutMs = envNumber('OCENTRA_PARENT_SOCIAL_CHILD_INTERVENTION_TIMEOUT_MS', 45_000);

await main();

async function main() {
  await runCommand('cmd', ['/c', 'npm run build:contracts']);
  const [
    { BrowserChildInterventionPageDefaults, renderBrowserChildInterventionPage },
    { createSocialChildInterventionPageModels },
  ] = await Promise.all([
    import('@ocentra-parent/portal-domain/browser-child-intervention-page'),
    import('@ocentra-parent/portal-domain/social-child-intervention-page-model'),
  ]);
  await runCommand('cmd', [
    '/c',
    'npm run test --workspace @ocentra-parent/portal-domain -- social-child-intervention-page-model.test.ts',
  ]);
  await runCommand('cargo', ['build', '-p', 'ocentra-parent-agent-service']);
  await mkdir(evidenceDirectory, { recursive: true });
  await mkdir(screenshotDirectory, { recursive: true });

  const runRoot = await mkdtemp(join(tmpdir(), 'ocentra-parent-social-child-intervention-'));
  const htmlPath = join(runRoot, 'browser-intervention-page.html');
  const agentPort = await freePort();
  const service = spawnAgentService(runRoot, agentPort, htmlPath);
  const serviceOutput = collectOutput(service);
  const browser = await chromium.launch();

  try {
    await waitForHealth(agentPort, serviceOutput);
    const page = await browser.newPage({ viewport: { width: 1280, height: 820 } });
    const modelResult = createSocialChildInterventionPageModels(validSnapshot(), {
      backdrop: {
        imageUrl: proofBackdropDataUrl(),
        label: 'Captured social route behind intervention',
      },
      bridge: 'social-child-intervention-proof',
      requestedUrlForSurface: (surface) => `https://social.example.invalid/${surface.surfaceKind}`,
    });
    if (modelResult.state !== 'renderable') {
      throw new Error(`Expected renderable social child intervention models, received ${modelResult.state}`);
    }

    const servedPages = [];
    for (const model of modelResult.models) {
      const html = renderBrowserChildInterventionPage(model);
      await writeFile(htmlPath, html, 'utf8');
      const servedUrl = `http://127.0.0.1:${agentPort}/api/browser/intervention/page?target=${encodeURIComponent(
        model.requestedUrl
      )}`;
      const response = await fetch(servedUrl);
      const servedHtml = await response.text();
      await page.goto(servedUrl, { waitUntil: 'networkidle' });
      const screenshotPath = join(screenshotDirectory, `${model.action}-${model.deliveryState}.png`);
      await page.screenshot({ fullPage: true, path: screenshotPath });
      servedPages.push({
        action: model.action,
        assertions: {
          blockMarkerPresent: servedHtml.includes(BrowserChildInterventionPageDefaults.BlockMarker),
          bridgePayloadPresent: servedHtml.includes('ocentra-child-approval-request'),
          cacheDisabled: response.headers.get('cache-control') === 'no-store',
          endpointServedHtml: response.ok,
          requestedUrlPresent: servedHtml.includes(model.requestedUrl),
          screenshotCaptured: true,
        },
        deliveryState: model.deliveryState,
        outcome: model.outcome,
        requestedUrl: model.requestedUrl,
        screenshot: relative(repoRoot, screenshotPath),
        status: response.status,
        targetType: model.targetType,
      });
    }

    const proof = {
      schemaVersion: 1,
      checkedAt: new Date().toISOString(),
      commit: await git(['rev-parse', 'HEAD']),
      workpackIds: [workpackId],
      proofMode: 'real-child-agent-served-social-intervention-pages',
      productClaimReady: false,
      artifacts: {
        proof: 'test-results/social-child-intervention-page-proof/proof.json',
        outputProof: `output/browser-plan-proof/${workpackId}/07-rendered-child-ui-proof.json`,
        screenshots: `output/browser-plan-proof/${workpackId}/06-ui-snapshots`,
      },
      assertions: [
        'SOCIAL-21 state contracts map to the shared BrowserChildInterventionPageModel renderer.',
        'The Rust child-agent intervention endpoint serves each rendered social child page with no-store caching.',
        'The rendered pages include approval-hold, block, warn, parent-review, time-limit, and native-unavailable states.',
        'The ask-parent bridge payload is present without claiming notification delivery, policy execution, native control, or enforcement.',
      ],
      nonClaims: [
        'This proof does not claim final policy decisions or enforcement.',
        'This proof does not claim notification delivery or parent delivery confirmation.',
        'This proof does not claim native social app control or connector authorization.',
        'This proof does not claim browser navigation was blocked; it proves served child intervention HTML only.',
      ],
      servedPages,
    };
    const outputProofPath = join(outputDirectory, '07-rendered-child-ui-proof.json');
    const proofPath = join(evidenceDirectory, 'proof.json');
    await writeFile(outputProofPath, `${JSON.stringify(proof, null, 2)}\n`, 'utf8');
    await writeFile(proofPath, `${JSON.stringify(proof, null, 2)}\n`, 'utf8');
    await writeFile(join(outputDirectory, '10-validation-commands.log'), validationLog(), 'utf8');

    const ok = servedPages.every((entry) => Object.values(entry.assertions).every(Boolean));
    console.log(`social-child-intervention-page-proof-ok=${ok}`);
    console.log(`evidence=${relative(repoRoot, proofPath)}`);
    console.log(`screenshots=${relative(repoRoot, screenshotDirectory)}`);
    if (!ok) {
      process.exitCode = 1;
    }
  } finally {
    await browser.close();
    await stopProcessTreeAndWait(service);
  }
}

function validSnapshot() {
  return {
    schemaVersion: 'social-child-approval-block-ux-contract',
    familyId: 'family-social-child-ux',
    childProfileId: 'child-social-child-ux',
    deviceId: 'device-social-child-ux',
    generatedAt: '2026-06-06T04:45:00.000Z',
    surfaces: [
      surface('approval-request-pending', 'waiting-parent', 'wait-for-parent', ['parent-approval-needed'], {
        parentApprovalRequestRef: 'parent-approval-request-social',
      }),
      surface('blocked-social-route-candidate', 'blocked-contract-only', 'open-safe-back', ['route-block-candidate']),
      surface('warning-social-route-candidate', 'child-readable', 'acknowledge-warning', ['route-warning-candidate']),
      surface('manual-review-required', 'manual-required', 'manual-review', ['manual-review-needed']),
      surface('time-limit-candidate', 'child-readable', 'acknowledge-warning', ['time-limit-not-applied']),
      surface('native-app-unavailable', 'unavailable', 'no-action', ['native-app-proof-unavailable']),
    ],
    claimBoundaries: {
      renderedChildUi: 'not-claimed',
      notificationDelivery: 'not-claimed',
      browserNavigationBlock: 'not-claimed',
      blockPageRender: 'not-claimed',
      timeLimitApply: 'not-claimed',
      finalPolicyDecision: 'not-claimed',
      connectorAuthorization: 'not-claimed',
      nativeAppControl: 'not-claimed',
      enforcement: 'not-claimed',
    },
  };
}

function surface(surfaceKind, state, primaryAction, reasons, overrides = {}) {
  return {
    surfaceId: `social-child-ux-${surfaceKind}`,
    surfaceKind,
    state,
    primaryAction,
    sourceEvidenceRefs: [`parent-evidence-${surfaceKind}`],
    parentApprovalRequestRef: null,
    gatePlanRef: surfaceKind === 'blocked-social-route-candidate' ? 'parent-gate-plan-social-route' : null,
    reasons,
    renderedChildUiClaimed: false,
    notificationDeliveredClaimed: false,
    browserNavigationBlockedClaimed: false,
    blockPageRenderedClaimed: false,
    timeLimitAppliedClaimed: false,
    finalPolicyDecisionClaimed: false,
    connectorAuthorizationClaimed: false,
    nativeAppControlClaimed: false,
    enforcementClaimed: false,
    ...overrides,
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

function proofBackdropDataUrl() {
  const svg =
    '<svg xmlns="http://www.w3.org/2000/svg" width="1280" height="720" viewBox="0 0 1280 720"><rect width="1280" height="720" fill="#0f172a"/><rect x="92" y="84" width="1096" height="552" rx="22" fill="#111827" stroke="#334155" stroke-width="3"/><rect x="130" y="132" width="560" height="360" rx="18" fill="#1f2937"/><rect x="730" y="132" width="390" height="42" rx="10" fill="#374151"/><rect x="730" y="204" width="330" height="28" rx="8" fill="#374151"/><rect x="730" y="258" width="286" height="28" rx="8" fill="#374151"/><circle cx="410" cy="312" r="76" fill="#2563eb"/><path d="M370 320h84M410 278v84" stroke="#dbeafe" stroke-width="18" stroke-linecap="round"/></svg>';
  return `data:image/svg+xml;base64,${Buffer.from(svg).toString('base64')}`;
}

function validationLog() {
  return [
    'cmd /c npm run build:contracts',
    'cmd /c npm run test --workspace @ocentra-parent/portal-domain -- social-child-intervention-page-model.test.ts',
    'cargo build -p ocentra-parent-agent-service',
    'cmd /c node scripts/test/social-child-intervention-page-proof.mjs',
    '',
  ].join('\n');
}

function envNumber(name, fallback) {
  const parsed = Number(process.env[name]);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}
