import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-plan-proof', 'screen-plan-closure-audit');
const proofPath = join(outputRoot, 'proof-summary.json');
const checklistPath = join(repoRoot, 'docs', 'plans', 'screen-plan', 'implementation-checklist.md');
const windowsOcrSelectionPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  'windows-ocr-candidate-selection',
  'proof-summary.json'
);

const checklist = readText(checklistPath);
const windowsOcrSelection = readJson(windowsOcrSelectionPath);
const externalGatesPath = join(repoRoot, 'output', 'screen-plan-proof', 'external-gates', 'proof-summary.json');
const externalGates = readJson(externalGatesPath);
const localPlatformProofPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  'local-platform-proof-batch',
  'proof-summary.json'
);
const localPlatformProof = readJson(localPlatformProofPath);
const finalProductPathPath = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'final-product-path',
  'proof-summary.json'
);
const finalProductPath = readJson(finalProductPathPath);
const finalAdapterAuditPath = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'final-adapter-dependency-audit',
  'proof-summary.json'
);
const finalAdapterAudit = readJson(finalAdapterAuditPath);
const retentionSweeperPath = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'service-retention-sweeper',
  'proof-summary.json'
);
const retentionSweeper = readJson(retentionSweeperPath);
const deletionRetentionCustodyPath = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'deletion-retention-custody',
  'proof-summary.json'
);
const deletionRetentionCustody = readJson(deletionRetentionCustodyPath);
const serviceForegroundPath = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'service-foreground',
  'proof-summary.json'
);
const serviceForeground = readJson(serviceForegroundPath);
const serviceCadencePath = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'service-cadence',
  'proof-summary.json'
);
const serviceCadence = readJson(serviceCadencePath);
const serviceDisabledSuppressionPath = join(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'service-disabled-suppression',
  'proof-summary.json'
);
const serviceDisabledSuppression = readJson(serviceDisabledSuppressionPath);
const adapterCustodyArtifacts = [
  {
    label: 'Linux host adapter custody',
    path: 'output/screen-ai-pipeline-proof/linux-host-adapter-custody/proof-summary.json',
    expectedStatus: 'linux-host-custody-artifact-written-final-execution-blocked',
  },
  {
    label: 'Android mobile-control custody',
    path: 'output/screen-ai-pipeline-proof/android-mobile-control-custody/proof-summary.json',
    expectedStatus: 'android-mobile-control-custody-artifact-written-final-execution-blocked',
  },
  {
    label: 'iOS mobile-control custody',
    path: 'output/screen-ai-pipeline-proof/ios-mobile-control-custody/proof-summary.json',
    expectedStatus: 'ios-mobile-control-custody-artifact-written-final-execution-blocked',
  },
].map((artifact) => ({
  ...artifact,
  summary: readJson(join(repoRoot, artifact.path)),
}));
const liveViewArtifacts = [
  {
    label: 'live-view loopback transport',
    path: 'output/screen-plan-proof/live-view-session-transport/proof-summary.json',
    assert(summary) {
      return (
        summary.assertions?.realPixelsCaptured === true &&
        summary.assertions?.localTransportDeliveredFrame === true &&
        summary.assertions?.rawFrameDeletedAfterTransport === true &&
        summary.assertions?.noRawFrameRetention === true &&
        summary.assertions?.noRemoteInput === true
      );
    },
  },
  {
    label: 'live-view platform permission gate',
    path: 'output/screen-plan-proof/live-view-platform-permission/proof-summary.json',
    assert(summary) {
      return (
        summary.assertions?.productionReadinessRequiresStructuredEvidence === true &&
        summary.assertions?.productionReadinessRejectsCaptureOnlyPermission === true &&
        summary.assertions?.productionReadinessRejectsRawFramePromptArtifact === true &&
        summary.gapStatus?.liveViewProductReady === false
      );
    },
  },
  {
    label: 'live-view parent UI persistence',
    path: 'output/screen-plan-proof/live-view-parent-ui-persistence/proof-summary.json',
    assert(summary) {
      return (
        summary.assertions?.parentSettingsRouteRenderedLiveViewRow === true &&
        summary.assertions?.explicitParentOptInPersisted === true &&
        summary.assertions?.productLiveViewStillBlocked === true &&
        summary.assertions?.noFrameRetentionNoRecordingNoRemoteInput === true
      );
    },
  },
  {
    label: 'live-view service session boundary',
    path: 'output/screen-plan-proof/live-view-service-session/proof-summary.json',
    assert(summary) {
      return (
        summary.assertions?.realLoopbackTransportArtifactConsumed === true &&
        summary.assertions?.rawFrameDeletionCarriedForward === true &&
        summary.assertions?.productReadinessOverclaimRejected === true &&
        summary.gapStatus?.liveViewProductReady === false
      );
    },
  },
  {
    label: 'live-view runtime boundary',
    path: 'output/screen-plan-proof/live-view-runtime/proof-summary.json',
    assert(summary) {
      return (
        summary.assertions?.rustRuntimeValidationPassed === true &&
        summary.assertions?.productReadinessStillBlocked === true &&
        summary.gapStatus?.liveViewProductReady === false
      );
    },
  },
  {
    label: 'live-view worker startup gate',
    path: 'output/screen-plan-proof/live-view-worker-startup/proof-summary.json',
    assert(summary) {
      return (
        summary.assertions?.rustWorkerStartupValidationPassed === true &&
        summary.assertions?.workerRemainsStoppedWithoutExternalGates === true &&
        summary.assertions?.serviceWorkerExecutionRecordStartsAfterAllGates === true &&
        summary.gapStatus?.liveViewProductReady === false
      );
    },
  },
  {
    label: 'live-view forced relay/cache execution',
    path: 'output/screen-plan-proof/live-view-relay-cache/proof-summary.json',
    assert(summary) {
      return (
        summary.assertions?.realPixelsCaptured === true &&
        summary.assertions?.relayEnvelopeEncrypted === true &&
        summary.assertions?.relayCacheDeletedAfterDelivery === true &&
        summary.assertions?.rawFrameDeletedAfterRelay === true &&
        summary.assertions?.noRawFrameCache === true &&
        summary.assertions?.noSessionRecording === true &&
        summary.assertions?.noRemoteInput === true
      );
    },
  },
].map((artifact) => ({
  ...artifact,
  summary: readJson(join(repoRoot, artifact.path)),
}));
const workpacks = [
  {
    id: '10',
    label: 'macOS capture adapter plan/proof',
    status: workpackStatus('10 macOS capture adapter plan/proof'),
    readinessProof: 'output/screen-plan-proof/macos/proof-summary.json',
    productReadyField: 'gapStatus.productMacosCaptureReady',
    gate: 'Requires real macOS ScreenCaptureKit/permission proof on macOS hardware.',
  },
  {
    id: '11',
    label: 'Linux capture adapter plan/proof',
    status: workpackStatus('11 Linux capture adapter plan/proof'),
    readinessProof: 'output/screen-plan-proof/linux/proof-summary.json',
    productReadyField: 'gapStatus.productLinuxCaptureReady',
    gate: 'WSLg/X11 selected-window plus local VLM external-gate proof exists; native Linux root-display and Wayland/PipeWire portal parity still require real Linux desktop session proof.',
  },
  {
    id: '12',
    label: 'Android MediaProjection adapter plan/proof',
    status: workpackStatus('12 Android MediaProjection adapter plan/proof'),
    readinessProof: 'output/screen-plan-proof/android/proof-summary.json',
    productReadyField: 'gapStatus.productAndroidCaptureReady',
    gate: 'Emulator consent/capture/deletion and stop-callback behavior are proved; physical-device capture/deletion and local OCR on physical Android capture remain required before broad Android product claim.',
  },
  {
    id: '13',
    label: 'iOS ReplayKit adapter plan/proof',
    status: workpackStatus('13 iOS ReplayKit adapter plan/proof'),
    readinessProof: 'output/screen-plan-proof/ios/proof-summary.json',
    productReadyField: 'gapStatus.productIosCaptureReady',
    gate: 'Requires real iOS ReplayKit entitlement/device proof.',
  },
  {
    id: '28',
    label: 'Live view optional mode',
    status: workpackStatus('28 Live view optional mode'),
    readinessProof: 'output/screen-plan-proof/live-view-worker-startup/proof-summary.json',
    productReadyField: null,
    gate: 'Fail-closed platform permission gate, structured production-readiness evidence bundle, local loopback live-frame transport proof, parent UI persistence proof, service-session readiness boundary proof, Rust service runtime decision proof, worker startup gate proof, and local forced-relay/cache execution proof exist; real platform live-view prompt screenshots, actual production worker start, physical-device parity, hosted relay infrastructure, and privacy/legal approval remain.',
  },
  {
    id: '30',
    label: 'Test suite, Playwright, rollout, PR gate',
    status: workpackStatus('30 Test suite, Playwright, rollout, PR gate'),
    readinessProof: null,
    productReadyField: null,
    gate: 'Final closure waits for partial platform/model gates or explicit product non-claim handoff.',
  },
  {
    id: '34',
    label: 'OCR Tesseract baseline',
    status: workpackStatus('34 OCR Tesseract baseline'),
    readinessProof: 'output/screen-plan-proof/34-ocr-tesseract-baseline/proof-summary.json',
    productReadyField: null,
    gate: 'Local Tesseract extraction, CPU/memory measurement, derived failure-mode capture, and same-image comparison against the isolated local PaddleOCR 2.x fallback are proved; Tesseract is retained as a measured fallback while the current Windows service OCR route is WinRT and current PP-OCRv5 still extracts zero text. Cross-platform OCR parity, broad language coverage, and final production quality remain open.',
  },
  {
    id: '35',
    label: 'OCR PaddleOCR/PP-OCR evaluation',
    status: workpackStatus('35 OCR PaddleOCR/PP-OCR evaluation'),
    readinessProof: 'output/screen-plan-proof/35-ocr-paddleocr-ppocr-evaluation/proof-summary.json',
    productReadyField: null,
    gate: 'Current PP-OCRv5 mobile-detector inference and cached server-detector inference now run locally with oneDNN/MKLDNN disabled but extract zero text from the real proof image, and deleted preprocessing variants also extract zero text; an isolated pinned PaddleOCR 2.x fallback extracts comparable text locally. PaddleOCR is not selected; current Windows service OCR route selection is WinRT, while PP-OCRv5 quality/resource resolution and broad quality remain open.',
  },
  {
    id: '36',
    label: 'Small VLM guided classifier evaluation',
    status: workpackStatus('36 Small VLM guided classifier evaluation'),
    readinessProof: 'output/screen-plan-proof/36-small-vlm-guided-classifier-evaluation/proof-summary.json',
    productReadyField: null,
    gate: 'Evaluation complete for the current Windows proof route: local llama.cpp/Qwen2-VL runtime detection, retained controlled local VLM matrix, retained nine-scenario live operator matrix, bounded retained VLM inputs, managed-browser CDP crop capture path, retained proof-image VLM wall/CPU/RSS measurement, public-live video/school/game/shopping/social-feed CDP crop quality, current Windows local VLM route selection, and measured rollout/fallback gate are proved. Authenticated-account social proof, production model quality, cross-platform parity, and broader hardware rollout thresholds remain non-claims.',
  },
];

