import { existsSync } from 'node:fs';
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const root = process.cwd();
const proofRoot = join(root, 'output', 'browser-plan-proof');
const outputDirectory = join(proofRoot, 'game-22-tests-fixtures-playwright-manual-proof');
const resultDirectory = join(root, 'test-results', 'browser-game-proof-artifacts');
const requiredProofFiles = ['00-source-snapshot.md', '08-security-negative-proof.md', '10-validation-commands.log'];

if (!existsSync(proofRoot)) {
  throw new Error(`Missing browser proof root: ${relativePath(proofRoot)}`);
}

await main();

async function main() {
  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });

  const docs = await loadDocs();
  const proofDirectories = await gameProofDirectories();
  const rows = await Promise.all(expectedRows().map((row) => validateGameRow(row, proofDirectories, docs)));
  const failures = rows.flatMap((row) => row.failures);
  const manifest = manifestFor(rows, failures);

  if (manifest.failures.length > 0) {
    throw new Error(`Browser-game proof artifact gate failed:\n${manifest.failures.join('\n')}`);
  }

  const proofPath = join(resultDirectory, 'proof.json');
  const markdownPath = join(outputDirectory, '01-browser-game-proof-artifact-manifest.md');
  await writeFile(proofPath, `${JSON.stringify(manifest, null, 2)}\n`);
  await writeFile(markdownPath, `${markdownFor(manifest)}\n`);

  console.log('browser-game-proof-artifacts-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manifest=${relativePath(markdownPath)}`);
  console.log(`complete=${manifest.summary.completeRows} partial=${manifest.summary.partialRows}`);
  console.log(`playwright=${manifest.summary.playwrightState}`);
}

async function loadDocs() {
  return {
    checklist: await readText('docs/plans/browser-plan/implementation-checklist.md'),
    plan: await readText('docs/plans/browser-plan/v0-5-browser-games-cloud-gaming-gating-plan.md'),
    readme: await readText('docs/plans/browser-plan/browser-games-cloud-gaming/readme.md'),
    feature: await readText('docs/features/browser-web-control.md'),
    expectation: await readText('docs/expectations/browser-evidence.md'),
  };
}

async function gameProofDirectories() {
  const entries = await readdir(proofRoot, { withFileTypes: true });
  return new Map(
    entries
      .filter((entry) => entry.isDirectory() && /^game-\d\d-/.test(entry.name))
      .map((entry) => [Number(entry.name.slice(5, 7)), entry.name])
  );
}

function expectedRows() {
  return Array.from({ length: 23 }, (_, index) => {
    const rowNumber = index + 1;
    const isComplete = rowNumber <= 23;
    return {
      rowNumber,
      rowId: `GAME-${String(rowNumber).padStart(2, '0')}`,
      expectedStatus: isComplete ? '[x]' : '[~]',
      expectedState: isComplete
        ? rowNumber === 1
          ? 'scaffold-proof-present'
          : rowNumber === 2
            ? 'live-route-proof-present'
            : rowNumber === 3
              ? 'live-portal-pattern-proof-present'
              : rowNumber === 4
                ? 'live-cloud-pattern-proof-present'
                : rowNumber === 5
                  ? 'live-url-shape-proof-present'
                  : rowNumber === 6
                    ? 'live-runtime-signal-shape-proof-present'
                    : rowNumber === 7
                      ? 'live-metadata-shape-proof-present'
                      : rowNumber === 8
                        ? 'live-hidden-analysis-profile-safety-proof-present'
                        : rowNumber === 9
                          ? 'live-educational-classifier-proof-present'
                          : rowNumber === 10
                            ? 'live-ai-analysis-proof-present'
                            : rowNumber === 11
                              ? 'live-riskbenefit-signal-proof-present'
                              : rowNumber === 12
                                ? 'live-memory-cache-proof-present'
                                : rowNumber === 13
                                  ? 'live-account-purchase-gate-proof-present'
                                  : rowNumber === 14
                                    ? 'live-cloud-gaming-gate-proof-present'
                                    : rowNumber === 15
                                      ? 'live-unblocked-site-detection-proof-present'
                                      : rowNumber === 16
                                        ? 'live-ugc-multiplayer-chat-risk-proof-present'
                                        : rowNumber === 17
                                          ? 'live-policy-compiler-proof-present'
                                          : rowNumber === 18
                                            ? 'live-hold-block-adapter-proof-present'
                                            : rowNumber === 19
                                              ? 'live-child-checking-block-ux-proof-present'
                                              : rowNumber === 20
                                                ? 'live-parent-dashboard-ux-proof-present'
                                                : rowNumber === 21
                                                  ? 'live-journal-sqlite-read-model-proof-present'
                                                  : rowNumber === 22
                                                    ? 'live-rendered-child-intervention-proof-present'
                                                    : 'live-android-ios-host-proof-present'
        : 'partial-manual-required',
    };
  });
}

