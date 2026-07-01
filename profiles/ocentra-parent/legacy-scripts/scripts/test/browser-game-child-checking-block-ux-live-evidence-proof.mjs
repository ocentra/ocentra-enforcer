import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameChildCheckingBlockSurfaceSchema,
  BrowserGameChildCheckingBlockUxSnapshotSchema,
} from '@ocentra-parent/schema-domain/browser-game-child-checking-block-ux';

const repoRoot = process.cwd();
const proofId = 'browser-game-child-checking-block-ux-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-19-child-game-checking-block-ux',
  '02-live-child-checking-block-ux-shape-proof.json'
);

const targets = [
  {
    targetId: 'scratch-unknown-checking',
    url: 'https://scratch.mit.edu/explore/projects/games/',
    surfaceKind: 'checking-unknown-game',
    state: 'checking-contract-only',
    primaryAction: 'wait-for-classification',
    primaryTextToken: 'browser-game.child.checking.title',
    reasons: ['unknown-game-needs-classification'],
    analysisRef: 'parent-evidence-game-analysis-unknown-checking',
    policyCandidateRef: null,
    parentApprovalRequestRef: null,
    adapterProofRef: 'parent-evidence-child-ux-checking-adapter-proof',
  },
  {
    targetId: 'roblox-approval-needed',
    url: 'https://www.roblox.com/discover',
    surfaceKind: 'approval-required-game',
    state: 'waiting-parent',
    primaryAction: 'wait-for-parent',
    primaryTextToken: 'browser-game.child.approval.title',
    reasons: ['parent-approval-needed'],
    analysisRef: 'parent-evidence-game-analysis-ugc-risk',
    policyCandidateRef: 'parent-evidence-game-policy-parent-review',
    parentApprovalRequestRef: 'parent-evidence-game-parent-approval-request',
    adapterProofRef: 'parent-evidence-child-ux-approval-adapter-proof',
  },
  {
    targetId: 'hooda-blocked-candidate',
    url: 'https://www.hoodamath.com/games/unblocked.html',
    surfaceKind: 'blocked-game-candidate',
    state: 'blocked-contract-only',
    primaryAction: 'open-safe-back',
    primaryTextToken: 'browser-game.child.blocked.title',
    reasons: ['game-block-candidate'],
    analysisRef: 'parent-evidence-game-analysis-unblocked-risk',
    policyCandidateRef: 'parent-evidence-game-policy-block-unblocked',
    parentApprovalRequestRef: null,
    adapterProofRef: 'parent-evidence-child-ux-block-adapter-proof',
  },
  {
    targetId: 'code-org-educational-readable',
    url: 'https://code.org/minecraft',
    surfaceKind: 'educational-game-allowed',
    state: 'child-readable',
    primaryAction: 'acknowledge',
    primaryTextToken: 'browser-game.child.educational-allowed.title',
    reasons: ['educational-game-allowed-contract'],
    analysisRef: 'parent-evidence-game-analysis-educational',
    policyCandidateRef: 'parent-evidence-game-policy-educational-allow',
    parentApprovalRequestRef: null,
    adapterProofRef: null,
  },
  {
    targetId: 'coolmath-time-limit-readable',
    url: 'https://www.coolmathgames.com/0-run',
    surfaceKind: 'game-time-limit-candidate',
    state: 'child-readable',
    primaryAction: 'acknowledge',
    primaryTextToken: 'browser-game.child.time-limited.title',
    reasons: ['time-limit-not-applied'],
    analysisRef: 'parent-evidence-game-analysis-time-budget',
    policyCandidateRef: 'parent-evidence-game-policy-time-limit',
    parentApprovalRequestRef: null,
    adapterProofRef: null,
  },
  {
    targetId: 'xbox-cloud-manual-required',
    url: 'https://www.xbox.com/en-US/play',
    surfaceKind: 'cloud-gaming-manual-required',
    state: 'manual-required',
    primaryAction: 'manual-review',
    primaryTextToken: 'browser-game.child.manual.title',
    reasons: ['cloud-gaming-proof-manual-required'],
    analysisRef: null,
    policyCandidateRef: null,
    parentApprovalRequestRef: null,
    adapterProofRef: null,
  },
  {
    targetId: 'native-game-unavailable',
    url: 'https://store.steampowered.com/',
    surfaceKind: 'native-game-control-unavailable',
    state: 'unavailable',
    primaryAction: 'no-action',
    primaryTextToken: 'browser-game.child.unavailable.title',
    reasons: ['native-game-proof-unavailable'],
    analysisRef: null,
    policyCandidateRef: null,
    parentApprovalRequestRef: null,
    adapterProofRef: null,
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const surfaces = captures.map(surfaceFor);
const snapshot = snapshotFor(surfaces);
const negativeChecks = runNegativeChecks(surfaces[0], snapshot);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game child UX public captures to return HTTP 2xx/3xx responses');
}
if (!surfaces.every((surface) => BrowserGameChildCheckingBlockSurfaceSchema.safeParse(surface).success)) {
  throw new Error('Expected every browser-game child checking/block UX surface to parse');
}
if (!BrowserGameChildCheckingBlockUxSnapshotSchema.safeParse(snapshot).success) {
  throw new Error('Expected browser-game child checking/block UX snapshot to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  const failedChecks = negativeChecks.filter((check) => !check.rejected).map((check) => check.name);
  throw new Error(
    `Expected browser-game child checking/block UX negative checks to reject overclaims: ${failedChecks.join(', ')}`
  );
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-child-checking-block-ux-shapes',
  targets: captures,
  surfaces,
  snapshot,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    surfaceCount: surfaces.length,
    negativeChecks: negativeChecks.length,
    surfaceKinds: [...new Set(surfaces.map((surface) => surface.surfaceKind))],
    states: [...new Set(surfaces.map((surface) => surface.state))],
    rawUrlPersisted: false,
    rawPageBodyPersisted: false,
    rawGamePayloadPersisted: false,
    rawChildCopyClaimed: false,
    renderedChildUiClaimed: false,
    notificationDeliveredClaimed: false,
    browserNavigationBlockedClaimed: false,
    blockPageRenderedClaimed: false,
    timeLimitAppliedClaimed: false,
    finalPolicyDecisionClaimed: false,
    cloudFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-child-checking-block-ux-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`targets=${captures.length} surfaces=${surfaces.length} negativeChecks=${negativeChecks.length}`);

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
    rawChildCopyPersisted: false,
  };
}

