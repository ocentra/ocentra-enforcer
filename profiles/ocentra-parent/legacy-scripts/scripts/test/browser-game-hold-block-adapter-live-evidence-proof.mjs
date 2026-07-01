import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameHoldBlockAdapterPlanSchema,
  BrowserGameHoldBlockAdapterSnapshotSchema,
} from '@ocentra-parent/schema-domain/browser-game-hold-block-adapter';

const repoRoot = process.cwd();
const proofId = 'browser-game-hold-block-adapter-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-18-managed-browser-game-hold-block-adapter',
  '02-live-hold-block-adapter-shape-proof.json'
);

const targets = [
  {
    targetId: 'scratch-unknown-checking-hold',
    url: 'https://scratch.mit.edu/explore/projects/games/',
    targetKind: 'unknown-managed-game',
    requestedAction: 'hold-until-classified',
    fallbackAction: 'show-checking-page',
    adapterState: 'adapter-proof-present',
    deliveryMode: 'managed-intervention-proof-ref',
    reasonCodes: ['unknown-game-needs-classification', 'managed-intervention-proof-present'],
    policyCandidateRef: 'parent-evidence-game-policy-unknown-checking',
    childUxSurfaceRef: 'parent-evidence-game-child-ux-checking',
    managedInterventionAdapterProofRef: 'parent-evidence-managed-intervention-checking-proof',
    adapterAuditRef: 'parent-evidence-game-adapter-audit-checking',
  },
  {
    targetId: 'roblox-parent-approval-hold',
    url: 'https://www.roblox.com/discover',
    targetKind: 'managed-game-account-flow',
    requestedAction: 'hold-until-parent-approval',
    fallbackAction: 'show-approval-page',
    adapterState: 'adapter-proof-present',
    deliveryMode: 'managed-intervention-proof-ref',
    reasonCodes: ['policy-candidate-parent-review', 'managed-intervention-proof-present'],
    policyCandidateRef: 'parent-evidence-game-policy-parent-review',
    childUxSurfaceRef: 'parent-evidence-game-child-ux-approval',
    managedInterventionAdapterProofRef: 'parent-evidence-managed-intervention-approval-proof',
    adapterAuditRef: 'parent-evidence-game-adapter-audit-approval',
  },
  {
    targetId: 'hooda-unblocked-block-page',
    url: 'https://www.hoodamath.com/games/unblocked.html',
    targetKind: 'managed-browser-game-page',
    requestedAction: 'block-game-route',
    fallbackAction: 'show-block-page',
    adapterState: 'adapter-proof-present',
    deliveryMode: 'managed-intervention-proof-ref',
    reasonCodes: ['policy-candidate-block', 'managed-intervention-proof-present'],
    policyCandidateRef: 'parent-evidence-game-policy-block-unblocked',
    childUxSurfaceRef: 'parent-evidence-game-child-ux-block',
    managedInterventionAdapterProofRef: 'parent-evidence-managed-intervention-block-proof',
    adapterAuditRef: 'parent-evidence-game-adapter-audit-block',
  },
  {
    targetId: 'poki-warning-before-play',
    url: 'https://poki.com/en/g/subway-surfers',
    targetKind: 'managed-game-portal',
    requestedAction: 'warn-before-play',
    fallbackAction: 'show-warning-page',
    adapterState: 'adapter-proof-present',
    deliveryMode: 'managed-intervention-proof-ref',
    reasonCodes: ['policy-candidate-warn', 'managed-intervention-proof-present'],
    policyCandidateRef: 'parent-evidence-game-policy-warn-risk',
    childUxSurfaceRef: 'parent-evidence-game-child-ux-warning',
    managedInterventionAdapterProofRef: 'parent-evidence-managed-intervention-warning-proof',
    adapterAuditRef: 'parent-evidence-game-adapter-audit-warning',
  },
  {
    targetId: 'code-org-educational-allow',
    url: 'https://code.org/minecraft',
    targetKind: 'managed-game-portal',
    requestedAction: 'allow-educational-game',
    fallbackAction: 'continue-session',
    adapterState: 'candidate-only',
    deliveryMode: 'contract-only',
    reasonCodes: ['educational-allow-candidate'],
    policyCandidateRef: 'parent-evidence-game-policy-educational-allow',
    childUxSurfaceRef: 'parent-evidence-game-child-ux-educational-allow',
    managedInterventionAdapterProofRef: null,
    adapterAuditRef: null,
  },
  {
    targetId: 'coolmath-time-limit-candidate',
    url: 'https://www.coolmathgames.com/0-run',
    targetKind: 'managed-browser-game-page',
    requestedAction: 'time-limit-candidate',
    fallbackAction: 'manual-review',
    adapterState: 'candidate-only',
    deliveryMode: 'contract-only',
    reasonCodes: ['policy-candidate-time-limit'],
    policyCandidateRef: 'parent-evidence-game-policy-time-limit',
    childUxSurfaceRef: 'parent-evidence-game-child-ux-time-limit',
    managedInterventionAdapterProofRef: null,
    adapterAuditRef: null,
  },
  {
    targetId: 'xbox-cloud-manual-required',
    url: 'https://www.xbox.com/en-US/play',
    targetKind: 'manual-required',
    requestedAction: 'manual-required',
    fallbackAction: 'manual-review',
    adapterState: 'manual-required',
    deliveryMode: 'manual-required',
    reasonCodes: ['cloud-gaming-proof-manual-required'],
    policyCandidateRef: null,
    childUxSurfaceRef: null,
    managedInterventionAdapterProofRef: null,
    adapterAuditRef: null,
  },
  {
    targetId: 'native-game-control-unavailable',
    url: 'https://store.steampowered.com/',
    targetKind: 'manual-required',
    requestedAction: 'unavailable',
    fallbackAction: 'no-action',
    adapterState: 'unavailable',
    deliveryMode: 'unavailable',
    reasonCodes: ['native-game-control-unavailable'],
    policyCandidateRef: null,
    childUxSurfaceRef: null,
    managedInterventionAdapterProofRef: null,
    adapterAuditRef: null,
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const plans = captures.map(planFor);
const snapshot = snapshotFor(plans);
const negativeChecks = runNegativeChecks(plans[0], snapshot);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game hold/block adapter public captures to return HTTP 2xx/3xx responses');
}
if (!plans.every((plan) => BrowserGameHoldBlockAdapterPlanSchema.safeParse(plan).success)) {
  throw new Error('Expected every browser-game hold/block adapter plan to parse');
}
if (!BrowserGameHoldBlockAdapterSnapshotSchema.safeParse(snapshot).success) {
  throw new Error('Expected browser-game hold/block adapter snapshot to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  const failedChecks = negativeChecks.filter((check) => !check.rejected).map((check) => check.name);
  throw new Error(
    `Expected browser-game hold/block adapter negative checks to reject overclaims: ${failedChecks.join(', ')}`
  );
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-hold-block-adapter-shapes',
  targets: captures,
  plans,
  snapshot,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    planCount: plans.length,
    negativeChecks: negativeChecks.length,
    requestedActions: [...new Set(plans.map((plan) => plan.requestedAction))],
    deliveryModes: [...new Set(plans.map((plan) => plan.deliveryMode))],
    rawUrlPersisted: false,
    rawPageBodyIncluded: false,
    rawGamePayloadIncluded: false,
    childCookieSessionReused: false,
    unmanagedBrowserExactUrlClaimed: false,
    browserMutationExecutedClaimed: false,
    renderedChildPageClaimed: false,
    notificationDeliveredClaimed: false,
    finalPolicyDecisionClaimed: false,
    timeLimitAppliedClaimed: false,
    cloudFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-hold-block-adapter-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`targets=${captures.length} plans=${plans.length} negativeChecks=${negativeChecks.length}`);

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
    childCookieSessionReused: false,
  };
}

