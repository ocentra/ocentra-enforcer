import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  SocialParentApprovalDecisionSchema,
  SocialParentApprovalRequestSchema,
} from '../../packages/schema-domain/dist/social-parent-approval.js';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, '..', '..');
const proofRoot = join(repoRoot, 'output/browser-plan-proof/social-07-parent-approval-contracts');
const testResultPath = join(repoRoot, 'test-results/social-parent-approval-live-evidence-proof/proof.json');
const outputProofPath = join(proofRoot, '11-live-approval-proof.json');
const observedAt = new Date().toISOString();

const sourceProof = readJson('test-results/social-account-identity-live-evidence-proof/proof.json');

mkdirSync(proofRoot, { recursive: true });
mkdirSync(dirname(testResultPath), { recursive: true });

assertLiveIdentityProof(sourceProof);

const approvalRows = sourceProof.liveIdentityRows.map(liveApprovalRow);
const parseChecks = approvalRows.map((row) => {
  const request = SocialParentApprovalRequestSchema.parse(buildRequest(row));
  const decision = SocialParentApprovalDecisionSchema.parse(buildManualRequiredDecision(row));
  return {
    targetId: row.targetId,
    requestedUrlSha256: row.requestedUrlSha256,
    finalUrlSha256: row.finalUrlSha256,
    subjectKind: request.subjectKind,
    requestState: request.requestState,
    decisionKind: decision.decisionKind,
    decisionState: decision.decisionState,
    deliveryState: request.deliveryState,
    sourceEvidenceRefs: request.sourceEvidenceRefs,
    socialRouteEvidenceRef: request.socialRouteEvidenceRef,
    accountFlowEvidenceRef: request.accountFlowEvidenceRef,
    formShapeEvidenceRef: request.formShapeEvidenceRef,
    accountIdentityRef: request.accountIdentityRef,
    rawMessageCaptured: request.rawMessageCaptured,
    rawAccountIdentityCaptured: request.rawAccountIdentityCaptured,
    credentialCaptured: request.credentialCaptured,
    notificationDeliveredClaimed: request.notificationDeliveredClaimed,
    uiRenderedClaimed: request.uiRenderedClaimed,
    policyDecisionClaimed: request.policyDecisionClaimed,
    enforcementClaimed: request.enforcementClaimed,
    nativeAppControlClaimed: request.nativeAppControlClaimed,
    connectorAuthorizationClaimed: request.connectorAuthorizationClaimed,
    childNotifiedClaimed: decision.childNotifiedClaimed,
    actionRef: decision.actionRef,
    accepted: true,
  };
});

if (!parseChecks.every((check) => check.accepted)) {
  throw new Error('Expected every SOCIAL-07 live approval row to parse as contract-only request/decision evidence');
}

const negativeChecks = [
  rejectsRequestMutation('raw-message-capture-rejected', approvalRows[0], { rawMessageCaptured: true }),
  rejectsRequestMutation('raw-account-identity-capture-rejected', approvalRows[1], {
    rawAccountIdentityCaptured: true,
  }),
  rejectsRequestMutation('credential-capture-rejected', approvalRows[2], { credentialCaptured: true }),
  rejectsRequestMutation('notification-delivery-claim-rejected', approvalRows[3], {
    notificationDeliveredClaimed: true,
  }),
  rejectsRequestMutation('ui-render-claim-rejected', approvalRows[0], { uiRenderedClaimed: true }),
  rejectsRequestMutation('policy-decision-claim-rejected', approvalRows[1], { policyDecisionClaimed: true }),
  rejectsRequestMutation('enforcement-claim-rejected', approvalRows[2], { enforcementClaimed: true }),
  rejectsRequestMutation('native-app-control-claim-rejected', approvalRows[3], { nativeAppControlClaimed: true }),
  rejectsRequestMutation('connector-authorization-claim-rejected', approvalRows[0], {
    connectorAuthorizationClaimed: true,
  }),
  rejectsRequestMutation('missing-account-flow-ref-rejected', approvalRows[1], { accountFlowEvidenceRef: null }),
  rejectsDecisionMutation('decision-notification-delivery-rejected', approvalRows[0], {
    notificationDeliveredClaimed: true,
  }),
  rejectsDecisionMutation('decision-ui-render-rejected', approvalRows[1], { uiRenderedClaimed: true }),
  rejectsDecisionMutation('decision-child-notified-rejected', approvalRows[2], { childNotifiedClaimed: true }),
  rejectsDecisionMutation('decision-policy-claim-rejected', approvalRows[3], { policyDecisionClaimed: true }),
  rejectsDecisionMutation('decision-action-execution-rejected', approvalRows[0], {
    decisionKind: 'allow-once',
    decisionState: 'recorded',
    decidedByActorId: 'parent-actor-not-live-proof',
    actionRef: 'action-ref-not-executed',
  }),
  rejectsDecisionMutation('decision-enforcement-claim-rejected', approvalRows[1], { enforcementClaimed: true }),
  rejectsDecisionMutation('decision-native-control-claim-rejected', approvalRows[2], { nativeAppControlClaimed: true }),
  rejectsDecisionMutation('decision-connector-claim-rejected', approvalRows[3], {
    connectorAuthorizationClaimed: true,
  }),
];

