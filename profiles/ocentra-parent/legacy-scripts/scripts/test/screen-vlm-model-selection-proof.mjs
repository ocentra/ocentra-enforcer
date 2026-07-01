import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..', '..');
const outputDir = join(repoRoot, 'output', 'screen-plan-proof', '36-vlm-model-selection');
const proofPath = join(outputDir, 'proof-summary.json');

const readinessPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  '36-small-vlm-guided-classifier-evaluation',
  'proof-summary.json'
);
const resourceCropPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  '36-vlm-resource-crop-readiness',
  'proof-summary.json'
);
const runtimeMeasurementPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  '36-vlm-runtime-resource-measurement',
  'proof-summary.json'
);
const liveCropQualityPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  '36-vlm-live-crop-quality',
  'proof-summary.json'
);

const generatedAt = new Date().toISOString();
const readiness = readJson(readinessPath);
const resourceCrop = readJson(resourceCropPath);
const runtimeMeasurement = readJson(runtimeMeasurementPath);
const liveCropQuality = readJson(liveCropQualityPath);

const selectedRoute = {
  selectionStatus: 'selected-current-windows-local-proof-route',
  providerKind: 'localVision',
  runtimeBinary: runtimeMeasurement.modelRuntime.runtimeBinary,
  model: runtimeMeasurement.modelRuntime.model,
  mmproj: runtimeMeasurement.modelRuntime.mmproj,
  modelRuntimeRef: runtimeMeasurement.modelRuntime.modelRuntimeRef,
  promptOrTemplateVersion: 'screen-vlm-guided-current-route-selection-v1',
  selectedFor: [
    'windows-service-proof-path',
    'guided-screen-classification',
    'public-live-managed-browser-crop',
    'retained-parent-proof-screenshots',
  ],
};

const candidateEvidence = {
  localLlamaRuntimeAvailable:
    readiness.localLlamaRuntime.binaryExists === true &&
    readiness.localLlamaRuntime.modelExists === true &&
    readiness.localLlamaRuntime.mmprojExists === true &&
    readiness.providerRuntimeState.localLlamaRuntimeAvailable === true,
  retainedMatrixAvailable:
    readiness.localLlamaRuntime.matrixProofPresent === true &&
    readiness.localLlamaRuntime.matrixAnalyzedByRealLocalVlm === true &&
    readiness.localLlamaRuntime.matrixSchemaValidated === true &&
    readiness.localLlamaRuntime.matrixRawImagesDeleted === true,
  retainedLiveOperatorAvailable:
    readiness.localLlamaRuntime.liveOperatorProofPresent === true &&
    readiness.localLlamaRuntime.liveOperatorLocalVlmAnalysisProof === true &&
    readiness.localLlamaRuntime.liveOperatorRawImagesDeleted === true,
  boundedInputsAvailable:
    resourceCrop.assertions.allVlmInputsWithinMaxPixelBudget === true &&
    resourceCrop.assertions.cdpCropPathExistsButDoesNotClaimVlmCropQuality === true,
  runtimeResourcesWithinEnvelope:
    runtimeMeasurement.assertions.localRuntimeExecuted === true &&
    runtimeMeasurement.assertions.parseableMeaningfulJson === true &&
    runtimeMeasurement.assertions.allSamplesWithinResourceEnvelope === true,
  publicLiveCropQualityPassed:
    liveCropQuality.assertions.everyRequiredPublicScenarioPassed === true &&
    liveCropQuality.assertions.realLivePagesLoaded === true &&
    liveCropQuality.assertions.localVlmExecutedForEveryCrop === true &&
    liveCropQuality.assertions.expectedTermsDetectedByVlmForEveryCrop === true &&
    liveCropQuality.assertions.expectedCategoryMatchedForEveryCrop === true,
  localOnlyCustody:
    readiness.assertions.localOnly === true &&
    runtimeMeasurement.assertions.noRemoteAiUsed === true &&
    liveCropQuality.assertions.noRemoteAiUsed === true,
  deletionCustody:
    runtimeMeasurement.assertions.noRawCaptureRetention === true &&
    liveCropQuality.assertions.rawCropsDeleted === true &&
    liveCropQuality.assertions.noRawImageRetained === true,
};

