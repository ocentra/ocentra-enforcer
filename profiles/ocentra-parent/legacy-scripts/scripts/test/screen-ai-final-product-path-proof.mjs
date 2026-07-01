import {
  BrowserScenarioIds,
  LiveScenarioIds,
  ProofPath,
  SourcePaths,
  existsPath,
  readJson,
  repoPath,
  writeProofOutputs,
} from './screen-ai-final-product-path-proof-values.mjs';

const failures = [];
const liveOperator = load(SourcePaths.liveOperator);
const liveOperatorArtifactGate = load(SourcePaths.liveOperatorArtifactGate);
const liveOperatorEvidenceBundle = load(SourcePaths.liveOperatorEvidenceBundle);
const liveOperatorAi = load(SourcePaths.liveOperatorAi);
const actionDispatch = load(SourcePaths.actionDispatch);
const aiPlanClosure = load(SourcePaths.aiPlanClosure);
const adapterBlockerLedger = load(SourcePaths.adapterBlockerLedger);
const adapterDependencyHandoff = load(SourcePaths.adapterDependencyHandoff);
const blockActionDispatch = load(SourcePaths.blockActionDispatch);
const deletionRetentionCustody = load(SourcePaths.deletionRetentionCustody);
const finalAdapterAudit = load(SourcePaths.finalAdapterAudit);
const childAgentPolicyAuthority = load(SourcePaths.childAgentPolicyAuthority);
const householdMeshEventBridge = load(SourcePaths.householdMeshEventBridge);
const householdMeshScreenAi = load(SourcePaths.householdMeshScreenAi);
const householdProviderRouteSelection = load(SourcePaths.householdProviderRouteSelection);
const householdProviderResultValidation = load(SourcePaths.householdProviderResultValidation);
const portalChain = load(SourcePaths.portalChain);
const protectedSurface = load(SourcePaths.protectedSurface);
const readModel = load(SourcePaths.readModel);
const noRawScreenTransferMesh = load(SourcePaths.noRawScreenTransferMesh);
const mobileDormantProvider = load(SourcePaths.mobileDormantProvider);
const retentionSweeper = load(SourcePaths.retentionSweeper);
const screenPlanClosure = load(SourcePaths.screenPlanClosure);
const serviceReadModel = load(SourcePaths.serviceReadModel);
const serviceAnalysisRowReady = load(SourcePaths.serviceAnalysisRowReady);
const serviceCaptureEventProducer = load(SourcePaths.serviceCaptureEventProducer);
const serviceDeletionEventProducer = load(SourcePaths.serviceDeletionEventProducer);
const serviceEventBridge = load(SourcePaths.serviceEventBridge);
const serviceEventSubscription = load(SourcePaths.serviceEventSubscription);
const servicePolicyRefProducer = load(SourcePaths.servicePolicyRefProducer);
const serviceWinRtOcrPolicy = load(SourcePaths.serviceWinRtOcrPolicy);

const liveRows = validateLiveOperator();
const authenticatedAccountSocialProof = validateAuthenticatedAccountProofAlignment();
const closure = {
  realTriggerRows: liveRows.length,
  browserLiveRows: liveRows.filter((row) => BrowserScenarioIds.has(row.scenarioId)).length,
  localAiRows: liveRows.filter((row) => row.localAiAnalyzed).length,
  policyDryRunRows: liveRows.filter((row) => row.policyDryRun).length,
  parentExplanationSnapshots: liveRows.filter((row) => row.parentExplanationSnapshotExists).length,
  rawDeletionRows: liveRows.filter((row) => row.rawImageDeleted).length,
  actionDispatchProven: validateActionDispatch(),
  portalReadModelProven: validatePortalChain(),
  readModelRows: validateReadModel(),
  serviceBackedReadModelProven: validateServiceReadModel(),
  serviceEventChainProven: validateServiceEventChain(),
  serviceWinRtOcrPolicyProven: validateServiceWinRtOcrPolicy(),
  retentionCustodyProven: validateDeletionCustody(),
  protectedSurfaceSkipProven: validateProtectedSurface(),
  finalAdapterAuditProven: validateFinalAdapterAudit(),
  adapterDependencyHandoffProven: validateAdapterDependencyHandoff(),
  householdMeshBoundaryProven: validateHouseholdMeshBoundary(),
  retainedLiveOperatorBundlePortable: validateLiveOperatorEvidenceBundle(),
  screenPlanClosureAudited: validateScreenPlanClosure(),
  aiPlanClosureAudited: validateAiPlanClosure(),
};

assert(
  closure.realTriggerRows === LiveScenarioIds.length,
  `expected ${LiveScenarioIds.length} live/operator trigger rows, got ${closure.realTriggerRows}`
);
assert(closure.browserLiveRows === BrowserScenarioIds.size, 'expected all required live browser URL rows');
assert(closure.localAiRows === 8, `expected 8 local AI analyzed rows, got ${closure.localAiRows}`);
assert(closure.policyDryRunRows === 8, `expected 8 policy dry-run rows, got ${closure.policyDryRunRows}`);
assert(
  closure.parentExplanationSnapshots === 8,
  `expected 8 parent explanation snapshots, got ${closure.parentExplanationSnapshots}`
);
assert(closure.rawDeletionRows === 8, `expected 8 raw deletion rows, got ${closure.rawDeletionRows}`);

