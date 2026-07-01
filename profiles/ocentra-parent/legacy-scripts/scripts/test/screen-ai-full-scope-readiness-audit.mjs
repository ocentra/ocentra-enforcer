import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = resolve(repoRoot, 'output', 'screen-ai-pipeline-proof', 'full-scope-readiness-audit');
const proofPath = join(outputDir, 'proof-summary.json');
const snapshotPath = join(outputDir, '00-source-snapshot.md');
const commandsPath = join(outputDir, '10-validation-commands.log');

const sourceArtifacts = {
  screenPlanClosure: 'output/screen-plan-proof/screen-plan-closure-audit/proof-summary.json',
  aiPlanClosure: 'output/ai-plan-proof/local-ai-plan-closure-audit/proof-summary.json',
  adapterBlockerLedger: 'output/screen-ai-pipeline-proof/adapter-blocker-ledger/proof-summary.json',
  adapterDependencyHandoff: 'output/screen-ai-pipeline-proof/adapter-dependency-handoff/proof-summary.json',
  adapterDependencyHandoffRows:
    'output/screen-ai-pipeline-proof/adapter-dependency-handoff/adapter-dependency-handoff.json',
  finalProductPath: 'output/screen-ai-pipeline-proof/final-product-path/proof-summary.json',
  liveOperatorEvidenceBundle: 'output/screen-ai-pipeline-proof/live-operator-evidence-bundle/proof-summary.json',
  finalAdapterAudit: 'output/screen-ai-pipeline-proof/final-adapter-dependency-audit/proof-summary.json',
  linuxHostExecution: 'output/screen-ai-pipeline-proof/linux-host-adapter-execution/proof-summary.json',
  linuxWslgExternalGate: 'output/screen-plan-proof/linux-wslg-external-gate-analysis/proof-summary.json',
  androidPhysicalTargetReadiness: 'output/screen-plan-proof/android-physical-target-readiness/proof-summary.json',
  androidPhysicalExternalGate: 'output/screen-plan-proof/android-physical-external-gate-analysis/proof-summary.json',
  productChecklistDelta: 'output/screen-ai-pipeline-proof/product-checklist-delta/proof-summary.json',
  productChecklistDeltaMarkdown:
    'output/screen-ai-pipeline-proof/product-checklist-delta/product-capability-checklist-delta.md',
  pipelineChecklist: 'docs/plans/screen-ai-pipeline-plan/implementation-checklist.md',
};

