import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameApprovalDecisionSchema,
  BrowserGameApprovalRequestSchema,
} from '@ocentra-parent/schema-domain/browser-game-account-purchase-gate';

const repoRoot = process.cwd();
const proofId = 'browser-game-account-purchase-gate-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-13-game-account-signup-purchase-gating',
  '02-live-account-purchase-gate-shape-proof.json'
);

const targets = [
  {
    targetId: 'roblox-login',
    url: 'https://www.roblox.com/login',
    requestKind: 'game-login',
    requestState: 'pending-contract-only',
    decisionKind: 'approve-account-candidate',
    reasonCodes: ['login-route', 'parent-rule-requires-approval'],
    confidence: 'medium',
  },
  {
    targetId: 'roblox-subscription',
    url: 'https://www.roblox.com/premium/membership',
    requestKind: 'subscription-purchase',
    requestState: 'pending-contract-only',
    decisionKind: 'approve-purchase-candidate',
    reasonCodes: ['subscription-route', 'parent-rule-requires-approval'],
    confidence: 'medium',
  },
  {
    targetId: 'steam-app-purchase',
    url: 'https://store.steampowered.com/app/730/CounterStrike_2/',
    requestKind: 'game-purchase',
    requestState: 'blocked-candidate',
    decisionKind: 'block-candidate',
    reasonCodes: ['purchase-route', 'parent-rule-blocks-flow'],
    confidence: 'medium',
  },
  {
    targetId: 'xbox-cloud-start',
    url: 'https://www.xbox.com/en-US/play',
    requestKind: 'cloud-gaming-start',
    requestState: 'manual-required',
    decisionKind: 'manual-required',
    reasonCodes: ['cloud-gaming-route', 'manual-required'],
    confidence: 'low',
  },
  {
    targetId: 'code-org-sign-in',
    url: 'https://studio.code.org/users/sign_in',
    requestKind: 'game-account-creation',
    requestState: 'pending-contract-only',
    decisionKind: 'approve-account-candidate',
    reasonCodes: ['account-creation-route', 'educational-account-requires-approval', 'parent-rule-requires-approval'],
    confidence: 'medium',
  },
  {
    targetId: 'playstation-store-unknown-start',
    url: 'https://store.playstation.com/en-us/pages/latest',
    requestKind: 'unknown-game-start',
    requestState: 'unavailable',
    decisionKind: 'manual-required',
    reasonCodes: ['unknown-game-route', 'missing-route-proof'],
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
  throw new Error('Expected all browser-game approval public captures to return HTTP 2xx/3xx responses');
}
if (!requests.every((request) => BrowserGameApprovalRequestSchema.safeParse(request).success)) {
  throw new Error('Expected every browser-game approval request to parse');
}
if (!decisions.every((decision) => BrowserGameApprovalDecisionSchema.safeParse(decision).success)) {
  throw new Error('Expected every browser-game approval decision to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  const failedChecks = negativeChecks.filter((check) => !check.rejected).map((check) => check.name);
  throw new Error(`Expected browser-game approval negative checks to reject overclaims: ${failedChecks.join(', ')}`);
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-account-purchase-gate-shapes',
  targets: captures,
  requests,
  decisions,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    requestCount: requests.length,
    decisionCount: decisions.length,
    negativeChecks: negativeChecks.length,
    requestKinds: [...new Set(requests.map((request) => request.requestKind))],
    requestStates: [...new Set(requests.map((request) => request.requestState))],
    decisionKinds: [...new Set(decisions.map((decision) => decision.decisionKind))],
    rawUrlStored: false,
    rawGameTitleStored: false,
    rawAccountIdentifierCaptured: false,
    credentialCaptured: false,
    formSubmittedClaimed: false,
    accountCreatedClaimed: false,
    purchaseExecutedClaimed: false,
    paymentInfoCaptured: false,
    launcherDownloadClaimed: false,
    notificationDeliveredClaimed: false,
    uiRenderedClaimed: false,
    childNotifiedClaimed: false,
    policyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-account-purchase-gate-live-evidence-proof-ok=true');
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
    rawGameTitlePersisted: false,
    rawAccountIdentifierPersisted: false,
    credentialPersisted: false,
    paymentInfoPersisted: false,
  };
}

function requestFor(capture) {
  const target = targetFor(capture.targetId);
  const unavailable = target.requestState === 'unavailable';
  return {
    schemaVersion: 'v0.6',
    approvalRequestId: `browser-game-approval-request-${capture.targetId}`,
    familyId: 'family-browser-game-approval-live-proof',
    childProfileId: 'child-browser-game-approval-live-proof',
    requestedByDeviceId: 'device-browser-game-approval-live-proof',
    requestedAt: startedAt,
    expiresAt: target.requestState === 'pending-contract-only' ? expiresAt(15) : null,
    requestKind: target.requestKind,
    requestState: target.requestState,
    confidence: target.confidence,
    sourceEvidenceRefs: [
      `parent-proof-${proofId}-${capture.targetId}-source`,
      `parent-proof-${proofId}-${capture.targetId}-response-hash`,
    ],
    managedRouteEvidenceRef: unavailable ? null : `parent-proof-${proofId}-${capture.targetId}-managed-route-ref`,
    gameTitleEvidenceRef: unavailable ? null : `parent-proof-${proofId}-${capture.targetId}-title-ref`,
    aiAnalysisRef:
      target.requestState === 'manual-required' ? null : `parent-proof-${proofId}-${capture.targetId}-ai-ref`,
    parentRuleRef: unavailable ? null : `parent-proof-${proofId}-${capture.targetId}-parent-rule-ref`,
    reasonCodes: target.reasonCodes,
    deliveryState: 'contract-only',
    rawUrlStored: false,
    rawGameTitleStored: false,
    rawAccountIdentifierCaptured: false,
    credentialCaptured: false,
    formSubmittedClaimed: false,
    accountCreatedClaimed: false,
    purchaseExecutedClaimed: false,
    paymentInfoCaptured: false,
    launcherDownloadClaimed: false,
    notificationDeliveredClaimed: false,
    uiRenderedClaimed: false,
    childNotifiedClaimed: false,
    policyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    enforcementClaimed: false,
  };
}

function decisionFor(capture) {
  const target = targetFor(capture.targetId);
  const manual = target.decisionKind === 'manual-required';
  return {
    schemaVersion: 'v0.6',
    approvalDecisionId: `browser-game-approval-decision-${capture.targetId}`,
    approvalRequestId: `browser-game-approval-request-${capture.targetId}`,
    familyId: 'family-browser-game-approval-live-proof',
    childProfileId: 'child-browser-game-approval-live-proof',
    decidedAt: startedAt,
    decidedByActorId: manual ? null : 'parent-actor-browser-game-approval-live-proof',
    decisionKind: target.decisionKind,
    decisionState: manual ? 'manual-required' : 'recorded-contract-only',
    sourceEvidenceRefs: [
      `parent-proof-${proofId}-${capture.targetId}-source`,
      `parent-proof-${proofId}-${capture.targetId}-decision-ref`,
    ],
    policyVersionRef: manual ? null : 'policy-version-browser-game-approval-live-proof',
    actionCandidateRef: manual ? null : `action-candidate-browser-game-approval-${capture.targetId}`,
    reasonCodes: manual ? [...new Set([...target.reasonCodes, 'manual-required'])] : target.reasonCodes,
    deliveryState: 'contract-only',
    notificationDeliveredClaimed: false,
    uiRenderedClaimed: false,
    childNotifiedClaimed: false,
    policyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    accountCreatedClaimed: false,
    purchaseExecutedClaimed: false,
    launcherDownloadClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    enforcementClaimed: false,
  };
}

function runNegativeChecks(validRequest, validDecision) {
  const invalidRequests = [
    ['request-raw-url', { rawUrlStored: true }],
    ['request-raw-game-title', { rawGameTitleStored: true }],
    ['request-account-id', { rawAccountIdentifierCaptured: true }],
    ['request-credential', { credentialCaptured: true }],
    ['request-form-submit', { formSubmittedClaimed: true }],
    ['request-account-created', { accountCreatedClaimed: true }],
    ['request-purchase-executed', { purchaseExecutedClaimed: true }],
    ['request-payment-info', { paymentInfoCaptured: true }],
    ['request-launcher-download', { launcherDownloadClaimed: true }],
    ['request-notification-delivered', { notificationDeliveredClaimed: true }],
    ['request-ui-rendered', { uiRenderedClaimed: true }],
    ['request-child-notified', { childNotifiedClaimed: true }],
    ['request-policy-decision', { policyDecisionClaimed: true }],
    ['request-runtime-gate', { runtimeGateExecutedClaimed: true }],
    ['request-native-game-control', { nativeGameControlClaimed: true }],
    ['request-cloud-frame-analysis', { cloudFrameAnalysisClaimed: true }],
    ['request-enforcement', { enforcementClaimed: true }],
    ['request-pending-missing-route-ref', { managedRouteEvidenceRef: null }],
    ['request-pending-missing-rule-ref', { parentRuleRef: null }],
    ['request-pending-no-expiry', { expiresAt: null }],
    [
      'request-login-wrong-reason',
      { requestKind: 'game-login', reasonCodes: ['account-creation-route', 'parent-rule-requires-approval'] },
    ],
    [
      'request-blocked-wrong-rule',
      { requestState: 'blocked-candidate', reasonCodes: ['purchase-route', 'parent-rule-requires-approval'] },
    ],
    ['request-unavailable-with-route-ref', { requestState: 'unavailable', managedRouteEvidenceRef: 'bad-route-ref' }],
  ];
  const invalidDecisions = [
    ['decision-notification-delivered', { notificationDeliveredClaimed: true }],
    ['decision-ui-rendered', { uiRenderedClaimed: true }],
    ['decision-child-notified', { childNotifiedClaimed: true }],
    ['decision-policy-decision', { policyDecisionClaimed: true }],
    ['decision-runtime-gate', { runtimeGateExecutedClaimed: true }],
    ['decision-account-created', { accountCreatedClaimed: true }],
    ['decision-purchase-executed', { purchaseExecutedClaimed: true }],
    ['decision-launcher-download', { launcherDownloadClaimed: true }],
    ['decision-native-game-control', { nativeGameControlClaimed: true }],
    ['decision-cloud-frame-analysis', { cloudFrameAnalysisClaimed: true }],
    ['decision-enforcement', { enforcementClaimed: true }],
    ['decision-recorded-no-actor', { decidedByActorId: null }],
    ['decision-recorded-no-policy', { policyVersionRef: null }],
    ['decision-recorded-no-action', { actionCandidateRef: null }],
    ['decision-manual-recorded-state', { decisionKind: 'manual-required', decisionState: 'recorded-contract-only' }],
  ];
  return [
    ...invalidRequests.map(([name, invalid]) => negativeRequestCheck(name, validRequest, invalid)),
    ...invalidDecisions.map(([name, invalid]) => negativeDecisionCheck(name, validDecision, invalid)),
  ];
}

function negativeRequestCheck(name, validRequest, invalid) {
  return {
    name,
    rejected: !BrowserGameApprovalRequestSchema.safeParse({ ...validRequest, ...invalid }).success,
  };
}

function negativeDecisionCheck(name, validDecision, invalid) {
  return {
    name,
    rejected: !BrowserGameApprovalDecisionSchema.safeParse({ ...validDecision, ...invalid }).success,
  };
}

function targetFor(targetId) {
  const target = targets.find((entry) => entry.targetId === targetId);
  if (!target) {
    throw new Error(`Unknown target ${targetId}`);
  }
  return target;
}

function expiresAt(minutes) {
  return new Date(Date.parse(startedAt) + minutes * 60 * 1000).toISOString();
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
