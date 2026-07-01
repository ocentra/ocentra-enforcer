import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  BrowserSocialRouteEvidenceSchema,
  BrowserSocialRouteSchemaVersion,
} from '@ocentra-parent/schema-domain/browser-social-platform-route-schemas';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const proofRoot = join(repoRoot, 'output/browser-plan-proof/social-02-platform-route-contracts');
const testResultPath = join(repoRoot, 'test-results/social-platform-route-live-evidence-proof/proof.json');
const outputProofPath = join(proofRoot, '11-live-evidence-route-schema-proof.json');
const observedAt = new Date().toISOString();

const sourceProofs = {
  feedRouteClassification: readJson('test-results/social-feed-route-classification-live-proof/proof.json'),
  accountCreationGate: readJson('test-results/social-account-creation-live-proof/proof.json'),
  unmanagedBypass: readJson('test-results/social-unmanaged-bypass-live-process-proof/proof.json'),
  androidNativeHost: readJson('test-results/social-android-native-app-host-proof/proof.json'),
  iosNativeHost: readJson('test-results/social-ios-screen-time-host-proof/proof.json'),
};

mkdirSync(proofRoot, { recursive: true });
mkdirSync(dirname(testResultPath), { recursive: true });

const managedRows = [
  ...managedFeedRouteRows(sourceProofs.feedRouteClassification),
  ...managedAccountRouteRows(sourceProofs.accountCreationGate),
];
const unmanagedRows = unmanagedBypassRows(sourceProofs.unmanagedBypass);
const nativeRows = nativeManualRequiredRows(sourceProofs.androidNativeHost, sourceProofs.iosNativeHost);
const routeRows = [...managedRows, ...unmanagedRows, ...nativeRows];

const parseChecks = routeRows.map((row) => ({
  socialRouteEvidenceId: row.socialRouteEvidenceId,
  sourceKind: row.sourceKind,
  proofState: row.proofState,
  platform: row.platform,
  routeKind: row.routeKind,
  accepted: BrowserSocialRouteEvidenceSchema.safeParse(row).success,
}));

if (!parseChecks.every((check) => check.accepted)) {
  throw new Error('Expected all SOCIAL-02 live-evidence route rows to parse through BrowserSocialRouteEvidenceSchema');
}

const negativeChecks = routeRows.flatMap((row) => [
  rejects(`${row.socialRouteEvidenceId}:account-identity-claim`, { ...row, accountIdentityClaimed: true }),
  rejects(`${row.socialRouteEvidenceId}:message-content-claim`, { ...row, messageContentClaimed: true }),
  rejects(`${row.socialRouteEvidenceId}:feed-content-semantics-claim`, { ...row, feedContentSemanticsClaimed: true }),
  rejects(`${row.socialRouteEvidenceId}:policy-decision-claim`, { ...row, policyDecisionClaimed: true }),
  rejects(`${row.socialRouteEvidenceId}:enforcement-claim`, { ...row, enforcementClaimed: true }),
]);

negativeChecks.push(
  rejects('unmanaged-bypass-promoted-to-managed-route', {
    ...unmanagedRows[0],
    sourceKind: 'managed-browser-url-shape',
    proofState: 'route-evidence',
    routeKind: 'feed',
    exactManagedBrowserRouteEvidence: true,
    unmanagedBypassOnly: false,
    urlShapeClassificationId: 'dishonest-unmanaged-url-shape-promotion',
    urlShapeTargetKind: 'social-feed',
  }),
  rejects('native-manual-required-promoted-to-route-evidence', {
    ...nativeRows[0],
    sourceKind: 'managed-browser-url-shape',
    proofState: 'route-evidence',
    routeKind: 'video',
    exactManagedBrowserRouteEvidence: true,
    urlShapeClassificationId: 'dishonest-native-url-shape-promotion',
    urlShapeTargetKind: 'video',
  })
);

if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected SOCIAL-02 dishonest authority and source-promotion checks to reject');
}