const completeRows = [...checklist.matchAll(/\| \[x\]\s+\|\s+([^|]+?)\s+\|/g)].map((match) => match[1].trim());
const partialRows = [...checklist.matchAll(/\| \[~\]\s+\|\s+([^|]+?)\s+\|/g)].map((match) => match[1].trim());
const openRows = [...checklist.matchAll(/\| \[ \]\s+\|\s+([^|]+?)\s+\|/g)].map((match) => match[1].trim());
const auditedWorkpacks = workpacks.map((workpack) => {
  const readinessProofPresent =
    workpack.readinessProof === null ? null : existsSync(join(repoRoot, workpack.readinessProof));
  const readiness = workpack.readinessProof === null ? null : readJson(join(repoRoot, workpack.readinessProof));
  const productReady =
    readiness === null || workpack.productReadyField === null
      ? null
      : readNested(readiness, workpack.productReadyField);
  return {
    ...workpack,
    readinessProofPresent,
    productReady,
  };
});
const missingReadinessProofs = auditedWorkpacks.filter(
  (workpack) => workpack.readinessProof !== undefined && workpack.readinessProofPresent !== true
);
const productBlockedWorkpacks = auditedWorkpacks.filter((workpack) => workpack.productReady === false);
const remainingProductGates = auditedWorkpacks.filter(
  (workpack) => workpack.status !== 'x' || workpack.productReady === false
);

