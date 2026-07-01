import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameDashboardPanelSchema,
  BrowserGameDashboardUxSnapshotSchema,
} from '@ocentra-parent/schema-domain/browser-game-dashboard-ux';

const repoRoot = process.cwd();
const proofId = 'browser-game-dashboard-ux-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-20-parent-browser-game-dashboard-ux',
  '02-live-dashboard-ux-shape-proof.json'
);

const targets = [
  {
    targetId: 'scratch-detected-game-review',
    url: 'https://scratch.mit.edu/explore/projects/games/',
    panelKind: 'detected-game-review',
    status: 'ready-for-review',
    primaryAction: 'review-detected-game',
    severity: 'info',
    reasons: ['detected-game-evidence-ready'],
    approvalRequestRef: null,
    policyCandidateRef: 'parent-evidence-game-policy-detected-review',
    mobileCapabilityRef: null,
  },
  {
    targetId: 'roblox-unknown-approval-queue',
    url: 'https://www.roblox.com/discover',
    panelKind: 'unknown-game-approval-queue',
    status: 'ready-for-review',
    primaryAction: 'open-parent-approval',
    severity: 'warning',
    reasons: ['unknown-game-parent-review-needed'],
    approvalRequestRef: 'parent-evidence-game-approval-request-roblox',
    policyCandidateRef: 'parent-evidence-game-policy-parent-review',
    mobileCapabilityRef: null,
  },
  {
    targetId: 'xbox-cloud-approval-manual',
    url: 'https://www.xbox.com/en-US/play',
    panelKind: 'cloud-gaming-approval',
    status: 'manual-required',
    primaryAction: 'review-cloud-gaming',
    severity: 'warning',
    reasons: ['cloud-gaming-manual-required'],
    approvalRequestRef: null,
    policyCandidateRef: null,
    mobileCapabilityRef: 'parent-evidence-game-mobile-cloud-capability-manual',
  },
  {
    targetId: 'code-org-educational-allowlist',
    url: 'https://code.org/minecraft',
    panelKind: 'educational-game-allowlist',
    status: 'contract-only',
    primaryAction: 'review-educational-allowlist',
    severity: 'info',
    reasons: ['educational-allowlist-contract-only'],
    approvalRequestRef: null,
    policyCandidateRef: 'parent-evidence-game-policy-educational-allow',
    mobileCapabilityRef: null,
  },
  {
    targetId: 'coolmath-time-budget-candidate',
    url: 'https://www.coolmathgames.com/0-run',
    panelKind: 'game-time-budget-candidates',
    status: 'contract-only',
    primaryAction: 'review-time-budget',
    severity: 'info',
    reasons: ['time-budget-candidate-only'],
    approvalRequestRef: null,
    policyCandidateRef: 'parent-evidence-game-policy-time-budget',
    mobileCapabilityRef: null,
  },
  {
    targetId: 'steam-native-capability-gap',
    url: 'https://store.steampowered.com/',
    panelKind: 'mobile-native-capability-gaps',
    status: 'manual-required',
    primaryAction: 'review-mobile-capability',
    severity: 'warning',
    reasons: ['mobile-native-proof-gap'],
    approvalRequestRef: null,
    policyCandidateRef: null,
    mobileCapabilityRef: 'parent-evidence-game-native-capability-gap',
  },
  {
    targetId: 'recroom-platform-manual-gap',
    url: 'https://recroom.com/',
    panelKind: 'manual-required-gaps',
    status: 'manual-required',
    primaryAction: 'manual-review',
    severity: 'warning',
    reasons: ['platform-proof-gap'],
    approvalRequestRef: null,
    policyCandidateRef: null,
    mobileCapabilityRef: null,
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const panels = captures.map(panelFor);
const snapshot = snapshotFor(panels);
const negativeChecks = runNegativeChecks(panels[0], snapshot);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game dashboard public captures to return HTTP 2xx/3xx responses');
}
if (!panels.every((panel) => BrowserGameDashboardPanelSchema.safeParse(panel).success)) {
  throw new Error('Expected every browser-game dashboard panel to parse');
}
if (!BrowserGameDashboardUxSnapshotSchema.safeParse(snapshot).success) {
  throw new Error('Expected browser-game dashboard UX snapshot to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  const failedChecks = negativeChecks.filter((check) => !check.rejected).map((check) => check.name);
  throw new Error(
    `Expected browser-game dashboard UX negative checks to reject overclaims: ${failedChecks.join(', ')}`
  );
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-dashboard-ux-shapes',
  targets: captures,
  panels,
  snapshot,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    panelCount: panels.length,
    negativeChecks: negativeChecks.length,
    panelKinds: [...new Set(panels.map((panel) => panel.panelKind))],
    statuses: [...new Set(panels.map((panel) => panel.status))],
    rawUrlPersisted: false,
    rawPageBodyPersisted: false,
    rawGamePayloadPersisted: false,
    renderedPortalUiClaimed: false,
    notificationClaimed: false,
    runtimeDataFetchClaimed: false,
    finalPolicyDecisionClaimed: false,
    cloudFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-dashboard-ux-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`targets=${captures.length} panels=${panels.length} negativeChecks=${negativeChecks.length}`);

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
  };
}