const expectedRemainingAdapterDependencies = [
  {
    rowId: 'screen-ai-broad-installed-app-manual-required',
    boundary: 'broad-installed-app-blocking',
    handoffOwner: 'codex-c',
    expectedProofFile:
      'output/app-game-plan-proof/screen-derived-broad-installed-app-apply-rollback-audit/proof-summary.json',
    custodyArtifact: null,
    expectedContractKeys: [
      'sourcePolicyDecisionRef',
      'sourceActivityEvidenceRef',
      'applyResultRef',
      'rollbackOrExpiryRef',
      'auditRef',
      'rawImageRetained',
      'rawImageDeletedBeforeAdapter',
      'finalAdapterCompletionClaimed',
    ],
  },
  {
    rowId: 'screen-ai-host-network-domain-manual-required',
    boundary: 'host-network-domain-blocking',
    handoffOwner: 'E-D',
    expectedProofFile:
      'output/network-plan-proof/screen-derived-host-network-domain-apply-rollback-audit/proof-summary.json',
    custodyArtifact: null,
    expectedContractKeys: [
      'sourcePolicyDecisionRef',
      'sourceNetworkEvidenceRef',
      'applyResultRef',
      'rollbackOrExpiryRef',
      'auditRef',
      'rawImageRetained',
      'rawImageDeletedBeforeAdapter',
      'finalAdapterCompletionClaimed',
    ],
  },
  {
    rowId: 'screen-ai-managed-active-tab-not-claimed',
    boundary: 'managed-exact-active-tab-enforcement',
    handoffOwner: 'codex-d',
    expectedProofFile:
      'output/browser-plan-proof/screen-derived-managed-active-tab-apply-rollback-audit/proof-summary.json',
    custodyArtifact: null,
    expectedContractKeys: [
      'sourcePolicyDecisionRef',
      'sourceBrowserEvidenceRef',
      'applyResultRef',
      'rollbackOrExpiryRef',
      'auditRef',
      'rawImageRetained',
      'rawImageDeletedBeforeAdapter',
      'finalAdapterCompletionClaimed',
    ],
  },
  {
    rowId: 'screen-ai-android-mobile-control-manual-required',
    boundary: 'android-mobile-control-adapter',
    handoffOwner: 'primary/mobile-child-agent-sequencing',
    expectedProofFile:
      'output/mobile-plan-proof/screen-derived-android-mobile-control-apply-rollback-audit/proof-summary.json',
    custodyArtifact: 'output/screen-ai-pipeline-proof/android-mobile-control-custody/proof-summary.json',
    expectedContractKeys: [
      'sourcePolicyDecisionRef',
      'sourceMobileEvidenceRef',
      'applyResultRef',
      'rollbackOrExpiryRef',
      'auditRef',
      'rawImageRetained',
      'rawImageDeletedBeforeAdapter',
      'finalAdapterCompletionClaimed',
    ],
  },
  {
    rowId: 'screen-ai-ios-mobile-control-manual-required',
    boundary: 'ios-mobile-control-adapter',
    handoffOwner: 'primary/mobile-child-agent-sequencing',
    expectedProofFile:
      'output/mobile-plan-proof/screen-derived-ios-mobile-control-apply-rollback-audit/proof-summary.json',
    custodyArtifact: 'output/screen-ai-pipeline-proof/ios-mobile-control-custody/proof-summary.json',
    expectedContractKeys: [
      'sourcePolicyDecisionRef',
      'sourceMobileEvidenceRef',
      'applyResultRef',
      'rollbackOrExpiryRef',
      'auditRef',
      'rawImageRetained',
      'rawImageDeletedBeforeAdapter',
      'finalAdapterCompletionClaimed',
    ],
  },
];

const failures = [];
const screenPlanClosure = readJson(sourceArtifacts.screenPlanClosure);
const aiPlanClosure = readJson(sourceArtifacts.aiPlanClosure);
const adapterBlockerLedger = readJson(sourceArtifacts.adapterBlockerLedger);
const adapterDependencyHandoff = readJson(sourceArtifacts.adapterDependencyHandoff);
const adapterDependencyHandoffRows = readJson(sourceArtifacts.adapterDependencyHandoffRows);
const finalProductPath = readJson(sourceArtifacts.finalProductPath);
const liveOperatorEvidenceBundle = readJson(sourceArtifacts.liveOperatorEvidenceBundle);
const finalAdapterAudit = readJson(sourceArtifacts.finalAdapterAudit);
const linuxHostExecution = readJson(sourceArtifacts.linuxHostExecution);
const linuxWslgExternalGate = readJson(sourceArtifacts.linuxWslgExternalGate);
const androidPhysicalTargetReadiness = readOptionalJson(sourceArtifacts.androidPhysicalTargetReadiness);
const androidPhysicalExternalGate = readOptionalJson(sourceArtifacts.androidPhysicalExternalGate);
const productChecklistDelta = readJson(sourceArtifacts.productChecklistDelta);
const productChecklistDeltaMarkdown = readText(sourceArtifacts.productChecklistDeltaMarkdown);
const pipelineChecklist = readText(sourceArtifacts.pipelineChecklist);

