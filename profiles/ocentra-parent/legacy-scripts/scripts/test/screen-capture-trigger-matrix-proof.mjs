import { spawn, spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const outputDir = join('output', 'screen-plan-proof', 'real-capture', 'trigger-matrix');
const fixtureTitle = 'Ocentra Screen Trigger Matrix Proof';
const nativeFixtureTitle = 'Ocentra Native App Screen Trigger Proof';

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });

const fixturePath = writeFixture();
let browser;
let nativeApp;

try {
  browser = await chromium.launch({
    headless: false,
    args: ['--window-size=920,620', '--window-position=120,120'],
  });
  const page = await browser.newPage({ viewport: { width: 920, height: 620 } });
  await page.goto(pathToFileURL(resolve(fixturePath)).href);
  await page.bringToFront();
  await page.waitForTimeout(1000);

  const browserUse = runCaptureScenario('browser-use-active-window', {
    requestedTrigger: 'managedBrowserUrlChange',
    proofHarnessTrigger: 'headed-playwright-url-change-to-product-scheduler',
    targetTitle: fixtureTitle,
  });

  const nativeAppUse = await runNativeAppScenario();

  await page.locator('#state').evaluate((node) => {
    node.textContent = 'Timed cadence frame 1';
  });
  await page.waitForTimeout(750);

  const timedFirst = runCaptureScenario('timed-cadence-frame-1', {
    requestedTrigger: 'timedCadence',
    proofHarnessTrigger: 'scheduler-due-cadence-proof',
    targetTitle: fixtureTitle,
    lastCaptureAt: 1_779_999_900,
  });

  await page.locator('#state').evaluate((node) => {
    node.textContent = 'Timed cadence frame 2';
  });
  await page.waitForTimeout(1250);

  const timedSecond = runCaptureScenario('timed-cadence-frame-2', {
    requestedTrigger: 'timedCadence',
    proofHarnessTrigger: 'scheduler-due-cadence-proof',
    targetTitle: fixtureTitle,
    lastCaptureAt: 1_779_999_900,
  });

  const scenarios = [browserUse, nativeAppUse, timedFirst, timedSecond].filter((scenario) => scenario !== undefined);
  const imageDigests = scenarios.map((scenario) => scenario.imageDigest).filter((imageDigest) => imageDigest !== undefined);
  const summary = {
    proof: 'screen-capture-trigger-matrix-proof',
    outputDir,
    platform: process.platform,
    realCaptureRuns: scenarios.length,
    capturedRuns: scenarios.filter((scenario) => scenario.captured).length,
    productSchedulerDecisions: scenarios.filter((scenario) => scenario.productTriggerWired).length,
    allRawImagesDeleted: scenarios.every((scenario) => scenario.rawImageDeleted),
    distinctCapturedFrames: new Set(imageDigests).size === imageDigests.length,
    selectedWindowScopeMatched: scenarios.every((scenario) => scenario.actualScope === 'selectedWindow'),
    productSchedulerImplemented: true,
    nativeAppForegroundTriggerCaptured: nativeAppUse?.captured === true,
    productServiceForegroundWatcherImplemented: false,
    productServiceTimerImplemented: false,
    degradedIsCaptureProof: false,
    scenarios,
    nonClaims: [
      'This proof runs the Rust trigger scheduler and real capture adapter from a harness; it does not claim background service timer wiring.',
      'Browser URL ownership remains with the browser-plan lane; this proof consumes a managed-browser trigger input and captures the visible window.',
    ],
  };
  writeJson(join(outputDir, 'proof-summary.json'), summary);
  if (
    process.platform === 'win32' &&
    (summary.capturedRuns !== scenarios.length ||
      summary.productSchedulerDecisions !== scenarios.length ||
      !summary.allRawImagesDeleted ||
      !summary.distinctCapturedFrames ||
      !summary.selectedWindowScopeMatched)
  ) {
    throw new Error(`Windows trigger matrix proof incomplete: ${JSON.stringify(summary, null, 2)}`);
  }
  console.log(JSON.stringify(summary, null, 2));
} finally {
  if (nativeApp !== undefined) {
    nativeApp.kill();
  }
  if (browser !== undefined) {
    await browser.close();
  }
}

async function runNativeAppScenario() {
  if (process.platform !== 'win32') {
    return null;
  }
  const nativeFixturePath = join(outputDir, `${nativeFixtureTitle}.txt`);
  writeFileSync(nativeFixturePath, 'Native app foreground trigger proof');
  nativeApp = spawn('notepad.exe', [nativeFixturePath], {
    detached: false,
    stdio: 'ignore',
  });
  await new Promise((resolve) => setTimeout(resolve, 1500));
  return runCaptureScenario('native-app-foreground-start', {
    requestedTrigger: 'nativeAppForegroundStart',
    proofHarnessTrigger: 'real-notepad-window-to-product-scheduler',
    targetTitle: nativeFixtureTitle,
  });
}

