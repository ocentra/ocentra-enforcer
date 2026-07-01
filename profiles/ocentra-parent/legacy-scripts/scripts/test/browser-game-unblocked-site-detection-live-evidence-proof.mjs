import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameUnblockedSiteDetectionSchema,
  BrowserGameUnblockedSiteSignalSchema,
} from '@ocentra-parent/schema-domain/browser-game-unblocked-site-detection';

const repoRoot = process.cwd();
const proofId = 'browser-game-unblocked-site-detection-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-15-unblocked-game-site-detection',
  '02-live-unblocked-site-detection-shape-proof.json'
);

const targets = [
  {
    targetId: 'hooda-unblocked',
    url: 'https://www.hoodamath.com/games/unblocked.html',
    surfaceKind: 'managed-browser-route',
    classificationKind: 'unblocked-game-site',
    detectionState: 'candidate',
    actionCandidate: 'block-during-school-candidate',
    confidence: 'high',
    signalKinds: ['unblocked-domain-keyword', 'school-bypass-language'],
    reasonCodes: ['domain-keyword-match', 'portal-index-detected', 'school-bypass-portal'],
  },
  {
    targetId: 'unblocked-games-76',
    url: 'https://unblockedgames76.gitlab.io/',
    surfaceKind: 'portal-index',
    classificationKind: 'game-portal-bypass',
    detectionState: 'candidate',
    actionCandidate: 'parent-review-candidate',
    confidence: 'medium',
    signalKinds: ['unblocked-domain-keyword', 'game-portal-index'],
    reasonCodes: ['domain-keyword-match', 'portal-index-detected'],
  },
  {
    targetId: 'google-search-unblocked',
    url: 'https://www.google.com/search?q=unblocked+games',
    surfaceKind: 'search-intent',
    classificationKind: 'unknown-game-portal',
    detectionState: 'candidate',
    actionCandidate: 'parent-review-candidate',
    confidence: 'low',
    signalKinds: ['search-query-intent', 'unblocked-domain-keyword'],
    reasonCodes: ['search-intent-unblocked-games', 'domain-keyword-match'],
  },
  {
    targetId: 'poki-games',
    url: 'https://poki.com/',
    surfaceKind: 'portal-index',
    classificationKind: 'unknown-game-portal',
    detectionState: 'candidate',
    actionCandidate: 'allow-specific-game-candidate',
    confidence: 'medium',
    signalKinds: ['game-portal-index', 'managed-browser-game-proof'],
    reasonCodes: ['portal-index-detected'],
  },
  {
    targetId: 'coolmath-run',
    url: 'https://www.coolmathgames.com/0-run',
    surfaceKind: 'iframe-embed',
    classificationKind: 'hidden-origin-game-embed',
    detectionState: 'candidate',
    actionCandidate: 'block-unknown-iframe-candidate',
    confidence: 'medium',
    signalKinds: ['external-game-iframe', 'hidden-game-origin'],
    reasonCodes: ['external-game-iframe', 'hidden-game-origin'],
  },
  {
    targetId: 'archive-msdos-games',
    url: 'https://archive.org/details/softwarelibrary_msdos_games',
    surfaceKind: 'unmanaged-browser-bypass',
    classificationKind: 'unmanaged-browser-game-bypass',
    detectionState: 'candidate',
    actionCandidate: 'bypass-evidence-only-candidate',
    confidence: 'low',
    signalKinds: ['unmanaged-browser-process-only'],
    reasonCodes: ['unmanaged-browser-process-only'],
  },
  {
    targetId: 'math-playground-manual',
    url: 'https://www.mathplayground.com/',
    surfaceKind: 'manual-required',
    classificationKind: 'unknown',
    detectionState: 'manual-required',
    actionCandidate: 'manual-review-candidate',
    confidence: 'unknown',
    signalKinds: ['unknown-signal'],
    reasonCodes: ['manual-required', 'unavailable-proof'],
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const detections = captures.map(detectionFor);
const signals = detections.flatMap((detection) => detection.signalRows);
const negativeChecks = runNegativeChecks(detections[0], signals[0]);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game unblocked-site public captures to return HTTP 2xx/3xx responses');
}
if (!signals.every((signal) => BrowserGameUnblockedSiteSignalSchema.safeParse(signal).success)) {
  throw new Error('Expected every browser-game unblocked-site signal to parse');
}
if (!detections.every((detection) => BrowserGameUnblockedSiteDetectionSchema.safeParse(detection).success)) {
  throw new Error('Expected every browser-game unblocked-site detection to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  const failedChecks = negativeChecks.filter((check) => !check.rejected).map((check) => check.name);
  throw new Error(
    `Expected browser-game unblocked-site negative checks to reject overclaims: ${failedChecks.join(', ')}`
  );
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-unblocked-site-detection-shapes',
  targets: captures,
  detections,
  signals,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    detectionCount: detections.length,
    signalCount: signals.length,
    negativeChecks: negativeChecks.length,
    surfaceKinds: [...new Set(detections.map((detection) => detection.surfaceKind))],
    classificationKinds: [...new Set(detections.map((detection) => detection.classificationKind))],
    actionCandidates: [...new Set(detections.map((detection) => detection.actionCandidate))],
    rawUrlStored: false,
    rawPageBodyStored: false,
    rawSearchQueryStored: false,
    iframeContentCaptured: false,
    exactUnmanagedUrlClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    accountOrPurchaseFlowClaimed: false,
    uiRenderedClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-unblocked-site-detection-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(
  `targets=${captures.length} detections=${detections.length} signals=${signals.length} negativeChecks=${negativeChecks.length}`
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
    rawPageBodyPersisted: false,
    rawSearchQueryPersisted: false,
    iframeContentPersisted: false,
  };
}

function detectionFor(capture) {
  const target = targetFor(capture.targetId);
  const candidate = target.detectionState === 'candidate';
  const unmanaged = target.surfaceKind === 'unmanaged-browser-bypass';
  const iframe = target.actionCandidate === 'block-unknown-iframe-candidate';
  const portal = ['parent-review-candidate', 'allow-specific-game-candidate'].includes(target.actionCandidate);
  const searchIntent = target.surfaceKind === 'search-intent';
  const policyBacked = ['block-during-school-candidate', 'allow-specific-game-candidate'].includes(
    target.actionCandidate
  );
  return {
    schemaVersion: 'browser-game-unblocked-site-detection-contract',
    detectionId: `browser-game-unblocked-detection-${capture.targetId}`,
    familyId: 'family-browser-game-unblocked-live-proof',
    childProfileId: 'child-browser-game-unblocked-live-proof',
    deviceId: 'device-browser-game-unblocked-live-proof',
    detectedAt: startedAt,
    surfaceKind: target.surfaceKind,
    classificationKind: target.classificationKind,
    detectionState: target.detectionState,
    confidence: target.confidence,
    sourceEvidenceRefs: [
      `parent-proof-${proofId}-${capture.targetId}-source`,
      `parent-proof-${proofId}-${capture.targetId}-response-hash`,
    ],
    signalRows: target.signalKinds.map((signalKind, index) => signalFor(capture, target, signalKind, index)),
    actionCandidate: target.actionCandidate,
    managedRouteEvidenceRef:
      candidate && !unmanaged ? `parent-proof-${proofId}-${capture.targetId}-managed-route-ref` : null,
    portalIndexEvidenceRef: portal ? `parent-proof-${proofId}-${capture.targetId}-portal-index-ref` : null,
    iframeEvidenceRef: iframe ? `parent-proof-${proofId}-${capture.targetId}-iframe-ref` : null,
    searchIntentEvidenceRef: searchIntent ? `parent-proof-${proofId}-${capture.targetId}-search-intent-ref` : null,
    unmanagedProcessEvidenceRef: unmanaged ? `parent-proof-${proofId}-${capture.targetId}-process-only-ref` : null,
    parentPolicyRef: policyBacked ? 'policy-version-browser-game-unblocked-live-proof' : null,
    reasonCodes: target.reasonCodes,
    deliveryState: 'contract-only',
    rawUrlStored: false,
    rawPageBodyStored: false,
    rawSearchQueryStored: false,
    iframeContentCaptured: false,
    exactUnmanagedUrlClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    accountOrPurchaseFlowClaimed: false,
    uiRenderedClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    enforcementClaimed: false,
  };
}

function signalFor(capture, target, signalKind, index) {
  return {
    signalId: `browser-game-unblocked-signal-${capture.targetId}-${index}`,
    signalKind,
    surfaceKind: target.surfaceKind,
    detectionState: target.detectionState,
    confidence: target.confidence,
    evidenceRefs: [`parent-proof-${proofId}-${capture.targetId}-signal-${index}`],
    rawUrlStored: false,
    rawPageBodyStored: false,
    rawSearchQueryStored: false,
    iframeContentCaptured: false,
    exactUnmanagedUrlClaimed: false,
    policyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    enforcementClaimed: false,
  };
}

function runNegativeChecks(validDetection, validSignal) {
  const invalidDetections = [
    ['rawUrlStored', { rawUrlStored: true }],
    ['rawPageBodyStored', { rawPageBodyStored: true }],
    ['rawSearchQueryStored', { rawSearchQueryStored: true }],
    ['iframeContentCaptured', { iframeContentCaptured: true }],
    ['exactUnmanagedUrlClaimed', { exactUnmanagedUrlClaimed: true }],
    ['nativeGameControlClaimed', { nativeGameControlClaimed: true }],
    ['cloudFrameAnalysisClaimed', { cloudFrameAnalysisClaimed: true }],
    ['accountOrPurchaseFlowClaimed', { accountOrPurchaseFlowClaimed: true }],
    ['uiRenderedClaimed', { uiRenderedClaimed: true }],
    ['finalPolicyDecisionClaimed', { finalPolicyDecisionClaimed: true }],
    ['runtimeGateExecutedClaimed', { runtimeGateExecutedClaimed: true }],
    ['enforcementClaimed', { enforcementClaimed: true }],
    ['emptySignalRows', { signalRows: [] }],
    ['candidateUnknownConfidence', { confidence: 'unknown' }],
    ['candidateUnknownClassification', { classificationKind: 'unknown' }],
    ['candidateMissingManagedRoute', { managedRouteEvidenceRef: null }],
    ['blockSchoolMissingPolicy', { actionCandidate: 'block-during-school-candidate', parentPolicyRef: null }],
    ['parentReviewMissingPortal', { actionCandidate: 'parent-review-candidate', portalIndexEvidenceRef: null }],
    ['iframeMissingIframeRef', { actionCandidate: 'block-unknown-iframe-candidate', iframeEvidenceRef: null }],
    [
      'bypassMissingProcessRef',
      { actionCandidate: 'bypass-evidence-only-candidate', unmanagedProcessEvidenceRef: null },
    ],
    ['unknownCandidateSignal', { signalRows: [{ ...validSignal, signalKind: 'unknown-signal' }] }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameUnblockedSiteDetectionSchema.safeParse({ ...validDetection, ...override }).success,
  }));

  const invalidSignals = [
    ['signalRawUrlStored', { rawUrlStored: true }],
    ['signalRawPageBodyStored', { rawPageBodyStored: true }],
    ['signalRawSearchQueryStored', { rawSearchQueryStored: true }],
    ['signalIframeContentCaptured', { iframeContentCaptured: true }],
    ['signalExactUnmanagedUrlClaimed', { exactUnmanagedUrlClaimed: true }],
    ['signalPolicyDecisionClaimed', { policyDecisionClaimed: true }],
    ['signalRuntimeGateExecutedClaimed', { runtimeGateExecutedClaimed: true }],
    ['signalEnforcementClaimed', { enforcementClaimed: true }],
    ['signalCandidateUnknownKind', { signalKind: 'unknown-signal' }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameUnblockedSiteSignalSchema.safeParse({ ...validSignal, ...override }).success,
  }));

  return [...invalidDetections, ...invalidSignals];
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