if (failures.length > 0) {
  throw new Error(
    `Screen AI final product path proof failed:\n${failures.map((failure) => `- ${failure}`).join('\n')}`
  );
}

const proof = {
  status: 'ok',
  proofKind: 'screen-ai-final-product-path-proof',
  generatedAt: new Date().toISOString(),
  sourceArtifacts: SourcePaths,
  closure: {
    ...closure,
    finalPathEvidenceComplete: true,
    screenAndAiPrerequisitesStacked: closure.screenPlanClosureAudited && closure.aiPlanClosureAudited,
    broadBrowserNetworkMobileProductComplete: false,
    adapterProductCompleteBlockedByAudit: true,
    finalPipelineProductComplete: false,
    finalPipelineProductCompleteBlockedByAdapterGate: true,
    custodyArtifactRows: finalAdapterAudit.closure?.custodyArtifactRows,
    adapterBlockerRowsMapped: adapterBlockerLedger.closure?.blockerRows,
    adapterDependencyRowsMapped: adapterDependencyHandoff.closure?.dependencyRowsMapped,
    adapterDependencyHandoffRequired: closure.adapterDependencyHandoffProven,
    householdMeshConsumesRedactedRefsOnly: closure.householdMeshBoundaryProven,
    publicSocialSurfaceProof: true,
    authenticatedAccountSocialProof,
    householdRouteSelectionCovered: true,
    householdMeshBridgeMediated: true,
    childAgentPolicyAuthorityCovered: true,
    mobileDormantProviderFallbackCovered: true,
    serviceEventProducersAndSubscriberCovered: closure.serviceEventChainProven,
    serviceWinRtOcrLivePolicyCovered: closure.serviceWinRtOcrPolicyProven,
    singleRuntimeSessionRerun: serviceWinRtOcrPolicy.assertions?.sourceProofRerunByThisGate === true,
    retainedRealRunArtifactsVerified: true,
    retainedLiveOperatorBundlePortable: closure.retainedLiveOperatorBundlePortable,
    rawScreenshotsRetainedByDefault: false,
    remoteAiUsedForChildSafety: false,
  },
  liveRows,
  nonClaims: [
    'This verifier validates retained real-run artifacts and does not rerun the live operator capture or model inference session.',
    'This proof requires the sanitized live-operator evidence bundle so remote review can inspect retained redacted source, AI, policy, deletion, and parent explanation artifacts without raw screenshots or encrypted queues.',
    authenticatedAccountSocialProof
      ? 'Authenticated-account social proof is included only because the retained optional logged-in live-operator row, artifact gate, and portable evidence bundle validate it; managed-browser trigger producer ownership and broad browser/network/mobile/Linux adapters remain separate unless their own execution artifacts are cited.'
      : 'Managed-browser trigger producer ownership, authenticated-account social proof, and broad browser/network/mobile/Linux adapters remain separate unless their own execution artifacts are cited.',
    'The custody-aware final adapter audit is required by this proof and keeps broad/browser/network/mobile/native-Linux product-complete adapter execution blocked while WSL2 Linux execution remains separately proved.',
    'The adapter blocker ledger and dependency handoff are required by this proof; they map upstream execution artifacts without upgrading product-complete claims.',
    'The screen-plan and AI-plan closure audits are required by this proof; they stack prerequisites without overriding remaining external adapter and platform gates.',
    'Household mesh provider routing artifacts are required by this proof; provider work may carry redacted/custody refs only and child-agent validation remains local before policy.',
    'Service event producer/subscriber artifacts are required by this proof; broad/browser/network/mobile/native-Linux product adapter execution remains separate.',
    'The real Windows service WinRT OCR policy artifact is required by this proof and must rerun live public browser capture/OCR before consuming the row through typed policy dry-run contracts.',
    'The proof closes the stacked real trigger-to-analysis-to-policy-to-action/read-model-to-deletion evidence path from current artifacts; it does not make raw screenshot retention or live view product claims.',
  ],
};

writeProofOutputs(proof);
console.log(`screen-ai-final-product-path-proof-ok:${ProofPath}`);

function validateLiveOperator() {
  assert(liveOperator.proof === 'screen-ai-live-operator-proof', 'live operator summary proof id mismatch');
  assert(
    liveOperatorArtifactGate.proof === 'screen-ai-live-operator-artifact-gate',
    'live operator artifact gate proof id mismatch'
  );
  assert(liveOperator.fullRequiredMatrixComplete === true, 'live operator matrix is not complete');
  assert(
    liveOperatorArtifactGate.publicSocialSurfaceRows === 1,
    'live operator gate must prove exactly one public social surface row'
  );
  assert(liveOperator.liveExternalUrlProof === true, 'live operator missing live external URL proof');
  assert(liveOperator.localVlmAnalysisProof === true, 'live operator missing local VLM proof');
  assert(liveOperator.policyDryRunProof === true, 'live operator missing policy dry-run proof');
  assert(liveOperator.rawImagesDeletedAfterAnalysis === true, 'live operator missing raw deletion proof');
  assert(liveOperator.controlledFixtureProof === false, 'live operator still marks controlled fixture proof');
  assert(liveOperatorAi.proof === 'screen-ai-live-operator-proof', 'AI live operator summary proof id mismatch');

  return LiveScenarioIds.map((scenarioId) => validateLiveScenario(scenarioId));
}

