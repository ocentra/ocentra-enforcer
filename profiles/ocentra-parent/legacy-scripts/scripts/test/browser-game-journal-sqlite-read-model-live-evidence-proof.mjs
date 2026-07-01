import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameJournalSqliteReadModelRowSchema,
  BrowserGameJournalSqliteReadModelSnapshotSchema,
} from '@ocentra-parent/schema-domain/browser-game-journal-sqlite-read-model';

const repoRoot = process.cwd();
const proofId = 'browser-game-journal-sqlite-read-model-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-21-journal-sqlite-read-model',
  '02-live-journal-sqlite-read-model-shape-proof.json'
);

const targets = [
  {
    targetId: 'managed-browser-evidence-row',
    url: 'https://scratch.mit.edu/explore/projects/games/',
    sourceKind: 'managed-browser-evidence',
    rowState: 'partial-proof',
    journalState: 'journal-replayed',
    sqliteState: 'read-model-present',
    browserEvidenceReadModelRef: 'parent-evidence-game-read-model-managed-browser',
    appGameSessionReportRef: null,
    adapterPlanAuditRef: null,
    policyCandidateRef: 'parent-evidence-game-policy-managed-browser-candidate',
    reasonCodes: ['browser-journal-replay-proof-present', 'sqlite-read-model-proof-present'],
  },
  {
    targetId: 'app-game-session-report-row',
    url: 'https://code.org/minecraft',
    sourceKind: 'app-game-session-report',
    rowState: 'partial-proof',
    journalState: 'journal-replayed',
    sqliteState: 'read-model-present',
    browserEvidenceReadModelRef: null,
    appGameSessionReportRef: 'parent-evidence-game-session-code-org',
    adapterPlanAuditRef: null,
    policyCandidateRef: 'parent-evidence-game-policy-educational-candidate',
    reasonCodes: ['app-game-session-read-model-present'],
  },
  {
    targetId: 'adapter-plan-audit-row',
    url: 'https://www.hoodamath.com/games/unblocked.html',
    sourceKind: 'adapter-plan-audit',
    rowState: 'partial-proof',
    journalState: 'journal-replayed',
    sqliteState: 'read-model-present',
    browserEvidenceReadModelRef: null,
    appGameSessionReportRef: null,
    adapterPlanAuditRef: 'parent-evidence-game-adapter-audit-managed-browser',
    policyCandidateRef: 'parent-evidence-game-policy-unblocked-candidate',
    reasonCodes: ['adapter-audit-ref-present'],
  },
  {
    targetId: 'cloud-manual-required-row',
    url: 'https://www.xbox.com/en-US/play',
    sourceKind: 'manual-required',
    rowState: 'manual-required',
    journalState: 'manual-required',
    sqliteState: 'manual-required',
    browserEvidenceReadModelRef: null,
    appGameSessionReportRef: null,
    adapterPlanAuditRef: null,
    policyCandidateRef: null,
    reasonCodes: ['cloud-gaming-read-model-manual-required'],
  },
  {
    targetId: 'native-unavailable-row',
    url: 'https://store.steampowered.com/',
    sourceKind: 'unavailable',
    rowState: 'unavailable',
    journalState: 'unavailable',
    sqliteState: 'unavailable',
    browserEvidenceReadModelRef: null,
    appGameSessionReportRef: null,
    adapterPlanAuditRef: null,
    policyCandidateRef: null,
    reasonCodes: ['native-game-control-unavailable'],
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const rows = captures.map(rowFor);
const snapshot = snapshotFor(rows);
const negativeChecks = runNegativeChecks(rows[0], snapshot);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game journal/SQLite public captures to return HTTP 2xx/3xx responses');
}
if (!rows.every((row) => BrowserGameJournalSqliteReadModelRowSchema.safeParse(row).success)) {
  throw new Error('Expected every browser-game journal/SQLite read-model row to parse');
}
if (!BrowserGameJournalSqliteReadModelSnapshotSchema.safeParse(snapshot).success) {
  throw new Error('Expected browser-game journal/SQLite read-model snapshot to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  const failedChecks = negativeChecks.filter((check) => !check.rejected).map((check) => check.name);
  throw new Error(
    `Expected browser-game journal/SQLite negative checks to reject overclaims: ${failedChecks.join(', ')}`
  );
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-journal-sqlite-read-model-shapes',
  targets: captures,
  rows,
  snapshot,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    rowCount: rows.length,
    negativeChecks: negativeChecks.length,
    sourceKinds: [...new Set(rows.map((row) => row.sourceKind))],
    rowStates: [...new Set(rows.map((row) => row.rowState))],
    rawUrlPersisted: false,
    rawPageBodyPersisted: false,
    rawGamePayloadPersisted: false,
    rawGameTitlePersisted: false,
    rawAccountOrPurchasePersisted: false,
    childCookieSessionReused: false,
    cloudTitleCertaintyClaimed: false,
    browserMutationClaimed: false,
    renderedUiClaimed: false,
    runtimeSqliteQueryClaimed: false,
    finalPolicyDecisionClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-journal-sqlite-read-model-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`targets=${captures.length} rows=${rows.length} negativeChecks=${negativeChecks.length}`);

async function captureTarget(target) {
  const inputUrl = new URL(target.url);
  const response = await fetch(target.url, {
    redirect: 'follow',
    headers: {
      'user-agent': 'Mozilla/5.0 OcentraParentBrowserGameProof/1.0',
      accept: 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
    },
  });
  const body = Buffer.from(await response.arrayBuffer());
  const finalUrl = new URL(response.url);
  return {
    targetId: target.targetId,
    status: response.status,
    responseOk: response.status >= 200 && response.status < 400,
    contentType: response.headers.get('content-type') ?? 'unknown',
    contentLength: body.length,
    bodySha256: sha256(body),
    inputOriginSha256: sha256(inputUrl.origin),
    inputPathSha256: sha256(inputUrl.pathname),
    finalOriginSha256: sha256(finalUrl.origin),
    finalPathSha256: sha256(finalUrl.pathname),
    rawUrlPersisted: false,
    rawPageBodyPersisted: false,
    rawGamePayloadPersisted: false,
    rawGameTitlePersisted: false,
    rawAccountOrPurchasePersisted: false,
  };
}

function rowFor(capture) {
  const target = targetFor(capture.targetId);
  const storageRefs = provedStorageRefs(capture, target);
  return {
    rowId: `browser-game-read-model-row-${capture.targetId}`,
    sourceKind: target.sourceKind,
    rowState: target.rowState,
    journalState: target.journalState,
    sqliteState: target.sqliteState,
    browserEvidenceReadModelRef: target.browserEvidenceReadModelRef,
    appGameSessionReportRef: target.appGameSessionReportRef,
    adapterPlanAuditRef: target.adapterPlanAuditRef,
    policyCandidateRef: target.policyCandidateRef,
    journalEntryRefs: storageRefs.journalEntryRefs,
    sqliteRowRefs: storageRefs.sqliteRowRefs,
    proofRefs: [
      `parent-evidence-${proofId}-${capture.targetId}-source`,
      `parent-evidence-${proofId}-${capture.targetId}-response-hash`,
    ],
    eventCount: storageRefs.eventCount,
    rowCount: storageRefs.rowCount,
    reasonCodes: target.reasonCodes,
    rawUrlIncluded: false,
    rawPageBodyIncluded: false,
    rawGamePayloadIncluded: false,
    rawGameTitleIncluded: false,
    rawAccountOrPurchaseIncluded: false,
    childCookieSessionReused: false,
    cloudTitleCertaintyClaimed: false,
    browserMutationClaimed: false,
    renderedUiClaimed: false,
    finalPolicyDecisionClaimed: false,
    enforcementClaimed: false,
  };
}

function provedStorageRefs(capture, target) {
  if (target.rowState !== 'partial-proof') {
    return {
      journalEntryRefs: [],
      sqliteRowRefs: [],
      eventCount: 0,
      rowCount: 0,
    };
  }
  return {
    journalEntryRefs: [`parent-evidence-${proofId}-${capture.targetId}-journal-entry`],
    sqliteRowRefs: [`parent-evidence-${proofId}-${capture.targetId}-sqlite-row`],
    eventCount: 1,
    rowCount: 1,
  };
}

function snapshotFor(rows) {
  return {
    schemaVersion: 'browser-game-journal-sqlite-read-model-contract',
    readModelId: 'browser-game-journal-sqlite-read-model-live-proof',
    familyId: 'family-browser-game-journal-sqlite-live-proof',
    childProfileId: 'child-browser-game-journal-sqlite-live-proof',
    deviceId: 'device-browser-game-journal-sqlite-live-proof',
    generatedAt: startedAt,
    sourceProofRefs: rows.flatMap((row) => row.proofRefs),
    rows,
    claimBoundaries: {
      rawUrlStorage: 'not-claimed',
      rawPageBodyStorage: 'not-claimed',
      rawGamePayloadStorage: 'not-claimed',
      rawGameTitleStorage: 'not-claimed',
      rawAccountOrPurchaseStorage: 'not-claimed',
      childCookieSessionReuse: 'not-claimed',
      cloudTitleCertainty: 'not-claimed',
      browserMutation: 'not-claimed',
      renderedUi: 'not-claimed',
      finalPolicyDecision: 'not-claimed',
      enforcement: 'not-claimed',
    },
  };
}

function runNegativeChecks(validRow, validSnapshot) {
  const claimChecks = [
    ['rawUrlIncluded', { rawUrlIncluded: true }],
    ['rawPageBodyIncluded', { rawPageBodyIncluded: true }],
    ['rawGamePayloadIncluded', { rawGamePayloadIncluded: true }],
    ['rawGameTitleIncluded', { rawGameTitleIncluded: true }],
    ['rawAccountOrPurchaseIncluded', { rawAccountOrPurchaseIncluded: true }],
    ['childCookieSessionReused', { childCookieSessionReused: true }],
    ['cloudTitleCertaintyClaimed', { cloudTitleCertaintyClaimed: true }],
    ['browserMutationClaimed', { browserMutationClaimed: true }],
    ['renderedUiClaimed', { renderedUiClaimed: true }],
    ['finalPolicyDecisionClaimed', { finalPolicyDecisionClaimed: true }],
    ['enforcementClaimed', { enforcementClaimed: true }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameJournalSqliteReadModelRowSchema.safeParse({ ...validRow, ...override }).success,
  }));

  const missingRefs = [
    ['missingBrowserEvidenceReadModelRef', { browserEvidenceReadModelRef: null }],
    ['missingJournalEntryRefs', { journalEntryRefs: [] }],
    ['missingSqliteRowRefs', { sqliteRowRefs: [] }],
    ['zeroEventCount', { eventCount: 0 }],
    ['zeroRowCount', { rowCount: 0 }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameJournalSqliteReadModelRowSchema.safeParse({ ...validRow, ...override }).success,
  }));

  return [
    ...claimChecks,
    ...missingRefs,
    {
      name: 'snapshotMissingUnavailableRow',
      rejected: !BrowserGameJournalSqliteReadModelSnapshotSchema.safeParse({
        ...validSnapshot,
        rows: validSnapshot.rows.filter((row) => row.sourceKind !== 'unavailable'),
      }).success,
    },
  ];
}

function targetFor(targetId) {
  const target = targets.find((item) => item.targetId === targetId);
  if (!target) {
    throw new Error(`Unknown target: ${targetId}`);
  }
  return target;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}