function runCaptureScenario(scenarioId, options) {
  const scenarioDir = join(outputDir, scenarioId);
  mkdirSync(scenarioDir, { recursive: true });
  const schedulerDecision = runScheduleDecision(scenarioDir, options);
  writeJson(join(scenarioDir, '00-trigger-request.json'), {
    scenarioId,
    requestedTrigger: options.requestedTrigger,
    productTriggerWired: schedulerDecision.decision === 'enqueueCapture',
    schedulerDecision: schedulerDecision.decision,
    schedulerSuppression: schedulerDecision.suppression,
    proofHarnessTrigger: options.proofHarnessTrigger,
    targetTitle: options.targetTitle,
    expectedScope: 'selectedWindow',
  });
  if (schedulerDecision.decision !== 'enqueueCapture') {
    throw new Error(`scheduler suppressed ${scenarioId}: ${JSON.stringify(schedulerDecision, null, 2)}`);
  }
  const result = spawnSync(
    'cargo',
    ['run', '-p', 'ocentra-parent-screen-capture-adapter', '--example', 'screen_capture_real_proof', '--', scenarioDir],
    {
      cwd: process.cwd(),
      encoding: 'utf8',
      shell: process.platform === 'win32',
      env: {
        ...process.env,
        OCENTRA_SCREEN_CAPTURE_WINDOW_TITLE_CONTAINS: options.targetTitle,
      },
    }
  );
  writeFileSync(join(scenarioDir, 'cargo-stdout.log'), result.stdout ?? '');
  writeFileSync(join(scenarioDir, 'cargo-stderr.log'), result.stderr ?? '');
  if (result.status !== 0) {
    throw new Error(`capture scenario ${scenarioId} failed with status ${result.status}`);
  }
  const metadata = readJson(join(scenarioDir, '02-capture-metadata.json'));
  const deletion = metadata.captured ? readJson(join(scenarioDir, '04-deletion-proof.json')) : null;
  const scenarioSummary = {
    scenarioId,
    requestedTrigger: options.requestedTrigger,
    productTriggerWired: schedulerDecision.productSchedulerImplemented === true,
    schedulerDecision: schedulerDecision.decision,
    schedulerReason: schedulerDecision.reason,
    proofHarnessTrigger: options.proofHarnessTrigger,
    captured: metadata.captured === true,
    status: metadata.status,
    actualScope: metadata.actualScope ?? null,
    imageDigest: metadata.imageDigest ?? null,
    imageByteSize: metadata.imageByteSize ?? null,
    rawImageDeleted: deletion?.rawImageDeleted === true,
    encryptedQueueOmitsRawDigest: deletion?.encryptedQueueContainsRawDigest === false,
  };
  writeJson(join(scenarioDir, '06-scenario-summary.json'), scenarioSummary);
  return scenarioSummary;
}

function runScheduleDecision(scenarioDir, options) {
  const result = spawnSync(
    'cargo',
    [
      'run',
      '-p',
      'ocentra-parent-screen-capture-adapter',
      '--example',
      'screen_capture_schedule_decision',
      '--',
      scenarioDir,
    ],
    {
      cwd: process.cwd(),
      encoding: 'utf8',
      shell: process.platform === 'win32',
      env: {
        ...process.env,
        OCENTRA_SCREEN_CAPTURE_TRIGGER: options.requestedTrigger,
        OCENTRA_SCREEN_CAPTURE_ALLOWED_SCOPE: 'selectedWindow',
        OCENTRA_SCREEN_CAPTURE_REQUESTED_SCOPE: 'selectedWindow',
        OCENTRA_SCREEN_CAPTURE_OBSERVED_AT: '1780000000',
        ...(options.lastCaptureAt === undefined
          ? {}
          : { OCENTRA_SCREEN_LAST_CAPTURE_AT: String(options.lastCaptureAt) }),
      },
    }
  );
  writeFileSync(join(scenarioDir, 'scheduler-stdout.log'), result.stdout ?? '');
  writeFileSync(join(scenarioDir, 'scheduler-stderr.log'), result.stderr ?? '');
  if (result.status !== 0) {
    throw new Error(`scheduler scenario ${scenarioDir} failed with status ${result.status}`);
  }
  return readJson(join(scenarioDir, '00-scheduler-decision.json'));
}

function writeFixture() {
  const fixturePath = join(outputDir, 'controlled-trigger-fixture.html');
  writeFileSync(
    fixturePath,
    `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>${fixtureTitle}</title>
    <style>
      body { margin: 0; font-family: Arial, sans-serif; background: #07101c; color: #e6fbff; }
      main { min-height: 100vh; display: grid; place-items: center; }
      section { border: 6px solid #38bdf8; padding: 42px; width: 720px; background: #0f172a; }
      h1 { font-size: 46px; margin: 0 0 20px; }
      p { font-size: 32px; margin: 12px 0; }
    </style>
  </head>
  <body>
    <main>
      <section>
        <h1>Screen Trigger Proof</h1>
        <p id="state">Browser use active window</p>
      </section>
    </main>
  </body>
</html>
`
  );
  return fixturePath;
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}