function validateLiveOperatorEvidenceBundle() {
  assert(
    liveOperatorEvidenceBundle.proof === 'screen-ai-live-operator-evidence-bundle',
    'live operator evidence bundle proof id mismatch'
  );
  assert(liveOperatorEvidenceBundle.bundlePortableForReview === true, 'live operator bundle is not portable');
  assert(liveOperatorEvidenceBundle.scenarioCount === LiveScenarioIds.length, 'live operator bundle scenario mismatch');
  assert(liveOperatorEvidenceBundle.localVlmRows >= 8, 'live operator bundle missing local VLM rows');
  assert(liveOperatorEvidenceBundle.policyDryRunRows >= 8, 'live operator bundle missing policy dry-run rows');
  assert(
    liveOperatorEvidenceBundle.parentExplanationScreenshots >= 8,
    'live operator bundle missing parent explanation screenshots'
  );
  assert(liveOperatorEvidenceBundle.rawScreenshotFilesCopied === false, 'live operator bundle copied raw screenshots');
  assert(
    liveOperatorEvidenceBundle.encryptedQueueFilesCopied === false,
    'live operator bundle copied encrypted queues'
  );
  assert(
    liveOperatorEvidenceBundle.authenticatedAccountSocialProof ===
      liveOperatorArtifactGate.authenticatedAccountSocialProof,
    'live operator bundle and artifact gate disagree on authenticated-account proof'
  );

  for (const scenarioId of LiveScenarioIds) {
    const row = liveOperatorEvidenceBundle.scenarios.find((candidate) => candidate.scenarioId === scenarioId);
    assert(Boolean(row), `live operator bundle missing ${scenarioId}`);
    assert(row.rawImagePathRetained === false, `${scenarioId} retained raw image path in bundle`);
    for (const artifact of row.copiedArtifacts ?? []) {
      assert(existsPath(repoPath(artifact.path)), `${scenarioId} bundle artifact missing ${artifact.path}`);
      assert(!artifact.path.includes('/capture/'), `${scenarioId} bundle copied capture artifact ${artifact.path}`);
      assert(!artifact.path.includes('/queue/'), `${scenarioId} bundle copied queue artifact ${artifact.path}`);
    }
  }

  return true;
}

function validateAuthenticatedAccountProofAlignment() {
  const gateClaim = liveOperatorArtifactGate.authenticatedAccountSocialProof === true;
  const bundleClaim = liveOperatorEvidenceBundle.authenticatedAccountSocialProof === true;
  const summaryClaim = liveOperator.authenticatedAccountSocialProof === true;
  assert(gateClaim === bundleClaim, 'artifact gate and bundle disagree on account proof');
  assert(gateClaim === summaryClaim, 'artifact gate and live operator summary disagree on account proof');
  if (!gateClaim) {
    return false;
  }
  const accountRows = liveOperatorArtifactGate.optionalAuthenticatedAccountScenarios ?? [];
  assert(accountRows.length > 0, 'account proof claimed without optional account rows');
  assert(
    accountRows.every((row) => row.authenticatedAccountProof === true && row.localVlmAnalysisProof === true),
    'account proof rows missing account or local VLM proof'
  );
  return true;
}

function validateLiveScenario(scenarioId) {
  const summaryRow = liveOperator.scenarios.find((row) => row.scenarioId === scenarioId);
  assert(Boolean(summaryRow), `missing live operator scenario ${scenarioId}`);

  if (scenarioId === 'protected-unsupported-state') {
    return validateProtectedScenario(summaryRow);
  }

  const scenarioRoot = `output/ai-plan-proof/live-operator/${scenarioId}`;
  const source = load(`${scenarioRoot}/01-redacted-source-evidence.json`);
  const capture = load(`${scenarioRoot}/02-capture-proof-ref.json`);
  const ai = load(`${scenarioRoot}/06-ai-result.json`);
  const policy = load(`${scenarioRoot}/07-policy-decision.json`);
  const deletion = load(`${scenarioRoot}/08-deletion-after-analysis.json`);
  const explanationPath = `${scenarioRoot}/10-parent-explanation.png`;

  if (BrowserScenarioIds.has(scenarioId)) {
    assert(source.liveExternalUrl === true, `${scenarioId} is not marked as live external URL`);
    assert(source.protocol === 'https', `${scenarioId} source protocol is not HTTPS`);
    assert(source.pageReadiness?.loaded === true, `${scenarioId} page readiness is not loaded`);
    assert(source.pageReadiness?.visibleTextLength > 0, `${scenarioId} has no visible text readiness evidence`);
    assert(
      Object.values(source.pageReadiness?.readinessAssertions ?? {}).every(Boolean),
      `${scenarioId} readiness failed`
    );
  }

  assert(capture.captureMetadata?.captured === true, `${scenarioId} capture metadata is not captured`);
  assert(capture.captureMetadata?.status === 'available', `${scenarioId} capture status is not available`);
  assert(capture.captureMetadata?.rawImagePersistedInProof === false, `${scenarioId} persisted raw image in proof`);
  assert(capture.rawImagePathNotRetained === true, `${scenarioId} raw image path retained`);
  assert(ai.screenResult?.providerKind === 'localVision', `${scenarioId} did not use localVision`);
  assert(ai.screenResult?.imageDeletionState === 'deleted', `${scenarioId} screen result image not deleted`);
  assert(ai.screenResult?.rawImageRetained === false, `${scenarioId} screen result retained raw image`);
  assert(ai.localAiSafetyResult?.modelRuntime?.privacyMode === 'local-only', `${scenarioId} AI runtime not local-only`);
  assert(policy.policyDecision?.dryRun === true, `${scenarioId} policy decision is not dry-run`);
  assert(
    policy.policyDecision?.localAiResultId === ai.localAiSafetyResult?.resultId,
    `${scenarioId} policy lost AI result ref`
  );
  assert(deletion.rawImageDeletedAfterAnalysis === true, `${scenarioId} raw image not deleted`);
  assert(deletion.existsAfterDelete === false, `${scenarioId} raw temp still exists after delete`);
  assert(existsPath(repoPath(explanationPath)), `${scenarioId} parent explanation screenshot missing`);

  return {
    scenarioId,
    realTrigger: true,
    category: ai.screenResult.primaryCategory,
    policyAction: policy.policyDecision.action,
    localAiAnalyzed: true,
    policyDryRun: true,
    rawImageDeleted: true,
    parentExplanationSnapshotExists: true,
  };
}

