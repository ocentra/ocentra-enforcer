import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const repoRoot = process.cwd();
const outputRoot = join(repoRoot, 'output', 'ai-plan-proof', 'local-ai-plan-closure-audit');
const proofPath = join(outputRoot, 'proof-summary.json');
const checklistPath = join(repoRoot, 'docs', 'plans', 'ai-plan', 'implementation-checklist.md');
const featureDocPath = join(repoRoot, 'docs', 'features', 'local-ai-safety-evaluator.md');

const checklist = readText(checklistPath);
const featureDoc = readText(featureDocPath);
const localVlm = readProof('output/ai-plan-proof/real-analysis/proof-summary.json');
const liveOperatorGate = readProof('output/screen-ai-pipeline-proof/live-operator-artifact-gate/proof-summary.json');
const serviceWinRtOcr = readProof('output/screen-ai-pipeline-proof/service-winrt-ocr/proof-summary.json');
const storedEvidenceContext = readProof('output/ai-plan-proof/local-ai-stored-evidence-context/proof-summary.json');
const storedEvidenceIntegration = readProof(
  'output/ai-plan-proof/local-ai-stored-evidence-integration-proof/proof-summary.json'
);
const deterministicClassifier = readProof(
  'output/ai-plan-proof/local-ai-deterministic-classifier-proof/proof-summary.json'
);
const textInference = readProof('output/ai-plan-proof/local-ai-text-inference-dry-run/proof-summary.json');
const resultJournal = readProof('output/ai-plan-proof/local-ai-result-journal-sqlite-proof/proof-summary.json');
const recentMemory = readProof('output/ai-plan-proof/local-ai-recent-memory-window-proof/proof-summary.json');
const graphReference = readProof('output/ai-plan-proof/local-ai-graph-reference-contract-proof/proof-summary.json');
const contractCompleteness = readProof('output/ai-plan-proof/local-ai-contract-completeness-proof/proof-summary.json');
const parentRuleContext = readProof(
  'output/ai-plan-proof/local-ai-parent-rule-context-builder-proof/proof-summary.json'
);
const providerScheduler = readProof('output/ai-plan-proof/local-ai-provider-scheduler-proof/proof.json');
const runtimeProvider = readProof('output/ai-plan-proof/local-ai-runtime-provider-proof/proof.json');
const householdRouteSelection = readProof(
  'output/ai-plan-proof/household-ai-provider-route-selection-proof/proof-summary.json'
);
const householdAdvertisementHeartbeat = readProof(
  'output/ai-plan-proof/household-ai-provider-advertisement-heartbeat-proof/proof-summary.json'
);
const householdClaimLease = readProof(
  'output/ai-plan-proof/household-ai-provider-claim-lease-proof/proof-summary.json'
);
const noRawScreenTransferMesh = readProof('output/ai-plan-proof/no-raw-screen-transfer-mesh/proof-summary.json');
const householdProviderResultValidation = readProof(
  'output/ai-plan-proof/household-ai-provider-result-validation/proof-summary.json'
);
const householdMeshEventBridge = readProof('output/ai-plan-proof/household-mesh-event-bridge-proof/proof-summary.json');
const childAgentPolicyAuthority = readProof(
  'output/ai-plan-proof/child-agent-ai-policy-authority-proof/proof-summary.json'
);
const mobileDormantProvider = readProof('output/ai-plan-proof/mobile-dormant-ai-provider-proof/proof-summary.json');
const policyConsumption = readProof(
  'output/ai-plan-proof/local-ai-policy-enforcement-consumption-proof/proof-summary.json'
);

const completeRows = [...checklist.matchAll(/\| \[x\]\s+\|\s+([^|]+?)\s+\|/g)].map((match) => match[1].trim());
const partialRows = [...checklist.matchAll(/\| \[~\]\s+\|\s+([^|]+?)\s+\|/g)].map((match) => match[1].trim());
const openRows = [...checklist.matchAll(/\| \[ \]\s+\|\s+([^|]+?)\s+\|/g)].map((match) => match[1].trim());
const openChecklistItems = [...checklist.matchAll(/^- \[ \]\s+(.+)$/gm)].map((match) => match[1].trim());
const expectedOpenChecklistItems = [
  'Final product-complete pipeline proof is deferred to `docs/plans/screen-ai-pipeline-plan` after screen and AI prerequisites are merged or explicitly stacked.',
];
const staleMeshFollowUpText = 'Provider advertisement, heartbeat, and capability contracts remain follow-up work.';
const staleMeshSnapshotText =
  'Provider advertisement/capability contracts and physical household LAN product readiness remain planned.';