function panelFor(capture) {
  const target = targetFor(capture.targetId);
  return {
    panelId: `browser-game-dashboard-panel-${capture.targetId}`,
    panelKind: target.panelKind,
    status: target.status,
    primaryAction: target.primaryAction,
    severity: target.severity,
    sortOrder: targets.findIndex((candidate) => candidate.targetId === capture.targetId),
    sourceEvidenceRefs: [
      `parent-proof-${proofId}-${capture.targetId}-source`,
      `parent-proof-${proofId}-${capture.targetId}-response-hash`,
    ],
    approvalRequestRef: target.approvalRequestRef,
    policyCandidateRef: target.policyCandidateRef,
    mobileCapabilityRef: target.mobileCapabilityRef,
    reasons: target.reasons,
    renderedPortalUiClaimed: false,
    notificationClaimed: false,
    runtimeDataFetchClaimed: false,
    finalPolicyDecisionClaimed: false,
    cloudFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    enforcementClaimed: false,
  };
}

function snapshotFor(panels) {
  return {
    schemaVersion: 'browser-game-dashboard-ux-contract',
    familyId: 'family-browser-game-dashboard-live-proof',
    childProfileId: 'child-browser-game-dashboard-live-proof',
    generatedAt: startedAt,
    panels,
    claimBoundaries: {
      renderedPortalUi: 'not-claimed',
      notificationDelivery: 'not-claimed',
      runtimeDataFetch: 'not-claimed',
      finalPolicyDecision: 'not-claimed',
      cloudFrameAnalysis: 'not-claimed',
      nativeGameControl: 'not-claimed',
      enforcement: 'not-claimed',
    },
  };
}

function runNegativeChecks(validPanel, validSnapshot) {
  const invalidPanels = [
    ['renderedPortalUiClaimed', { renderedPortalUiClaimed: true }],
    ['notificationClaimed', { notificationClaimed: true }],
    ['runtimeDataFetchClaimed', { runtimeDataFetchClaimed: true }],
    ['finalPolicyDecisionClaimed', { finalPolicyDecisionClaimed: true }],
    ['cloudFrameAnalysisClaimed', { cloudFrameAnalysisClaimed: true }],
    ['nativeGameControlClaimed', { nativeGameControlClaimed: true }],
    ['enforcementClaimed', { enforcementClaimed: true }],
    ['detectedGameWrongAction', { primaryAction: 'manual-review' }],
    ['detectedGameManualStatus', { status: 'manual-required' }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameDashboardPanelSchema.safeParse({ ...validPanel, ...override }).success,
  }));

  const approvalPanel = validSnapshot.panels.find((panel) => panel.panelKind === 'unknown-game-approval-queue');
  const mobilePanel = validSnapshot.panels.find((panel) => panel.panelKind === 'mobile-native-capability-gaps');
  const incompleteSnapshot = {
    ...validSnapshot,
    panels: validSnapshot.panels.filter((panel) => panel.panelKind !== 'manual-required-gaps'),
  };
  const duplicateSnapshot = {
    ...validSnapshot,
    panels: [...validSnapshot.panels, validSnapshot.panels[0]],
  };

  return [
    ...invalidPanels,
    {
      name: 'approvalPanelMissingApprovalRef',
      rejected: !BrowserGameDashboardPanelSchema.safeParse({ ...approvalPanel, approvalRequestRef: null }).success,
    },
    {
      name: 'mobilePanelMissingCapabilityRef',
      rejected: !BrowserGameDashboardPanelSchema.safeParse({ ...mobilePanel, mobileCapabilityRef: null }).success,
    },
    {
      name: 'snapshotMissingManualRequiredPanel',
      rejected: !BrowserGameDashboardUxSnapshotSchema.safeParse(incompleteSnapshot).success,
    },
    {
      name: 'snapshotDuplicatePanelKind',
      rejected: !BrowserGameDashboardUxSnapshotSchema.safeParse(duplicateSnapshot).success,
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