assert(screenPlanClosure.assertions?.noProductCompleteClaim === true, 'screen-plan closure overclaims completion');
assert(
  screenPlanClosure.assertions?.finalProductPathRequiresAdapterAudit === true,
  'screen-plan closure does not require final adapter audit'
);
assert(
  screenPlanClosure.assertions?.adapterAuditKeepsProductCompletionBlocked === true,
  'screen-plan closure lost adapter blocker'
);
assert(
  screenPlanClosure.assertions?.liveViewEvidenceGatesProved === true,
  'screen-plan closure lost live-view evidence gates'
);
assert(
  screenPlanClosure.assertions?.liveViewProductReadyClaimed === false,
  'screen-plan closure overclaims live-view product readiness'
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
  'screen-plan closure lost foreground watcher proof'
);
assert(
  screenPlanClosure.assertions?.serviceEncryptedQueueExpiryDeletionProved === true,
  'screen-plan closure lost encrypted queue expiry deletion proof'
);
assert(
  screenPlanClosure.assertions?.deleteFailedVisibilityProved === true,
  'screen-plan closure lost delete-failed visibility proof'
);
assert(
  aiPlanClosure.closure?.controlledCapturedScreensAnalyzed === true,
  'AI closure lost controlled capture analysis'
);
assert(aiPlanClosure.closure?.liveOperatorArtifactsAnalyzed === true, 'AI closure lost live operator analysis');
assert(
  aiPlanClosure.closure?.serviceOcrAnalyzedCapturedPixels === true,
  'AI closure lost service OCR captured-pixel proof'
);
assert(
  aiPlanClosure.closure?.meshChecklistStatusConsistent === true,
  'AI closure lost household mesh checklist consistency guard'
);
assert(
  aiPlanClosure.closure?.householdProviderRouteSelectionCovered === true,
  'AI closure lost household provider route selection proof'
);
assert(
  aiPlanClosure.closure?.householdProviderAdvertisementHeartbeatCovered === true,
  'AI closure lost household provider advertisement/heartbeat proof'
);
assert(
  aiPlanClosure.closure?.householdProviderClaimLeaseCovered === true,
  'AI closure lost household provider claim/lease proof'
);
assert(aiPlanClosure.closure?.householdNoRawTransferCovered === true, 'AI closure lost no-raw-transfer proof');
assert(
  aiPlanClosure.closure?.householdProviderResultValidationCovered === true,
  'AI closure lost provider result validation proof'
);
assert(
  aiPlanClosure.closure?.householdMeshEventBridgeCovered === true,
  'AI closure lost household mesh event bridge proof'
);
assert(
  aiPlanClosure.closure?.childAgentPolicyAuthorityCovered === true,
  'AI closure lost child-agent policy authority proof'
);
assert(
  aiPlanClosure.closure?.mobileDormantProviderFallbackCovered === true,
  'AI closure lost mobile dormant provider fallback proof'
);
assert(aiPlanClosure.closure?.remoteApiAiClaimed === false, 'AI closure claims remote/API AI');
assert(aiPlanClosure.closure?.rawImageRetainedByDefault === false, 'AI closure permits raw image retention');
assert(
  aiPlanClosure.closure?.finalProductCompleteDeferredToPipeline === true,
  'AI closure no longer defers final completion to pipeline'
);

