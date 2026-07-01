import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SocialDecisionMemoryCacheSnapshotSchema } from '../../packages/schema-domain/dist/social-decision-memory-cache.js';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const social12ProofPath = join(repoRoot, 'test-results/social-policy-live-evidence-compiler-proof/proof.json');
const outputDirectory = join(
  repoRoot,
  'output/browser-plan-proof/social-19-memory-cache-account-video-channel-decisions'
);
const outputProofPath = join(outputDirectory, '11-live-evidence-decision-memory-proof.json');
const testResultPath = join(repoRoot, 'test-results/social-decision-memory-live-evidence-proof/proof.json');
const observedAt = new Date().toISOString();

const builtFiles = [
  'packages/schema-domain/dist/social-decision-memory-cache-values.js',
  'packages/schema-domain/dist/social-decision-memory-cache.js',
];

assertBuiltContractsExist();
if (!existsSync(social12ProofPath)) {
  throw new Error(`Missing SOCIAL-12 live-evidence compiler proof: ${relativePath(social12ProofPath)}`);
}

const social12Proof = JSON.parse(readFileSync(social12ProofPath, 'utf8'));
if (social12Proof.policyCandidates.length < 3) {
  throw new Error(`Expected at least 3 SOCIAL-12 policy candidates, received ${social12Proof.policyCandidates.length}`);
}
if (social12Proof.liveEvidenceBoundary.finalPolicyDecisionClaimed) {
  throw new Error('SOCIAL-12 source proof must not claim final policy decisions');
}

const videoCandidate = firstCandidateFor('social-video');
const feedCandidate = firstCandidateFor('social-feed');
const snapshot = memorySnapshotFor(videoCandidate, feedCandidate);
const parsedSnapshot = SocialDecisionMemoryCacheSnapshotSchema.parse(snapshot);
const negativeChecks = buildNegativeChecks(parsedSnapshot);
if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected all SOCIAL-19 negative checks to reject dishonest memory/cache claims');
}

const proof = {
  schemaVersion: 1,
  proofId: 'social-decision-memory-live-evidence-proof',
  generatedAt: observedAt,
  branch: git(['branch', '--show-current']),
  commit: git(['rev-parse', 'HEAD']),
  baseCommit: git(['rev-parse', 'origin/main']),
  sourceProof: relativePath(social12ProofPath),
  liveEvidenceBoundary: {
    sourcePolicyCompilerUsesLiveSocial11Refs: true,
    sourcePolicyCandidateCount: social12Proof.policyCandidates.length,
    sourcePolicyFinalDecisionClaimed: social12Proof.liveEvidenceBoundary.finalPolicyDecisionClaimed,
    memorySnapshotEntryCount: parsedSnapshot.entries.length,
    accountEntryState: entryFor(parsedSnapshot, 'account-ref').memoryState,
    videoEntryState: entryFor(parsedSnapshot, 'video-ref').memoryState,
    channelEntryState: entryFor(parsedSnapshot, 'channel-ref').memoryState,
    finalPolicyDecisionClaimed: false,
    runtimeCacheStoreClaimed: false,
    aiCacheClaimed: false,
    rawAccountDataStored: false,
    rawVideoContentStored: false,
    rawMessageContentStored: false,
    connectorDataStored: false,
    uiDeliveredClaimed: false,
    nativeAppControlClaimed: false,
    enforcementClaimed: false,
  },
  snapshot: {
    snapshotId: parsedSnapshot.snapshotId,
    capturedAt: parsedSnapshot.capturedAt,
    retentionBounded: parsedSnapshot.retentionBounded,
    rawContentStored: parsedSnapshot.rawContentStored,
    runtimeStoreClaimed: parsedSnapshot.runtimeStoreClaimed,
    entries: parsedSnapshot.entries.map((entry) => ({
      entryId: entry.entryId,
      subjectKind: entry.subjectKind,
      memoryState: entry.memoryState,
      decisionSource: entry.decisionSource,
      actionCandidate: entry.actionCandidate,
      reasonCodes: entry.reasonCodes,
      confidence: entry.confidence,
      ttlClass: entry.ttlClass,
      canReuseForPolicyInput: entry.canReuseForPolicyInput,
      sourceEvidenceRefs: entry.sourceEvidenceRefs,
      decisionRefs: entry.decisionRefs,
      invalidationReasons: entry.invalidationReasons,
      finalPolicyDecisionClaimed: entry.finalPolicyDecisionClaimed,
      runtimeCacheStoreClaimed: entry.runtimeCacheStoreClaimed,
      aiCacheClaimed: entry.aiCacheClaimed,
      rawAccountDataStored: entry.rawAccountDataStored,
      rawVideoContentStored: entry.rawVideoContentStored,
      rawMessageContentStored: entry.rawMessageContentStored,
      connectorDataStored: entry.connectorDataStored,
      uiDeliveredClaimed: entry.uiDeliveredClaimed,
      nativeAppControlClaimed: entry.nativeAppControlClaimed,
      enforcementClaimed: entry.enforcementClaimed,
    })),
  },
  parseChecks: {
    snapshotAccepted: true,
    requiredSubjectsPresent: ['account-ref', 'video-ref', 'channel-ref'].every((subject) =>
      parsedSnapshot.entries.some((entry) => entry.subjectKind === subject)
    ),
  },
  negativeChecks,
  noClaimChecks: {
    runtimeCacheStore: false,
    finalPolicyDecision: false,
    rawContentStorage: false,
    aiCache: false,
    connectorDataStorage: false,
    uiDelivery: false,
    nativeAppControl: false,
    enforcement: false,
  },
};

