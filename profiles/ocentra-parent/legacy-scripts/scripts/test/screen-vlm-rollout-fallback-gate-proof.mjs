import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..', '..');
const outputDir = join(repoRoot, 'output', 'screen-plan-proof', '36-vlm-rollout-fallback-gate');
const proofPath = join(outputDir, 'proof-summary.json');
const modelSelectionPath = join(
  repoRoot,
  'output',
  'screen-plan-proof',
  '36-vlm-model-selection',
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

const modelSelection = readJson(modelSelectionPath);
const resourceCrop = readJson(resourceCropPath);
const runtimeMeasurement = readJson(runtimeMeasurementPath);
const liveCropQuality = readJson(liveCropQualityPath);

const maxImagePixels = resourceCrop.modelConstants.maxImagePixels;
const budgets = runtimeMeasurement.budgets;
const observed = runtimeMeasurement.summary;

const decisions = [
  routeDecision('current-windows-local-vlm-route', {
    runtimeAvailable: modelSelection.assertions.selectedRouteHasRuntimeArtifacts,
    withinImageBudget: resourceCrop.captureBudgetSummary.allSamplesWithinPixelBudget,
    withinResourceEnvelope: observed.allSamplesWithinResourceEnvelope,
    publicLiveQualityPassed: liveCropQuality.assertions.expectedCategoryMatchedForEveryCrop,
    deletionCustodyProved: modelSelection.assertions.selectedRoutePreservesDeletionCustody,
    authenticatedSocialQualityProved: false,
  }),
  routeDecision('runtime-missing', {
    runtimeAvailable: false,
    withinImageBudget: true,
    withinResourceEnvelope: true,
    publicLiveQualityPassed: true,
    deletionCustodyProved: true,
    authenticatedSocialQualityProved: false,
  }),
  routeDecision('oversized-image', {
    runtimeAvailable: true,
    withinImageBudget: false,
    withinResourceEnvelope: true,
    publicLiveQualityPassed: true,
    deletionCustodyProved: true,
    authenticatedSocialQualityProved: false,
  }),
  routeDecision('resource-over-budget', {
    runtimeAvailable: true,
    withinImageBudget: true,
    withinResourceEnvelope: false,
    publicLiveQualityPassed: true,
    deletionCustodyProved: true,
    authenticatedSocialQualityProved: false,
  }),
  routeDecision('authenticated-social-quality-missing', {
    runtimeAvailable: true,
    withinImageBudget: true,
    withinResourceEnvelope: true,
    publicLiveQualityPassed: true,
    deletionCustodyProved: true,
    authenticatedSocialQualityProved: false,
    requiresAuthenticatedSocialQuality: true,
  }),
];

const currentRoute = decisions[0];

const proof = {
  proof: 'screen-vlm-rollout-fallback-gate-proof',
  generatedAt: new Date().toISOString(),
  proofTier: 'P2_CURRENT_WINDOWS_VLM_ROLLOUT_FALLBACK_GATE',
  claim:
    'The current Windows local VLM route is allowed only inside the measured image/resource/local-custody envelope; missing runtime, oversized inputs, over-budget resources, or unproved authenticated-social quality fall back to OCR/manual-required instead of using a remote provider or retaining raw screenshots.',
  sourceEvidence: {
    modelSelection: relativePath(modelSelectionPath),
    modelSelectionPresent: existsSync(modelSelectionPath),
    resourceCrop: relativePath(resourceCropPath),
    resourceCropPresent: existsSync(resourceCropPath),
    runtimeMeasurement: relativePath(runtimeMeasurementPath),
    runtimeMeasurementPresent: existsSync(runtimeMeasurementPath),
    liveCropQuality: relativePath(liveCropQualityPath),
    liveCropQualityPresent: existsSync(liveCropQualityPath),
    liveCropQualityScenarioCount: liveCropQuality.scenarioCount,
    liveCropQualityCategoriesCovered: liveCropQuality.summary.categoriesCovered,
    liveCropQualityHostsCovered: liveCropQuality.summary.publicHostsCovered,
  },
  measuredEnvelope: {
    maxImagePixels,
    maxObservedPixels: resourceCrop.captureBudgetSummary.maxPixelCount,
    maxImageByteSize: resourceCrop.captureBudgetSummary.maxImageByteSize,
    maxWallMs: budgets.maxWallMs,
    maxWallMsObserved: observed.maxWallMsObserved,
    maxPeakWorkingSetBytes: budgets.maxPeakWorkingSetBytes,
    maxPeakWorkingSetBytesObserved: observed.maxPeakWorkingSetBytesObserved,
    maxCpuSeconds: budgets.maxCpuSeconds,
    maxCpuSecondsObserved: observed.maxCpuSecondsObserved,
  },
  selectedRoute: {
    ...modelSelection.selectedRoute,
    decision: currentRoute.decision,
    fallback: currentRoute.fallback,
  },
  rolloutDecisions: decisions,
  assertions: {
    currentRouteAllowedInMeasuredEnvelope: currentRoute.decision === 'allow-local-vlm',
    missingRuntimeFallsBack: decisions[1].fallback === 'manual-required',
    oversizedImageFallsBack: decisions[2].fallback === 'ocr-first-or-manual-required',
    overBudgetResourcesFallBack: decisions[3].fallback === 'ocr-first-or-manual-required',
    authenticatedSocialRequiresSeparateProof: decisions[4].fallback === 'manual-required-auth-social-proof',
    noRemoteProviderSelected: modelSelection.assertions.noRemoteProviderSelected === true,
    noRawImageRetentionRequired: liveCropQuality.assertions.noRawImageRetained === true,
  },
  completedChecklistClaims: [
    'current Windows local VLM rollout gate allows Qwen2-VL only within the measured local image/resource envelope',
    'real public video, school/productivity, browser game, shopping, and public social/feed crop categories are covered before selecting the current Windows local VLM route',
    'runtime-missing, oversized-input, over-budget, and authenticated-social-unproved states fall back to OCR/manual-required instead of remote AI',
  ],
  openChecklistClaims: [
    'broad production rollout thresholds across more hardware profiles remain open',
    'authenticated-account social/feed quality remains outside this public-live proof',
    'cross-platform VLM model/runtime parity remains open',
  ],
  nonClaims: [
    'This proof does not claim broad hardware rollout readiness.',
    'This proof does not claim authenticated-account social/feed quality.',
    'This proof does not use remote/API AI or retain raw screenshots.',
  ],
};

if (!Object.values(proof.assertions).every(Boolean)) {
  throw new Error(`screen VLM rollout fallback gate assertions failed: ${JSON.stringify(proof.assertions)}`);
}

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(proof, null, 2)}\n`);
console.log(`screen-vlm-rollout-fallback-gate-proof-ok:${proofPath}`);

function routeDecision(name, evidence) {
  if (!evidence.runtimeAvailable) {
    return blocked(name, evidence, 'manual-required');
  }
  if (!evidence.withinImageBudget || !evidence.withinResourceEnvelope) {
    return blocked(name, evidence, 'ocr-first-or-manual-required');
  }
  if (!evidence.deletionCustodyProved || !evidence.publicLiveQualityPassed) {
    return blocked(name, evidence, 'manual-required');
  }
  if (evidence.requiresAuthenticatedSocialQuality && !evidence.authenticatedSocialQualityProved) {
    return blocked(name, evidence, 'manual-required-auth-social-proof');
  }

  return {
    name,
    decision: 'allow-local-vlm',
    fallback: null,
    evidence,
  };
}

function blocked(name, evidence, fallback) {
  return {
    name,
    decision: 'block-local-vlm-route',
    fallback,
    evidence,
  };
}

function readJson(path) {
  if (!existsSync(path)) {
    throw new Error(`Expected proof artifact at ${path}`);
  }

  return JSON.parse(readFileSync(path, 'utf8'));
}

function relativePath(path) {
  return relative(repoRoot, path).replace(/\\/gu, '/');
}