assert(finalProductPath.status === 'ok', 'final product path proof is not ok');
assert(finalProductPath.closure?.finalPathEvidenceComplete === true, 'final path evidence is incomplete');
assert(
  finalProductPath.closure?.screenAndAiPrerequisitesStacked === true,
  'final path does not stack screen and AI prerequisites'
);
assert(
  finalProductPath.closure?.serviceEventProducersAndSubscriberCovered === true,
  'final path lost service event producer/subscriber proof'
);
assert(
  finalProductPath.closure?.serviceWinRtOcrLivePolicyCovered === true,
  'final path lost service WinRT OCR policy proof'
);
assert(
  finalProductPath.closure?.singleRuntimeSessionRerun === true,
  'final path lost fresh service OCR source rerun proof'
);
assert(
  finalProductPath.closure?.householdRouteSelectionCovered === true,
  'final path lost household route selection coverage'
);
assert(
  finalProductPath.closure?.householdMeshBridgeMediated === true,
  'final path lost household mesh bridge mediation coverage'
);
assert(
  finalProductPath.closure?.childAgentPolicyAuthorityCovered === true,
  'final path lost child-agent policy authority coverage'
);
assert(
  finalProductPath.closure?.mobileDormantProviderFallbackCovered === true,
  'final path lost mobile dormant provider fallback coverage'
);
assert(finalProductPath.closure?.publicSocialSurfaceProof === true, 'final path lost public social surface proof');
assert(
  finalProductPath.closure?.retainedLiveOperatorBundlePortable === true,
  'final path lost portable live operator bundle requirement'
);
assert(
  typeof finalProductPath.closure?.authenticatedAccountSocialProof === 'boolean',
  'final path authenticated-account social proof state is missing'
);
assert(finalProductPath.closure?.rawScreenshotsRetainedByDefault === false, 'final path retains raw screenshots');
assert(finalProductPath.closure?.remoteAiUsedForChildSafety === false, 'final path uses remote AI for child safety');
assert(finalProductPath.closure?.finalPipelineProductComplete === false, 'final path claims product-complete');
assert(
  finalProductPath.closure?.finalPipelineProductCompleteBlockedByAdapterGate === true,
  'final path is not blocked by adapter gate'
);
assert(
  finalProductPath.closure?.adapterDependencyHandoffRequired === true,
  'final path does not require adapter dependency handoff'
);
assert(finalProductPath.closure?.adapterBlockerRowsMapped === 5, 'final path lost adapter blocker row mapping');
assert(
  finalProductPath.closure?.adapterDependencyRowsMapped === 5,
  'final path lost adapter dependency handoff row mapping'
);

assert(
  liveOperatorEvidenceBundle.proof === 'screen-ai-live-operator-evidence-bundle',
  'live operator evidence bundle proof id changed'
);
assert(liveOperatorEvidenceBundle.bundlePortableForReview === true, 'live operator bundle is not portable');
assert(liveOperatorEvidenceBundle.scenarioCount === 9, 'live operator bundle scenario count changed');
assert(liveOperatorEvidenceBundle.localVlmRows === 8, 'live operator bundle lost local VLM rows');
assert(liveOperatorEvidenceBundle.policyDryRunRows === 8, 'live operator bundle lost policy dry-run rows');
assert(liveOperatorEvidenceBundle.parentExplanationScreenshots === 8, 'live operator bundle lost screenshots');
assert(liveOperatorEvidenceBundle.rawScreenshotFilesCopied === false, 'live operator bundle copied raw screenshots');
assert(liveOperatorEvidenceBundle.encryptedQueueFilesCopied === false, 'live operator bundle copied encrypted queues');

assert(adapterBlockerLedger.status === 'blocked-but-actionable', 'adapter blocker ledger is not actionable');
assert(adapterBlockerLedger.closure?.blockerRows === 5, 'adapter blocker ledger row count changed');
assert(adapterBlockerLedger.closure?.claimUpgradeRows === 0, 'adapter blocker ledger contains claim upgrades');
assert(
  adapterDependencyHandoff.status === 'adapter-dependency-handoff-ready-upstream-execution-required',
  'adapter dependency handoff status changed'
);
assert(adapterDependencyHandoff.closure?.dependencyRowsMapped === 5, 'adapter dependency handoff row count changed');
assert(
  adapterDependencyHandoff.closure?.expectedProofFilesMapped === true,
  'adapter dependency handoff lost expected proof mapping'
);
assert(
  adapterDependencyHandoff.closure?.expectedContractShapesMapped === true,
  'adapter dependency handoff lost expected contract mapping'
);
assert(
  adapterDependencyHandoff.closure?.productCompleteClaimed === false,
  'adapter dependency handoff claims product completion'
);

