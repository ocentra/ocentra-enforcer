import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameUgcRiskAssessmentSchema,
  BrowserGameUgcRiskRowSchema,
} from '@ocentra-parent/schema-domain/browser-game-ugc-multiplayer-chat-risk';

const repoRoot = process.cwd();
const proofId = 'browser-game-ugc-multiplayer-chat-risk-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-16-ugc-multiplayer-chat-risk-model',
  '02-live-ugc-multiplayer-chat-risk-shape-proof.json'
);

const targets = [
  {
    targetId: 'roblox-discover-ugc',
    url: 'https://www.roblox.com/discover',
    platformSurfaceKind: 'ugc-game-page',
    recommendedControl: 'block-unknown-ugc-candidate',
    confidence: 'medium',
    riskRows: [
      risk('ugc-world', 'managed-route', 'high', 'medium'),
      risk('unknown-player-contact', 'public-risk-context', 'high', 'medium'),
      risk('chat-contact', 'chat-control-capability', 'high', 'medium'),
    ],
    refs: {
      parentRuleRef: 'parent-proof-browser-game-ugc-rule-ref',
      chatControlCapabilityRef: 'parent-proof-browser-game-chat-control-ref',
      mobileCapabilityRef: 'parent-proof-browser-game-mobile-capability-ref',
    },
  },
  {
    targetId: 'scratch-games-ugc',
    url: 'https://scratch.mit.edu/explore/projects/games',
    platformSurfaceKind: 'experience-page',
    recommendedControl: 'parent-review-candidate',
    confidence: 'medium',
    riskRows: [
      risk('unsafe-user-created-experience', 'platform-metadata', 'high', 'medium'),
      risk('voice-contact', 'chat-control-capability', 'medium', 'medium'),
    ],
    refs: {
      parentRuleRef: 'parent-proof-browser-game-ugc-rule-ref',
      chatControlCapabilityRef: 'parent-proof-browser-game-voice-control-ref',
    },
  },
  {
    targetId: 'minecraft-marketplace',
    url: 'https://www.minecraft.net/en-us/marketplace',
    platformSurfaceKind: 'experience-page',
    recommendedControl: 'approved-experience-only-candidate',
    confidence: 'medium',
    riskRows: [
      risk('ugc-world', 'approved-experience', 'medium', 'medium'),
      risk('virtual-currency', 'purchase-control-capability', 'medium', 'medium'),
    ],
    refs: {
      parentRuleRef: 'parent-proof-browser-game-approved-experience-rule-ref',
      approvedExperienceRef: 'parent-proof-browser-game-approved-experience-ref',
      purchaseApprovalCapabilityRef: 'parent-proof-browser-game-purchase-control-ref',
    },
  },
  {
    targetId: 'chess-online-multiplayer',
    url: 'https://www.chess.com/play/online',
    platformSurfaceKind: 'multiplayer-lobby',
    recommendedControl: 'time-limit-candidate',
    confidence: 'medium',
    riskRows: [risk('unknown-player-contact', 'managed-route', 'medium', 'medium')],
    refs: {
      parentRuleRef: 'parent-proof-browser-game-time-limit-rule-ref',
    },
  },
  {
    targetId: 'steam-community-chat',
    url: 'https://steamcommunity.com/chat/',
    platformSurfaceKind: 'profile-friends-messages',
    recommendedControl: 'block-chat-candidate',
    confidence: 'medium',
    riskRows: [
      risk('chat-contact', 'chat-control-capability', 'high', 'medium'),
      risk('off-platform-contact', 'public-risk-context', 'medium', 'medium'),
    ],
    refs: {
      chatControlCapabilityRef: 'parent-proof-browser-game-chat-control-ref',
    },
  },
  {
    targetId: 'recroom-manual',
    url: 'https://recroom.com/',
    platformSurfaceKind: 'manual-required',
    recommendedControl: 'manual-review-candidate',
    confidence: 'low',
    degradedState: 'manual-required',
    uncertaintyReasons: ['manual-required', 'missing-capability-proof'],
    riskRows: [manualRisk('manual-required')],
  },
  {
    targetId: 'xbox-cloud-app-launch',
    url: 'https://www.xbox.com/en-US/play',
    platformSurfaceKind: 'web-to-app-launch',
    recommendedControl: 'manual-review-candidate',
    confidence: 'low',
    degradedState: 'degraded',
    uncertaintyReasons: ['missing-capability-proof', 'low-confidence'],
    riskRows: [risk('web-to-app-launch-risk', 'mobile-capability', 'medium', 'low')],
    refs: {
      mobileCapabilityRef: 'parent-proof-browser-game-mobile-capability-ref',
    },
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const assessments = captures.map(assessmentFor);
const riskRows = assessments.flatMap((assessment) => assessment.riskRows);
const negativeChecks = runNegativeChecks(assessments[0], riskRows[0]);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game UGC public captures to return HTTP 2xx/3xx responses');
}
if (!riskRows.every((row) => BrowserGameUgcRiskRowSchema.safeParse(row).success)) {
  throw new Error('Expected every browser-game UGC risk row to parse');
}
if (!assessments.every((assessment) => BrowserGameUgcRiskAssessmentSchema.safeParse(assessment).success)) {
  throw new Error('Expected every browser-game UGC risk assessment to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  const failedChecks = negativeChecks.filter((check) => !check.rejected).map((check) => check.name);
  throw new Error(`Expected browser-game UGC risk negative checks to reject overclaims: ${failedChecks.join(', ')}`);
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-ugc-multiplayer-chat-risk-shapes',
  targets: captures,
  assessments,
  riskRows,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    assessmentCount: assessments.length,
    riskRowCount: riskRows.length,
    negativeChecks: negativeChecks.length,
    platformSurfaceKinds: [...new Set(assessments.map((assessment) => assessment.platformSurfaceKind))],
    recommendedControls: [...new Set(assessments.map((assessment) => assessment.recommendedControl))],
    riskKinds: [...new Set(riskRows.map((row) => row.riskKind))],
    rawChatContentRead: false,
    rawProfileContentStored: false,
    rawExperienceIdentifierStored: false,
    rawAccountIdentifierStored: false,
    rawGamePayloadUsed: false,
    webToAppLaunchExecuted: false,
    purchaseExecutionClaimed: false,
    nativeGameControlClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    uiRenderedClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-ugc-multiplayer-chat-risk-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(
  `targets=${captures.length} assessments=${assessments.length} riskRows=${riskRows.length} negativeChecks=${negativeChecks.length}`
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
    rawChatContentPersisted: false,
    rawProfileContentPersisted: false,
    rawExperienceIdentifierPersisted: false,
    rawAccountIdentifierPersisted: false,
  };
}

function assessmentFor(capture) {
  const target = targetFor(capture.targetId);
  return {
    schemaVersion: 'browser-game-ugc-multiplayer-chat-risk-contract',
    assessmentId: `browser-game-ugc-risk-assessment-${capture.targetId}`,
    familyId: 'family-browser-game-ugc-live-proof',
    childProfileId: 'child-browser-game-ugc-live-proof',
    deviceId: 'device-browser-game-ugc-live-proof',
    assessedAt: startedAt,
    platformSurfaceKind: target.platformSurfaceKind,
    sourceEvidenceRefs: [
      `parent-proof-${proofId}-${capture.targetId}-source`,
      `parent-proof-${proofId}-${capture.targetId}-response-hash`,
    ],
    riskRows: target.riskRows.map((row, index) => rowFor(capture.targetId, row, index)),
    recommendedControl: target.recommendedControl,
    confidence: target.confidence,
    degradedState: target.degradedState ?? 'none',
    uncertaintyReasons: target.uncertaintyReasons ?? [],
    parentRuleRef: target.refs?.parentRuleRef ?? null,
    approvedExperienceRef: target.refs?.approvedExperienceRef ?? null,
    chatControlCapabilityRef: target.refs?.chatControlCapabilityRef ?? null,
    purchaseApprovalCapabilityRef: target.refs?.purchaseApprovalCapabilityRef ?? null,
    mobileCapabilityRef: target.refs?.mobileCapabilityRef ?? null,
    rawChatContentRead: false,
    rawProfileContentStored: false,
    rawExperienceIdentifierStored: false,
    rawAccountIdentifierStored: false,
    rawGamePayloadUsed: false,
    webToAppLaunchExecuted: false,
    purchaseExecutionClaimed: false,
    nativeGameControlClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    uiRenderedClaimed: false,
    enforcementClaimed: false,
  };
}

function rowFor(targetId, row, index) {
  return {
    riskRowId: `browser-game-ugc-risk-row-${targetId}-${index + 1}`,
    evidenceKind: row.evidenceKind,
    riskKind: row.riskKind,
    state: row.state,
    severity: row.severity,
    confidence: row.confidence,
    evidenceRefs: [`parent-proof-${proofId}-${targetId}-risk-${index + 1}`],
    rawChatContentRead: false,
    rawProfileContentStored: false,
    rawExperienceIdentifierStored: false,
    rawAccountIdentifierStored: false,
    rawGamePayloadUsed: false,
    webToAppLaunchExecuted: false,
    purchaseExecutionClaimed: false,
    nativeGameControlClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    enforcementClaimed: false,
  };
}

function runNegativeChecks(validAssessment, validRiskRow) {
  const invalidAssessments = [
    ['assessmentRawChatContentRead', { rawChatContentRead: true }],
    ['assessmentRawProfileContentStored', { rawProfileContentStored: true }],
    ['assessmentRawExperienceIdentifierStored', { rawExperienceIdentifierStored: true }],
    ['assessmentRawAccountIdentifierStored', { rawAccountIdentifierStored: true }],
    ['assessmentRawGamePayloadUsed', { rawGamePayloadUsed: true }],
    ['assessmentWebToAppLaunchExecuted', { webToAppLaunchExecuted: true }],
    ['assessmentPurchaseExecutionClaimed', { purchaseExecutionClaimed: true }],
    ['assessmentNativeGameControlClaimed', { nativeGameControlClaimed: true }],
    ['assessmentFinalPolicyDecisionClaimed', { finalPolicyDecisionClaimed: true }],
    ['assessmentRuntimeGateExecutedClaimed', { runtimeGateExecutedClaimed: true }],
    ['assessmentUiRenderedClaimed', { uiRenderedClaimed: true }],
    ['assessmentEnforcementClaimed', { enforcementClaimed: true }],
    ['assessmentEmptyRiskRows', { riskRows: [] }],
    ['assessmentCandidateManualSurface', { platformSurfaceKind: 'manual-required' }],
    [
      'assessmentBlockChatMissingCapability',
      { recommendedControl: 'block-chat-candidate', chatControlCapabilityRef: null },
    ],
    [
      'assessmentPurchaseMissingCapability',
      {
        recommendedControl: 'purchase-approval-candidate',
        riskRows: [
          rowFor('invalid-purchase', risk('in-game-purchase', 'purchase-control-capability', 'medium', 'medium'), 0),
        ],
        purchaseApprovalCapabilityRef: null,
      },
    ],
    [
      'assessmentApprovedExperienceMissingRefs',
      { recommendedControl: 'approved-experience-only-candidate', approvedExperienceRef: null, parentRuleRef: null },
    ],
    [
      'assessmentDegradedHighConfidence',
      { degradedState: 'degraded', confidence: 'high', uncertaintyReasons: ['low-confidence'] },
    ],
    [
      'assessmentDegradedNoUncertainty',
      { degradedState: 'degraded', recommendedControl: 'manual-review-candidate', uncertaintyReasons: [] },
    ],
    ['assessmentUnknownConfidenceActive', { confidence: 'unknown' }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameUgcRiskAssessmentSchema.safeParse({ ...validAssessment, ...override }).success,
  }));

  const invalidRows = [
    ['rowRawChatContentRead', { rawChatContentRead: true }],
    ['rowRawProfileContentStored', { rawProfileContentStored: true }],
    ['rowRawExperienceIdentifierStored', { rawExperienceIdentifierStored: true }],
    ['rowRawAccountIdentifierStored', { rawAccountIdentifierStored: true }],
    ['rowRawGamePayloadUsed', { rawGamePayloadUsed: true }],
    ['rowWebToAppLaunchExecuted', { webToAppLaunchExecuted: true }],
    ['rowPurchaseExecutionClaimed', { purchaseExecutionClaimed: true }],
    ['rowNativeGameControlClaimed', { nativeGameControlClaimed: true }],
    ['rowFinalPolicyDecisionClaimed', { finalPolicyDecisionClaimed: true }],
    ['rowRuntimeGateExecutedClaimed', { runtimeGateExecutedClaimed: true }],
    ['rowEnforcementClaimed', { enforcementClaimed: true }],
    ['rowCandidateUnknownRisk', { riskKind: 'unknown-risk' }],
    ['rowCandidateUnknownConfidence', { confidence: 'unknown' }],
  ].map(([name, override]) => ({
    name,
    rejected: !BrowserGameUgcRiskRowSchema.safeParse({ ...validRiskRow, ...override }).success,
  }));

  return [...invalidAssessments, ...invalidRows];
}

function risk(riskKind, evidenceKind, severity, confidence) {
  return {
    riskKind,
    evidenceKind,
    state: 'candidate',
    severity,
    confidence,
  };
}

function manualRisk(state) {
  return {
    riskKind: 'manual-required',
    evidenceKind: 'manual-required',
    state,
    severity: 'unknown',
    confidence: 'unknown',
  };
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
