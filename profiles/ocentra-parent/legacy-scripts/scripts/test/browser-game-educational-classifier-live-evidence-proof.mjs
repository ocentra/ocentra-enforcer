import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import {
  BrowserGameEducationalClassifierResultSchema,
  BrowserGameEducationalEvidenceRowSchema,
} from '@ocentra-parent/schema-domain/browser-game-educational-classifier';

const repoRoot = process.cwd();
const proofId = 'browser-game-educational-classifier-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-09-educational-game-classifier-contract',
  '02-live-educational-classifier-shape-proof.json'
);

const targets = [
  {
    targetId: 'code-org-minecraft',
    url: 'https://code.org/minecraft',
    category: 'coding',
    outcome: 'educational-candidate',
    recommendedGate: 'allow-during-homework-candidate',
    evidenceKinds: ['school-platform', 'subject-metadata'],
    verifiedKinds: ['school-platform'],
    confidence: 'high',
  },
  {
    targetId: 'chess-play-online',
    url: 'https://www.chess.com/play/online',
    category: 'chess-logic',
    outcome: 'educational-candidate',
    recommendedGate: 'allow-with-time-limit-candidate',
    evidenceKinds: ['parent-allowlist', 'subject-metadata'],
    verifiedKinds: ['parent-allowlist'],
    confidence: 'high',
  },
  {
    targetId: 'coolmath-run-3',
    url: 'https://www.coolmathgames.com/0-run-3',
    category: 'unknown-educational-category',
    outcome: 'misleading-educational-claim',
    recommendedGate: 'block-portal-candidate',
    evidenceKinds: ['platform-self-label', 'domain-reputation'],
    verifiedKinds: [],
    confidence: 'medium',
  },
  {
    targetId: 'poki-subway-surfers',
    url: 'https://poki.com/en/g/subway-surfers',
    category: 'problem-solving',
    outcome: 'entertainment-candidate',
    recommendedGate: 'parent-review-candidate',
    evidenceKinds: ['domain-reputation', 'page-metadata'],
    verifiedKinds: [],
    confidence: 'medium',
  },
  {
    targetId: 'xbox-cloud-play',
    url: 'https://www.xbox.com/en-US/play',
    category: 'unknown-educational-category',
    outcome: 'manual-required',
    recommendedGate: 'manual-review-candidate',
    evidenceKinds: ['manual-required'],
    verifiedKinds: [],
    confidence: 'low',
    degradedState: 'manual-required',
    uncertaintyReasons: ['manual-required', 'missing-school-source'],
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const captures = await Promise.all(targets.map(captureTarget));
const evidenceRows = captures.flatMap((capture) => evidenceRowsFor(capture));
const classifierResults = captures.map(classifierResultFor);
const negativeChecks = runNegativeChecks(evidenceRows[0], classifierResults[0]);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all educational classifier public captures to return HTTP 2xx/3xx responses');
}
if (!evidenceRows.every((row) => BrowserGameEducationalEvidenceRowSchema.safeParse(row).success)) {
  throw new Error('Expected every educational classifier evidence row to parse');
}
if (!classifierResults.every((result) => BrowserGameEducationalClassifierResultSchema.safeParse(result).success)) {
  throw new Error('Expected every educational classifier result to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected educational classifier negative checks to reject overclaims');
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-educational-classifier-shapes',
  targets: captures,
  evidenceRows,
  classifierResults,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    evidenceRows: evidenceRows.length,
    classifierResults: classifierResults.length,
    negativeChecks: negativeChecks.length,
    educationalCandidates: classifierResults.filter((result) => result.outcome === 'educational-candidate').length,
    entertainmentCandidates: classifierResults.filter((result) => result.outcome === 'entertainment-candidate').length,
    misleadingEducationalClaims: classifierResults.filter((result) => result.outcome === 'misleading-educational-claim')
      .length,
    manualRequired: classifierResults.filter((result) => result.outcome === 'manual-required').length,
    rawPageBodyUsed: false,
    rawGamePayloadUsed: false,
    rawModelTextUsed: false,
    platformLabelTreatedAsAuthority: false,
    accountOrPurchaseExecutionClaimed: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    uiRenderedClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-educational-classifier-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(
  `targets=${captures.length} evidenceRows=${evidenceRows.length} classifierResults=${classifierResults.length} negativeChecks=${negativeChecks.length}`
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
    category: target.category,
    outcome: target.outcome,
    recommendedGate: target.recommendedGate,
    evidenceKinds: target.evidenceKinds,
    rawPageBodyPersisted: false,
    rawGamePayloadPersisted: false,
    rawModelTextPersisted: false,
    platformLabelAuthorityPersisted: false,
    policyDecisionPersisted: false,
  };
}

function evidenceRowsFor(capture) {
  const target = targetFor(capture.targetId);
  return target.evidenceKinds.map((evidenceKind) =>
    evidenceRow({
      evidenceRowId: `educational-evidence-${capture.targetId}-${evidenceKind}`,
      evidenceKind,
      evidenceRefs: [`parent-proof-${proofId}-${capture.targetId}-${evidenceKind}`],
      confidence: confidenceForEvidence(target, evidenceKind),
      schoolOrParentVerified: target.verifiedKinds.includes(evidenceKind),
      platformSelfLabelOnly: evidenceKind === 'platform-self-label',
    })
  );
}

function classifierResultFor(capture) {
  const target = targetFor(capture.targetId);
  const rows = evidenceRowsFor(capture);
  const degradedState = target.degradedState ?? 'none';
  return {
    schemaVersion: 'browser-game-educational-classifier-contract',
    classifierResultId: `educational-classifier-result-${capture.targetId}`,
    familyId: 'family-browser-game-live-education-proof',
    childProfileId: 'child-browser-game-live-education-proof',
    deviceId: 'device-browser-game-live-education-proof',
    classifiedAt: startedAt,
    sourceEvidenceRefs: rows.flatMap((row) => row.evidenceRefs),
    evidenceRows: rows,
    category: target.category,
    outcome: target.outcome,
    confidence: target.confidence,
    recommendedGate: target.recommendedGate,
    degradedState,
    uncertaintyReasons: target.uncertaintyReasons ?? [],
    homeworkContextRef:
      target.outcome === 'educational-candidate' ? `parent-proof-${proofId}-${capture.targetId}-homework` : null,
    parentAllowlistRef: target.verifiedKinds.includes('parent-allowlist')
      ? `parent-proof-${proofId}-${capture.targetId}-parent-allowlist`
      : null,
    schoolSourceRef: target.verifiedKinds.includes('school-platform')
      ? `parent-proof-${proofId}-${capture.targetId}-school-source`
      : null,
    aiAnalysisRef: null,
    metadataRef: `parent-proof-${proofId}-${capture.targetId}-metadata-shape`,
    rawPageBodyUsed: false,
    rawGamePayloadUsed: false,
    rawModelTextUsed: false,
    platformLabelTreatedAsAuthority: false,
    finalPolicyDecisionClaimed: false,
    runtimeGateExecutedClaimed: false,
    uiRenderedClaimed: false,
    accountOrPurchaseExecutionClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    enforcementClaimed: false,
  };
}

function evidenceRow(overrides = {}) {
  return {
    evidenceRowId: 'educational-evidence-live-proof',
    evidenceKind: 'school-provided-url',
    evidenceRefs: ['educational-evidence-live-proof-ref'],
    confidence: 'medium',
    schoolOrParentVerified: false,
    platformSelfLabelOnly: false,
    rawPageBodyUsed: false,
    rawGamePayloadUsed: false,
    rawModelTextUsed: false,
    accountOrPurchaseExecutionClaimed: false,
    nativeGameControlClaimed: false,
    cloudFrameAnalysisClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
    ...overrides,
  };
}

function confidenceForEvidence(target, evidenceKind) {
  if (evidenceKind === 'manual-required') {
    return 'unknown';
  }
  if (evidenceKind === 'platform-self-label') {
    return 'low';
  }
  return target.confidence === 'high' ? 'high' : 'medium';
}

function runNegativeChecks(validRow, validResult) {
  const invalidRowClaims = [
    ['row-raw-page-body', { rawPageBodyUsed: true }],
    ['row-raw-game-payload', { rawGamePayloadUsed: true }],
    ['row-raw-model-text', { rawModelTextUsed: true }],
    ['row-account-purchase-execution', { accountOrPurchaseExecutionClaimed: true }],
    ['row-native-game-control', { nativeGameControlClaimed: true }],
    ['row-cloud-frame-analysis', { cloudFrameAnalysisClaimed: true }],
    ['row-policy-decision', { policyDecisionClaimed: true }],
    ['row-enforcement', { enforcementClaimed: true }],
    [
      'row-platform-label-high-authority',
      {
        evidenceKind: 'platform-self-label',
        confidence: 'high',
        platformSelfLabelOnly: true,
        schoolOrParentVerified: true,
      },
    ],
  ];
  const invalidResultClaims = [
    ['result-raw-page-body', { rawPageBodyUsed: true }],
    ['result-raw-game-payload', { rawGamePayloadUsed: true }],
    ['result-raw-model-text', { rawModelTextUsed: true }],
    ['result-platform-label-authority', { platformLabelTreatedAsAuthority: true }],
    ['result-final-policy-decision', { finalPolicyDecisionClaimed: true }],
    ['result-runtime-gate', { runtimeGateExecutedClaimed: true }],
    ['result-ui-rendered', { uiRenderedClaimed: true }],
    ['result-account-purchase-execution', { accountOrPurchaseExecutionClaimed: true }],
    ['result-native-game-control', { nativeGameControlClaimed: true }],
    ['result-cloud-frame-analysis', { cloudFrameAnalysisClaimed: true }],
    ['result-enforcement', { enforcementClaimed: true }],
    ['result-empty-evidence', { evidenceRows: [] }],
    ['result-educational-unknown-category', { category: 'unknown-educational-category' }],
    [
      'result-degraded-high-confidence',
      {
        degradedState: 'degraded',
        confidence: 'high',
        outcome: 'unknown-candidate',
        recommendedGate: 'manual-review-candidate',
        uncertaintyReasons: ['low-confidence'],
      },
    ],
  ];
  return [
    ...invalidRowClaims.map(([name, invalid]) => negativeRowCheck(name, validRow, invalid)),
    ...invalidResultClaims.map(([name, invalid]) => negativeResultCheck(name, validResult, invalid)),
  ];
}

function negativeRowCheck(name, validRow, invalid) {
  return {
    name,
    rejected: !BrowserGameEducationalEvidenceRowSchema.safeParse({ ...validRow, ...invalid }).success,
  };
}

function negativeResultCheck(name, validResult, invalid) {
  return {
    name,
    rejected: !BrowserGameEducationalClassifierResultSchema.safeParse({ ...validResult, ...invalid }).success,
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