assert(finalAdapterAudit.status === 'blocked-by-upstream-adapter-artifacts', 'adapter audit is not blocked');
assert(
  finalAdapterAudit.closure?.finalPathFreshServiceRerunProved === true,
  'adapter audit does not consume fresh service rerun proof'
);
assert(finalAdapterAudit.closure?.blockedAdapterRows === 5, 'adapter blocker row count changed');
assert(finalAdapterAudit.closure?.custodyArtifactRows === 3, 'adapter custody row count changed');
assert(finalAdapterAudit.closure?.linuxHostExecutionRows === 1, 'adapter audit lost Linux execution row');
assert(finalAdapterAudit.closure?.claimUpgradeRows === 0, 'adapter audit contains claim upgrades');
assert(
  linuxHostExecution.status === 'linux-host-adapter-execution-proved-wsl2',
  'Linux execution proof status changed'
);
assert(
  linuxHostExecution.closure?.linuxWsl2HostMutationExecuted === true,
  'Linux execution proof did not mutate the WSL2 host target'
);
assert(
  linuxHostExecution.closure?.linuxWsl2RollbackExecuted === true,
  'Linux execution proof did not roll back the WSL2 host target'
);
assert(
  linuxHostExecution.closure?.nativeLinuxDesktopProductReady === false,
  'Linux execution proof overclaims native Linux desktop readiness'
);
assert(
  linuxWslgExternalGate.assertions?.linuxExternalGateSatisfied === true,
  'Linux WSLg external gate proof is not satisfied'
);
assert(
  androidPhysicalExternalGate === null || androidPhysicalExternalGate.assertions?.androidExternalGateSatisfied === true,
  'Android physical external gate proof exists but is not satisfied'
);
assert(
  androidPhysicalTargetReadiness === null ||
    androidPhysicalTargetReadiness.assertions?.targetIsPhysicalAndroid === true,
  'Android physical target readiness proof exists but did not observe a physical target'
);

assert(
  productChecklistDelta.status === 'doc-delta-ready-product-checklist-locked',
  'product checklist delta status changed'
);
assert(productChecklistDelta.closure?.productChecklistEdited === false, 'product checklist delta edited checklist');
assert(
  productChecklistDelta.closure?.finalPathFreshServiceRerunProved === true,
  'product checklist delta does not carry fresh service rerun proof'
);
assert(
  productChecklistDelta.closure?.linuxWsl2HostExecutionProved === true,
  'product checklist delta does not carry Linux WSL2 execution proof'
);
assert(
  productChecklistDeltaMarkdown.includes('fresh service OCR source rerun evidence'),
  'product checklist delta markdown omits fresh service rerun evidence'
);
assert(
  productChecklistDeltaMarkdown.includes('WSL2 Linux host adapter execution proof'),
  'product checklist delta markdown omits Linux execution proof'
);
assert(
  pipelineChecklist.includes(
    '- [ ] Browser, network, mobile, and broad block adapters proven from screen-derived decisions before product-complete action claims.'
  ),
  'pipeline checklist product-complete adapter row is no longer open'
);

const blockedAdapterRows = finalAdapterAudit.nextRequiredArtifacts.map((row) => ({
  rowId: row.rowId,
  boundary: row.boundary,
  handoffOwner: row.handoffOwner,
  expectedProofFile: row.expectedProofFile,
  custodyArtifact: row.custodyArtifact,
  missingArtifact: row.missingArtifact,
}));

assertRemainingAdapterDependencies();

if (failures.length > 0) {
  throw new Error(
    `Screen AI full-scope readiness audit failed:\n${failures.map((failure) => `- ${failure}`).join('\n')}`
  );
}