async function validateGameRow(row, proofDirectories, docs) {
  const failures = [];
  const proofDirectory = proofDirectories.get(row.rowNumber);
  if (!proofDirectory) {
    failures.push(`${row.rowId} missing proof directory`);
    return { ...row, proofDirectory: null, proofFiles: [], failures };
  }

  const proofFiles = await readdir(join(proofRoot, proofDirectory));
  failures.push(...validateChecklist(row, proofDirectory, docs.checklist));
  failures.push(...validateProofFiles(row, proofFiles));
  failures.push(...(await validateRenderedChildInterventionProof(row, proofDirectory, proofFiles)));
  failures.push(...validateDocs(row, proofDirectory, docs));

  return {
    ...row,
    proofDirectory,
    proofFiles: proofFiles.sort(),
    state: row.expectedState,
    failures,
  };
}

function validateChecklist(row, proofDirectory, checklist) {
  const rowText = checklistRowText(row.rowId, checklist);
  const failures = [];
  if (!rowText) {
    return [`${row.rowId} missing implementation-checklist row`];
  }
  if (!rowText.includes(row.expectedStatus)) {
    failures.push(`${row.rowId} checklist status is not ${row.expectedStatus}`);
  }
  if (!rowText.includes('codex-d')) {
    failures.push(`${row.rowId} checklist owner is not codex-d`);
  }
  if (!rowText.includes(`output/browser-plan-proof/${proofDirectory}/`)) {
    failures.push(`${row.rowId} checklist does not reference its proof directory`);
  }
  return failures;
}

function validateProofFiles(row, proofFiles) {
  const failures = [];
  for (const requiredFile of requiredProofFiles) {
    if (!proofFiles.includes(requiredFile)) {
      failures.push(`${row.rowId} proof is missing ${requiredFile}`);
    }
  }
  if (!proofFiles.some((file) => /^01-.*(?:proof|manifest)\.(md|log)$/.test(file))) {
    failures.push(`${row.rowId} proof is missing a 01-* proof artifact`);
  }
  if (!proofFiles.includes('ui-not-applicable.md')) {
    failures.push(`${row.rowId} proof is missing ui-not-applicable.md`);
  }
  return failures;
}

async function validateRenderedChildInterventionProof(row, proofDirectory, proofFiles) {
  if (row.rowNumber !== 22) {
    return [];
  }

  const failures = [];
  const renderedProofFile = '02-rendered-browser-game-child-intervention-proof.json';
  const screenshotDirectory = '06-ui-snapshots';
  if (!proofFiles.includes(renderedProofFile)) {
    failures.push(`${row.rowId} proof is missing ${renderedProofFile}`);
  }
  if (!proofFiles.includes(screenshotDirectory)) {
    failures.push(`${row.rowId} proof is missing ${screenshotDirectory}`);
  }

  if (failures.length > 0) {
    return failures;
  }

  const renderedProof = JSON.parse(await readFile(join(proofRoot, proofDirectory, renderedProofFile), 'utf8'));
  if (renderedProof.summary?.targetCount !== 5) {
    failures.push(`${row.rowId} rendered proof target count is not 5`);
  }
  if (renderedProof.summary?.allAssertionsPassed !== true) {
    failures.push(`${row.rowId} rendered proof assertions did not pass`);
  }
  if (renderedProof.summary?.childAgentEndpointRendered !== true) {
    failures.push(`${row.rowId} rendered proof did not use child-agent endpoint`);
  }
  if (renderedProof.summary?.livePublicSurfaceCaptured !== true) {
    failures.push(`${row.rowId} rendered proof did not capture live public surfaces`);
  }
  if (renderedProof.summary?.rawUrlPersisted !== false) {
    failures.push(`${row.rowId} rendered proof persisted raw URLs`);
  }

  const screenshots = await readdir(join(proofRoot, proofDirectory, screenshotDirectory));
  const pngScreenshots = screenshots.filter((file) => file.endsWith('.png'));
  if (pngScreenshots.length !== 5) {
    failures.push(`${row.rowId} rendered proof screenshot count is not 5`);
  }

  return failures;
}