writeJson(testResultPath, proof);
writeJson(outputProofPath, proof);

console.log('social-decision-memory-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(testResultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`entryCount=${parsedSnapshot.entries.length}`);
console.log(`states=${parsedSnapshot.entries.map((entry) => entry.memoryState).join(',')}`);

function firstCandidateFor(targetKind) {
  const candidate = social12Proof.policyCandidates.find((row) => row.targetKind === targetKind);
  if (!candidate) {
    throw new Error(`Missing SOCIAL-12 policy candidate for ${targetKind}`);
  }
  return candidate;
}

function memorySnapshotFor(videoCandidate, feedCandidate) {
  return {
    schemaVersion: 'social-decision-memory-cache',
    snapshotId: 'social-decision-memory-live-evidence-snapshot',
    familyId: 'family-social-memory',
    childProfileId: 'child-social-memory',
    capturedAt: observedAt,
    entries: [accountEntry(videoCandidate), videoEntry(videoCandidate), channelEntry(feedCandidate)],
    retentionBounded: true,
    rawContentStored: false,
    runtimeStoreClaimed: false,
  };
}

function accountEntry(candidate) {
  return memoryEntry('account-ref', candidate, {
    entryId: 'social-memory-account-ref-live-evidence',
    memoryState: 'miss',
    decisionSource: 'unavailable',
    actionCandidate: 'unknown-candidate',
    reasonCodes: ['unknown-evidence'],
    confidence: 'unknown',
    cacheKeys: [...baseKeys(), cacheKey('social-account-ref', 'account-ref-live-evidence-manual-required')],
    decisionRefs: [],
    invalidationReasons: ['ttl-expired'],
    canReuseForPolicyInput: false,
  });
}

function videoEntry(candidate) {
  return memoryEntry('video-ref', candidate, {
    entryId: 'social-memory-video-ref-live-evidence',
    ttlClass: 'stable-video-decision',
    ttlMs: 86400000,
    actionCandidate: candidate.actionCandidate,
    reasonCodes: candidate.reasonCodes,
    confidence: candidate.confidence,
    cacheKeys: [...baseKeys(), cacheKey('platform-video-ref', candidate.decisionCandidateId)],
    decisionRefs: [candidate.decisionCandidateId],
  });
}

function channelEntry(candidate) {
  return memoryEntry('channel-ref', candidate, {
    entryId: 'social-memory-channel-ref-live-evidence',
    memoryState: 'stale-hit',
    ttlClass: 'channel-decision',
    ttlMs: 43200000,
    actionCandidate: candidate.actionCandidate,
    reasonCodes: candidate.reasonCodes,
    confidence: candidate.confidence,
    cacheKeys: [...baseKeys(), cacheKey('platform-channel-ref', candidate.decisionCandidateId)],
    decisionRefs: [candidate.decisionCandidateId],
    invalidationReasons: ['parent-rule-changed'],
    canReuseForPolicyInput: false,
  });
}

function memoryEntry(subjectKind, candidate, overrides) {
  return {
    schemaVersion: 'social-decision-memory-cache',
    entryId: 'social-memory-entry-live-evidence',
    familyId: 'family-social-memory',
    childProfileId: 'child-social-memory',
    policyVersionRef: 'policy-version-social-live-evidence-proof',
    storedAt: observedAt,
    expiresAt: new Date(Date.parse(observedAt) + 12 * 60 * 60 * 1000).toISOString(),
    ttlMs: 43200000,
    ttlClass: 'account-decision',
    subjectKind,
    memoryState: 'fresh-hit',
    decisionSource: 'parent-decision-candidate',
    actionCandidate: candidate.actionCandidate,
    reasonCodes: candidate.reasonCodes,
    confidence: candidate.confidence,
    cacheKeys: baseKeys(),
    sourceEvidenceRefs: candidate.sourceEvidenceRefs,
    decisionRefs: [candidate.decisionCandidateId],
    invalidationReasons: [],
    canReuseForPolicyInput: true,
    finalPolicyDecisionClaimed: false,
    runtimeCacheStoreClaimed: false,
    aiCacheClaimed: false,
    rawAccountDataStored: false,
    rawVideoContentStored: false,
    rawMessageContentStored: false,
    connectorDataStored: false,
    uiDeliveredClaimed: false,
    nativeAppControlClaimed: false,
    enforcementClaimed: false,
    ...overrides,
  };
}

function baseKeys() {
  return [
    cacheKey('policy-version', 'policy-version-social-live-evidence-proof'),
    cacheKey('child-profile', 'child-social-memory'),
    cacheKey('parent-rule-set', 'parent-rule-set-social-memory'),
  ];
}

function cacheKey(keyKind, keyValue) {
  return { keyKind, keyValue };
}

function entryFor(snapshot, subjectKind) {
  return snapshot.entries.find((entry) => entry.subjectKind === subjectKind);
}

function buildNegativeChecks(validSnapshot) {
  const video = entryFor(validSnapshot, 'video-ref');
  const invalidEntries = [
    ['entry-final-policy', { ...video, finalPolicyDecisionClaimed: true }],
    ['entry-runtime-store', { ...video, runtimeCacheStoreClaimed: true }],
    ['entry-ai-cache', { ...video, aiCacheClaimed: true }],
    ['entry-raw-account', { ...video, rawAccountDataStored: true }],
    ['entry-raw-video', { ...video, rawVideoContentStored: true }],
    ['entry-raw-message', { ...video, rawMessageContentStored: true }],
    ['entry-connector-data', { ...video, connectorDataStored: true }],
    ['entry-ui-delivery', { ...video, uiDeliveredClaimed: true }],
    ['entry-native-app', { ...video, nativeAppControlClaimed: true }],
    ['entry-enforcement', { ...video, enforcementClaimed: true }],
    ['fresh-hit-without-decision-ref', { ...video, decisionRefs: [] }],
  ];
  const invalidSnapshots = [
    ['snapshot-raw-content', { ...validSnapshot, rawContentStored: true }],
    ['snapshot-runtime-store', { ...validSnapshot, runtimeStoreClaimed: true }],
    [
      'snapshot-missing-channel',
      {
        ...validSnapshot,
        entries: validSnapshot.entries.filter((entry) => entry.subjectKind !== 'channel-ref'),
      },
    ],
  ];

  return [
    ...invalidEntries.map(([label, entry]) => ({
      label,
      rejected: !SocialDecisionMemoryCacheSnapshotSchema.safeParse({
        ...validSnapshot,
        entries: validSnapshot.entries.map((candidate) => (candidate.subjectKind === 'video-ref' ? entry : candidate)),
      }).success,
    })),
    ...invalidSnapshots.map(([label, snapshot]) => ({
      label,
      rejected: !SocialDecisionMemoryCacheSnapshotSchema.safeParse(snapshot).success,
    })),
  ];
}

function assertBuiltContractsExist() {
  for (const builtFile of builtFiles) {
    const builtPath = join(repoRoot, builtFile);
    if (!existsSync(builtPath)) {
      throw new Error(`Missing built contract file. Run npm run build:contracts first: ${builtFile}`);
    }
  }
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}