const proof = {
  status:
    androidPhysicalExternalGate?.assertions?.androidExternalGateSatisfied === true
      ? 'ready-except-external-adapter-and-product-checklist-dependencies'
      : 'blocked-by-physical-android-external-gate-and-external-adapter-dependencies',
  proofKind: 'screen-ai-full-scope-readiness-audit',
  generatedAt: new Date().toISOString(),
  sourceArtifacts,
  closure: {
    screenPlanPrerequisitesAudited: true,
    aiPlanPrerequisitesAudited: true,
    finalPipelineEvidenceComplete: true,
    serviceEventRuntimeCovered: true,
    serviceCadenceRuntimeCovered: true,
    serviceDisabledSuppressionCovered: true,
    serviceForegroundRuntimeCovered: true,
    serviceDeletionCustodyCovered: true,
    serviceWinRtOcrPolicyFreshRerunCovered: true,
    screenPlanLiveViewEvidenceGatesCovered: true,
    liveViewProductReadyClaimed: false,
    householdMeshChecklistConsistent: true,
    householdProviderRouteSelectionCovered: true,
    householdProviderAdvertisementHeartbeatCovered: true,
    householdProviderClaimLeaseCovered: true,
    householdMeshNoRawProviderValidationCovered: true,
    householdMeshEventBridgeCovered: true,
    childAgentPolicyAuthorityCovered: true,
    mobileDormantProviderFallbackCovered: true,
    publicSocialSurfaceProof: true,
    authenticatedAccountSocialProof: finalProductPath.closure?.authenticatedAccountSocialProof === true,
    rawScreenshotsRetainedByDefault: false,
    remoteAiUsedForChildSafety: false,
    productChecklistDeltaReadyButNotApplied: true,
    linuxWsl2HostExecutionProved: true,
    linuxWslgExternalGateProved: true,
    androidPhysicalTargetReadinessRecorded: androidPhysicalTargetReadiness?.assertions?.targetObservedOnline === true,
    androidPhysicalTargetLockedBehindKeyguard:
      androidPhysicalTargetReadiness?.keyguard?.lockedBehindCredentialPrompt === true,
    androidPhysicalExternalGateProved: androidPhysicalExternalGate?.assertions?.androidExternalGateSatisfied === true,
    nativeLinuxDesktopProductReady: false,
    finalPipelineProductComplete: false,
    finalPipelineProductCompleteBlockedByAdapterGate: true,
    adapterDependencyHandoffRequired: true,
    adapterBlockerRowsMapped: adapterBlockerLedger.closure?.blockerRows,
    adapterDependencyRowsMapped: adapterDependencyHandoff.closure?.dependencyRowsMapped,
    physicalAndroidExternalGateRequired: androidPhysicalExternalGate?.assertions?.androidExternalGateSatisfied !== true,
    physicalAndroidUnlockRequired:
      androidPhysicalTargetReadiness?.keyguard?.unlockRequiredBeforeMediaProjection === true,
    externalAdapterDependencyRows: blockedAdapterRows.length,
    remainingAdapterDependencyOwnersStable: true,
    remainingAdapterExpectedProofFilesStable: true,
    remainingAdapterExpectedContractShapesStable: true,
    remainingAdapterProductCompleteRowOpen: true,
  },
  blockedAdapterRows: blockedAdapterRows.map((row) => ({
    ...row,
    expectedContractKeys: expectedRemainingAdapterDependencies.find((expected) => expected.rowId === row.rowId)
      ?.expectedContractKeys,
  })),
  productChecklistDelta: {
    status: productChecklistDelta.status,
    deltaMarkdown: productChecklistDelta.deltaMarkdown,
    productChecklistEdited: productChecklistDelta.closure?.productChecklistEdited === true,
    finalPathFreshServiceRerunProved: productChecklistDelta.closure?.finalPathFreshServiceRerunProved === true,
    linuxWsl2HostExecutionProved: productChecklistDelta.closure?.linuxWsl2HostExecutionProved === true,
  },
  nonClaims: [
    'This audit does not edit docs/product-capability-checklist.md.',
    'This audit does not claim product-complete screen, AI, or pipeline execution.',
    'This audit does not fabricate physical Android external proof; when absent, the physical Android gate remains a blocker while other local proofs stay auditable.',
    'This audit may cite physical Android target-readiness proof to explain a locked-device blocker, but target readiness is not capture proof.',
    'This audit does not implement broad installed-app, host network/domain, managed active-tab, Android, iOS, or native Linux desktop product-complete execution artifacts.',
    'This audit does not replace live macOS, native Linux Wayland/PipeWire/root-display, physical Android, physical iOS, authenticated-account social, live-view production, or production OCR/VLM quality gates still listed by the screen-plan closure audit.',
  ],
};

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
writeFileSync(snapshotPath, sourceSnapshot(proof));
writeFileSync(commandsPath, validationCommands());
console.log(`screen-ai-full-scope-readiness-audit-ok:${relativePath(proofPath)}`);