const proof = {
  schemaVersion: 1,
  proofId: 'social-platform-route-live-evidence-proof',
  generatedAt: observedAt,
  branch: git(['branch', '--show-current']),
  commit: git(['rev-parse', 'HEAD']),
  baseCommit: git(['rev-parse', 'origin/main']),
  sourceProofs: {
    feedRouteClassification: sourceProofSummary(sourceProofs.feedRouteClassification),
    accountCreationGate: sourceProofSummary(sourceProofs.accountCreationGate),
    unmanagedBypass: sourceProofSummary(sourceProofs.unmanagedBypass),
    androidNativeHost: sourceProofSummary(sourceProofs.androidNativeHost),
    iosNativeHost: sourceProofSummary(sourceProofs.iosNativeHost),
  },
  liveEvidenceSummary: {
    realPublicSocialSurfacesUsed: true,
    managedRouteRows: managedRows.length,
    unmanagedBypassRows: unmanagedRows.length,
    nativeManualRequiredRows: nativeRows.length,
    totalRouteRows: routeRows.length,
    generatedOrFixturePageUsed: false,
    rawPageBodyPersisted: false,
    rawDomPersisted: false,
    rawTitlePersisted: false,
    rawMessageOrFeedContentPersisted: false,
    accountIdentityClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
  },
  routeRows: routeRows.map(redactedRouteRowForProof),
  parseChecks,
  negativeChecks,
};

writeJson(testResultPath, proof);
writeJson(outputProofPath, proof);