const staleFeatureEventingText = 'degraded-result events, and live service consumers remain planned.';
const staleFeatureMeshPlanText = 'These are planned gaps, not completed product claims.';

assert(localVlm.analyzedByRealLocalVlm === true, 'local VLM proof must analyze captured screens.');
assert(localVlm.schemaValidated === true, 'local VLM proof must schema-validate screen evidence.');
assert(localVlm.localAiSafetyResultValidated === true, 'local VLM proof must validate local AI safety results.');
assert(localVlm.policyDecisionValidated === true, 'local VLM proof must validate policy dry-run decisions.');
assert(localVlm.rawImagesDeletedAfterAnalysis === true, 'local VLM proof must delete raw images after analysis.');
assert(localVlm.scenarioCount >= 16, 'local VLM proof must retain the controlled analysis matrix.');

assert(liveOperatorGate.validatedScenarioCount === 9, 'live operator gate must validate all required scenarios.');
assert(liveOperatorGate.realLiveUrlRows >= 7, 'live operator gate must retain live external URL rows.');
assert(liveOperatorGate.localVlmRows >= 8, 'live operator gate must retain local VLM rows.');
assert(liveOperatorGate.policyDryRunRows >= 8, 'live operator gate must retain policy dry-run rows.');
assert(liveOperatorGate.rawImagesDeletedAfterAnalysis === true, 'live operator gate must prove raw deletion.');
assert(liveOperatorGate.productCompleteClaimed === false, 'live operator gate must not claim product completion.');

assert(serviceWinRtOcr.analysisRow?.providerKind === 'localOcr', 'service OCR proof must use local OCR.');
assert(serviceWinRtOcr.analysisRow?.modelId === 'windows-winrt-ocr', 'service OCR proof must name WinRT OCR.');
assert(serviceWinRtOcr.analysisRow?.imageDeletionState === 'deleted', 'service OCR proof must delete image material.');
assert(serviceWinRtOcr.analysisRow?.rawImageRetained === false, 'service OCR row must not retain raw image.');
assert(serviceWinRtOcr.assertions?.policyConsumedOcrResult === true, 'service OCR proof must feed policy dry-run.');
assert(serviceWinRtOcr.assertions?.adapterTemporaryImageDeleted === true, 'service OCR temp image must be deleted.');

assert(storedEvidenceContext.summary?.readyRows === 1, 'stored context must have one ready row.');
assert(storedEvidenceContext.summary?.remoteAiUsed === false, 'stored context must not use remote AI.');
assert(storedEvidenceContext.summary?.rawImagesRetained === false, 'stored context must not retain raw images.');
assert(
  storedEvidenceIntegration.assertions?.allStoredEvidenceRefsReachEvaluationInput === true,
  'stored evidence refs must reach evaluation input.'
);
assert(
  storedEvidenceIntegration.assertions?.dryRunConsumesStoredEvidenceRefs === true,
  'stored dry-run must consume refs.'
);
assert(
  storedEvidenceIntegration.assertions?.noRemoteOrPolicyAuthority === true,
  'stored integration must stay local-only.'
);