const candidateRejected = [
  {
    candidate: 'lm-studio-cli-server',
    reason: 'CLI detected but local server and loaded models are unavailable in the current proof environment.',
    evidence: readiness.localProviderRuntimeProbe.lmStudioServerStatus,
  },
  {
    candidate: 'ollama',
    reason: 'Ollama command is unavailable in the current proof environment.',
    evidence: readiness.localProviderRuntimeProbe.ollama,
  },
  {
    candidate: 'llama-server-path',
    reason: 'Generic llama-server command is unavailable; the cached llama-mtmd CLI is the measured route.',
    evidence: readiness.localProviderRuntimeProbe.llamaServer,
  },
];

const proof = {
  proof: 'screen-vlm-model-selection-proof',
  generatedAt,
  proofTier: 'P2_CURRENT_LOCAL_VLM_ROUTE_SELECTION',
  claim:
    'The current Windows screen-plan proof path selects the cached local llama.cpp/Qwen2-VL route for guided VLM classification because it is the only measured local route with runtime availability, bounded input custody, resource measurement, public-live crop quality, local-only custody, and deletion proof.',
  selectedRoute,
  sourceEvidence: {
    readiness: relativePath(readinessPath),
    resourceCrop: relativePath(resourceCropPath),
    runtimeMeasurement: relativePath(runtimeMeasurementPath),
    liveCropQuality: relativePath(liveCropQualityPath),
  },
  candidateEvidence,
  candidateRejected,
  selectionDecision: {
    selected: Object.values(candidateEvidence).every(Boolean),
    selectedModelRuntimeRef: selectedRoute.modelRuntimeRef,
    selectedProviderKind: selectedRoute.providerKind,
    selectedModelLabel: 'Qwen2-VL-2B-Instruct-Q4_K_M via llama-mtmd-cli b9279',
    decisionScope: 'current Windows local screen-plan proof route',
  },
  remainingQualityGates: [
    'authenticated-account social/feed proof',
    'broader platform parity',
    'longer model calibration set',
    'production rollout policy thresholds',
    'hardware-specific model fallback matrix',
  ],
  assertions: {
    selectedRouteHasRuntimeArtifacts: candidateEvidence.localLlamaRuntimeAvailable,
    selectedRouteHasRealLocalVlmProof: candidateEvidence.retainedMatrixAvailable,
    selectedRouteHasLiveOperatorProof: candidateEvidence.retainedLiveOperatorAvailable,
    selectedRouteHasBoundedInputProof: candidateEvidence.boundedInputsAvailable,
    selectedRouteHasResourceMeasurement: candidateEvidence.runtimeResourcesWithinEnvelope,
    selectedRouteHasPublicLiveCropQuality: candidateEvidence.publicLiveCropQualityPassed,
    selectedRouteIsLocalOnly: candidateEvidence.localOnlyCustody,
    selectedRoutePreservesDeletionCustody: candidateEvidence.deletionCustody,
    noRemoteProviderSelected: selectedRoute.providerKind === 'localVision',
  },
  completedChecklistClaims: [
    'current Windows local VLM proof route selects cached llama.cpp/Qwen2-VL after runtime, resource, quality, local-only, and deletion evidence',
    'LM Studio, Ollama, and generic llama-server remain non-selected in this environment because they lack equivalent measured readiness',
  ],
  openChecklistClaims: [
    'authenticated-account social crop quality remains outside this public-live proof',
    'broad production rollout thresholds and hardware fallback matrix remain open',
  ],
  nonClaims: [
    'This proof selects the current local Windows proof route; it does not claim every platform or hardware profile.',
    'This proof does not claim authenticated-account social/feed model quality.',
    'This proof does not use remote/API AI or retain raw screenshots.',
  ],
};

if (!Object.values(proof.assertions).every(Boolean) || !proof.selectionDecision.selected) {
  throw new Error(`screen VLM model selection proof failed: ${JSON.stringify(proof.assertions)}`);
}

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`screen-vlm-model-selection-proof-ok:${proofPath}`);

function readJson(path) {
  if (!existsSync(path)) {
    throw new Error(`Expected proof artifact at ${path}`);
  }

  return JSON.parse(readFileSync(path, 'utf8'));
}

function relativePath(path) {
  return relative(repoRoot, path).replace(/\\/gu, '/');
}
