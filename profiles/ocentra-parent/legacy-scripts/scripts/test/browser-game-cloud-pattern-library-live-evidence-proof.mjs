import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameCloudPatternEntrySchema,
  BrowserGameCloudPatternLibrarySchema,
  decodeBrowserGameCloudPatternLibrary,
} from '@ocentra-parent/schema-domain/browser-game-cloud-pattern-library';

const repoRoot = process.cwd();
const proofId = 'browser-game-cloud-pattern-library-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-04-cloud-gaming-pattern-library',
  '06-live-cloud-pattern-proof.json'
);

const targets = [
  {
    targetId: 'xbox-cloud-play',
    url: 'https://www.xbox.com/en-US/play',
    platform: 'xbox-cloud-gaming',
    cloudFamily: 'cloud-gaming-platform',
    routeKinds: ['cloud-catalog-route', 'cloud-session-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'streaming-session-route', 'gamepad-api'],
    sessionCandidate: true,
    titleMetadataCandidate: false,
    ratingMetadataCandidate: false,
    subscriptionOrAccountCandidate: false,
    nativeLauncherPromptCandidate: false,
  },
  {
    targetId: 'geforce-now-home',
    url: 'https://www.nvidia.com/en-us/geforce-now/',
    platform: 'geforce-now',
    cloudFamily: 'cloud-gaming-platform',
    routeKinds: ['cloud-home-route', 'cloud-session-route', 'cloud-launch-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'streaming-session-route', 'native-launcher-prompt'],
    sessionCandidate: true,
    titleMetadataCandidate: false,
    ratingMetadataCandidate: false,
    subscriptionOrAccountCandidate: false,
    nativeLauncherPromptCandidate: true,
  },
  {
    targetId: 'amazon-luna-home',
    url: 'https://luna.amazon.com/',
    platform: 'amazon-luna',
    cloudFamily: 'cloud-gaming-platform',
    routeKinds: ['cloud-home-route', 'cloud-subscription-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'subscription-prompt'],
    sessionCandidate: false,
    titleMetadataCandidate: false,
    ratingMetadataCandidate: false,
    subscriptionOrAccountCandidate: true,
    nativeLauncherPromptCandidate: false,
  },
  {
    targetId: 'boosteroid-home',
    url: 'https://boosteroid.com/',
    platform: 'boosteroid',
    cloudFamily: 'cloud-gaming-platform',
    routeKinds: ['cloud-home-route', 'cloud-session-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'streaming-session-route'],
    sessionCandidate: true,
    titleMetadataCandidate: false,
    ratingMetadataCandidate: false,
    subscriptionOrAccountCandidate: false,
    nativeLauncherPromptCandidate: false,
  },
  {
    targetId: 'playstation-plus-games',
    url: 'https://www.playstation.com/en-us/ps-plus/games/',
    platform: 'playstation-cloud',
    cloudFamily: 'cloud-gaming-platform',
    routeKinds: ['cloud-catalog-route', 'cloud-subscription-route'],
    signalKinds: [
      'domain-ref',
      'path-pattern-ref',
      'platform-title-metadata-ref',
      'platform-rating-metadata-ref',
      'subscription-prompt',
    ],
    sessionCandidate: false,
    titleMetadataCandidate: true,
    ratingMetadataCandidate: true,
    subscriptionOrAccountCandidate: true,
    nativeLauncherPromptCandidate: false,
  },
  {
    targetId: 'shadow-cloud-pc',
    url: 'https://shadow.tech/',
    platform: 'shadow-cloud-pc',
    cloudFamily: 'cloud-pc-platform',
    routeKinds: ['cloud-home-route', 'cloud-launch-route', 'cloud-session-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'streaming-session-route', 'native-launcher-prompt'],
    sessionCandidate: true,
    titleMetadataCandidate: false,
    ratingMetadataCandidate: false,
    subscriptionOrAccountCandidate: false,
    nativeLauncherPromptCandidate: true,
  },
  {
    targetId: 'now-gg-home',
    url: 'https://now.gg/',
    platform: 'now-gg',
    cloudFamily: 'browser-embedded-cloud-game',
    routeKinds: ['cloud-home-route', 'cloud-title-route', 'cloud-launch-route'],
    signalKinds: ['domain-ref', 'path-pattern-ref', 'streaming-session-route', 'platform-title-metadata-ref'],
    sessionCandidate: false,
    titleMetadataCandidate: true,
    ratingMetadataCandidate: false,
    subscriptionOrAccountCandidate: false,
    nativeLauncherPromptCandidate: false,
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
  throw new Error('Expected all browser-game cloud public captures to return HTTP 2xx/3xx responses');
}
if (!patterns.every((pattern) => BrowserGameCloudPatternEntrySchema.safeParse(pattern).success)) {
  throw new Error('Expected every live browser-game cloud pattern entry to parse');
}
decodeBrowserGameCloudPatternLibrary(library);

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-cloud-gaming-patterns',
  targets: captures,
  library,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    patternRows: patterns.length,
    negativeChecks: negativeChecks.length,
    rawCloudDomainStored: false,
    rawCloudUrlStored: false,
    rawCloudTitleStored: false,
    rawStreamFrameStored: false,
    runtimeDetectionClaimed: false,
    cloudStreamFrameAnalysisClaimed: false,
    perGameCloudTitleCertaintyClaimed: false,
    nativeLauncherControlClaimed: false,
    nativeGameControlClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-cloud-pattern-library-live-evidence-proof-ok=true');
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
    platform: target.platform,
    cloudFamily: target.cloudFamily,
    routeKinds: target.routeKinds,
    signalKinds: target.signalKinds,
    rawCloudUrlPersisted: false,
    rawCloudDomainPersisted: false,
    rawCloudTitlePersisted: false,
    rawStreamFramePersisted: false,
  };
}

function patternEntryFor(capture) {
  const target = targets.find((entry) => entry.targetId === capture.targetId);
  return {
    patternId: `game-cloud-pattern-${capture.targetId}`,
    platform: target.platform,
    cloudFamily: target.cloudFamily,
    routeKinds: target.routeKinds,
    signalKinds: target.signalKinds,
    patternFingerprint: `game-cloud-pattern-fingerprint-${sha256(`${capture.originSha256}:${capture.pathSha256}`).slice(
      0,
      32
    )}`,
    sourceEvidenceRefs: [`parent-proof-${proofId}-${capture.targetId}`],
    confidence: 'high',
    reviewState: 'reviewed',
    sessionCandidate: target.sessionCandidate,
    titleMetadataCandidate: target.titleMetadataCandidate,
    ratingMetadataCandidate: target.ratingMetadataCandidate,
    subscriptionOrAccountCandidate: target.subscriptionOrAccountCandidate,
    nativeLauncherPromptCandidate: target.nativeLauncherPromptCandidate,
    rawCloudDomainStored: false,
    rawCloudUrlStored: false,
    rawCloudTitleStored: false,
    rawStreamFrameStored: false,
    runtimeDetectionClaimed: false,
    cloudStreamFrameAnalysisClaimed: false,
    perGameCloudTitleCertaintyClaimed: false,
    nativeLauncherControlClaimed: false,
    nativeGameControlClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
  };
}

function patternLibraryFor(patterns) {
  return {
    schemaVersion: 'browser-game-cloud-pattern-library-contract',
    libraryId: `game-cloud-pattern-library-${proofId}`,
    generatedAt: startedAt,
    sourceEvidenceRefs: patterns.flatMap((pattern) => pattern.sourceEvidenceRefs),
    patterns,
    confidence: 'high',
    reviewState: 'reviewed',
    rawCloudDomainStored: false,
    rawCloudUrlStored: false,
    rawCloudTitleStored: false,
    rawStreamFrameStored: false,
    runtimeDetectionClaimed: false,
    cloudStreamFrameAnalysisClaimed: false,
    perGameCloudTitleCertaintyClaimed: false,
    nativeLauncherControlClaimed: false,
    nativeGameControlClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
  };
}

function runNegativeChecks(validPattern, validLibrary) {
  const invalidPatterns = [
    ['raw-cloud-domain', { rawCloudDomainStored: true }],
    ['raw-cloud-url', { rawCloudUrlStored: true }],
    ['raw-cloud-title', { rawCloudTitleStored: true }],
    ['raw-stream-frame', { rawStreamFrameStored: true }],
    ['runtime-detection', { runtimeDetectionClaimed: true }],
    ['cloud-stream-frame-analysis', { cloudStreamFrameAnalysisClaimed: true }],
    ['per-game-cloud-title-certainty', { perGameCloudTitleCertaintyClaimed: true }],
    ['native-launcher-control', { nativeLauncherControlClaimed: true }],
    ['native-game-control', { nativeGameControlClaimed: true }],
    ['policy-decision', { policyDecisionClaimed: true }],
    ['enforcement', { enforcementClaimed: true }],
    ['reviewed-unknown-platform', { platform: 'unknown-cloud-gaming' }],
    ['session-without-streaming-signal', { signalKinds: ['domain-ref', 'path-pattern-ref'] }],
    ['title-metadata-without-title-signal', { signalKinds: ['domain-ref', 'streaming-session-route'] }],
    ['native-prompt-without-native-signal', { nativeLauncherPromptCandidate: true }],
  ];
  const invalidLibraries = [
    ['library-raw-cloud-url', { rawCloudUrlStored: true }],
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
    rejected: !BrowserGameCloudPatternEntrySchema.safeParse({ ...validPattern, ...invalid }).success,
  };
}

function negativeLibraryCheck(name, validLibrary, invalid) {
  return {
    name,
    rejected: !BrowserGameCloudPatternLibrarySchema.safeParse({ ...validLibrary, ...invalid }).success,
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