assert(
  completeRows.includes('19 Sensitive text and redaction model'),
  'WP19 must remain closed after selected-policy proof.'
);
assert(
  partialRows.includes('28 Live view optional mode'),
  'Live view must remain partial until real platform transport proof exists.'
);
assert(
  partialRows.includes('13 iOS ReplayKit adapter plan/proof'),
  'iOS ReplayKit must remain partial after source-doc readiness but before real iOS proof.'
);
assert(missingReadinessProofs.length === 0, 'Closure audit expects current readiness proof artifacts to exist.');
assert(
  productBlockedWorkpacks.length >= 4,
  'Closure audit expects platform readiness artifacts to keep product readiness blocked.'
);
assert(
  auditedWorkpacks.some(
    (workpack) => workpack.label === 'macOS capture adapter plan/proof' && workpack.productReady === false
  ),
  'macOS capture must remain product-blocked until live macOS proof exists.'
);
assert(
  auditedWorkpacks.some(
    (workpack) => workpack.label === 'Linux capture adapter plan/proof' && workpack.productReady === false
  ),
  'Linux capture must remain product-blocked until native Linux session proof exists.'
);
assert(
  auditedWorkpacks.some(
    (workpack) => workpack.label === 'Android MediaProjection adapter plan/proof' && workpack.productReady === false
  ),
  'Android capture must remain product-blocked until physical parity proof exists.'
);
assert(
  auditedWorkpacks.some(
    (workpack) => workpack.label === 'iOS ReplayKit adapter plan/proof' && workpack.productReady === false
  ),
  'iOS capture must remain product-blocked until physical ReplayKit proof exists.'
);
assert(
  windowsOcrSelection.assertions?.windowsServiceOcrSelected === true,
  'Closure audit expects the Windows OCR route selection artifact to select WinRT OCR.'
);
assert(
  windowsOcrSelection.selectedCurrentRoute?.modelId === 'windows-winrt-ocr',
  'Closure audit expects Windows OCR selection to name windows-winrt-ocr.'
);
assert(
  externalGates.assertions?.currentBranchMustRemainNonClaim === true,
  'Closure audit expects external gates to keep the screen plan in non-claim state.'
);
assert(
  externalGates.assertions?.authenticatedAccountSocialGateEnumerated === true,
  'Closure audit expects authenticated-account social proof to be an explicit external gate.'
);
assert(
  externalGates.counts?.missingGateCount > 0,
  'Closure audit expects remaining external gates to be missing until real artifacts are attached.'
);
assert(
  localPlatformProof.closure?.windowsCaptureComplete === true,
  'Local platform batch must prove Windows capture complete.'
);
assert(
  localPlatformProof.closure?.androidEmulatorCaptureComplete === true,
  'Local platform batch must prove Android emulator MediaProjection capture complete.'
);
assert(
  typeof localPlatformProof.closure?.androidPhysicalCaptureComplete === 'boolean',
  'Local platform batch must account for Android physical parity.'
);
assert(
  localPlatformProof.closure?.linuxWslgCaptureComplete === true,
  'Local platform batch must prove Linux WSLg selected-window capture complete.'
);
assert(
  localPlatformProof.closure?.linuxWslgExternalGateComplete === true,
  'Local platform batch must prove Linux WSLg external gate complete.'
);
assert(
  externalGates.gateResults?.some(
    (gate) => gate.gateId === 'linux-desktop-session-capture' && gate.status === 'satisfied'
  ) === true,
  'Closure audit expects the Linux desktop-session external gate to be satisfied by the WSLg proof.'
);
assert(
  localPlatformProof.closure?.nativeLinuxWaylandComplete === false,
  'Local platform batch must keep native Linux Wayland/PipeWire parity open.'
);
assert(localPlatformProof.closure?.macosCaptureComplete === false, 'macOS capture must remain external here.');
assert(localPlatformProof.closure?.iosCaptureComplete === false, 'iOS capture must remain external here.');
assert(
  localPlatformProof.closure?.productCompletePlatformCaptureReady === false,
  'Local platform batch must not claim full platform product readiness.'
);
assert(finalProductPath.closure?.finalAdapterAuditProven === true, 'Final path must prove final adapter audit.');
assert(
  finalProductPath.closure?.adapterProductCompleteBlockedByAudit === true,
  'Final path must keep product-complete adapters blocked by audit.'
);
assert(
  finalProductPath.closure?.broadBrowserNetworkMobileProductComplete === false,
  'Final path must not claim broad/browser/network/mobile product completion.'
);
assert(
  finalAdapterAudit.closure?.custodyArtifactRows === adapterCustodyArtifacts.length,
  'Final adapter audit must consume all custody artifacts.'
);
assert(finalAdapterAudit.closure?.claimUpgradeRows === 0, 'Final adapter audit must not contain claim upgrades.');
assert(
  finalAdapterAudit.custodyRows?.every((row) => row.finalAdapterCompletionClaimed === false) === true,
  'Final adapter custody rows must not claim final adapter completion.'
);
assert(
  retentionSweeper.assertions?.capturePhaseCreatedEncryptedExpiringQueueRecord === true,
  'Retention sweeper proof must show the service capture phase created encrypted expiring queue records.'
);
assert(
  retentionSweeper.assertions?.retentionSweeperRemovedExpiredQueueRecord === true,
  'Retention sweeper proof must show expired encrypted queue records were removed.'
);
assert(
  retentionSweeper.assertions?.expiredDeletionSurfacedInActivityReadModel === true,
  'Retention sweeper proof must surface expired deletion in the Activity Screen read model.'
);
assert(
  deletionRetentionCustody.assertions?.deleteFailureRemainsVisible === true,
  'Deletion custody proof must keep delete-failure state visible.'
);
assert(
  deletionRetentionCustody.assertions?.readModelSurfacesExpiredAndDeleteFailedCounts === true,
  'Deletion custody proof must surface expired and delete-failed counts in the read model.'
);
assert(
  serviceForeground.assertions?.foregroundWatcherCapturedBeforeSecondFocus === true,
  'Service foreground proof must show capture before the second foreground focus.'
);
assert(
  serviceForeground.assertions?.foregroundWatcherCapturedAfterSecondWindowFocus === true,
  'Service foreground proof must show capture after the second foreground focus.'
);
assert(
  serviceForeground.assertions?.activityReadModelReachedViaWebSocket === true,
  'Service foreground proof must reach the Activity Screen read model through WebSocket.'
);
assert(
  serviceCadence.assertions?.threeTimedCadenceFramesCaptured === true,
  'Service cadence proof must show three timed cadence frames.'
);
assert(
  serviceCadence.assertions?.queueBackpressureHeldAtThreePendingFrames === true,
  'Service cadence proof must show pending queue backpressure.'
);
assert(
  serviceCadence.assertions?.activityReadModelReachedViaWebSocket === true,
  'Service cadence proof must reach the Activity Screen read model through WebSocket.'
);
assert(
  serviceDisabledSuppression.assertions?.disabledPhaseCreatedNoNewCaptureRows === true,
  'Disabled suppression proof must prevent new capture rows.'
);
assert(
  serviceDisabledSuppression.assertions?.disabledPhaseCreatedNoNewQueueRecords === true,
  'Disabled suppression proof must prevent new queue records.'
);
assert(
  serviceDisabledSuppression.assertions?.disabledPhaseCreatedNoLocalVisionRows === true,
  'Disabled suppression proof must prevent local vision rows.'
);
for (const artifact of adapterCustodyArtifacts) {
  assert(artifact.summary.status === artifact.expectedStatus, `${artifact.label} status changed.`);
  assert(
    artifact.summary.closure?.finalAdapterCompletionClaimed === false,
    `${artifact.label} must not claim final adapter completion.`
  );
  assert(
    artifact.summary.closure?.productCompleteAdapterRowStillOpen === true,
    `${artifact.label} must keep product-complete adapter row open.`
  );
}
for (const artifact of liveViewArtifacts) {
  assert(artifact.assert(artifact.summary), `${artifact.label} live-view closure assertion failed.`);
}
assert(
  liveViewArtifacts.every((artifact) =>
    (artifact.summary.nonClaims ?? []).some((nonClaim) => nonClaim.toLowerCase().includes('product'))
  ),
  'Every live-view artifact must preserve a product-readiness non-claim.'
);