function validateProtectedScenario(summaryRow) {
  const source = load(
    'output/ai-plan-proof/live-operator/protected-unsupported-state/01-redacted-source-evidence.json'
  );
  assert(summaryRow.status === 'passed', 'protected scenario did not pass');
  assert(summaryRow.policyDecisionValidated === false, 'protected scenario claimed policy validation');
  assert(
    source.protectedOrUnsupportedState === 'protectedSurface',
    'protected scenario did not preserve protected state'
  );
  assert(source.liveExternalUrl === false, 'protected scenario claimed live external URL');
  return {
    scenarioId: 'protected-unsupported-state',
    realTrigger: true,
    category: null,
    policyAction: null,
    localAiAnalyzed: false,
    policyDryRun: false,
    rawImageDeleted: false,
    parentExplanationSnapshotExists: false,
  };
}

function validateActionDispatch() {
  assert(actionDispatch.policyDecisionLinkedToAdapter === true, 'time-limit action did not link policy to adapter');
  assert(actionDispatch.realWindowsAdapterProof === true, 'time-limit adapter proof is not real Windows proof');
  assert(actionDispatch.rawImageDeletedBeforeDispatch === true, 'time-limit dispatch occurred before raw deletion');
  assert(actionDispatch.adapterResultCode === 'process-terminated', 'time-limit adapter did not terminate process');
  assert(blockActionDispatch.policyDecisionLinkedToAdapter === true, 'block action did not link policy to adapter');
  assert(blockActionDispatch.realWindowsBlockAdapterProof === true, 'block adapter proof is not real Windows proof');
  assert(blockActionDispatch.adapterStatus === 'actually-enforced', 'block adapter was not actually enforced');
  assert(blockActionDispatch.rawImageDeletedBeforeDispatch === true, 'block dispatch occurred before raw deletion');
  return true;
}

function validatePortalChain() {
  assert(portalChain.status === 'ok', 'portal chain status is not ok');
  const rendered = new Set(portalChain.renderedAssertions ?? []);
  for (const expected of ['AI provider localVision', 'Policy eligible Yes', 'Raw image deleted']) {
    assert(rendered.has(expected), `portal chain missing rendered assertion ${expected}`);
  }
  assert(existsPath(portalChain.artifact?.screenshot), 'portal chain screenshot missing');
  return true;
}

function validateReadModel() {
  assert(readModel.status === 'ok', 'parent explanation read-model proof status is not ok');
  assert(readModel.summary?.rowCount > 0, 'parent explanation read-model has no rows');
  assert(readModel.summary?.rawImageShown === false, 'read-model shows raw image');
  assert(readModel.summary?.rawImageRetained === false, 'read-model retains raw image');
  assert(readModel.summary?.remoteAiUsed === false, 'read-model used remote AI');
  assert(readModel.summary?.portalRuntimeClaimed === false, 'read-model claims portal runtime');
  return readModel.summary.rowCount;
}

function validateServiceReadModel() {
  assert(serviceReadModel.status === 'ok', 'service read-model proof status is not ok');
  assert(
    serviceReadModel.proofKind === 'screen-summary-parent-explanation-service-read-model',
    'service read-model proof kind mismatch'
  );
  assert(
    serviceReadModel.closure?.serviceBackedWebSocketReadModel === true,
    'service read-model is not service-backed WebSocket proof'
  );
  assert(
    serviceReadModel.closure?.queryStoreIngestPreservedExplanationRefs === true,
    'service read-model lost query-store explanation refs'
  );
  assert(
    serviceReadModel.closure?.rawScreenshotsRetainedByDefault === false,
    'service read-model retains raw screenshots'
  );
  assert(serviceReadModel.closure?.remoteAiUsedForChildSafety === false, 'service read-model used remote AI');
  assert(serviceReadModel.closure?.portalUiRenderingClaimed === false, 'service read-model claims portal UI');
  assert(serviceReadModel.serviceEvent?.activityReadModelKind === 'screen', 'service event is not screen read model');
  assert(serviceReadModel.serviceEvent?.activitySurfaceState === 'ready', 'service event is not ready');
  assert(serviceReadModel.row?.imageDeletionState === 'deleted', 'service row image is not deleted');
  assert(
    serviceReadModel.row?.custodyState === 'child-device-journal',
    'service row custody is not child-device journal'
  );
  assert((serviceReadModel.row?.parentExplanationRefs ?? []).length > 0, 'service row has no parent explanation refs');
  assert(
    (serviceReadModel.row?.deletionReasons ?? []).includes('screen-image-deleted'),
    'service row lacks deleted-image reason'
  );
  return true;
}