if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error(
    'Expected SOCIAL-07 delivery, UI, policy/action, connector, native, and enforcement checks to reject'
  );
}

const proof = {
  schemaVersion: 1,
  proofId: 'social-parent-approval-live-evidence-proof',
  generatedAt: observedAt,
  branch: git(['branch', '--show-current']),
  commit: git(['rev-parse', 'HEAD']),
  baseCommit: git(['rev-parse', 'origin/main']),
  sourceProof: sourceProofSummary(sourceProof),
  liveEvidenceSummary: {
    realPublicSocialSurfacesUsed: true,
    generatedOrFixturePageUsed: false,
    passiveNavigationOnly: true,
    approvalRows: approvalRows.length,
    contractOnlyRequests: true,
    realParentDecisionClaimed: false,
    runtimeApprovalStoreClaimed: false,
    rawMessageCaptured: false,
    rawAccountIdentityCaptured: false,
    credentialCaptured: false,
    notificationDeliveredClaimed: false,
    uiRenderedClaimed: false,
    childNotifiedClaimed: false,
    policyDecisionClaimed: false,
    actionExecutionClaimed: false,
    enforcementClaimed: false,
    nativeAppControlClaimed: false,
    connectorAuthorizationClaimed: false,
  },
  liveApprovalRows: approvalRows.map(redactedLiveApprovalRow),
  parseChecks,
  negativeChecks,
};

writeJson(testResultPath, proof);
writeJson(outputProofPath, proof);

