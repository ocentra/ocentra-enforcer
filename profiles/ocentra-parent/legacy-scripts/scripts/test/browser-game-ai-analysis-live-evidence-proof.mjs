import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameAiAnalysisInputSchema,
  BrowserGameAiAnalysisResultSchema,
} from '@ocentra-parent/schema-domain/browser-game-ai-analysis';

const repoRoot = process.cwd();
const proofId = 'browser-game-ai-analysis-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-10-browser-game-ai-analysis-contract',
  '02-live-ai-analysis-shape-proof.json'
);

const targets = [
  {
    targetId: 'poki-subway-surfers',
    url: 'https://poki.com/en/g/subway-surfers',
    task: 'risk-classification',
    custodyLabel: 'managed-browser',
    gameSurfaceKind: 'game-portal',
    modifiers: ['canvas', 'iframe-embedded'],
    benefitSignals: ['problem-solving'],
    riskSignals: ['addiction-loop', 'privacy-risk'],
    recommendedPolicyInput: 'parent-review-candidate',
    confidence: 'medium',
  },
  {
    targetId: 'code-org-minecraft',
    url: 'https://code.org/minecraft',
    task: 'educational-game-check',
    custodyLabel: 'hidden-analysis-profile',
    gameSurfaceKind: 'educational-game',
    modifiers: ['school-context'],
    benefitSignals: ['educational-value', 'homework-relevance', 'skill-building'],
    riskSignals: [],
    recommendedPolicyInput: 'allow-candidate',
    confidence: 'high',
  },
  {
    targetId: 'chess-play-online',
    url: 'https://www.chess.com/play/online',
    task: 'game-classification',
    custodyLabel: 'managed-browser',
    gameSurfaceKind: 'browser-game',
    modifiers: ['multiplayer'],
    benefitSignals: ['problem-solving', 'skill-building'],
    riskSignals: ['multiplayer-contact'],
    recommendedPolicyInput: 'time-limit-candidate',
    confidence: 'medium',
  },
  {
    targetId: 'xbox-cloud-play',
    url: 'https://www.xbox.com/en-US/play',
    task: 'cloud-gaming-detection',
    custodyLabel: 'manual-required',
    gameSurfaceKind: 'cloud-gaming',
    modifiers: ['cloud-streaming', 'gamepad', 'fullscreen'],
    benefitSignals: [],
    riskSignals: ['unknown-risk'],
    recommendedPolicyInput: 'manual-review-candidate',
    confidence: 'low',
    degradedState: 'manual-required',
    uncertaintyReasons: ['manual-required', 'missing-runtime-signal'],
  },
  {
    targetId: 'roblox-discover',
    url: 'https://www.roblox.com/discover',
    task: 'ugc-game-risk',
    custodyLabel: 'unmanaged-browser-bypass',
    gameSurfaceKind: 'ugc-multiplayer-game',
    modifiers: ['multiplayer', 'chat', 'unknown'],
    benefitSignals: ['creativity'],
    riskSignals: ['user-generated-content-risk', 'chat-risk', 'privacy-risk'],
    recommendedPolicyInput: 'parent-review-candidate',
    confidence: 'medium',
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const inputs = captures.map(inputFor);
const results = captures.map(resultFor);
const negativeChecks = runNegativeChecks(inputs[0], results[0]);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game AI analysis public captures to return HTTP 2xx/3xx responses');
}
if (!inputs.every((input) => BrowserGameAiAnalysisInputSchema.safeParse(input).success)) {
  throw new Error('Expected every browser-game AI analysis input to parse');
}
if (!results.every((result) => BrowserGameAiAnalysisResultSchema.safeParse(result).success)) {
  throw new Error('Expected every browser-game AI analysis result to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected browser-game AI analysis negative checks to reject overclaims');
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-ai-analysis-shapes',
  targets: captures,
  inputs,
  results,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    inputs: inputs.length,
    results: results.length,
    negativeChecks: negativeChecks.length,
    tasks: [...new Set(inputs.map((input) => input.task))],
    candidatePolicyInputs: [...new Set(results.map((result) => result.recommendedPolicyInput))],
    rawUrlIncluded: false,
    rawPageBodyIncluded: false,
    rawGamePayloadIncluded: false,
    rawScreenFrameIncluded: false,
    rawModelTextIncluded: false,
    rawModelTextStored: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    uiRenderedClaimed: false,
    accountOrPurchaseExecutionClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    enforcementClaimed: false,
    modelExecutionClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-ai-analysis-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(
  `targets=${captures.length} inputs=${inputs.length} results=${results.length} negativeChecks=${negativeChecks.length}`
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
    task: target.task,
    custodyLabel: target.custodyLabel,
    gameSurfaceKind: target.gameSurfaceKind,
    rawUrlPersisted: false,
    rawPageBodyPersisted: false,
    rawGamePayloadPersisted: false,
    rawScreenFramePersisted: false,
    rawModelTextPersisted: false,
  };
}

function inputFor(capture) {
  const target = targetFor(capture.targetId);
  const unmanaged = target.custodyLabel === 'unmanaged-browser-bypass';
  const manual = target.custodyLabel === 'manual-required';
  return {
    schemaVersion: 'browser-game-ai-analysis-contract',
    requestId: `ai-analysis-request-${capture.targetId}`,
    familyId: 'family-browser-game-ai-live-proof',
    childProfileId: 'child-browser-game-ai-live-proof',
    deviceId: 'device-browser-game-ai-live-proof',
    requestedAt: startedAt,
    sourceEvidenceRefs: [`parent-proof-${proofId}-${capture.targetId}-source`],
    browserEvidenceRef: unmanaged ? null : `parent-proof-${proofId}-${capture.targetId}-browser-evidence`,
    urlShapeRef: `parent-proof-${proofId}-${capture.targetId}-url-shape`,
    runtimeSignalRef: manual ? null : `parent-proof-${proofId}-${capture.targetId}-runtime-signal`,
    metadataEvidenceRefs: [`parent-proof-${proofId}-${capture.targetId}-metadata`],
    screenSummaryRefs: [`parent-proof-${proofId}-${capture.targetId}-screen-summary`],
    parentRuleRefs: [`parent-proof-${proofId}-${capture.targetId}-parent-rule`],
    recentActivityRef: `parent-proof-${proofId}-${capture.targetId}-recent-activity`,
    memoryRefs: [`parent-proof-${proofId}-${capture.targetId}-memory`],
    task: target.task,
    custodyLabel: target.custodyLabel,
    rawUrlIncluded: false,
    rawPageBodyIncluded: false,
    rawGamePayloadIncluded: false,
    rawScreenFrameIncluded: false,
    rawModelTextIncluded: false,
    accountOrPurchaseExecutionClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    enforcementClaimed: false,
  };
}

function resultFor(capture) {
  const target = targetFor(capture.targetId);
  const degradedState = target.degradedState ?? 'none';
  return {
    schemaVersion: 'browser-game-ai-analysis-contract',
    analysisId: `ai-analysis-result-${capture.targetId}`,
    requestId: `ai-analysis-request-${capture.targetId}`,
    familyId: 'family-browser-game-ai-live-proof',
    childProfileId: 'child-browser-game-ai-live-proof',
    deviceId: 'device-browser-game-ai-live-proof',
    analyzedAt: startedAt,
    expiresAt: degradedState === 'none' ? expiresAt() : null,
    sourceEvidenceRefs: [`parent-proof-${proofId}-${capture.targetId}-source`],
    parentRuleRefs: [`parent-proof-${proofId}-${capture.targetId}-parent-rule`],
    task: target.task,
    isGame: true,
    gameSurfaceKind: target.gameSurfaceKind,
    modifiers: target.modifiers,
    benefitSignals: target.benefitSignals,
    riskSignals: target.riskSignals,
    recommendedPolicyInput: target.recommendedPolicyInput,
    confidence: target.confidence,
    uncertaintyReasons: target.uncertaintyReasons ?? [],
    parentSummaryRef: `parent-proof-${proofId}-${capture.targetId}-parent-summary`,
    childSafeSummaryRef: `parent-proof-${proofId}-${capture.targetId}-child-summary`,
    modelRuntimeRef: `parent-proof-${proofId}-${capture.targetId}-model-runtime-ref`,
    promptTemplateVersion: 'browser-game-ai-analysis-live-proof-template-v1',
    degradedState,
    rawModelTextStored: false,
    rawPageBodyStored: false,
    rawGamePayloadStored: false,
    rawScreenFrameStored: false,
    accountOrPurchaseExecutionClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    uiRenderedClaimed: false,
    enforcementClaimed: false,
  };
}

function runNegativeChecks(validInput, validResult) {
  const invalidInputs = [
    ['input-raw-url', { rawUrlIncluded: true }],
    ['input-raw-page-body', { rawPageBodyIncluded: true }],
    ['input-raw-game-payload', { rawGamePayloadIncluded: true }],
    ['input-raw-screen-frame', { rawScreenFrameIncluded: true }],
    ['input-raw-model-text', { rawModelTextIncluded: true }],
    ['input-account-purchase-execution', { accountOrPurchaseExecutionClaimed: true }],
    ['input-native-game-control', { nativeGameControlClaimed: true }],
    ['input-cloud-frame-analysis', { cloudFrameAnalysisClaimed: true }],
    ['input-final-policy-decision', { finalPolicyDecisionClaimed: true }],
    ['input-runtime-gate', { runtimeGateExecutedClaimed: true }],
    ['input-enforcement', { enforcementClaimed: true }],
    ['input-managed-missing-browser-ref', { browserEvidenceRef: null }],
    [
      'input-unmanaged-with-browser-ref',
      { custodyLabel: 'unmanaged-browser-bypass', browserEvidenceRef: 'browser-ref-should-not-exist' },
    ],
  ];
  const invalidResults = [
    ['result-raw-model-text', { rawModelTextStored: true }],
    ['result-raw-page-body', { rawPageBodyStored: true }],
    ['result-raw-game-payload', { rawGamePayloadStored: true }],
    ['result-raw-screen-frame', { rawScreenFrameStored: true }],
    ['result-account-purchase-execution', { accountOrPurchaseExecutionClaimed: true }],
    ['result-native-game-control', { nativeGameControlClaimed: true }],
    ['result-cloud-frame-analysis', { cloudFrameAnalysisClaimed: true }],
    ['result-final-policy-decision', { finalPolicyDecisionClaimed: true }],
    ['result-runtime-gate', { runtimeGateExecutedClaimed: true }],
    ['result-ui-rendered', { uiRenderedClaimed: true }],
    ['result-enforcement', { enforcementClaimed: true }],
    ['result-no-expiry', { expiresAt: null }],
    ['result-allow-with-high-risk', { recommendedPolicyInput: 'allow-candidate', riskSignals: ['violence'] }],
    ['result-block-without-signal', { recommendedPolicyInput: 'block-candidate', riskSignals: [], benefitSignals: [] }],
    [
      'result-degraded-high-confidence',
      { degradedState: 'degraded', confidence: 'high', uncertaintyReasons: ['low-confidence'] },
    ],
  ];
  return [
    ...invalidInputs.map(([name, invalid]) => negativeInputCheck(name, validInput, invalid)),
    ...invalidResults.map(([name, invalid]) => negativeResultCheck(name, validResult, invalid)),
  ];
}

function negativeInputCheck(name, validInput, invalid) {
  return {
    name,
    rejected: !BrowserGameAiAnalysisInputSchema.safeParse({ ...validInput, ...invalid }).success,
  };
}

function negativeResultCheck(name, validResult, invalid) {
  return {
    name,
    rejected: !BrowserGameAiAnalysisResultSchema.safeParse({ ...validResult, ...invalid }).success,
  };
}

function targetFor(targetId) {
  const target = targets.find((entry) => entry.targetId === targetId);
  if (!target) {
    throw new Error(`Unknown target ${targetId}`);
  }
  return target;
}

function expiresAt() {
  return new Date(Date.parse(startedAt) + 30 * 60 * 1000).toISOString();
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