function validateServiceEventChain() {
  assert(
    serviceEventSubscription.proofMode === 'screen-service-event-subscription',
    'service event subscription proof mode mismatch'
  );
  assert(
    serviceEventSubscription.claimsProved?.some((claim) =>
      claim.includes('retains the screen event subscription runtime')
    ),
    'service startup subscription retention claim missing'
  );
  assert(
    serviceEventSubscription.claimsProved?.some((claim) =>
      claim.includes('row-ready subscriber and dispatches through the real event bus')
    ),
    'service row-ready real-event-bus subscription claim missing'
  );
  assert(
    serviceCaptureEventProducer.proofMode === 'screen-service-capture-event-producer',
    'service capture event producer proof mode mismatch'
  );
  assert(
    serviceCaptureEventProducer.claimsProved?.some((claim) =>
      claim.includes('service cadence runtime publishes capture/queue events')
    ),
    'service cadence capture event producer claim missing'
  );
  assert(
    serviceCaptureEventProducer.claimsProved?.some((claim) =>
      claim.includes('service foreground runtime publishes capture/queue events')
    ),
    'service foreground capture event producer claim missing'
  );
  assert(
    serviceAnalysisRowReady.proofMode === 'screen-service-analysis-row-ready',
    'service analysis row-ready proof mode mismatch'
  );
  assert(
    serviceAnalysisRowReady.claimsProved?.some((claim) => claim.includes('publishes screen.service.row.ready')),
    'service analysis row-ready publication claim missing'
  );
  assert(
    servicePolicyRefProducer.proofMode === 'screen-service-policy-ref-producer',
    'service policy-ref producer proof mode mismatch'
  );
  assert(
    servicePolicyRefProducer.claimsProved?.some((claim) =>
      claim.includes('carry policy decision, action, reason, parent rule, explanation, and deletion proof refs')
    ),
    'service policy-ref producer refs claim missing'
  );
  assert(
    serviceDeletionEventProducer.proofMode === 'screen-service-deletion-event-producer',
    'service deletion event producer proof mode mismatch'
  );
  assert(
    serviceDeletionEventProducer.claimsProved?.some((claim) =>
      claim.includes('retention sweeper runtime publishes deletion events')
    ),
    'service deletion event producer claim missing'
  );
  assert(serviceEventBridge.proofMode === 'screen-service-event-bridge', 'service event bridge proof mode mismatch');
  assert(
    serviceEventBridge.claimsProved?.some((claim) =>
      claim.includes('rows publish the ordered typed screen event chain')
    ),
    'service event bridge ordered chain claim missing'
  );
  assert(
    serviceEventBridge.claimsProved?.some((claim) =>
      claim.includes(
        'degraded AI rows publish capture, queue, AI, deletion, and portal events without policy or action refs'
      )
    ),
    'service event bridge degraded chain claim missing'
  );
  for (const expectedEvent of [
    'screen.capture.observed',
    'screen.queue.encrypted',
    'screen.ai.analysis.requested',
    'screen.ai.analysis.completed',
    'screen.summary.committed',
    'screen.policy.decision.completed',
    'screen.action.dry-run.recorded',
    'screen.deletion.committed',
    'screen.portal-read-model.updated',
  ]) {
    assert(
      serviceEventSubscription.eventChain?.includes(expectedEvent),
      `service subscription missing ${expectedEvent}`
    );
    assert(serviceEventBridge.eventChain?.includes(expectedEvent), `service bridge missing ${expectedEvent}`);
  }
  return true;
}

function validateServiceWinRtOcrPolicy() {
  assert(
    serviceWinRtOcrPolicy.proof === 'screen-ai-service-winrt-ocr-policy-proof',
    'service OCR policy proof id mismatch'
  );
  assert(
    serviceWinRtOcrPolicy.proofTier === 'P3_REAL_CAPTURE_LOCAL_OCR_POLICY_CONSUMPTION',
    'service OCR policy proof tier mismatch'
  );
  assert(
    serviceWinRtOcrPolicy.assertions?.sourceProofRerunByThisGate === true,
    'service OCR policy proof did not rerun the source service OCR proof'
  );
  assert(
    serviceWinRtOcrPolicy.sourceLiveSurface?.kind === 'live-public-browser-page',
    'service OCR policy proof is not live public browser source'
  );
  assert(
    serviceWinRtOcrPolicy.sourceLiveSurface?.url === 'https://en.wikipedia.org/wiki/Mathematics',
    'service OCR policy source URL changed'
  );
  assert(
    serviceWinRtOcrPolicy.sourceAnalysisRow?.providerKind === 'localOcr',
    'service OCR policy source did not use localOcr'
  );
  assert(
    serviceWinRtOcrPolicy.sourceAnalysisRow?.modelId === 'windows-winrt-ocr',
    'service OCR policy source did not use Windows WinRT OCR'
  );
  assert(
    serviceWinRtOcrPolicy.sourceAnalysisRow?.policyEligible === true,
    'service OCR policy source row is not policy eligible'
  );
  assert(
    serviceWinRtOcrPolicy.sourceAnalysisRow?.rawImageRetained === false,
    'service OCR policy source retained raw image'
  );
  assert(
    serviceWinRtOcrPolicy.sourceAnalysisRow?.imageDeletionState === 'deleted',
    'service OCR policy source image is not deleted'
  );
  assert(serviceWinRtOcrPolicy.policy?.dryRun === true, 'service OCR policy is not dry-run');
  assert(serviceWinRtOcrPolicy.policy?.action === 'allow', 'service OCR policy action changed');
  assert(
    serviceWinRtOcrPolicy.policy?.enforcementHandoffState === 'disabled',
    'service OCR policy enabled enforcement handoff'
  );
  for (const assertion of [
    'sourceProofRerunByThisGate',
    'sourceUsedLivePublicBrowserPixels',
    'sourceRanWindowsWinRtOcr',
    'sourceReadModelReachedViaWebSocket',
    'sourceQueueDrained',
    'sourceTempImageDeleted',
    'sourceRawImageNotRetained',
    'policyDecisionParsedByParentDomain',
    'policyConsumedExactActivityRow',
    'policyDryRunOnly',
    'activityReadModelCarriesPolicyRefs',
    'deletionCustodyPreserved',
  ]) {
    assert(serviceWinRtOcrPolicy.assertions?.[assertion] === true, `service OCR policy assertion ${assertion} failed`);
  }
  return true;
}