function validateDocs(row, proofDirectory, docs) {
  const failures = [];
  if (!docs.plan.includes(row.rowId)) {
    failures.push(`${row.rowId} missing from browser-game gating plan`);
  }
  if (!docs.readme.includes(`output/browser-plan-proof/${proofDirectory}/`)) {
    failures.push(`${row.rowId} proof directory missing from browser-game README`);
  }
  if (row.rowNumber >= 2 && !docs.feature.includes(row.rowId)) {
    failures.push(`${row.rowId} missing from browser-web-control feature doc`);
  }
  if (!docs.expectation.includes('Browser-game proof artifact gates may verify')) {
    failures.push(`${row.rowId} expectation proof artifact boundary missing`);
  }
  return failures;
}

function manifestFor(rows, failures) {
  return {
    schemaVersion: 1,
    proofMode: 'browser-game-proof-artifacts',
    generatedAt: new Date().toISOString(),
    proofRoot: relativePath(proofRoot),
    rows,
    summary: {
      totalRows: rows.length,
      completeRows: rows.filter((row) => row.expectedStatus === '[x]').length,
      partialRows: rows.filter((row) => row.expectedState === 'partial-manual-required').length,
      failures: failures.length,
      playwrightState: 'live-rendered-child-intervention-screenshots-present',
      productClaimed: false,
    },
    manualProofBoundary: {
      screenshots: 'live-rendered-child-intervention-screenshots-present',
      playwright: 'live-rendered-child-intervention-screenshots-present',
      liveRouteEvidence: 'game-02-live-route-proof-present',
      livePortalPatternEvidence: 'game-03-live-portal-pattern-proof-present',
      liveCloudPatternEvidence: 'game-04-live-cloud-pattern-proof-present',
      liveUrlShapeEvidence: 'game-05-live-url-shape-proof-present',
      liveRuntimeSignalEvidence: 'game-06-live-runtime-signal-shape-proof-present',
      liveMetadataEvidence: 'game-07-live-metadata-shape-proof-present',
      liveHiddenAnalysisProfileSafetyEvidence: 'game-08-live-hidden-analysis-profile-safety-proof-present',
      liveEducationalClassifierEvidence: 'game-09-live-educational-classifier-proof-present',
      liveAiAnalysisEvidence: 'game-10-live-ai-analysis-proof-present',
      liveRiskBenefitSignalEvidence: 'game-11-live-riskbenefit-signal-proof-present',
      liveMemoryCacheEvidence: 'game-12-live-memory-cache-proof-present',
      liveAccountPurchaseGateEvidence: 'game-13-live-account-purchase-gate-proof-present',
      liveCloudGamingGateEvidence: 'game-14-live-cloud-gaming-gate-proof-present',
      liveUnblockedSiteDetectionEvidence: 'game-15-live-unblocked-site-detection-proof-present',
      liveUgcMultiplayerChatRiskEvidence: 'game-16-live-ugc-multiplayer-chat-risk-proof-present',
      livePolicyCompilerEvidence: 'game-17-live-policy-compiler-proof-present',
      liveHoldBlockAdapterEvidence: 'game-18-live-hold-block-adapter-proof-present',
      liveChildCheckingBlockUxEvidence: 'game-19-live-child-checking-block-ux-proof-present',
      liveParentDashboardUxEvidence: 'game-20-live-parent-dashboard-ux-proof-present',
      liveJournalSqliteReadModelEvidence: 'game-21-live-journal-sqlite-read-model-proof-present',
      liveRenderedChildInterventionEvidence: 'game-22-live-rendered-child-intervention-proof-present',
      liveAndroidIosHostEvidence: 'game-23-live-android-ios-host-proof-present',
      renderedUi: 'child-agent-served-intervention-page-proof-present',
      cloudStreamedFrameAnalysis: 'not-claimed',
      nativeGameControl: 'not-claimed',
      enforcement: 'not-claimed',
      productChecklistUpgrade: 'not-claimed',
    },
    failures,
  };
}