function readJson(path) {
  return JSON.parse(readText(path));
}

function readOptionalJson(path) {
  const absolute = resolve(repoRoot, path);
  return existsSync(absolute) ? JSON.parse(readFileSync(absolute, 'utf8')) : null;
}

function readText(path) {
  const absolute = resolve(repoRoot, path);
  assert(existsSync(absolute), `missing source artifact ${path}`);
  return readFileSync(absolute, 'utf8');
}

function assertRemainingAdapterDependencies() {
  assert(
    blockedAdapterRows.length === expectedRemainingAdapterDependencies.length,
    'remaining adapter dependency count changed'
  );

  for (const expected of expectedRemainingAdapterDependencies) {
    const blockedRow = blockedAdapterRows.find((row) => row.rowId === expected.rowId);
    assert(Boolean(blockedRow), `missing blocked adapter row ${expected.rowId}`);
    if (!blockedRow) {
      continue;
    }

    assert(blockedRow.boundary === expected.boundary, `${expected.rowId} boundary changed`);
    assert(blockedRow.handoffOwner === expected.handoffOwner, `${expected.rowId} owner changed`);
    assert(blockedRow.expectedProofFile === expected.expectedProofFile, `${expected.rowId} expected proof changed`);
    assert(
      (blockedRow.custodyArtifact ?? null) === expected.custodyArtifact,
      `${expected.rowId} custody artifact changed`
    );

    const handoffRows = adapterDependencyHandoffRows.rows ?? [];
    const handoffRow = handoffRows.find((row) => row.rowId === expected.rowId);
    assert(Boolean(handoffRow), `missing dependency handoff row ${expected.rowId}`);
    if (!handoffRow) {
      continue;
    }

    assert(handoffRow.owningLane === expected.handoffOwner, `${expected.rowId} handoff owner changed`);
    assert(handoffRow.expectedProofFile === expected.expectedProofFile, `${expected.rowId} handoff proof changed`);
    assert(
      handoffRow.expectedContractShape?.rawImageRetained === false,
      `${expected.rowId} handoff allows raw image retention`
    );
    assert(
      handoffRow.expectedContractShape?.rawImageDeletedBeforeAdapter === true,
      `${expected.rowId} handoff does not require image deletion before adapter`
    );
    assert(
      handoffRow.expectedContractShape?.finalAdapterCompletionClaimed === true,
      `${expected.rowId} completed upstream proof would not claim its own adapter completion`
    );

    for (const key of expected.expectedContractKeys) {
      assert(
        Object.hasOwn(handoffRow.expectedContractShape ?? {}, key),
        `${expected.rowId} expected contract shape is missing ${key}`
      );
    }
  }
}

function sourceSnapshot(proof) {
  const rows = Object.entries(sourceArtifacts)
    .map(([name, path]) => `- ${name}: \`${path}\``)
    .join('\n');
  return [
    '# Screen AI Full Scope Readiness Audit',
    '',
    `Generated: ${proof.generatedAt}`,
    '',
    '## Source Artifacts',
    '',
    rows,
    '',
    '## Closure',
    '',
    '```json',
    JSON.stringify(proof.closure, null, 2),
    '```',
    '',
  ].join('\n');
}

function validationCommands() {
  return [
    'node --check scripts/test/screen-ai-full-scope-readiness-audit.mjs',
    'node scripts/test/screen-ai-full-scope-readiness-audit.mjs',
    'git diff --check',
    'npm run lanes:guard',
    'npm run hub:guard',
    '',
  ].join('\n');
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}

function assert(condition, message) {
  if (!condition) {
    failures.push(message);
  }
}