function validateDeletionCustody() {
  assert(
    Object.values(deletionRetentionCustody.assertions ?? {}).every(Boolean),
    'deletion-retention-custody assertion failed'
  );
  assert(
    retentionSweeper.assertions?.retentionSweeperRemovedExpiredQueueRecord === true,
    'retention sweeper did not remove queue'
  );
  assert(
    retentionSweeper.assertions?.expiredDeletionSurfacedInActivityReadModel === true,
    'expired deletion not in read model'
  );
  assert(retentionSweeper.ephemeralPathsDeletedAfterProof === true, 'retention sweeper left ephemeral paths');
  return true;
}

function validateProtectedSurface() {
  assert(protectedSurface.status === 'ok' || protectedSurface.proof, 'protected surface proof missing');
  return true;
}

function validateFinalAdapterAudit() {
  assert(
    finalAdapterAudit.status === 'blocked-by-upstream-adapter-artifacts',
    'final adapter audit status is not blocked'
  );
  assert(
    finalAdapterAudit.closure?.windowsOwnedProcessAdaptersProved === true,
    'final adapter audit lost Windows owned-process proof'
  );
  assert(
    finalAdapterAudit.closure?.broadBrowserNetworkMobileProductComplete === false,
    'final adapter audit unexpectedly claims product-complete adapters'
  );
  assert(
    finalAdapterAudit.closure?.openChecklistRowRetained === true,
    'final adapter audit did not retain open checklist row'
  );
  assert(
    finalAdapterAudit.closure?.custodyArtifactRows === 3,
    'final adapter audit did not consume three custody artifacts'
  );
  assert(
    finalAdapterAudit.closure?.linuxHostExecutionRows === 1,
    'final adapter audit did not consume Linux host execution proof'
  );
  assert(finalAdapterAudit.closure?.claimUpgradeRows === 0, 'final adapter audit contains claim upgrades');
  assert((finalAdapterAudit.blockedRows ?? []).length === 5, 'final adapter audit blocked row count changed');
  assert((finalAdapterAudit.custodyRows ?? []).length === 3, 'final adapter audit custody row count changed');
  assert(
    finalAdapterAudit.linuxExecutionRow?.executionClaimed === true &&
      finalAdapterAudit.linuxExecutionRow?.rollbackExecuted === true,
    'final adapter audit Linux execution row is not applied and rolled back'
  );
  assert(
    finalAdapterAudit.custodyRows?.every((row) => row.finalAdapterCompletionClaimed === false) === true,
    'final adapter audit custody row claims final completion'
  );
  assert(
    finalAdapterAudit.custodyRows?.every((row) => row.productCompleteAdapterRowStillOpen === true) === true,
    'final adapter audit custody row closes product-complete row'
  );
  return true;
}

function validateAdapterDependencyHandoff() {
  assert(adapterBlockerLedger.status === 'blocked-but-actionable', 'adapter blocker ledger is not actionable');
  assert(
    adapterBlockerLedger.closure?.adapterCompletionStillBlocked === true,
    'adapter blocker ledger no longer blocks adapter completion'
  );
  assert(adapterBlockerLedger.closure?.blockerRows === 5, 'adapter blocker ledger row count changed');
  assert(adapterBlockerLedger.closure?.claimUpgradeRows === 0, 'adapter blocker ledger contains claim upgrades');
  assert(
    adapterDependencyHandoff.status === 'adapter-dependency-handoff-ready-upstream-execution-required',
    'adapter dependency handoff status changed'
  );
  assert(
    adapterDependencyHandoff.closure?.dependencyRowsMapped === adapterBlockerLedger.closure?.blockerRows,
    'adapter dependency handoff row count does not match blocker ledger'
  );
  assert(
    adapterDependencyHandoff.closure?.expectedProofFilesMapped === true,
    'adapter dependency handoff does not map expected proof files'
  );
  assert(
    adapterDependencyHandoff.closure?.expectedContractShapesMapped === true,
    'adapter dependency handoff does not map expected contract shapes'
  );
  assert(
    adapterDependencyHandoff.closure?.productCompleteClaimed === false,
    'adapter dependency handoff claims product completion'
  );
  assert(
    adapterDependencyHandoff.closure?.rawImageRetainedByExpectedContracts === false,
    'adapter dependency handoff allows raw image retention'
  );
  assert(
    adapterDependencyHandoff.closure?.upstreamAppInstallContextMappedWithoutClaimUpgrade === true,
    'adapter dependency handoff must map app-install context without upgrading screen adapter claims'
  );
  return true;
}