assert(deterministicClassifier.summary?.failures === 0, 'deterministic classifier has failures.');
assert(deterministicClassifier.summary?.modelExecuted === false, 'deterministic classifier must not execute a model.');
assert(deterministicClassifier.summary?.remoteAiUsed === false, 'deterministic classifier must not use remote AI.');
assert(textInference.assertions?.localRuntimePreserved === true, 'text inference must preserve local runtime.');
assert(textInference.assertions?.noRemoteApiClaim === true, 'text inference must not claim remote API AI.');
assert(resultJournal.assertions?.readyResultJournaledAndIngested === true, 'journal ingest proof missing ready row.');
assert(resultJournal.assertions?.noRawPromptRetention === true, 'journal proof must reject raw prompt retention.');
assert(
  recentMemory.assertions?.sourceGroundedRecentActivitySelected === true,
  'recent memory must select source-grounded recent evidence.'
);
assert(recentMemory.assertions?.ungroundedMemoryOmitted === true, 'recent memory must omit ungrounded memory.');
assert(graphReference.summary?.selectedGraphReferenceCount === 1, 'graph proof must select one graph reference.');
assert(graphReference.claimBoundaries?.remoteAiUsed === false, 'graph proof must not use remote AI.');
assert(graphReference.claimBoundaries?.rawEvidenceRetained === false, 'graph proof must not retain raw evidence.');
assert(contractCompleteness.assertions?.allContractKindsPresent === true, 'contract completeness proof is not closed.');
assert(contractCompleteness.assertions?.memoryAndGraphCited === true, 'contract completeness must cite memory/graph.');
assert(parentRuleContext.assertions?.ungroundedRuleRejected === true, 'ungrounded parent rules must be rejected.');
assert(parentRuleContext.assertions?.ungroundedRuleDegrades === true, 'ungrounded parent rules must degrade.');
assert(
  providerScheduler.claimsProven?.includes('no-duplicate-local-model-load-for-same-physical-device'),
  'provider scheduler must block duplicate model load.'
);
assert(runtimeProvider.counts?.duplicateRuntimeBlocked > 0, 'runtime provider must block duplicate runtime work.');
assert(runtimeProvider.counts?.maxRuntimeAccessLaneCount === 1, 'runtime provider must keep one runtime lane.');
assert(
  policyConsumption.assertions?.actionDispatchConsumesPolicyDecision === true,
  'policy consumption guard missing.'
);
assert(
  policyConsumption.assertions?.localAiResultLinkedOnlyThroughPolicy === true,
  'AI result must link through policy.'
);
assert(
  householdAdvertisementHeartbeat.assertions?.freshTrustedProviderEligible === true,
  'household provider advertisement proof must retain one fresh eligible provider.'
);
assert(
  householdAdvertisementHeartbeat.assertions?.staleProviderRejected === true,
  'household provider advertisement proof must reject stale providers.'
);
assert(
  householdAdvertisementHeartbeat.assertions?.offlineProviderRejected === true,
  'household provider advertisement proof must reject offline providers.'
);
assert(
  householdAdvertisementHeartbeat.assertions?.revokedProviderRejected === true,
  'household provider advertisement proof must reject revoked providers.'
);
assert(
  householdAdvertisementHeartbeat.assertions?.unsupportedProviderRejected === true,
  'household provider advertisement proof must reject unsupported providers.'
);
assert(
  householdAdvertisementHeartbeat.assertions?.noRuntimePolicyEnforcementOrRawTransferClaims === true,
  'household provider advertisement proof must not overclaim runtime/policy/enforcement/raw transfer.'
);
assert(
  householdRouteSelection.routePriority?.includes('desktop-preferred') === true,
  'household route selection must prefer trusted desktop providers.'
);
assert(
  householdRouteSelection.routePriority?.includes('mobile-dormant') === true,
  'household route selection must preserve mobile dormant fallback order.'
);
assert(
  householdRouteSelection.rejectionCases?.includes('custody-mismatch') === true,
  'household route selection must reject custody mismatch.'
);
assert(
  householdRouteSelection.rejectionCases?.includes('unsupported-capability') === true,
  'household route selection must reject unsupported capabilities.'
);
assert(householdClaimLease.assertions?.oneLeasePerJob === true, 'household claim lease proof must keep one lease.');
assert(
  householdClaimLease.assertions?.duplicateClaimRejected === true,
  'household claim lease proof must reject duplicate claims.'
);
assert(
  householdClaimLease.assertions?.leaseExpiryRequeued === true,
  'household claim lease proof must requeue expired leases.'
);
assert(
  householdClaimLease.assertions?.maxAttemptDeadLettered === true,
  'household claim lease proof must dead-letter after max attempts.'
);
assert(
  householdClaimLease.assertions?.duplicateMessageIdempotent === true,
  'household claim lease proof must ignore duplicate messages idempotently.'
);
assert(
  householdClaimLease.assertions?.noRuntimePolicyEnforcementOrRawTransferClaims === true,
  'household claim lease proof must not overclaim runtime/policy/enforcement/raw transfer.'
);
assert(
  noRawScreenTransferMesh.claimsProved?.some((claim) => claim.includes('not raw screenshot transfer')) === true,
  'no-raw-transfer mesh proof must reject raw screenshot transfer.'
);
assert(
  noRawScreenTransferMesh.claimsNotProved?.includes('physical household LAN execution on a second installed device') ===
    true,
  'no-raw-transfer mesh proof must not claim physical household LAN execution.'
);
assert(
  householdProviderResultValidation.rejectionCases?.includes('raw-image-transfer') === true,
  'provider result validation must reject raw-image transfer.'
);
assert(
  householdProviderResultValidation.rejectionCases?.includes('provider-authority-violation') === true,
  'provider result validation must reject provider authority violation.'
);
assert(
  householdMeshEventBridge.rejectionCases?.includes('raw-screen-payload') === true,
  'household mesh event bridge must reject raw screen payloads.'
);
assert(
  householdMeshEventBridge.rejectionCases?.includes('direct-remote-publish') === true,
  'household mesh event bridge must reject direct remote local-bus publishing.'
);
assert(
  childAgentPolicyAuthority.assertions?.childAgentOwnsPolicyDecision === true,
  'child agent must own AI policy decisions.'
);
assert(
  childAgentPolicyAuthority.assertions?.providerCannotPublishPolicyOrEnforcement === true,
  'provider must not publish policy or enforcement.'
);
assert(
  mobileDormantProvider.mobileClaimsProved?.some((claim) => claim.includes('mobile providers stay dormant')) === true,
  'mobile provider proof must keep mobile dormant while desktop/laptop capacity exists.'
);
assert(
  mobileDormantProvider.mobileClaimsProved?.some((claim) =>
    claim.includes('eligible only for explicit light fallback')
  ) === true,
  'mobile provider proof must allow only explicit light fallback jobs.'
);

