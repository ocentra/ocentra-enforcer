import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const outputDir = join('output', 'screen-plan-proof', 'real-capture', 'scope-matrix');
const fixtureTitle = 'Ocentra Screen Scope Matrix Proof';

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });

const fixturePath = writeFixture();
let browser;

try {
  browser = await chromium.launch({
    headless: false,
    args: ['--window-size=860,560', '--window-position=180,160'],
  });
  const page = await browser.newPage({ viewport: { width: 860, height: 560 } });
  await page.goto(pathToFileURL(resolve(fixturePath)).href);
  await page.bringToFront();
  await page.waitForTimeout(1000);

  const activeWindow = runCaptureScope('active-window', {
    requestedScope: 'active-window',
    expectedActualScope: 'activeWindow',
  });
  const selectedWindow = runCaptureScope('selected-window', {
    requestedScope: 'selected-window',
    expectedActualScope: 'selectedWindow',
    targetTitle: fixtureTitle,
  });
  const primaryDisplay = runCaptureScope('primary-display', {
    requestedScope: 'primary-display',
    expectedActualScope: 'primaryDisplay',
  });

  const scopes = [activeWindow, selectedWindow, primaryDisplay];
  const summary = {
    proof: 'screen-capture-scope-matrix-proof',
    outputDir,
    platform: process.platform,
    realCaptureRuns: scopes.length,
    capturedRuns: scopes.filter((scope) => scope.captured).length,
    allRawImagesDeleted: scopes.every((scope) => scope.rawImageDeleted),
    allScopesMatched: scopes.every((scope) => scope.actualScope === scope.expectedActualScope),
    degradedIsCaptureProof: false,
    scopes,
    nonClaims: [
      'This proof exercises adapter scopes from a harness; it does not claim product scheduler wiring.',
      'Primary display is opt-in proof only and must stay disabled unless parent settings request it.',
    ],
  };
  writeJson(join(outputDir, 'proof-summary.json'), summary);

  if (
    process.platform === 'win32' &&
    (summary.capturedRuns !== scopes.length || !summary.allRawImagesDeleted || !summary.allScopesMatched)
  ) {
    throw new Error(`Windows scope matrix proof incomplete: ${JSON.stringify(summary, null, 2)}`);
  }

  console.log(JSON.stringify(summary, null, 2));
} finally {
  if (browser !== undefined) {
    await browser.close();
  }
}

function runCaptureScope(scenarioId, options) {
  const scenarioDir = join(outputDir, scenarioId);
  mkdirSync(scenarioDir, { recursive: true });
  writeJson(join(scenarioDir, '00-scope-request.json'), {
    scenarioId,
    requestedScope: options.requestedScope,
    expectedActualScope: options.expectedActualScope,
    targetTitle: options.targetTitle ?? null,
    productSettingWired: false,
    proofHarnessScope: true,
  });

  const result = spawnSync(
    'cargo',
    ['run', '-p', 'ocentra-parent-screen-capture-adapter', '--example', 'screen_capture_real_proof', '--', scenarioDir],
    {
      cwd: process.cwd(),
      encoding: 'utf8',
      shell: process.platform === 'win32',
      env: {
        ...process.env,
        OCENTRA_SCREEN_CAPTURE_SCOPE: options.requestedScope,
        ...(options.targetTitle === undefined
          ? {}
          : { OCENTRA_SCREEN_CAPTURE_WINDOW_TITLE_CONTAINS: options.targetTitle }),
      },
    }
  );

  writeFileSync(join(scenarioDir, 'cargo-stdout.log'), result.stdout ?? '');
  writeFileSync(join(scenarioDir, 'cargo-stderr.log'), result.stderr ?? '');
  if (result.status !== 0) {
    throw new Error(`capture scope ${scenarioId} failed with status ${result.status}`);
  }

  const metadata = readJson(join(scenarioDir, '02-capture-metadata.json'));
  const deletion = metadata.captured ? readJson(join(scenarioDir, '04-deletion-proof.json')) : null;
  const scopeSummary = {
    scenarioId,
    requestedScope: options.requestedScope,
    expectedActualScope: options.expectedActualScope,
    actualScope: metadata.actualScope ?? null,
    captured: metadata.captured === true,
    status: metadata.status,
    imageDigest: metadata.imageDigest ?? null,
    imageByteSize: metadata.imageByteSize ?? null,
    rawImageDeleted: deletion?.rawImageDeleted === true,
    encryptedQueueOmitsRawDigest: deletion?.encryptedQueueContainsRawDigest === false,
  };
  writeJson(join(scenarioDir, '06-scope-summary.json'), scopeSummary);
  return scopeSummary;
}

function writeFixture() {
  const fixturePath = join(outputDir, 'controlled-scope-fixture.html');
  writeFileSync(
    fixturePath,
    `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>${fixtureTitle}</title>
    <style>
      body { margin: 0; font-family: Arial, sans-serif; background: #08131f; color: #ecfeff; }
      main { min-height: 100vh; display: grid; place-items: center; }
      section { border: 6px solid #22d3ee; padding: 38px; width: 680px; background: #0f172a; }
      h1 { font-size: 44px; margin: 0 0 18px; }
      p { font-size: 28px; margin: 12px 0; }
    </style>
  </head>
  <body>
    <main>
      <section>
        <h1>Screen Scope Proof</h1>
        <p>Active window, selected window, and primary display are real capture scopes.</p>
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
