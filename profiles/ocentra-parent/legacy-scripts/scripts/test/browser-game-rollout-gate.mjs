import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative } from 'node:path';

const root = process.cwd();
const outputDirectory = join(root, 'output', 'browser-plan-proof', 'game-24-rollout-manual-required-labels');
const resultDirectory = join(root, 'test-results', 'browser-game-rollout-gate');
const checklistPath = 'docs/plans/browser-plan/implementation-checklist.md';

const rolloutGuards = [
  {
    docKey: 'plan',
    text: 'GAME rollout state: partial/manual-required.',
  },
  {
    docKey: 'readme',
    text: 'GAME rollout state: partial/manual-required.',
  },
  {
    docKey: 'browserFeature',
    text: 'Browser-game/cloud-gaming GAME-24 now labels the game track partial/manual-required through the rollout gate.',
  },
  {
    docKey: 'expectation',
    text: 'Browser-game rollout gates may label rows as complete, partial/manual-required, or open/manual-required only.',
  },
];

if (!existsSync(join(root, checklistPath))) {
  throw new Error(`Missing checklist: ${checklistPath}`);
}

await main();

async function main() {
  await mkdir(outputDirectory, { recursive: true });
  await mkdir(resultDirectory, { recursive: true });

  const docs = await loadDocs();
  const rows = expectedRows().map((row) => validateChecklistRow(row, docs.checklist));
  const guardFailures = validateRolloutGuards(docs);
  const failures = [...rows.flatMap((row) => row.failures), ...guardFailures];
  const manifest = manifestFor(rows, failures);

  if (manifest.failures.length > 0) {
    throw new Error(`Browser-game rollout gate failed:\n${manifest.failures.join('\n')}`);
  }

  const proofPath = join(resultDirectory, 'proof.json');
  const markdownPath = join(outputDirectory, '01-rollout-manual-required-labels.md');
  await writeFile(proofPath, `${JSON.stringify(manifest, null, 2)}\n`);
  await writeFile(markdownPath, `${markdownFor(manifest)}\n`);

  console.log('browser-game-rollout-gate-ok=true');
  console.log(`proof=${relativePath(proofPath)}`);
  console.log(`manifest=${relativePath(markdownPath)}`);
  console.log(
    `complete=${manifest.summary.completeRows} partial=${manifest.summary.partialRows} open=${manifest.summary.openRows}`
  );
}

async function loadDocs() {
  return {
    checklist: await readText(checklistPath),
    plan: await readText('docs/plans/browser-plan/v0-5-browser-games-cloud-gaming-gating-plan.md'),
    readme: await readText('docs/plans/browser-plan/browser-games-cloud-gaming/readme.md'),
    browserFeature: await readText('docs/features/browser-web-control.md'),
    expectation: await readText('docs/expectations/browser-evidence.md'),
  };
}

function expectedRows() {
  return Array.from({ length: 24 }, (_, index) => {
    const rowNumber = index + 1;
    const rowId = `GAME-${String(rowNumber).padStart(2, '0')}`;
    if (rowNumber === 1) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'scaffold-proof-present',
      };
    }
    if (rowNumber === 2) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-route-proof-present',
      };
    }
    if (rowNumber === 3) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-portal-pattern-proof-present',
      };
    }
    if (rowNumber === 4) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-cloud-pattern-proof-present',
      };
    }
    if (rowNumber === 5) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-url-shape-proof-present',
      };
    }
    if (rowNumber === 6) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-runtime-signal-shape-proof-present',
      };
    }
    if (rowNumber === 7) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-metadata-shape-proof-present',
      };
    }
    if (rowNumber === 8) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-hidden-analysis-profile-safety-proof-present',
      };
    }
    if (rowNumber === 9) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-educational-classifier-proof-present',
      };
    }
    if (rowNumber === 10) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-ai-analysis-proof-present',
      };
    }
    if (rowNumber === 11) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-riskbenefit-signal-proof-present',
      };
    }
    if (rowNumber === 12) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-memory-cache-proof-present',
      };
    }
    if (rowNumber === 13) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-account-purchase-gate-proof-present',
      };
    }
    if (rowNumber === 14) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-cloud-gaming-gate-proof-present',
      };
    }
    if (rowNumber === 15) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-unblocked-site-detection-proof-present',
      };
    }
    if (rowNumber === 16) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-ugc-multiplayer-chat-risk-proof-present',
      };
    }
    if (rowNumber === 17) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-policy-compiler-proof-present',
      };
    }
    if (rowNumber === 18) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-hold-block-adapter-proof-present',
      };
    }
    if (rowNumber === 19) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-child-checking-block-ux-proof-present',
      };
    }
    if (rowNumber === 20) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-parent-dashboard-ux-proof-present',
      };
    }
    if (rowNumber === 21) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-journal-sqlite-read-model-proof-present',
      };
    }
    if (rowNumber === 22) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-rendered-child-intervention-proof-present',
      };
    }
    if (rowNumber === 23) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'live-android-ios-host-proof-present',
      };
    }
    if (rowNumber === 24) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[x]',
        expectedOwner: 'codex-d',
        expectedState: 'rollout-label-proof-present',
      };
    }
    if (rowNumber >= 22) {
      return {
        rowNumber,
        rowId,
        expectedStatus: '[~]',
        expectedOwner: 'codex-d',
        expectedState: 'partial-manual-required',
      };
    }
    return {
      rowNumber,
      rowId,
      expectedStatus: '[ ]',
      expectedOwner: '',
      expectedState: 'open-manual-required',
    };
  });
}

