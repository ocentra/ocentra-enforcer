import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import { BrowserGameMemoryCacheSnapshotSchema } from '@ocentra-parent/schema-domain/browser-game-memory-cache';

const repoRoot = process.cwd();
const proofId = 'browser-game-memory-cache-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-12-browser-game-memory-cache',
  '02-live-memory-cache-shape-proof.json'
);

const targets = [
  {
    targetId: 'poki-subway-surfers',
    url: 'https://poki.com/en/g/subway-surfers',
    ttlClass: 'short-dynamic-game-page',
    actionCandidate: 'parent-review-candidate',
    reasonCodes: ['parent-rule-match'],
    confidence: 'medium',
    categoryRef: 'game-category-browser-arcade-ref',
  },
  {
    targetId: 'code-org-minecraft',
    url: 'https://code.org/minecraft',
    ttlClass: 'stable-approved-game',
    actionCandidate: 'allow-candidate',
    reasonCodes: ['educational-benefit-present', 'parent-rule-match'],
    confidence: 'high',
    categoryRef: 'game-category-educational-ref',
  },
  {
    targetId: 'chess-play-online',
    url: 'https://www.chess.com/play/online',
    ttlClass: 'stable-approved-game',
    actionCandidate: 'time-limit-candidate',
    reasonCodes: ['educational-benefit-present', 'parent-rule-match'],
    confidence: 'medium',
    categoryRef: 'game-category-skill-ref',
  },
  {
    targetId: 'xbox-cloud-play',
    url: 'https://www.xbox.com/en-US/play',
    ttlClass: 'cloud-launcher-page',
    actionCandidate: 'manual-review-candidate',
    reasonCodes: ['cloud-gaming-risk', 'manual-required'],
    confidence: 'low',
    categoryRef: 'game-category-cloud-ref',
  },
  {
    targetId: 'roblox-discover',
    url: 'https://www.roblox.com/discover',
    ttlClass: 'ugc-game-page',
    actionCandidate: 'parent-review-candidate',
    reasonCodes: ['ugc-chat-risk', 'parent-rule-match'],
    confidence: 'medium',
    categoryRef: 'game-category-ugc-ref',
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const snapshots = captures.map(snapshotFor);
const negativeChecks = runNegativeChecks(snapshots[0]);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game memory/cache public captures to return HTTP 2xx/3xx responses');
}
if (!snapshots.every((snapshot) => BrowserGameMemoryCacheSnapshotSchema.safeParse(snapshot).success)) {
  throw new Error('Expected every browser-game memory/cache snapshot to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  const failedChecks = negativeChecks.filter((check) => !check.rejected).map((check) => check.name);
  throw new Error(
    `Expected browser-game memory/cache negative checks to reject overclaims: ${failedChecks.join(', ')}`
  );
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-memory-cache-shapes',
  targets: captures,
  snapshots,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    snapshotCount: snapshots.length,
    entryCount: snapshots.flatMap((snapshot) => snapshot.entries).length,
    negativeChecks: negativeChecks.length,
    subjectKinds: [...new Set(snapshots.flatMap((snapshot) => snapshot.entries.map((entry) => entry.subjectKind)))],
    cacheKeyKinds: [
      ...new Set(
        snapshots.flatMap((snapshot) => snapshot.entries.flatMap((entry) => entry.cacheKeys.map((key) => key.keyKind)))
      ),
    ],
    rawUrlStored: false,
    rawPlatformGameIdStored: false,
    rawCloudGameTitleStored: false,
    rawGamePayloadStored: false,
    rawModelTextStored: false,
    runtimeCacheStoreClaimed: false,
    aiCacheClaimed: false,
    uiDeliveredClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    finalPolicyDecisionClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-memory-cache-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(
  `targets=${captures.length} snapshots=${snapshots.length} entries=${proof.summary.entryCount} negativeChecks=${negativeChecks.length}`
);

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
    rawPlatformGameIdPersisted: false,
    rawCloudGameTitlePersisted: false,
    rawGamePayloadPersisted: false,
    rawModelTextPersisted: false,
  };
}

function snapshotFor(capture) {
  const target = targetFor(capture.targetId);
  return {
    schemaVersion: 'browser-game-memory-cache-contract',
    snapshotId: `browser-game-memory-cache-snapshot-${capture.targetId}`,
    familyId: 'family-browser-game-memory-live-proof',
    childProfileId: 'child-browser-game-memory-live-proof',
    capturedAt: startedAt,
    entries: [gameUrlEntry(capture, target), categoryEntry(capture, target), parentDecisionEntry(capture, target)],
    retentionBounded: true,
    rawGameContentStored: false,
    runtimeStoreClaimed: false,
  };
}

function gameUrlEntry(capture, target) {
  return memoryEntry(capture, target, 'game-url-ref', {
    entryId: `browser-game-memory-url-${capture.targetId}`,
    ttlClass: target.ttlClass,
    ttlMs: ttlMsFor(target.ttlClass),
    actionCandidate: target.actionCandidate,
    reasonCodes: target.reasonCodes,
    confidence: target.confidence,
    cacheKeys: [
      ...baseKeys(capture),
      cacheKey('canonical-url-ref', `canonical-url-ref-${capture.inputOriginSha256.slice(0, 16)}`),
      cacheKey('domain-path-hash', `domain-path-hash-${capture.inputPathSha256.slice(0, 16)}`),
      cacheKey('evidence-ref', `evidence-ref-${capture.targetId}`),
    ],
    decisionRefs: [`browser-game-memory-decision-${capture.targetId}`],
  });
}

function categoryEntry(capture, target) {
  return memoryEntry(capture, target, 'category-ref', {
    entryId: `browser-game-memory-category-${capture.targetId}`,
    ttlClass: 'short-dynamic-game-page',
    ttlMs: 600000,
    actionCandidate: target.actionCandidate,
    reasonCodes: target.reasonCodes,
    confidence: target.confidence,
    cacheKeys: [...baseKeys(capture), cacheKey('game-category-ref', target.categoryRef)],
    decisionRefs: [`browser-game-memory-category-decision-${capture.targetId}`],
  });
}

function parentDecisionEntry(capture, target) {
  return memoryEntry(capture, target, 'parent-decision-ref', {
    entryId: `browser-game-memory-parent-decision-${capture.targetId}`,
    ttlClass: 'parent-approved-account-page',
    ttlMs: 43200000,
    actionCandidate: target.actionCandidate,
    reasonCodes: target.reasonCodes,
    confidence: target.confidence,
    cacheKeys: [...baseKeys(capture), cacheKey('parent-decision-ref', `parent-decision-ref-${capture.targetId}`)],
    decisionRefs: [`browser-game-memory-parent-decision-${capture.targetId}`],
  });
}

function memoryEntry(capture, target, subjectKind, overrides) {
  const ttlMs = overrides.ttlMs ?? ttlMsFor(overrides.ttlClass ?? target.ttlClass);
  return {
    schemaVersion: 'browser-game-memory-cache-contract',
    entryId: `browser-game-memory-entry-${capture.targetId}`,
    familyId: 'family-browser-game-memory-live-proof',
    childProfileId: 'child-browser-game-memory-live-proof',
    policyVersionRef: 'policy-version-browser-game-memory-live-proof',
    storedAt: startedAt,
    expiresAt: expiresAt(ttlMs),
    ttlMs,
    ttlClass: target.ttlClass,
    subjectKind,
    memoryState: 'fresh-hit',
    decisionSource: 'parent-decision-candidate',
    actionCandidate: target.actionCandidate,
    reasonCodes: target.reasonCodes,
    confidence: target.confidence,
    cacheKeys: baseKeys(capture),
    sourceEvidenceRefs: [`parent-proof-${proofId}-${capture.targetId}-source`],
    decisionRefs: [`browser-game-memory-decision-${capture.targetId}`],
    invalidationReasons: [],
    canReuseForPolicyInput: true,
    finalPolicyDecisionClaimed: false,
    runtimeCacheStoreClaimed: false,
    aiCacheClaimed: false,
    rawCanonicalUrlStored: false,
    rawPlatformGameIdStored: false,
    rawCloudGameTitleStored: false,
    rawGamePayloadStored: false,
    rawModelTextStored: false,
    uiDeliveredClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    enforcementClaimed: false,
    ...overrides,
  };
}

function runNegativeChecks(validSnapshot) {
  const invalidSnapshotChecks = [
    ['snapshot-raw-game-content', { rawGameContentStored: true }],
    ['snapshot-runtime-store', { runtimeStoreClaimed: true }],
    [
      'snapshot-missing-category-row',
      { entries: validSnapshot.entries.filter((entry) => entry.subjectKind !== 'category-ref') },
    ],
  ];
  const invalidEntryChecks = [
    ['entry-final-policy-decision', { finalPolicyDecisionClaimed: true }],
    ['entry-runtime-cache-store', { runtimeCacheStoreClaimed: true }],
    ['entry-ai-cache', { aiCacheClaimed: true }],
    ['entry-raw-canonical-url', { rawCanonicalUrlStored: true }],
    ['entry-raw-platform-game-id', { rawPlatformGameIdStored: true }],
    ['entry-raw-cloud-title', { rawCloudGameTitleStored: true }],
    ['entry-raw-game-payload', { rawGamePayloadStored: true }],
    ['entry-raw-model-text', { rawModelTextStored: true }],
    ['entry-ui-delivered', { uiDeliveredClaimed: true }],
    ['entry-native-game-control', { nativeGameControlClaimed: true }],
    ['entry-cloud-frame-analysis', { cloudFrameAnalysisClaimed: true }],
    ['entry-enforcement', { enforcementClaimed: true }],
    ['entry-missing-subject-key', { cacheKeys: baseKeys({ targetId: 'invalid' }) }],
    ['entry-overlong-dynamic-ttl', { ttlClass: 'short-dynamic-game-page', ttlMs: 600001 }],
    ['entry-fresh-no-decision-ref', { decisionRefs: [] }],
    [
      'entry-stale-reuse',
      { memoryState: 'stale-hit', invalidationReasons: ['ttl-expired'], canReuseForPolicyInput: true },
    ],
    [
      'entry-miss-with-decision-ref',
      {
        memoryState: 'miss',
        invalidationReasons: ['ttl-expired'],
        decisionRefs: ['bad-decision-ref'],
        canReuseForPolicyInput: false,
      },
    ],
  ];

  return [
    ...invalidSnapshotChecks.map(([name, invalid]) => negativeSnapshotCheck(name, validSnapshot, invalid)),
    ...invalidEntryChecks.map(([name, invalid]) =>
      negativeSnapshotCheck(name, validSnapshot, {
        entries: replaceEntry(validSnapshot, 'game-url-ref', invalid),
      })
    ),
  ];
}

function negativeSnapshotCheck(name, validSnapshot, invalid) {
  return {
    name,
    rejected: !BrowserGameMemoryCacheSnapshotSchema.safeParse({ ...validSnapshot, ...invalid }).success,
  };
}

function replaceEntry(snapshot, subjectKind, invalid) {
  return snapshot.entries.map((entry) => (entry.subjectKind === subjectKind ? { ...entry, ...invalid } : entry));
}

function baseKeys(capture) {
  return [
    cacheKey('policy-version', 'policy-version-browser-game-memory-live-proof'),
    cacheKey('child-profile', 'child-browser-game-memory-live-proof'),
    cacheKey('parent-rule-set', 'parent-rule-set-browser-game-memory-live-proof'),
    cacheKey('evidence-ref', `evidence-ref-${capture.targetId}`),
  ];
}

function cacheKey(keyKind, keyValue) {
  return { keyKind, keyValue };
}

function ttlMsFor(ttlClass) {
  if (ttlClass === 'stable-approved-game') {
    return 86400000;
  }
  if (ttlClass === 'parent-approved-account-page') {
    return 43200000;
  }
  return 600000;
}

function expiresAt(ttlMs) {
  return new Date(Date.parse(startedAt) + ttlMs).toISOString();
}

function targetFor(targetId) {
  const target = targets.find((entry) => entry.targetId === targetId);
  if (!target) {
    throw new Error(`Unknown target ${targetId}`);
  }
  return target;
}

async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}
