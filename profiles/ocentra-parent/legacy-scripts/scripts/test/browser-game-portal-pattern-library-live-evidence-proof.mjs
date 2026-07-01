import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGamePortalPatternEntrySchema,
  BrowserGamePortalPatternLibrarySchema,
  decodeBrowserGamePortalPatternLibrary,
} from '@ocentra-parent/schema-domain/browser-game-portal-pattern-library';

const repoRoot = process.cwd();
const proofId = 'browser-game-portal-pattern-library-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-03-known-game-portal-pattern-library',
  '05-live-pattern-library-proof.json'
);

const targets = [
  {
    targetId: 'crazygames-bloxdhop',
    url: 'https://www.crazygames.com/game/bloxdhop-io',
    portalFamily: 'known-game-portal',
    routeKinds: ['game-detail-route', 'play-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'game-id-segment'],
    educationalCandidate: false,
    ugcCandidate: true,
    purchaseFlowCandidate: false,
  },
  {
    targetId: 'poki-subway-surfers',
    url: 'https://poki.com/en/g/subway-surfers',
    portalFamily: 'known-game-portal',
    routeKinds: ['game-detail-route', 'play-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'game-id-segment'],
    educationalCandidate: false,
    ugcCandidate: false,
    purchaseFlowCandidate: false,
  },
  {
    targetId: 'coolmath-run-3',
    url: 'https://www.coolmathgames.com/0-run-3',
    portalFamily: 'educational-game-portal',
    routeKinds: ['game-detail-route', 'play-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'game-id-segment'],
    educationalCandidate: true,
    ugcCandidate: false,
    purchaseFlowCandidate: false,
  },
  {
    targetId: 'itch-html5-catalog',
    url: 'https://itch.io/games/html5',
    portalFamily: 'indie-game-marketplace',
    routeKinds: ['catalog-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'catalog-grid'],
    educationalCandidate: false,
    ugcCandidate: true,
    purchaseFlowCandidate: true,
  },
  {
    targetId: 'internet-archive-software-games',
    url: 'https://archive.org/details/softwarelibrary_msdos_games',
    portalFamily: 'classic-game-archive',
    routeKinds: ['catalog-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'catalog-grid'],
    educationalCandidate: false,
    ugcCandidate: false,
    purchaseFlowCandidate: false,
  },
  {
    targetId: 'chess-play',
    url: 'https://www.chess.com/play',
    portalFamily: 'known-game-portal',
    routeKinds: ['play-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref'],
    educationalCandidate: false,
    ugcCandidate: false,
    purchaseFlowCandidate: false,
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const patterns = captures.map(patternEntryFor);
const library = patternLibraryFor(patterns);
const negativeChecks = runNegativeChecks(patterns[0], library);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game portal public captures to return HTTP 2xx/3xx responses');
}
if (!patterns.every((pattern) => BrowserGamePortalPatternEntrySchema.safeParse(pattern).success)) {
  throw new Error('Expected every live browser-game portal pattern entry to parse');
}
decodeBrowserGamePortalPatternLibrary(library);

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-portal-patterns',
  targets: captures,
  library,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    patternRows: patterns.length,
    negativeChecks: negativeChecks.length,
    rawDomainStored: false,
    rawUrlStored: false,
    rawPageTitleStored: false,
    rawPageBodyStored: false,
    runtimeDetectionClaimed: false,
    aiClassificationClaimed: false,
    policyDecisionClaimed: false,
    cloudGamingOwnershipClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-portal-pattern-library-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`targets=${captures.length} negativeChecks=${negativeChecks.length}`);

async function captureTarget(target) {
  const parsed = new URL(target.url);
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
    originSha256: sha256(parsed.origin),
    pathSha256: sha256(parsed.pathname),
    finalOriginSha256: sha256(finalUrl.origin),
    finalPathSha256: sha256(finalUrl.pathname),
    portalFamily: target.portalFamily,
    routeKinds: target.routeKinds,
    signalKinds: target.signalKinds,
    rawUrlPersisted: false,
    rawDomainPersisted: false,
    rawPathPersisted: false,
    rawPageTitlePersisted: false,
    rawPageBodyPersisted: false,
  };
}

function patternEntryFor(capture) {
  const target = targets.find((entry) => entry.targetId === capture.targetId);
  return {
    patternId: `game-portal-pattern-${capture.targetId}`,
    portalFamily: target.portalFamily,
    routeKinds: target.routeKinds,
    signalKinds: target.signalKinds,
    patternFingerprint: `game-portal-pattern-fingerprint-${sha256(
      `${capture.originSha256}:${capture.pathSha256}`
    ).slice(0, 32)}`,
    sourceEvidenceRefs: [`parent-proof-${proofId}-${capture.targetId}`],
    confidence: 'high',
    reviewState: 'reviewed',
    educationalCandidate: target.educationalCandidate,
    ugcCandidate: target.ugcCandidate,
    purchaseFlowCandidate: target.purchaseFlowCandidate,
    cloudGamingCandidate: false,
    rawDomainStored: false,
    rawUrlStored: false,
    rawPageTitleStored: false,
    rawPageBodyStored: false,
    runtimeDetectionClaimed: false,
    aiClassificationClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
  };
}

function patternLibraryFor(patterns) {
  return {
    schemaVersion: 'browser-game-portal-pattern-library-contract',
    libraryId: `game-portal-pattern-library-${proofId}`,
    generatedAt: startedAt,
    sourceEvidenceRefs: patterns.flatMap((pattern) => pattern.sourceEvidenceRefs),
    patterns,
    confidence: 'high',
    reviewState: 'reviewed',
    rawDomainStored: false,
    rawUrlStored: false,
    rawPageTitleStored: false,
    rawPageBodyStored: false,
    runtimeDetectionClaimed: false,
    aiClassificationClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
  };
}

function runNegativeChecks(validPattern, validLibrary) {
  const invalidPatterns = [
    ['raw-domain', { rawDomainStored: true }],
    ['raw-url', { rawUrlStored: true }],
    ['raw-page-title', { rawPageTitleStored: true }],
    ['raw-page-body', { rawPageBodyStored: true }],
    ['runtime-detection', { runtimeDetectionClaimed: true }],
    ['ai-classification', { aiClassificationClaimed: true }],
    ['policy-decision', { policyDecisionClaimed: true }],
    ['enforcement', { enforcementClaimed: true }],
    ['cloud-gaming-ownership', { cloudGamingCandidate: true }],
    ['reviewed-unknown-family', { portalFamily: 'unknown' }],
    ['reviewed-unknown-route', { routeKinds: ['unknown-route'] }],
    ['reviewed-unknown-signal', { signalKinds: ['unknown-signal'] }],
  ];
  const invalidLibraries = [
    ['library-raw-url', { rawUrlStored: true }],
    ['library-runtime-detection', { runtimeDetectionClaimed: true }],
    ['library-policy-decision', { policyDecisionClaimed: true }],
    ['library-enforcement', { enforcementClaimed: true }],
    ['library-empty-patterns', { patterns: [] }],
  ];
  return [
    ...invalidPatterns.map(([name, invalid]) => negativePatternCheck(name, validPattern, invalid)),
    ...invalidLibraries.map(([name, invalid]) => negativeLibraryCheck(name, validLibrary, invalid)),
  ];
}

function negativePatternCheck(name, validPattern, invalid) {
  return {
    name,
    rejected: !BrowserGamePortalPatternEntrySchema.safeParse({ ...validPattern, ...invalid }).success,
  };
}

function negativeLibraryCheck(name, validLibrary, invalid) {
  return {
    name,
    rejected: !BrowserGamePortalPatternLibrarySchema.safeParse({ ...validLibrary, ...invalid }).success,
  };
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