assert(partialRows.length === 0, 'AI plan should not have partial table rows at closure audit time.');
assert(openRows.length === 0, `Unexpected AI plan open table rows: ${JSON.stringify(openRows)}`);
assert(
  openChecklistItems.length === expectedOpenChecklistItems.length,
  `Unexpected AI plan open checklist items: ${JSON.stringify(openChecklistItems)}`
);
assert(
  openChecklistItems.every((item) => expectedOpenChecklistItems.includes(item)),
  'AI open checklist item must be the pipeline deferral only.'
);
assert(
  !checklist.includes(staleMeshFollowUpText),
  'AI checklist must not mark household provider advertisement/heartbeat as follow-up after closure proof.'
);
assert(
  !checklist.includes(staleMeshSnapshotText),
  'AI snapshot table must not mark household provider advertisement/capability contracts as planned after proof.'
);
assert(!featureDoc.includes(staleFeatureEventingText), 'local AI feature doc has stale service eventing gap text.');
assert(!featureDoc.includes(staleFeatureMeshPlanText), 'local AI feature doc has stale household mesh proof gap text.');

const summary = {
  proof: 'local-ai-plan-closure-audit-proof',
  generatedAt: new Date().toISOString(),
  checklist: {
    path: relativePath(checklistPath),
    featureDocPath: relativePath(featureDocPath),
    completeCount: completeRows.length,
    partialCount: partialRows.length,
    openCount: openRows.length,
    openRows,
    openChecklistItems,
  },
  sourceArtifacts: {
    localVlm: 'output/ai-plan-proof/real-analysis/proof-summary.json',
    liveOperatorGate: 'output/screen-ai-pipeline-proof/live-operator-artifact-gate/proof-summary.json',
    serviceWinRtOcr: 'output/screen-ai-pipeline-proof/service-winrt-ocr/proof-summary.json',
    storedEvidenceContext: 'output/ai-plan-proof/local-ai-stored-evidence-context/proof-summary.json',
    storedEvidenceIntegration: 'output/ai-plan-proof/local-ai-stored-evidence-integration-proof/proof-summary.json',
    deterministicClassifier: 'output/ai-plan-proof/local-ai-deterministic-classifier-proof/proof-summary.json',
    textInference: 'output/ai-plan-proof/local-ai-text-inference-dry-run/proof-summary.json',
    resultJournal: 'output/ai-plan-proof/local-ai-result-journal-sqlite-proof/proof-summary.json',
    recentMemory: 'output/ai-plan-proof/local-ai-recent-memory-window-proof/proof-summary.json',
    graphReference: 'output/ai-plan-proof/local-ai-graph-reference-contract-proof/proof-summary.json',
    contractCompleteness: 'output/ai-plan-proof/local-ai-contract-completeness-proof/proof-summary.json',
    parentRuleContext: 'output/ai-plan-proof/local-ai-parent-rule-context-builder-proof/proof-summary.json',
    providerScheduler: 'output/ai-plan-proof/local-ai-provider-scheduler-proof/proof.json',
    runtimeProvider: 'output/ai-plan-proof/local-ai-runtime-provider-proof/proof.json',
    householdRouteSelection: 'output/ai-plan-proof/household-ai-provider-route-selection-proof/proof-summary.json',
    householdAdvertisementHeartbeat:
      'output/ai-plan-proof/household-ai-provider-advertisement-heartbeat-proof/proof-summary.json',
    householdClaimLease: 'output/ai-plan-proof/household-ai-provider-claim-lease-proof/proof-summary.json',
    noRawScreenTransferMesh: 'output/ai-plan-proof/no-raw-screen-transfer-mesh/proof-summary.json',
    householdProviderResultValidation:
      'output/ai-plan-proof/household-ai-provider-result-validation/proof-summary.json',
    householdMeshEventBridge: 'output/ai-plan-proof/household-mesh-event-bridge-proof/proof-summary.json',
    childAgentPolicyAuthority: 'output/ai-plan-proof/child-agent-ai-policy-authority-proof/proof-summary.json',
    mobileDormantProvider: 'output/ai-plan-proof/mobile-dormant-ai-provider-proof/proof-summary.json',
    policyConsumption: 'output/ai-plan-proof/local-ai-policy-enforcement-consumption-proof/proof-summary.json',
  },
  closure: {
    controlledCapturedScreensAnalyzed: true,
    liveOperatorArtifactsAnalyzed: true,
    serviceOcrAnalyzedCapturedPixels: true,
    storedEvidenceCanReachLocalAiInput: true,
    deterministicTextAndClassifierContractsCovered: true,
    memoryAndGraphRefsCovered: true,
    providerRuntimeAndSchedulerCovered: true,
    meshChecklistStatusConsistent: true,
    householdProviderRouteSelectionCovered: true,
    householdProviderAdvertisementHeartbeatCovered: true,
    householdProviderClaimLeaseCovered: true,
    householdNoRawTransferCovered: true,
    householdProviderResultValidationCovered: true,
    householdMeshEventBridgeCovered: true,
    childAgentPolicyAuthorityCovered: true,
    mobileDormantProviderFallbackCovered: true,
    policyOnlyConsumptionCovered: true,
    remoteApiAiClaimed: false,
    rawPromptRetained: false,
    rawImageRetainedByDefault: false,
    modelQualityClaimed: false,
    enforcementClaimedByAiPlan: false,
    finalProductCompleteDeferredToPipeline: true,
  },
  evidenceCounts: {
    controlledScenarioCount: localVlm.scenarioCount,
    controlledRealWindowCaptureCount: localVlm.realWindowCaptureCount,
    liveOperatorScenarioCount: liveOperatorGate.validatedScenarioCount,
    liveExternalUrlRows: liveOperatorGate.realLiveUrlRows,
    localVlmRows: liveOperatorGate.localVlmRows,
    policyDryRunRows: liveOperatorGate.policyDryRunRows,
    storedEvidenceRefs: storedEvidenceIntegration.evaluationInput.evidenceReferenceCount,
  },
  nonClaims: [
    'This audit proves AI-plan prerequisites from existing artifacts and does not rerun capture, OCR, VLM, or model inference.',
    'This audit does not claim production model quality, remote/API AI, policy authority, enforcement, portal UI completion, or product-complete pipeline closure.',
    'The remaining open item is intentionally owned by docs/plans/screen-ai-pipeline-plan after screen and AI prerequisites are stacked.',
  ],
};

mkdirSync(outputRoot, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`local-ai-plan-closure-audit-proof-ok:${proofPath}`);

function readProof(path) {
  assert(existsSync(join(repoRoot, path)), `Missing source proof: ${path}`);
  return JSON.parse(readFileSync(join(repoRoot, path), 'utf8'));
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function relativePath(path) {
  return path.replace(`${repoRoot}\\`, '').replaceAll('\\', '/');
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