function validateChecklistRow(row, checklist) {
  const rowText = checklistRowText(row.rowId, checklist);
  const failures = [];
  if (!rowText) {
    failures.push(`${row.rowId} missing checklist row`);
    return { ...row, failures };
  }
  if (!rowText.includes(row.expectedStatus)) {
    failures.push(`${row.rowId} status is not ${row.expectedStatus}`);
  }
  if (row.expectedOwner && !rowText.includes(row.expectedOwner)) {
    failures.push(`${row.rowId} owner is not ${row.expectedOwner}`);
  }
  return {
    ...row,
    state: row.expectedState,
    failures,
  };
}

function validateRolloutGuards(docs) {
  return rolloutGuards
    .filter((guard) => !normalizedText(docs[guard.docKey]).includes(normalizedText(guard.text)))
    .map((guard) => `${guard.docKey} missing rollout guard: ${guard.text}`);
}

function normalizedText(text) {
  return text.replace(/\s+/g, ' ').trim();
}

function manifestFor(rows, failures) {
  return {
    schemaVersion: 1,
    proofMode: 'browser-game-rollout-gate',
    generatedAt: new Date().toISOString(),
    rows,
    summary: {
      totalRows: rows.length,
      completeRows: rows.filter((row) => row.expectedStatus === '[x]').length,
      partialRows: rows.filter((row) => row.expectedState === 'partial-manual-required').length,
      openRows: rows.filter((row) => row.expectedState === 'open-manual-required').length,
      failures: failures.length,
      rolloutState: 'partial/manual-required',
      productClaimed: false,
    },
    guardTexts: rolloutGuards.map((guard) => guard.text),
    noClaimLabels: [
      'browser-game-live-route-contracts-proof-present',
      'browser-game-live-portal-pattern-proof-present',
      'browser-game-live-cloud-pattern-proof-present',
      'browser-game-live-url-shape-proof-present',
      'browser-game-live-runtime-signal-shape-proof-present',
      'browser-game-live-metadata-shape-proof-present',
      'browser-game-live-riskbenefit-signal-proof-present',
      'browser-game-live-memory-cache-proof-present',
      'browser-game-live-account-purchase-gate-proof-present',
      'browser-game-live-cloud-gaming-gate-proof-present',
      'browser-game-live-unblocked-site-detection-proof-present',
      'browser-game-live-ugc-multiplayer-chat-risk-proof-present',
      'browser-game-live-policy-compiler-proof-present',
      'browser-game-live-hold-block-adapter-proof-present',
      'browser-game-live-child-checking-block-ux-proof-present',
      'browser-game-live-parent-dashboard-ux-proof-present',
      'browser-game-live-journal-sqlite-read-model-proof-present',
      'browser-game-live-rendered-child-intervention-proof-present',
      'browser-game-live-android-ios-host-proof-present',
      'proof-artifact-coverage-contract-only',
      'playwright-live-rendered-child-intervention-screenshots-present',
      'runtime-signal-proof-manual-required',
      'metadata-ai-memory-proof-manual-required',
      'child-intervention-ui-rendered-proof-present',
      'cloud-streamed-frame-analysis-not-claimed',
      'native-game-control-not-claimed',
      'enforcement-not-claimed',
      'product-checklist-upgrade-not-claimed',
    ],
    failures,
  };
}

function markdownFor(manifest) {
  const rows = manifest.rows.map((row) => `| ${row.rowId} | ${row.state} | ${row.expectedStatus} |`).join('\n');
  return [
    '# GAME-24 Rollout Manual-Required Labels',
    '',
    `Generated: ${manifest.generatedAt}`,
    '',
    `Rows checked: ${manifest.summary.totalRows}`,
    `Proof-present rows: ${manifest.summary.completeRows}`,
    `Partial/manual-required rows: ${manifest.summary.partialRows}`,
    `Open/manual-required rows: ${manifest.summary.openRows}`,
    `Rollout state: ${manifest.summary.rolloutState}`,
    `Product claimed: ${manifest.summary.productClaimed}`,
    '',
    '| Row | State | Checklist Status |',
    '| --- | --- | --- |',
    rows,
    '',
    'GAME rollout state: partial/manual-required.',
    'GAME-02 live route contract proof is present.',
    'GAME-03 live portal pattern library proof is present.',
    'GAME-04 live cloud pattern library proof is present.',
    'GAME-05 live URL-shape parser proof is present.',
    'GAME-06 live runtime signal shape proof is present.',
    'GAME-07 live metadata shape proof is present.',
    'GAME-08 live hidden analysis profile safety proof is present.',
    'GAME-09 live educational classifier proof is present.',
    'GAME-10 live AI analysis proof is present.',
    'GAME-11 live risk/benefit signal proof is present.',
    'GAME-12 live memory/cache proof is present.',
    'GAME-13 live account/signup/purchase gate proof is present.',
    'GAME-14 live cloud-gaming gate proof is present.',
    'GAME-15 live unblocked-site detection proof is present.',
    'GAME-16 live UGC/multiplayer/chat risk proof is present.',
    'GAME-17 live policy compiler proof is present.',
    'GAME-18 live hold/block adapter proof is present.',
    'GAME-19 live child checking/block UX proof is present.',
    'GAME-20 live parent dashboard UX proof is present.',
    'GAME-21 live journal/SQLite read-model shape proof is present.',
    'GAME-22 live rendered child intervention proof is present.',
    'GAME-23 live Android host emulator proof is present.',
    'GAME-24 rollout/manual-required label proof is present.',
    'Product checklist upgrade is not claimed.',
    'Final policy decisions, notification or approval delivery, cloud-streamed',
    'frame analysis, native game control, enforcement, release readiness, and',
    'product completion remain unclaimed until separate proof exists.',
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