function planFor(capture) {
  const target = targetFor(capture.targetId);
  return {
    schemaVersion: 'browser-game-hold-block-adapter-contract',
    planId: `browser-game-hold-block-plan-${capture.targetId}`,
    familyId: 'family-browser-game-hold-block-live-proof',
    childProfileId: 'child-browser-game-hold-block-live-proof',
    deviceId: 'device-browser-game-hold-block-live-proof',
    createdAt: startedAt,
    targetKind: target.targetKind,
    requestedAction: target.requestedAction,
    adapterState: target.adapterState,
    deliveryMode: target.deliveryMode,
    fallbackAction: target.fallbackAction,
    sourceEvidenceRefs: [
      `parent-proof-${proofId}-${capture.targetId}-source`,
      `parent-proof-${proofId}-${capture.targetId}-response-hash`,
    ],
    policyCandidateRef: target.policyCandidateRef,
    childUxSurfaceRef: target.childUxSurfaceRef,
    managedInterventionAdapterProofRef: target.managedInterventionAdapterProofRef,
    adapterAuditRef: target.adapterAuditRef,
    reasonCodes: target.reasonCodes,
    rawUrlIncluded: false,
    rawPageBodyIncluded: false,
    rawGamePayloadIncluded: false,
    childCookieSessionReused: false,
    unmanagedBrowserExactUrlClaimed: false,
    browserMutationExecutedClaimed: false,
    renderedChildPageClaimed: false,
    notificationDeliveredClaimed: false,
    finalPolicyDecisionClaimed: false,
    timeLimitAppliedClaimed: false,
    cloudFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    enforcementClaimed: false,
  };
}