function surfaceFor(capture) {
  const target = targetFor(capture.targetId);
  return {
    surfaceId: `browser-game-child-ux-surface-${capture.targetId}`,
    surfaceKind: target.surfaceKind,
    state: target.state,
    primaryAction: target.primaryAction,
    primaryTextToken: target.primaryTextToken,
    sourceEvidenceRefs: [
      `parent-proof-${proofId}-${capture.targetId}-source`,
      `parent-proof-${proofId}-${capture.targetId}-response-hash`,
    ],
    gameEvidenceRef: `parent-proof-${proofId}-${capture.targetId}-game-evidence`,
    analysisRef: target.analysisRef,
    policyCandidateRef: target.policyCandidateRef,
    parentApprovalRequestRef: target.parentApprovalRequestRef,
    adapterProofRef: target.adapterProofRef,
    reasons: target.reasons,
    rawChildCopyClaimed: false,
    renderedChildUiClaimed: false,
    notificationDeliveredClaimed: false,
    browserNavigationBlockedClaimed: false,
    blockPageRenderedClaimed: false,
    timeLimitAppliedClaimed: false,
    finalPolicyDecisionClaimed: false,
    cloudFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    enforcementClaimed: false,
  };
}

function snapshotFor(surfaces) {
  return {
    schemaVersion: 'browser-game-child-checking-block-ux-contract',
    familyId: 'family-browser-game-child-ux-live-proof',
    childProfileId: 'child-browser-game-child-ux-live-proof',
    deviceId: 'device-browser-game-child-ux-live-proof',
    generatedAt: startedAt,
    surfaces,
    claimBoundaries: {
      rawChildCopy: 'not-claimed',
      renderedChildUi: 'not-claimed',
      notificationDelivery: 'not-claimed',
      browserNavigationBlock: 'not-claimed',
      blockPageRender: 'not-claimed',
      timeLimitApply: 'not-claimed',
      finalPolicyDecision: 'not-claimed',
      cloudFrameAnalysis: 'not-claimed',
      nativeGameControl: 'not-claimed',
      enforcement: 'not-claimed',
    },
  };
}

function runNegativeChecks(validSurface, validSnapshot) {
  const invalidSurfaces = [
    ['rawChildCopyClaimed', { rawChildCopyClaimed: true }],
    ['renderedChildUiClaimed', { renderedChildUiClaimed: true }],
    ['notificationDeliveredClaimed', { notificationDeliveredClaimed: true }],
    ['browserNavigationBlockedClaimed', { browserNavigationBlockedClaimed: true }],
    ['blockPageRenderedClaimed', { blockPageRenderedClaimed: true }],
    ['timeLimitAppliedClaimed', { timeLimitAppliedClaimed: true }],
    ['finalPolicyDecisionClaimed', { finalPolicyDecisionClaimed: true }],
    ['cloudFrameAnalysisClaimed', { cloudFrameAnalysisClaimed: true }],
    ['nativeGameControlClaimed', { nativeGameControlClaimed: true }],
    ['enforcementClaimed', { enforcementClaimed: true }],
    ['checkingMissingAnalysis', { analysisRef: null }],
    ['checkingWrongTextToken', { primaryTextToken: 'browser-game.child.blocked.title' }],
    ['checkingWrongAction', { primaryAction: 'acknowledge' }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameChildCheckingBlockSurfaceSchema.safeParse({ ...validSurface, ...override }).success,
  }));

  const incompleteSnapshot = {
    ...validSnapshot,
    surfaces: validSnapshot.surfaces.filter((surface) => surface.surfaceKind !== 'cloud-gaming-manual-required'),
  };
  const duplicateSnapshot = {
    ...validSnapshot,
    surfaces: [...validSnapshot.surfaces, validSnapshot.surfaces[0]],
  };

  return [
    ...invalidSurfaces,
    {
      name: 'snapshotMissingManualRequiredSurface',
      rejected: !BrowserGameChildCheckingBlockUxSnapshotSchema.safeParse(incompleteSnapshot).success,
    },
    {
      name: 'snapshotDuplicateSurfaceKind',
      rejected: !BrowserGameChildCheckingBlockUxSnapshotSchema.safeParse(duplicateSnapshot).success,
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