const summary = {
  proof: 'screen-plan-closure-audit',
  generatedAt: new Date().toISOString(),
  branchScope: 'codex/screen-ai-full-scope-b',
  checklist: {
    path: relativePath(checklistPath),
    completeCount: completeRows.length,
    partialCount: partialRows.length,
    openCount: openRows.length,
    completeRows,
    partialRows,
    openRows,
  },
  auditedProofGates: auditedWorkpacks.map((workpack) => ({
    id: workpack.id,
    label: workpack.label,
    status: workpack.status,
    readinessProof: workpack.readinessProof,
    readinessProofPresent: workpack.readinessProofPresent,
    productReady: workpack.productReady,
    gate: workpack.gate,
  })),
  remainingProductGates: remainingProductGates.map((workpack) => ({
    id: workpack.id,
    label: workpack.label,
    status: workpack.status,
    productReady: workpack.productReady,
    gate: workpack.gate,
  })),
  currentWindowsEvidence: [
    'output/screen-ai-pipeline-proof/service-winrt-ocr-redaction/proof-summary.json',
    'output/screen-ai-pipeline-proof/service-winrt-ocr-redaction/portal-screen-analysis-redaction.png',
    'output/screen-ai-pipeline-proof/service-winrt-ocr-redaction/parent-redaction-policy.json',
    'output/screen-ai-pipeline-proof/final-product-path/proof-summary.json',
    'output/screen-ai-pipeline-proof/final-adapter-dependency-audit/proof-summary.json',
    'output/screen-ai-pipeline-proof/service-retention-sweeper/proof-summary.json',
    'output/screen-ai-pipeline-proof/deletion-retention-custody/proof-summary.json',
    'output/screen-ai-pipeline-proof/service-foreground/proof-summary.json',
    'output/screen-ai-pipeline-proof/service-cadence/proof-summary.json',
    'output/screen-ai-pipeline-proof/service-disabled-suppression/proof-summary.json',
    'output/screen-ai-pipeline-proof/linux-host-adapter-custody/proof-summary.json',
    'output/screen-ai-pipeline-proof/android-mobile-control-custody/proof-summary.json',
    'output/screen-ai-pipeline-proof/ios-mobile-control-custody/proof-summary.json',
    ...liveViewArtifacts.map((artifact) => artifact.path),
    'output/screen-plan-proof/external-gates/proof-summary.json',
    'output/screen-plan-proof/local-platform-proof-batch/proof-summary.json',
    'output/screen-plan-proof/linux-wslg-external-gate-analysis/proof-summary.json',
    'output/screen-plan-proof/android-physical-external-gate-analysis/proof-summary.json',
    'output/screen-plan-proof/macos/proof-summary.json',
    'output/screen-plan-proof/linux/proof-summary.json',
    'output/screen-plan-proof/android/proof-summary.json',
    'output/screen-plan-proof/ios/proof-summary.json',
    'output/screen-plan-proof/windows-ocr-candidate-selection/proof-summary.json',
    'output/screen-plan-proof/36-vlm-resource-crop-readiness/proof-summary.json',
    'output/screen-plan-proof/36-vlm-runtime-resource-measurement/proof-summary.json',
    'output/screen-plan-proof/36-vlm-live-crop-quality/proof-summary.json',
    'output/screen-plan-proof/36-vlm-model-selection/proof-summary.json',
    'output/screen-plan-proof/36-vlm-rollout-fallback-gate/proof-summary.json',
  ].map((artifact) => ({
    artifact,
    present: existsSync(join(repoRoot, artifact)),
  })),
  assertions: {
    wp19Closed: completeRows.includes('19 Sensitive text and redaction model'),
    remainingGatesExplicit: partialRows.length + openRows.length > 0,
    readinessProofsPresent: missingReadinessProofs.length === 0,
    platformProductGatesRemainBlocked: productBlockedWorkpacks.length >= 4,
    wp34TesseractBaselineClosed: completeRows.includes('34 OCR Tesseract baseline'),
    externalGatesKeepProductNonClaim: externalGates.assertions.currentBranchMustRemainNonClaim === true,
    localWindowsAndroidLinuxProofsAccounted:
      localPlatformProof.closure.windowsCaptureComplete === true &&
      localPlatformProof.closure.androidEmulatorCaptureComplete === true &&
      typeof localPlatformProof.closure.androidPhysicalCaptureComplete === 'boolean' &&
      localPlatformProof.closure.linuxWslgCaptureComplete === true &&
      localPlatformProof.closure.linuxWslgExternalGateComplete === true,
    localPlatformExternalGatesRemainBlocked:
      localPlatformProof.closure.nativeLinuxWaylandComplete === false &&
      localPlatformProof.closure.macosCaptureComplete === false &&
      localPlatformProof.closure.iosCaptureComplete === false,
    finalProductPathRequiresAdapterAudit: finalProductPath.closure.finalAdapterAuditProven === true,
    adapterAuditKeepsProductCompletionBlocked:
      finalProductPath.closure.adapterProductCompleteBlockedByAudit === true &&
      finalAdapterAudit.closure.broadBrowserNetworkMobileProductComplete === false,
    serviceEncryptedQueueExpiryDeletionProved:
      retentionSweeper.assertions.capturePhaseCreatedEncryptedExpiringQueueRecord === true &&
      retentionSweeper.assertions.retentionSweeperRemovedExpiredQueueRecord === true &&
      retentionSweeper.assertions.expiredDeletionSurfacedInActivityReadModel === true,
    deleteFailedVisibilityProved:
      deletionRetentionCustody.assertions.deleteFailureRemainsVisible === true &&
      deletionRetentionCustody.assertions.readModelSurfacesExpiredAndDeleteFailedCounts === true,
    serviceForegroundWatcherProved:
      serviceForeground.assertions.foregroundWatcherCapturedBeforeSecondFocus === true &&
      serviceForeground.assertions.foregroundWatcherCapturedAfterSecondWindowFocus === true &&
      serviceForeground.assertions.activityReadModelReachedViaWebSocket === true,
    serviceCadenceRuntimeProved:
      serviceCadence.assertions.threeTimedCadenceFramesCaptured === true &&
      serviceCadence.assertions.queueBackpressureHeldAtThreePendingFrames === true &&
      serviceCadence.assertions.activityReadModelReachedViaWebSocket === true,
    serviceDisabledSuppressionProved:
      serviceDisabledSuppression.assertions.disabledPhaseCreatedNoNewCaptureRows === true &&
      serviceDisabledSuppression.assertions.disabledPhaseCreatedNoNewQueueRecords === true &&
      serviceDisabledSuppression.assertions.disabledPhaseCreatedNoLocalVisionRows === true,
    custodyArtifactsDoNotUpgradeClaims: adapterCustodyArtifacts.every(
      (artifact) =>
        artifact.summary.closure.finalAdapterCompletionClaimed === false &&
        artifact.summary.closure.productCompleteAdapterRowStillOpen === true
    ),
    liveViewEvidenceGatesProved: liveViewArtifacts.every((artifact) => artifact.assert(artifact.summary)),
    liveViewProductReadyClaimed: false,
    noProductCompleteClaim: true,
  },
  nonClaims: [
    'This audit does not complete macOS, native Linux Wayland/PipeWire/root-display parity, Android physical parity, iOS, live-view platform prompt screenshots/actual production worker start/physical-device parity/hosted relay infrastructure/privacy-legal approval, current PP-OCRv5 quality resolution, cross-platform OCR parity, authenticated-account social proof, or broader VLM hardware rollout-threshold gates.',
    'This audit does not replace real device/runtime proof for remaining partial rows.',
    'This audit exists to prevent product-complete wording before the remaining external proof gates are satisfied.',
  ],
};

mkdirSync(outputRoot, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-plan-closure-audit-proof-ok:${proofPath}`);

function workpackStatus(label) {
  const escaped = label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = checklist.match(new RegExp(`\\| \\[([^\\]]*)\\]\\s+\\|\\s+${escaped}\\s+\\|`));
  assert(match, `Missing checklist row: ${label}`);
  return match[1].trim() || 'open';
}

function readText(path) {
  return readFileSync(path, 'utf8');
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function readNested(value, path) {
  return path.split('.').reduce((current, key) => current?.[key], value);
}

function relativePath(path) {
  return path.replace(`${repoRoot}\\`, '').replaceAll('\\', '/');
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