function snapshotFor(plans) {
  return {
    schemaVersion: 'browser-game-hold-block-adapter-contract',
    familyId: 'family-browser-game-hold-block-live-proof',
    childProfileId: 'child-browser-game-hold-block-live-proof',
    deviceId: 'device-browser-game-hold-block-live-proof',
    generatedAt: startedAt,
    plans,
    claimBoundaries: {
      rawUrlStorage: 'not-claimed',
      rawPageBodyStorage: 'not-claimed',
      rawGamePayloadStorage: 'not-claimed',
      childCookieSessionReuse: 'not-claimed',
      unmanagedExactUrl: 'not-claimed',
      browserMutationExecution: 'not-claimed',
      renderedChildPage: 'not-claimed',
      notificationDelivery: 'not-claimed',
      finalPolicyDecision: 'not-claimed',
      timeLimitApplication: 'not-claimed',
      cloudFrameAnalysis: 'not-claimed',
      nativeGameControl: 'not-claimed',
      enforcement: 'not-claimed',
    },
  };
}

function runNegativeChecks(validPlan, validSnapshot) {
  const invalidPlans = [
    ['rawUrlIncluded', { rawUrlIncluded: true }],
    ['rawPageBodyIncluded', { rawPageBodyIncluded: true }],
    ['rawGamePayloadIncluded', { rawGamePayloadIncluded: true }],
    ['childCookieSessionReused', { childCookieSessionReused: true }],
    ['unmanagedBrowserExactUrlClaimed', { unmanagedBrowserExactUrlClaimed: true }],
    ['browserMutationExecutedClaimed', { browserMutationExecutedClaimed: true }],
    ['renderedChildPageClaimed', { renderedChildPageClaimed: true }],
    ['notificationDeliveredClaimed', { notificationDeliveredClaimed: true }],
    ['finalPolicyDecisionClaimed', { finalPolicyDecisionClaimed: true }],
    ['timeLimitAppliedClaimed', { timeLimitAppliedClaimed: true }],
    ['cloudFrameAnalysisClaimed', { cloudFrameAnalysisClaimed: true }],
    ['nativeGameControlClaimed', { nativeGameControlClaimed: true }],
    ['enforcementClaimed', { enforcementClaimed: true }],
    ['managedPlanMissingPolicyCandidate', { policyCandidateRef: null }],
    ['managedPlanMissingChildUxSurface', { childUxSurfaceRef: null }],
    ['managedPlanMissingAdapterProof', { managedInterventionAdapterProofRef: null }],
    ['managedPlanMissingAudit', { adapterAuditRef: null }],
    ['managedPlanContractOnlyDelivery', { deliveryMode: 'contract-only' }],
    ['managedPlanWrongFallback', { fallbackAction: 'show-warning-page' }],
    ['managedPlanMissingSpecificReason', { reasonCodes: ['managed-intervention-proof-present'] }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameHoldBlockAdapterPlanSchema.safeParse({ ...validPlan, ...override }).success,
  }));

  const incompleteSnapshot = {
    ...validSnapshot,
    plans: validSnapshot.plans.filter((plan) => plan.requestedAction !== 'unavailable'),
  };

  return [
    ...invalidPlans,
    {
      name: 'snapshotMissingUnavailableFallback',
      rejected: !BrowserGameHoldBlockAdapterSnapshotSchema.safeParse(incompleteSnapshot).success,
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