console.log('social-platform-route-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(testResultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`managed=${managedRows.length} unmanaged=${unmanagedRows.length} native=${nativeRows.length}`);

function managedFeedRouteRows(proof) {
  assertLiveCaptureProof(proof, 'social-feed-route-classification-live-proof');
  return proof.captures
    .filter((capture) => capture.contractClassificationCreated && capture.classificationSummary !== undefined)
    .map((capture) => {
      const summary = capture.classificationSummary;
      return managedRouteRow({
        socialRouteEvidenceId: summary.socialRouteEvidenceId,
        sourceEvidenceIds: summary.sourceEvidenceIds,
        urlShapeClassificationId: `${capture.targetId}-live-url-shape-classification`,
        urlShapeTargetKind: feedTargetKind(summary),
        platform: summary.platform,
        routeKind: summary.routeKind,
      });
    });
}

function managedAccountRouteRows(proof) {
  assertLiveCaptureProof(proof, 'social-account-creation-live-proof');
  return proof.captures
    .filter((capture) => capture.contractPlanCreated && capture.planSummary !== undefined)
    .map((capture) => {
      const summary = capture.planSummary;
      return managedRouteRow({
        socialRouteEvidenceId: summary.socialRouteEvidenceId,
        sourceEvidenceIds: summary.sourceEvidenceIds,
        urlShapeClassificationId: `${capture.targetId}-account-live-url-shape-classification`,
        urlShapeTargetKind: accountTargetKind(summary.accountFlowKind),
        platform: summary.platform,
        routeKind: accountRouteKind(summary.accountFlowKind),
      });
    });
}

function unmanagedBypassRows(proof) {
  assertProofId(proof, 'social-unmanaged-bypass-live-process-proof');
  if (!proof.liveProcessSummary?.realLocalBrowserProcessObserved) {
    throw new Error('Expected SOCIAL-15 proof to observe a real local browser process');
  }
  if (!proof.liveProcessSummary?.realPublicSocialSurfacesRequested) {
    throw new Error('Expected SOCIAL-15 proof to request real public social surfaces');
  }
  return proof.captures
    .filter((capture) => capture.processObserved && capture.evidence !== undefined)
    .slice(0, 2)
    .map((capture) => ({
      ...baseRouteRow(),
      socialRouteEvidenceId: `social-route-${capture.evidence.bypassEvidenceId}`,
      observedAt: capture.evidence.observedAt,
      sourceEvidenceIds: capture.evidence.sourceEvidenceIds,
      sourceKind: 'unmanaged-browser-bypass',
      proofState: 'bypass-only',
      platform: 'unknown-social',
      routeKind: 'unknown-social-route',
      unmanagedBypassOnly: true,
      manualRequired: true,
    }));
}

function nativeManualRequiredRows(androidProof, iosProof) {
  assertProofId(androidProof, 'social-android-native-app-host-proof');
  assertProofId(iosProof, 'social-ios-screen-time-host-proof');
  return [
    {
      ...baseRouteRow(),
      socialRouteEvidenceId: 'social-route-android-native-manual-required-host-proof',
      observedAt: androidProof.generatedAt,
      sourceEvidenceIds: ['parent-proof-social-android-native-host-device'],
      sourceKind: 'native-app-manual-required',
      proofState: 'manual-required',
      platform: 'tiktok',
      routeKind: 'unknown-social-route',
      manualRequired: true,
    },
    {
      ...baseRouteRow(),
      socialRouteEvidenceId: 'social-route-ios-native-manual-required-host-proof',
      observedAt: iosProof.generatedAt,
      sourceEvidenceIds: ['parent-proof-social-ios-screen-time-host'],
      sourceKind: 'native-app-manual-required',
      proofState: 'manual-required',
      platform: 'instagram',
      routeKind: 'unknown-social-route',
      manualRequired: true,
    },
  ];
}

function managedRouteRow({
  socialRouteEvidenceId,
  sourceEvidenceIds,
  urlShapeClassificationId,
  urlShapeTargetKind,
  platform,
  routeKind,
}) {
  return {
    ...baseRouteRow(),
    socialRouteEvidenceId,
    observedAt,
    sourceEvidenceIds,
    urlShapeClassificationId,
    urlShapeTargetKind,
    sourceKind: 'managed-browser-url-shape',
    proofState: 'route-evidence',
    platform,
    routeKind,
    exactManagedBrowserRouteEvidence: true,
  };
}

function baseRouteRow() {
  return {
    schemaVersion: BrowserSocialRouteSchemaVersion,
    socialRouteEvidenceId: 'social-route-evidence',
    observedAt,
    sourceEvidenceIds: ['social-route-source-evidence'],
    urlShapeClassificationId: null,
    urlShapeTargetKind: null,
    sourceKind: 'managed-browser-url-shape',
    proofState: 'route-evidence',
    platform: 'unknown-social',
    routeKind: 'unknown-social-route',
    platformAccountRef: null,
    parentApprovalRequestRef: null,
    exactManagedBrowserRouteEvidence: false,
    unmanagedBypassOnly: false,
    manualRequired: false,
    accountIdentityClaimed: false,
    messageContentClaimed: false,
    feedContentSemanticsClaimed: false,
    aiDecisionClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
    nativeAppControlClaimed: false,
    platformConnectorClaimed: false,
  };
}

function feedTargetKind(summary) {
  if (summary.surfaceKind === 'single-short-video') {
    return 'short-video';
  }
  if (summary.routeKind === 'video') {
    return 'video';
  }
  return 'social-feed';
}

function accountTargetKind(accountFlowKind) {
  return accountFlowKind === 'login-route' ? 'unknown' : 'social-upload-post';
}

function accountRouteKind(accountFlowKind) {
  return accountFlowKind === 'login-route' ? 'login' : 'account-signup';
}

function rejects(name, row) {
  return {
    name,
    rejected: !BrowserSocialRouteEvidenceSchema.safeParse(row).success,
  };
}

function redactedRouteRowForProof(row) {
  return {
    socialRouteEvidenceId: row.socialRouteEvidenceId,
    sourceEvidenceIds: row.sourceEvidenceIds,
    sourceKind: row.sourceKind,
    proofState: row.proofState,
    platform: row.platform,
    routeKind: row.routeKind,
    urlShapeTargetKind: row.urlShapeTargetKind,
    exactManagedBrowserRouteEvidence: row.exactManagedBrowserRouteEvidence,
    unmanagedBypassOnly: row.unmanagedBypassOnly,
    manualRequired: row.manualRequired,
    accountIdentityClaimed: row.accountIdentityClaimed,
    messageContentClaimed: row.messageContentClaimed,
    feedContentSemanticsClaimed: row.feedContentSemanticsClaimed,
    aiDecisionClaimed: row.aiDecisionClaimed,
    policyDecisionClaimed: row.policyDecisionClaimed,
    enforcementClaimed: row.enforcementClaimed,
    nativeAppControlClaimed: row.nativeAppControlClaimed,
    platformConnectorClaimed: row.platformConnectorClaimed,
  };
}

function sourceProofSummary(proof) {
  return {
    proofId: proof.proofId,
    generatedAt: proof.generatedAt,
    commit: proof.commit,
    baseCommit: proof.baseCommit,
  };
}

function assertLiveCaptureProof(proof, proofId) {
  assertProofId(proof, proofId);
  if (!proof.liveCaptureSummary?.realPublicSocialSurfacesUsed) {
    throw new Error(`Expected ${proofId} to cite real public social surfaces`);
  }
  if (proof.liveCaptureSummary.generatedOrFixturePageUsed) {
    throw new Error(`Expected ${proofId} to avoid generated or fixture pages`);
  }
}

function assertProofId(proof, proofId) {
  if (proof.proofId !== proofId) {
    throw new Error(`Expected proof id ${proofId}`);
  }
}

function readJson(path) {
  const absolutePath = join(repoRoot, path);
  if (!existsSync(absolutePath)) {
    throw new Error(`Missing proof input: ${path}`);
  }
  return JSON.parse(readFileSync(absolutePath, 'utf8'));
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}
