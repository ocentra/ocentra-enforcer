import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import { BrowserGameRiskBenefitSignalSetSchema } from '@ocentra-parent/schema-domain/browser-game-riskbenefit-signal';

const repoRoot = process.cwd();
const proofId = 'browser-game-riskbenefit-signal-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-11-game-risk-benefit-signal-model',
  '02-live-riskbenefit-signal-shape-proof.json'
);

const targets = [
  {
    targetId: 'poki-subway-surfers',
    url: 'https://poki.com/en/g/subway-surfers',
    signalSourceKind: 'game-ai-analysis',
    riskSignals: [risk('addiction-loop', 'medium', 'medium'), risk('privacy-risk', 'low', 'medium')],
    benefitSignals: [benefit('problem-solving', 'low', 'medium')],
    recommendedPolicyInput: 'parent-review-candidate',
    confidence: 'medium',
  },
  {
    targetId: 'code-org-minecraft',
    url: 'https://code.org/minecraft',
    signalSourceKind: 'game-ai-analysis',
    riskSignals: [],
    benefitSignals: [
      benefit('educational-value', 'medium', 'high'),
      benefit('homework-relevance', 'medium', 'high'),
      benefit('skill-building', 'medium', 'high'),
    ],
    recommendedPolicyInput: 'allow-candidate',
    confidence: 'high',
  },
  {
    targetId: 'chess-play-online',
    url: 'https://www.chess.com/play/online',
    signalSourceKind: 'game-metadata',
    riskSignals: [risk('multiplayer-contact', 'medium', 'medium')],
    benefitSignals: [benefit('problem-solving', 'medium', 'medium'), benefit('skill-building', 'medium', 'medium')],
    recommendedPolicyInput: 'time-limit-candidate',
    confidence: 'medium',
  },
  {
    targetId: 'xbox-cloud-play',
    url: 'https://www.xbox.com/en-US/play',
    signalSourceKind: 'manual-required',
    riskSignals: [unknownRisk('manual-required')],
    benefitSignals: [unknownBenefit('manual-required')],
    recommendedPolicyInput: 'manual-review-candidate',
    confidence: 'low',
    degradedState: 'manual-required',
    uncertaintyReasons: ['manual-required', 'missing-analysis'],
  },
  {
    targetId: 'roblox-discover',
    url: 'https://www.roblox.com/discover',
    signalSourceKind: 'game-ai-analysis',
    riskSignals: [
      risk('user-generated-content-risk', 'high', 'medium'),
      risk('chat-risk', 'high', 'medium'),
      risk('privacy-risk', 'medium', 'medium'),
    ],
    benefitSignals: [benefit('creativity', 'medium', 'medium')],
    recommendedPolicyInput: 'parent-review-candidate',
    confidence: 'medium',
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const signalSets = captures.map(signalSetFor);
const negativeChecks = runNegativeChecks(signalSets[0]);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game risk/benefit public captures to return HTTP 2xx/3xx responses');
}
if (!signalSets.every((signalSet) => BrowserGameRiskBenefitSignalSetSchema.safeParse(signalSet).success)) {
  throw new Error('Expected every browser-game risk/benefit signal set to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  const failedChecks = negativeChecks.filter((check) => !check.rejected).map((check) => check.name);
  throw new Error(
    `Expected browser-game risk/benefit negative checks to reject overclaims: ${failedChecks.join(', ')}`
  );
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-riskbenefit-signal-shapes',
  targets: captures,
  signalSets,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    signalSetCount: signalSets.length,
    riskSignalCount: signalSets.flatMap((signalSet) => signalSet.riskSignals).length,
    benefitSignalCount: signalSets.flatMap((signalSet) => signalSet.benefitSignals).length,
    negativeChecks: negativeChecks.length,
    recommendedPolicyInputs: [...new Set(signalSets.map((signalSet) => signalSet.recommendedPolicyInput))],
    rawUrlIncluded: false,
    rawPageBodyIncluded: false,
    rawGamePayloadIncluded: false,
    rawChatContentIncluded: false,
    rawModelTextIncluded: false,
    accountOrPurchaseExecutionClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-riskbenefit-signal-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(
  `targets=${captures.length} signalSets=${signalSets.length} risks=${proof.summary.riskSignalCount} benefits=${proof.summary.benefitSignalCount} negativeChecks=${negativeChecks.length}`
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
    rawGamePayloadPersisted: false,
    rawChatContentPersisted: false,
    rawModelTextPersisted: false,
  };
}

function signalSetFor(capture) {
  const target = targetFor(capture.targetId);
  const degradedState = target.degradedState ?? 'none';
  return {
    schemaVersion: 'browser-game-riskbenefit-signal-contract',
    signalSetId: `browser-game-signal-set-${capture.targetId}`,
    familyId: 'family-browser-game-riskbenefit-live-proof',
    childProfileId: 'child-browser-game-riskbenefit-live-proof',
    deviceId: 'device-browser-game-riskbenefit-live-proof',
    modeledAt: startedAt,
    sourceEvidenceRefs: [
      `parent-proof-${proofId}-${capture.targetId}-source`,
      `parent-proof-${proofId}-${capture.targetId}-response-hash`,
    ],
    signalSourceKind: target.signalSourceKind,
    analysisRef:
      target.signalSourceKind === 'game-ai-analysis'
        ? `parent-proof-${proofId}-${capture.targetId}-analysis-ref`
        : null,
    metadataRef: `parent-proof-${proofId}-${capture.targetId}-metadata-ref`,
    parentRuleRef: `parent-proof-${proofId}-${capture.targetId}-parent-rule-ref`,
    riskSignals: target.riskSignals.map((signal, index) =>
      signalFor(capture.targetId, signal, index, `parent-proof-${proofId}-${capture.targetId}-risk-evidence`)
    ),
    benefitSignals: target.benefitSignals.map((signal, index) =>
      signalFor(capture.targetId, signal, index, `parent-proof-${proofId}-${capture.targetId}-benefit-evidence`)
    ),
    recommendedPolicyInput: target.recommendedPolicyInput,
    confidence: target.confidence,
    degradedState,
    uncertaintyReasons: target.uncertaintyReasons ?? [],
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    enforcementClaimed: false,
    rawGamePayloadUsed: false,
    rawChatContentUsed: false,
    rawPageBodyUsed: false,
    rawModelTextUsed: false,
    accountOrPurchaseExecutionClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
  };
}

function signalFor(targetId, signal, index, evidenceRef) {
  const signalKind = signal.signalType === 'risk' ? 'riskSignal' : 'benefitSignal';
  return {
    signalId: `browser-game-${signalKind}-${targetId}-${index + 1}`,
    kind: signal.kind,
    severity: signal.severity,
    state: signal.state,
    confidence: signal.confidence,
    evidenceRefs: [evidenceRef],
    analysisRef: signal.state === 'candidate' ? `parent-proof-${proofId}-${targetId}-analysis-ref` : null,
    rawGamePayloadUsed: false,
    rawChatContentUsed: false,
    rawPageBodyUsed: false,
    rawModelTextUsed: false,
    accountOrPurchaseExecutionClaimed: false,
    cloudFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
  };
}

function runNegativeChecks(validSignalSet) {
  const invalidSets = [
    ['set-final-policy-decision', { finalPolicyDecisionClaimed: true }],
    ['set-runtime-gate', { runtimeGateExecutedClaimed: true }],
    ['set-enforcement', { enforcementClaimed: true }],
    ['set-raw-game-payload', { rawGamePayloadUsed: true }],
    ['set-raw-chat-content', { rawChatContentUsed: true }],
    ['set-raw-page-body', { rawPageBodyUsed: true }],
    ['set-raw-model-text', { rawModelTextUsed: true }],
    ['set-account-purchase-execution', { accountOrPurchaseExecutionClaimed: true }],
    ['set-native-game-control', { nativeGameControlClaimed: true }],
    ['set-cloud-frame-analysis', { cloudFrameAnalysisClaimed: true }],
    ['set-empty-signals', { riskSignals: [], benefitSignals: [] }],
    [
      'set-allow-with-high-risk',
      {
        recommendedPolicyInput: 'allow-candidate',
        riskSignals: [{ ...validSignalSet.riskSignals[0], severity: 'high' }],
      },
    ],
    ['set-block-without-risk', { recommendedPolicyInput: 'block-candidate', riskSignals: [] }],
    [
      'set-degraded-high-confidence',
      { degradedState: 'degraded', confidence: 'high', uncertaintyReasons: ['low-confidence'] },
    ],
    ['set-unknown-confidence', { confidence: 'unknown' }],
    ['set-manual-source-not-degraded', { signalSourceKind: 'manual-required' }],
  ];
  const invalidRiskSignals = [
    ['risk-raw-game-payload', { rawGamePayloadUsed: true }],
    ['risk-raw-chat-content', { rawChatContentUsed: true }],
    ['risk-raw-page-body', { rawPageBodyUsed: true }],
    ['risk-raw-model-text', { rawModelTextUsed: true }],
    ['risk-policy-decision', { policyDecisionClaimed: true }],
    ['risk-enforcement', { enforcementClaimed: true }],
    ['risk-unknown-candidate', { kind: 'unknown-risk' }],
  ];
  const invalidBenefitSignals = [
    ['benefit-raw-game-payload', { rawGamePayloadUsed: true }],
    ['benefit-raw-chat-content', { rawChatContentUsed: true }],
    ['benefit-raw-page-body', { rawPageBodyUsed: true }],
    ['benefit-raw-model-text', { rawModelTextUsed: true }],
    ['benefit-policy-decision', { policyDecisionClaimed: true }],
    ['benefit-enforcement', { enforcementClaimed: true }],
    ['benefit-unknown-candidate', { kind: 'unknown-benefit' }],
  ];

  return [
    ...invalidSets.map(([name, invalid]) => negativeSetCheck(name, validSignalSet, invalid)),
    ...invalidRiskSignals.map(([name, invalid]) =>
      negativeSetCheck(name, validSignalSet, {
        riskSignals: [{ ...validSignalSet.riskSignals[0], ...invalid }],
      })
    ),
    ...invalidBenefitSignals.map(([name, invalid]) =>
      negativeSetCheck(name, validSignalSet, {
        benefitSignals: [{ ...validSignalSet.benefitSignals[0], ...invalid }],
      })
    ),
  ];
}

function negativeSetCheck(name, validSignalSet, invalid) {
  return {
    name,
    rejected: !BrowserGameRiskBenefitSignalSetSchema.safeParse({ ...validSignalSet, ...invalid }).success,
  };
}

function risk(kind, severity, confidence) {
  return {
    signalType: 'risk',
    kind,
    severity,
    confidence,
    state: 'candidate',
  };
}

function benefit(kind, severity, confidence) {
  return {
    signalType: 'benefit',
    kind,
    severity,
    confidence,
    state: 'candidate',
  };
}

function unknownRisk(state) {
  return {
    signalType: 'risk',
    kind: 'unknown-risk',
    severity: 'unknown',
    confidence: 'unknown',
    state,
  };
}

function unknownBenefit(state) {
  return {
    signalType: 'benefit',
    kind: 'unknown-benefit',
    severity: 'unknown',
    confidence: 'unknown',
    state,
  };
}

function targetFor(targetId) {
  const target = targets.find((entry) => entry.targetId === targetId);
  if (!target) {
    throw new Error(`Unknown target ${targetId}`);
  }
  return target;
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