console.log('social-parent-approval-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(testResultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`rows=${approvalRows.length} negativeChecks=${negativeChecks.length}`);

function liveApprovalRow(row) {
  return {
    targetId: row.targetId,
    requestedUrlSha256: row.requestedUrlSha256,
    finalUrlSha256: row.finalUrlSha256,
    responseStatus: row.responseStatus,
    screenshotPath: row.screenshotPath,
    screenshotSha256: row.screenshotSha256,
    screenshotBytes: row.screenshotBytes,
    sourceEvidenceRefs: row.sourceEvidenceIds,
    approvalRequestId: `social-approval-live-request-${row.targetId}`,
    approvalDecisionId: `social-approval-live-decision-manual-${row.targetId}`,
    socialRouteEvidenceRef: row.socialRouteEvidenceId,
    accountFlowEvidenceRef: row.accountFlowEvidenceId,
    formShapeEvidenceRef: `social-account-live-form-shape-${row.targetId}`,
    accountIdentityRef: row.accountIdentityRef,
    expectedPlatform: row.expectedPlatform,
    expectedAccountFlowKind: row.expectedAccountFlowKind,
    subjectKind: subjectKindFor(row.expectedAccountFlowKind),
  };
}

function buildRequest(row) {
  return {
    schemaVersion: 'v0.6',
    approvalRequestId: row.approvalRequestId,
    familyId: 'family-social-live-proof',
    childProfileId: 'child-social-live-proof',
    requestedByDeviceId: 'managed-browser-device-social-live-proof',
    createdAt: observedAt,
    expiresAt: null,
    subjectKind: row.subjectKind,
    requestState: 'pending',
    sourceEvidenceRefs: row.sourceEvidenceRefs,
    socialRouteEvidenceRef: row.socialRouteEvidenceRef,
    accountFlowEvidenceRef: row.accountFlowEvidenceRef,
    formShapeEvidenceRef: row.formShapeEvidenceRef,
    accountIdentityRef: row.accountIdentityRef,
    deliveryState: 'contract-only',
    rawMessageCaptured: false,
    rawAccountIdentityCaptured: false,
    credentialCaptured: false,
    notificationDeliveredClaimed: false,
    uiRenderedClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
    nativeAppControlClaimed: false,
    connectorAuthorizationClaimed: false,
  };
}

function buildManualRequiredDecision(row) {
  return {
    schemaVersion: 'v0.6',
    approvalDecisionId: row.approvalDecisionId,
    approvalRequestId: row.approvalRequestId,
    familyId: 'family-social-live-proof',
    childProfileId: 'child-social-live-proof',
    decidedAt: observedAt,
    decidedByActorId: null,
    decisionKind: 'manual-required',
    decisionState: 'manual-required',
    sourceEvidenceRefs: row.sourceEvidenceRefs,
    policyVersionRef: null,
    actionRef: null,
    deliveryState: 'contract-only',
    notificationDeliveredClaimed: false,
    uiRenderedClaimed: false,
    childNotifiedClaimed: false,
    policyDecisionClaimed: false,
    enforcementClaimed: false,
    nativeAppControlClaimed: false,
    connectorAuthorizationClaimed: false,
  };
}

function rejectsRequestMutation(label, row, requestPatch) {
  const mutated = SocialParentApprovalRequestSchema.safeParse({
    ...buildRequest(row),
    ...requestPatch,
  });
  return {
    label,
    rejected: !mutated.success,
    reason: mutated.success ? 'accepted' : 'approval-request-schema-rejected',
  };
}

function rejectsDecisionMutation(label, row, decisionPatch) {
  const mutated = SocialParentApprovalDecisionSchema.safeParse({
    ...buildManualRequiredDecision(row),
    ...decisionPatch,
  });
  return {
    label,
    rejected: !mutated.success,
    reason: mutated.success ? 'accepted' : 'approval-decision-schema-rejected',
  };
}

function redactedLiveApprovalRow(row) {
  return {
    targetId: row.targetId,
    requestedUrlSha256: row.requestedUrlSha256,
    finalUrlSha256: row.finalUrlSha256,
    responseStatus: row.responseStatus,
    screenshotPath: row.screenshotPath,
    screenshotSha256: row.screenshotSha256,
    screenshotBytes: row.screenshotBytes,
    sourceEvidenceRefs: row.sourceEvidenceRefs,
    approvalRequestId: row.approvalRequestId,
    approvalDecisionId: row.approvalDecisionId,
    subjectKind: row.subjectKind,
    socialRouteEvidenceRef: row.socialRouteEvidenceRef,
    accountFlowEvidenceRef: row.accountFlowEvidenceRef,
    formShapeEvidenceRef: row.formShapeEvidenceRef,
    accountIdentityRef: row.accountIdentityRef,
    expectedPlatform: row.expectedPlatform,
    expectedAccountFlowKind: row.expectedAccountFlowKind,
  };
}

function subjectKindFor(accountFlowKind) {
  if (accountFlowKind === 'signup-route') {
    return 'social-account-signup';
  }
  if (accountFlowKind === 'login-route') {
    return 'social-login';
  }
  if (accountFlowKind === 'account-switch-route') {
    return 'social-account-switch';
  }
  return 'social-route-manual-required';
}

function assertLiveIdentityProof(proof) {
  if (proof?.proofId !== 'social-account-identity-live-evidence-proof') {
    throw new Error('Expected proofId social-account-identity-live-evidence-proof');
  }
  const summary = proof.liveEvidenceSummary;
  if (!summary?.realPublicSocialSurfacesUsed || summary.generatedOrFixturePageUsed || !summary.passiveNavigationOnly) {
    throw new Error('Expected SOCIAL-06 source proof to be passive real-public-social evidence');
  }
  if (
    summary.rawHandleCaptured ||
    summary.rawDisplayNameCaptured ||
    summary.rawPlatformAccountIdCaptured ||
    summary.credentialCaptured ||
    summary.identityVerifiedByPlatform
  ) {
    throw new Error('SOCIAL-07 cannot use source proof with raw identity, credentials, or platform verification');
  }
  if (summary.policyDecisionClaimed || summary.enforcementClaimed || summary.nativeAppControlClaimed) {
    throw new Error('SOCIAL-07 source proof cannot already claim policy, enforcement, or native control');
  }
}

function sourceProofSummary(proof) {
  return {
    proofId: proof.proofId,
    generatedAt: proof.generatedAt,
    branch: proof.branch,
    commit: proof.commit,
    identityRows: proof.liveEvidenceSummary.identityRows,
    realPublicSocialSurfacesUsed: proof.liveEvidenceSummary.realPublicSocialSurfacesUsed,
    generatedOrFixturePageUsed: proof.liveEvidenceSummary.generatedOrFixturePageUsed,
    passiveNavigationOnly: proof.liveEvidenceSummary.passiveNavigationOnly,
    rawHandleCaptured: proof.liveEvidenceSummary.rawHandleCaptured,
    credentialCaptured: proof.liveEvidenceSummary.credentialCaptured,
    identityVerifiedByPlatform: proof.liveEvidenceSummary.identityVerifiedByPlatform,
  };
}

function readJson(path) {
  return JSON.parse(readFileSync(join(repoRoot, path), 'utf8'));
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}