function validateHouseholdMeshBoundary() {
  assert(
    householdMeshScreenAi.proofMode === 'household-mesh-screen-ai',
    'household mesh screen AI proof mode mismatch'
  );
  for (const expectedEvent of [
    'screen.mesh.work.queued',
    'screen.mesh.claim.granted',
    'screen.mesh.lease.created',
    'screen.mesh.provider-result.returned',
    'screen.mesh.child-result.accepted',
    'screen.mesh.policy.requested',
  ]) {
    assert(householdMeshScreenAi.eventChain?.includes(expectedEvent), `household mesh missing ${expectedEvent}`);
  }
  assert(
    (householdMeshScreenAi.claimsProved ?? []).some((claim) => claim.includes('provider claim and lease phases')),
    'household mesh proof does not claim provider claim/lease phases'
  );
  assert(
    (householdMeshScreenAi.claimsProved ?? []).some((claim) =>
      claim.includes('validates provider result before policy')
    ),
    'household mesh proof lacks child-agent validation before policy'
  );
  assert(
    (householdMeshScreenAi.claimsProved ?? []).some((claim) =>
      claim.includes('cannot publish policy or enforcement events')
    ),
    'household mesh proof allows provider-authored policy/enforcement'
  );
  assert(
    (householdMeshScreenAi.claimsProved ?? []).some((claim) => claim.includes('not raw screenshot transfer')),
    'household mesh proof lacks no-raw-screenshot-transfer claim'
  );
  assert(
    householdProviderRouteSelection.proofMode === 'household-ai-provider-route-selection-proof',
    'household provider route selection proof mode mismatch'
  );
  assert(
    householdProviderRouteSelection.routePriority?.includes('desktop-preferred'),
    'household route selection lost desktop provider preference'
  );
  assert(
    householdProviderRouteSelection.routePriority?.includes('mobile-dormant'),
    'household route selection lost mobile dormant ordering'
  );
  assert(
    householdProviderRouteSelection.rejectionCases?.includes('custody-mismatch'),
    'household route selection lost custody mismatch rejection'
  );
  assert(
    noRawScreenTransferMesh.proofMode === 'no-raw-screen-transfer-mesh',
    'no-raw screen transfer proof mode mismatch'
  );
  assert(
    (noRawScreenTransferMesh.claimsProved ?? []).some((claim) => claim.includes('not raw screenshot transfer')),
    'no-raw screen transfer proof lacks no-raw-transfer claim'
  );
  assert(
    noRawScreenTransferMesh.claimsNotProved?.includes(
      'production mesh bridge transport over authenticated LAN messages'
    ),
    'no-raw screen transfer proof lost production transport non-claim'
  );
  assert(
    householdProviderResultValidation.proofMode === 'household-ai-provider-result-validation',
    'provider result validation proof mode mismatch'
  );
  for (const rejectionCase of [
    'duplicate-result',
    'expired-lease',
    'wrong-provider',
    'wrong-claim',
    'evidence-mismatch',
    'custody-mismatch',
    'raw-image-transfer',
    'provider-authority-violation',
  ]) {
    assert(
      householdProviderResultValidation.rejectionCases?.includes(rejectionCase),
      `provider result validation missing ${rejectionCase}`
    );
  }
  assert(
    (householdProviderResultValidation.claimsProved ?? []).some((claim) =>
      claim.includes('validates provider result before policy')
    ),
    'provider result validation proof lacks child-agent validation claim'
  );
  assert(
    householdMeshEventBridge.proofMode === 'household-mesh-event-bridge-proof',
    'household mesh event bridge proof mode mismatch'
  );
  assert(
    householdMeshEventBridge.rejectionCases?.includes('raw-screen-payload'),
    'household mesh event bridge lost raw screen payload rejection'
  );
  assert(
    householdMeshEventBridge.rejectionCases?.includes('direct-remote-publish'),
    'household mesh event bridge lost direct remote publish rejection'
  );
  assert(
    childAgentPolicyAuthority.assertions?.providerCannotPublishPolicyOrEnforcement === true,
    'child-agent authority proof allows provider policy/enforcement events'
  );
  assert(
    childAgentPolicyAuthority.assertions?.policyConsumesOnlyAcceptedChildResult === true,
    'child-agent authority proof no longer requires accepted child result before policy'
  );
  assert(
    mobileDormantProvider.mobileClaimsProved?.some((claim) => claim.includes('mobile providers stay dormant')),
    'mobile dormant provider proof lost dormant-mobile claim'
  );
  return true;
}