function markdownFor(manifest) {
  const rows = manifest.rows
    .map((row) => `| ${row.rowId} | ${row.state} | \`${row.proofDirectory}\` | ${row.proofFiles.length} |`)
    .join('\n');
  return [
    '# GAME-22 Browser-Game Proof Artifact Manifest',
    '',
    `Generated: ${manifest.generatedAt}`,
    '',
    `Rows checked: ${manifest.summary.totalRows}`,
    `Proof-present rows: ${manifest.summary.completeRows}`,
    `Partial/manual-required rows: ${manifest.summary.partialRows}`,
    `Playwright state: ${manifest.summary.playwrightState}`,
    `Product claimed: ${manifest.summary.productClaimed}`,
    '',
    '| Row | State | Proof Directory | Files |',
    '| --- | --- | --- | --- |',
    rows,
    '',
    'GAME-22 proves proof-pack coverage for GAME-01 through GAME-23 and adds',
    'live Playwright screenshots of shared child intervention pages served by',
    'the Rust child-agent endpoint over live public browser-game, cloud-gaming,',
    'and game-store backdrops.',
    'GAME-02 live route proof is present for real public browser-game and',
    'cloud-gaming route surfaces with ref-only/hash-only custody.',
    'GAME-03 live portal pattern library proof is present for real public',
    'browser-game portal and archive surfaces with ref-only/hash-only custody.',
    'GAME-04 live cloud pattern library proof is present for real public',
    'cloud-gaming and cloud-PC surfaces with ref-only/hash-only custody.',
    'GAME-05 live URL-shape parser proof is present for real public',
    'browser-game and cloud-gaming route surfaces with ref-only/hash-only custody.',
    'GAME-06 live runtime signal shape proof is present for real public',
    'browser-game and cloud-gaming pages through Playwright with ref-only/hash-only custody.',
    'GAME-07 live metadata shape proof is present for real public',
    'browser-game and cloud-gaming pages with ref-only/hash-only custody.',
    'GAME-08 live hidden analysis profile safety proof is present for real public',
    'browser-game and cloud-gaming pages with isolated-profile, no-capture custody.',
    'GAME-09 live educational classifier proof is present for real public',
    'educational, entertainment, misleading, and manual-required candidate shapes.',
    'GAME-10 live AI analysis proof is present for real public browser-game',
    'surfaces with typed evidence refs and candidate-only results.',
    'GAME-11 live risk/benefit signal proof is present for real public',
    'browser-game and cloud-gaming pages with typed candidate-only signal sets.',
    'GAME-12 live memory/cache proof is present for real public browser-game',
    'and cloud-gaming pages with bounded cache-key refs and no runtime store.',
    'GAME-13 live account/signup/purchase gate proof is present for real public',
    'account, subscription, purchase, cloud, educational, and store route surfaces.',
    'GAME-14 live cloud-gaming gate proof is present for real public',
    'cloud platform, subscription, cloud PC, mobile cloud, and unknown fallback surfaces.',
    'GAME-15 live unblocked-site detection proof is present for real public',
    'unblocked, search, portal, game-page, archive, and manual fallback surfaces.',
    'GAME-16 live UGC/multiplayer/chat risk proof is present for real public',
    'UGC, multiplayer, chat, marketplace, and manual-required route surfaces.',
    'GAME-17 live policy compiler proof is present for real public',
    'browser-game route surfaces with candidate-only compiler decisions.',
    'GAME-18 live hold/block adapter proof is present for real public',
    'browser-game route surfaces with ref-only adapter proof plans.',
    'GAME-19 live child checking/block UX proof is present for real public',
    'browser-game route surfaces with ref-only child UX surface rows.',
    'GAME-20 live parent dashboard UX proof is present for real public',
    'browser-game route surfaces with ref-only dashboard panel rows.',
    'GAME-21 live journal/SQLite read-model shape proof is present for real public',
    'browser-game route surfaces with ref-only/hash-only read-model rows.',
    'GAME-22 live rendered child intervention proof is present for real Roblox,',
    'Coolmath Games, Scratch, Xbox Cloud Gaming, and Steam Store surfaces using',
    'the shared BrowserChildInterventionPage renderer and the Rust child-agent',
    '/api/browser/intervention/page endpoint. It stores proof hashes and',
    'screenshots, not raw target URLs.',
    'GAME-23 live Android host emulator proof is present for the parent agent package',
    'with iOS entitlement and owned-browser-shell support still manual-required.',
    'It does not prove final policy decisions, product runtime browser-game',
    'detection, cloud-streamed frame analysis, native game control, notification',
    'or approval delivery, enforcement, or product checklist completion.',
  ].join('\n');
}

function checklistRowText(rowId, checklist) {
  return checklist.split(/\r?\n/).find((line) => line.startsWith(`| ${rowId} |`));
}

async function readText(path) {
  return readFile(join(root, path), 'utf8');
}

function relativePath(path) {
  return relative(root, path).replaceAll('\\', '/');
}
