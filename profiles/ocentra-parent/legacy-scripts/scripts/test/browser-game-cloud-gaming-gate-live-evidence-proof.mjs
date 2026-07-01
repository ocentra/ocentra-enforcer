import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameCloudGateDecisionSchema,
  BrowserGameCloudGateRequestSchema,
} from '@ocentra-parent/schema-domain/browser-game-cloud-gaming-gate';

const repoRoot = process.cwd();
const proofId = 'browser-game-cloud-gaming-gate-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-14-cloud-gaming-gating',
  '02-live-cloud-gaming-gate-shape-proof.json'
);

const targets = [
  {
    targetId: 'xbox-cloud-gaming',
    url: 'https://www.xbox.com/en-US/play',
    platform: 'xbox-cloud-gaming',
    gateSubject: 'cloud-platform-session',
    gateState: 'candidate',
    actionCandidate: 'allow-window-candidate',
    decisionKind: 'allow-session-candidate',
    signalKinds: ['known-cloud-domain', 'streaming-session-route', 'gamepad-api'],
    reasonCodes: ['known-cloud-domain', 'streaming-route', 'title-metadata-present'],
    confidence: 'medium',
  },
  {
    targetId: 'geforce-now',
    url: 'https://www.nvidia.com/en-us/geforce-now/',
    platform: 'geforce-now',
    gateSubject: 'unknown-cloud-game',
    gateState: 'candidate',
    actionCandidate: 'parent-review-candidate',
    decisionKind: 'parent-review-candidate',
    signalKinds: ['known-cloud-domain', 'streaming-session-route', 'unknown-title-fallback'],
    reasonCodes: ['known-cloud-domain', 'streaming-route', 'unknown-cloud-title', 'parent-approval-required'],
    confidence: 'medium',
  },
  {
    targetId: 'amazon-luna',
    url: 'https://luna.amazon.com/',
    platform: 'amazon-luna',
    gateSubject: 'time-budget-cloud-gaming',
    gateState: 'candidate',
    actionCandidate: 'time-limit-candidate',
    decisionKind: 'time-limit-session-candidate',
    signalKinds: ['known-cloud-domain', 'streaming-session-route', 'low-latency-network'],
    reasonCodes: ['known-cloud-domain', 'streaming-route', 'time-budget-candidate'],
    confidence: 'medium',
  },
  {
    targetId: 'boosteroid',
    url: 'https://boosteroid.com/',
    platform: 'boosteroid',
    gateSubject: 'school-night-cloud-gaming',
    gateState: 'candidate',
    actionCandidate: 'block-candidate',
    decisionKind: 'deny-session-candidate',
    signalKinds: ['known-cloud-domain', 'streaming-session-route', 'high-bandwidth-stream'],
    reasonCodes: ['known-cloud-domain', 'streaming-route', 'schedule-blocked'],
    confidence: 'medium',
  },
  {
    targetId: 'playstation-cloud',
    url: 'https://www.playstation.com/en-us/ps-plus/',
    platform: 'playstation-cloud',
    gateSubject: 'mature-cloud-game',
    gateState: 'candidate',
    actionCandidate: 'block-candidate',
    decisionKind: 'deny-session-candidate',
    signalKinds: ['known-cloud-domain', 'streaming-session-route', 'platform-rating-metadata'],
    reasonCodes: ['known-cloud-domain', 'streaming-route', 'rating-metadata-present', 'mature-title-risk'],
    confidence: 'low',
  },
  {
    targetId: 'shadow-cloud-pc',
    url: 'https://shadow.tech/',
    platform: 'shadow-cloud-pc',
    gateSubject: 'unknown-cloud-game',
    gateState: 'manual-required',
    actionCandidate: 'manual-review-candidate',
    decisionKind: 'manual-required',
    signalKinds: ['known-cloud-domain', 'high-bandwidth-stream', 'unknown-title-fallback'],
    reasonCodes: ['manual-required', 'content-frame-unavailable', 'cloud-title-unavailable'],
    confidence: 'low',
  },
  {
    targetId: 'now-gg',
    url: 'https://now.gg/',
    platform: 'now-gg',
    gateSubject: 'unknown-cloud-game',
    gateState: 'unavailable',
    actionCandidate: 'unknown-fallback-candidate',
    decisionKind: 'manual-required',
    signalKinds: ['known-cloud-domain', 'unknown-title-fallback'],
    reasonCodes: ['missing-platform-proof', 'runtime-signal-unavailable', 'manual-required'],
    confidence: 'unknown',
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const requests = captures.map(requestFor);
const decisions = captures.map(decisionFor);
const negativeChecks = runNegativeChecks(requests[0], decisions[0]);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game cloud-gaming public captures to return HTTP 2xx/3xx responses');
}
if (!requests.every((request) => BrowserGameCloudGateRequestSchema.safeParse(request).success)) {
  throw new Error('Expected every browser-game cloud-gaming gate request to parse');
}
if (!decisions.every((decision) => BrowserGameCloudGateDecisionSchema.safeParse(decision).success)) {
  throw new Error('Expected every browser-game cloud-gaming gate decision to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  const failedChecks = negativeChecks.filter((check) => !check.rejected).map((check) => check.name);
  throw new Error(
    `Expected browser-game cloud-gaming negative checks to reject overclaims: ${failedChecks.join(', ')}`
  );
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-cloud-gaming-gate-shapes',
  targets: captures,
  requests,
  decisions,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    requestCount: requests.length,
    decisionCount: decisions.length,
    negativeChecks: negativeChecks.length,
    platforms: [...new Set(requests.map((request) => request.platform))],
    gateSubjects: [...new Set(requests.map((request) => request.gateSubject))],
    gateStates: [...new Set(requests.map((request) => request.gateState))],
    decisionKinds: [...new Set(decisions.map((decision) => decision.decisionKind))],
    rawUrlStored: false,
    rawCloudTitleStored: false,
    rawStreamFrameStored: false,
    cloudStreamFrameAnalysisClaimed: false,
    perGameCloudTitleClaimed: false,
    nativeGameControlClaimed: false,
    nativeLauncherControlClaimed: false,
    gameChatContentClaimed: false,
    accountOrPurchaseFlowClaimed: false,
    notificationDeliveredClaimed: false,
    uiRenderedClaimed: false,
    childNotifiedClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-cloud-gaming-gate-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(
  `targets=${captures.length} requests=${requests.length} decisions=${decisions.length} negativeChecks=${negativeChecks.length}`
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
    rawCloudTitlePersisted: false,
    rawStreamFramePersisted: false,
  };
}

function requestFor(capture) {
  const target = targetFor(capture.targetId);
  const candidate = target.gateState === 'candidate';
  const unavailable = target.gateState === 'unavailable';
  const allowWindow = target.actionCandidate === 'allow-window-candidate';
  const parentReview = target.actionCandidate === 'parent-review-candidate';
  const hasPolicyCandidate = ['allow-window-candidate', 'block-candidate', 'time-limit-candidate'].includes(
    target.actionCandidate
  );
  const timeLimit = target.actionCandidate === 'time-limit-candidate';
  return {
    schemaVersion: 'v0.6',
    gateRequestId: `cloud-gate-request-${capture.targetId}`,
    familyId: 'family-browser-game-cloud-gate-live-proof',
    childProfileId: 'child-browser-game-cloud-gate-live-proof',
    requestedByDeviceId: 'device-browser-game-cloud-gate-live-proof',
    requestedAt: startedAt,
    expiresAt: candidate ? expiresAt(20) : null,
    platform: target.platform,
    gateSubject: target.gateSubject,
    gateState: target.gateState,
    actionCandidate: target.actionCandidate,
    confidence: target.confidence,
    sourceEvidenceRefs: [
      `parent-proof-${proofId}-${capture.targetId}-source`,
      `parent-proof-${proofId}-${capture.targetId}-response-hash`,
    ],
    signalKinds: target.signalKinds,
    managedRouteEvidenceRef: unavailable ? null : `parent-proof-${proofId}-${capture.targetId}-managed-route-ref`,
    platformTitleEvidenceRef:
      allowWindow && target.reasonCodes.includes('title-metadata-present')
        ? `parent-proof-${proofId}-${capture.targetId}-title-ref`
        : null,
    platformRatingEvidenceRef: target.reasonCodes.includes('rating-metadata-present')
      ? `parent-proof-${proofId}-${capture.targetId}-rating-ref`
      : null,
    policyCandidateRef: hasPolicyCandidate ? `parent-proof-${proofId}-${capture.targetId}-policy-candidate-ref` : null,
    parentApprovalRequestRef: parentReview
      ? `parent-proof-${proofId}-${capture.targetId}-parent-approval-request-ref`
      : null,
    scheduleContextRef: timeLimit ? `parent-proof-${proofId}-${capture.targetId}-schedule-context-ref` : null,
    mobileCapabilityRef: candidate ? `parent-proof-${proofId}-${capture.targetId}-mobile-capability-ref` : null,
    reasonCodes: target.reasonCodes,
    deliveryState: 'contract-only',
    rawCloudTitleStored: false,
    rawStreamFrameStored: false,
    cloudStreamFrameAnalysisClaimed: false,
    perGameCloudTitleClaimed: false,
    nativeGameControlClaimed: false,
    nativeLauncherControlClaimed: false,
    gameChatContentClaimed: false,
    accountOrPurchaseFlowClaimed: false,
    notificationDeliveredClaimed: false,
    uiRenderedClaimed: false,
    childNotifiedClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    enforcementClaimed: false,
  };
}

function decisionFor(capture) {
  const target = targetFor(capture.targetId);
  const manual = target.decisionKind === 'manual-required';
  return {
    schemaVersion: 'v0.6',
    gateDecisionId: `cloud-gate-decision-${capture.targetId}`,
    gateRequestId: `cloud-gate-request-${capture.targetId}`,
    familyId: 'family-browser-game-cloud-gate-live-proof',
    childProfileId: 'child-browser-game-cloud-gate-live-proof',
    decidedAt: startedAt,
    decidedByActorId: manual ? null : 'parent-actor-browser-game-cloud-gate-live-proof',
    decisionKind: target.decisionKind,
    decisionState: manual ? 'manual-required' : 'recorded-contract-only',
    sourceEvidenceRefs: [
      `parent-proof-${proofId}-${capture.targetId}-source`,
      `parent-proof-${proofId}-${capture.targetId}-decision-ref`,
    ],
    policyVersionRef: manual ? null : 'policy-version-browser-game-cloud-gate-live-proof',
    actionCandidateRef: manual ? null : `action-candidate-browser-game-cloud-gate-${capture.targetId}`,
    reasonCodes: manual ? [...new Set([...target.reasonCodes, 'manual-required'])] : target.reasonCodes,
    deliveryState: 'contract-only',
    notificationDeliveredClaimed: false,
    uiRenderedClaimed: false,
    childNotifiedClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    cloudStreamFrameAnalysisClaimed: false,
    perGameCloudTitleClaimed: false,
    nativeGameControlClaimed: false,
    nativeLauncherControlClaimed: false,
    enforcementClaimed: false,
  };
}

function runNegativeChecks(validRequest, validDecision) {
  const invalidRequests = [
    ['rawCloudTitleStored', { rawCloudTitleStored: true }],
    ['rawStreamFrameStored', { rawStreamFrameStored: true }],
    ['cloudStreamFrameAnalysisClaimed', { cloudStreamFrameAnalysisClaimed: true }],
    ['perGameCloudTitleClaimed', { perGameCloudTitleClaimed: true }],
    ['nativeGameControlClaimed', { nativeGameControlClaimed: true }],
    ['nativeLauncherControlClaimed', { nativeLauncherControlClaimed: true }],
    ['gameChatContentClaimed', { gameChatContentClaimed: true }],
    ['accountOrPurchaseFlowClaimed', { accountOrPurchaseFlowClaimed: true }],
    ['notificationDeliveredClaimed', { notificationDeliveredClaimed: true }],
    ['uiRenderedClaimed', { uiRenderedClaimed: true }],
    ['childNotifiedClaimed', { childNotifiedClaimed: true }],
    ['finalPolicyDecisionClaimed', { finalPolicyDecisionClaimed: true }],
    ['runtimeGateExecutedClaimed', { runtimeGateExecutedClaimed: true }],
    ['enforcementClaimed', { enforcementClaimed: true }],
    ['candidateMissingManagedRouteEvidence', { managedRouteEvidenceRef: null }],
    ['candidateMissingExpiresAt', { expiresAt: null }],
    ['parentReviewMissingApprovalRef', { actionCandidate: 'parent-review-candidate', parentApprovalRequestRef: null }],
    ['parentReviewMissingReason', { actionCandidate: 'parent-review-candidate', reasonCodes: ['known-cloud-domain'] }],
    ['blockMissingPolicyRef', { actionCandidate: 'block-candidate', policyCandidateRef: null }],
    ['timeLimitMissingScheduleRef', { actionCandidate: 'time-limit-candidate', scheduleContextRef: null }],
    [
      'allowMissingKnownCloudSignal',
      { actionCandidate: 'allow-window-candidate', signalKinds: ['streaming-session-route'] },
    ],
    ['manualMissingContentFrameReason', { gateState: 'manual-required', reasonCodes: ['manual-required'] }],
    ['unavailableKeepsManagedRouteRef', { gateState: 'unavailable', managedRouteEvidenceRef: 'cloud-route-ref' }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameCloudGateRequestSchema.safeParse({ ...validRequest, ...override }).success,
  }));

  const invalidDecisions = [
    ['manualDecisionRecordedState', { decisionKind: 'manual-required', decisionState: 'recorded-contract-only' }],
    ['recordedDecisionMissingActor', { decidedByActorId: null }],
    ['recordedDecisionMissingPolicy', { policyVersionRef: null }],
    ['recordedDecisionMissingAction', { actionCandidateRef: null }],
    ['decisionNotificationClaim', { notificationDeliveredClaimed: true }],
    ['decisionUiClaim', { uiRenderedClaimed: true }],
    ['decisionChildNotificationClaim', { childNotifiedClaimed: true }],
    ['decisionFinalPolicyClaim', { finalPolicyDecisionClaimed: true }],
    ['decisionRuntimeGateClaim', { runtimeGateExecutedClaimed: true }],
    ['decisionCloudFrameClaim', { cloudStreamFrameAnalysisClaimed: true }],
    ['decisionTitleClaim', { perGameCloudTitleClaimed: true }],
    ['decisionNativeControlClaim', { nativeGameControlClaimed: true }],
    ['decisionLauncherControlClaim', { nativeLauncherControlClaimed: true }],
    ['decisionEnforcementClaim', { enforcementClaimed: true }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameCloudGateDecisionSchema.safeParse({ ...validDecision, ...override }).success,
  }));

  return [...invalidRequests, ...invalidDecisions];
}

function targetFor(targetId) {
  const target = targets.find((item) => item.targetId === targetId);
  if (!target) {
    throw new Error(`Unknown target: ${targetId}`);
  }
  return target;
}

function expiresAt(minutes) {
  return new Date(Date.parse(startedAt) + minutes * 60_000).toISOString();
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