function validateScreenPlanClosure() {
  assert(screenPlanClosure.proof === 'screen-plan-closure-audit', 'screen-plan closure proof id mismatch');
  assert(screenPlanClosure.checklist?.openCount === 0, 'screen-plan closure still has open table rows');
  assert(
    (screenPlanClosure.checklist?.partialCount ?? 0) > 0,
    'screen-plan closure lost external partial-gate tracking'
  );
  assert(
    screenPlanClosure.assertions?.readinessProofsPresent === true,
    'screen-plan closure readiness proofs are not present'
  );
  assert(
    screenPlanClosure.assertions?.adapterAuditKeepsProductCompletionBlocked === true,
    'screen-plan closure no longer keeps adapter completion blocked'
  );
  assert(
    screenPlanClosure.assertions?.custodyArtifactsDoNotUpgradeClaims === true,
    'screen-plan closure custody artifacts upgrade claims'
  );
  assert(
    screenPlanClosure.assertions?.serviceCadenceRuntimeProved === true,
    'screen-plan closure lost service cadence runtime proof'
  );
  assert(
    screenPlanClosure.assertions?.serviceDisabledSuppressionProved === true,
    'screen-plan closure lost disabled suppression proof'
  );
  assert(
    screenPlanClosure.assertions?.serviceForegroundWatcherProved === true,
    'screen-plan closure lost service foreground watcher proof'
  );
  assert(
    screenPlanClosure.assertions?.serviceEncryptedQueueExpiryDeletionProved === true,
    'screen-plan closure lost encrypted queue expiry deletion proof'
  );
  assert(
    screenPlanClosure.assertions?.deleteFailedVisibilityProved === true,
    'screen-plan closure lost delete-failed visibility proof'
  );
  assert(screenPlanClosure.assertions?.noProductCompleteClaim === true, 'screen-plan closure claims product complete');
  assert(
    (screenPlanClosure.remainingProductGates ?? []).length > 0,
    'screen-plan closure lost remaining product gates'
  );
  return true;
}

function validateAiPlanClosure() {
  assert(aiPlanClosure.proof === 'local-ai-plan-closure-audit-proof', 'AI-plan closure proof id mismatch');
  assert(aiPlanClosure.checklist?.openCount === 0, 'AI-plan closure still has open table rows');
  assert(
    aiPlanClosure.closure?.controlledCapturedScreensAnalyzed === true,
    'AI-plan closure lost controlled captured-screen analysis'
  );
  assert(
    aiPlanClosure.closure?.liveOperatorArtifactsAnalyzed === true,
    'AI-plan closure lost live operator analysis coverage'
  );
  assert(
    aiPlanClosure.closure?.serviceOcrAnalyzedCapturedPixels === true,
    'AI-plan closure lost service OCR captured-pixel proof'
  );
  assert(
    aiPlanClosure.closure?.storedEvidenceCanReachLocalAiInput === true,
    'AI-plan closure lost stored-evidence local AI input proof'
  );
  assert(
    aiPlanClosure.closure?.providerRuntimeAndSchedulerCovered === true,
    'AI-plan closure lost provider runtime or scheduler coverage'
  );
  assert(
    aiPlanClosure.closure?.meshChecklistStatusConsistent === true,
    'AI-plan closure lost household mesh checklist consistency guard'
  );
  assert(
    aiPlanClosure.closure?.householdProviderRouteSelectionCovered === true,
    'AI-plan closure lost household provider route selection coverage'
  );
  assert(
    aiPlanClosure.closure?.householdProviderAdvertisementHeartbeatCovered === true,
    'AI-plan closure lost household provider advertisement/heartbeat coverage'
  );
  assert(
    aiPlanClosure.closure?.householdProviderClaimLeaseCovered === true,
    'AI-plan closure lost household provider claim/lease lifecycle coverage'
  );
  assert(
    aiPlanClosure.closure?.householdNoRawTransferCovered === true,
    'AI-plan closure lost household no-raw-transfer coverage'
  );
  assert(
    aiPlanClosure.closure?.householdProviderResultValidationCovered === true,
    'AI-plan closure lost household provider result validation coverage'
  );
  assert(
    aiPlanClosure.closure?.householdMeshEventBridgeCovered === true,
    'AI-plan closure lost household mesh event bridge coverage'
  );
  assert(
    aiPlanClosure.closure?.childAgentPolicyAuthorityCovered === true,
    'AI-plan closure lost child-agent policy authority coverage'
  );
  assert(
    aiPlanClosure.closure?.mobileDormantProviderFallbackCovered === true,
    'AI-plan closure lost mobile dormant provider fallback coverage'
  );
  assert(
    aiPlanClosure.closure?.policyOnlyConsumptionCovered === true,
    'AI-plan closure lost policy-only consumption coverage'
  );
  assert(aiPlanClosure.closure?.remoteApiAiClaimed === false, 'AI-plan closure claims remote/API AI');
  assert(aiPlanClosure.closure?.rawPromptRetained === false, 'AI-plan closure retains raw prompt');
  assert(aiPlanClosure.closure?.rawImageRetainedByDefault === false, 'AI-plan closure retains raw image by default');
  assert(aiPlanClosure.closure?.modelQualityClaimed === false, 'AI-plan closure claims model quality');
  assert(aiPlanClosure.closure?.enforcementClaimedByAiPlan === false, 'AI-plan closure claims enforcement');
  assert(
    aiPlanClosure.closure?.finalProductCompleteDeferredToPipeline === true,
    'AI-plan closure no longer defers final product-complete to pipeline'
  );
  return true;
}

function load(path) {
  return readJson(path, assert);
}

function assert(condition, message) {
  if (!condition) {
    failures.push(message);
  }
}
