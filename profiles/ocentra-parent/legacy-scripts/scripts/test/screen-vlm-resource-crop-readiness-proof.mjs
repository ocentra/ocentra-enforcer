import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join('output', 'screen-plan-proof', '36-vlm-resource-crop-readiness');
const proofPath = join(outputDir, 'proof-summary.json');
const matrixProofPath = join('output', 'ai-plan-proof', 'real-analysis', 'proof-summary.json');
const liveOperatorProofPath = join('output', 'screen-ai-pipeline-proof', 'live-operator', 'proof-summary.json');
const cdpCropProofPath = join(
  'output',
  'screen-plan-proof',
  '33-managed-browser-cdp-screenshot-capture-path',
  'proof-summary.json'
);
const maxImagePixels = 2073600;

mkdirSync(outputDir, { recursive: true });

const matrixProof = readJson(matrixProofPath);
const liveOperatorProof = readJson(liveOperatorProofPath);
const cdpCropProof = readJson(cdpCropProofPath);

const matrixCaptureSamples = matrixProof.scenarios
  .filter((scenario) => scenario.captureCount > 0 && scenario.scenarioId !== 'timed-cadence-repeated-analysis')
  .map((scenario) =>
    captureSample({
      proofSet: 'controlled-local-vlm-matrix',
      scenarioId: scenario.scenarioId,
      metadataPath: join(
        'output',
        'screen-ai-pipeline-proof',
        scenario.scenarioId,
        'capture',
        '02-capture-metadata.json'
      ),
      analyzedByRealLocalVlm: scenario.analyzedByRealLocalVlm === true,
      schemaValidated: scenario.schemaValidated === true,
      policyDecisionValidated: scenario.policyDecisionValidated === true,
      rawImagesDeletedAfterAnalysis: scenario.rawImagesDeletedAfterAnalysis === true,
    })
  );

const liveCaptureSamples = liveOperatorProof.scenarios
  .filter((scenario) => scenario.status === 'passed' && scenario.analyzedByRealLocalVlm === true)
  .map((scenario) =>
    captureSample({
      proofSet: 'live-operator-vlm-matrix',
      scenarioId: scenario.scenarioId,
      metadataPath: join(
        'output',
        'screen-ai-pipeline-proof',
        'live-operator',
        scenario.scenarioId,
        'capture',
        '02-capture-metadata.json'
      ),
      analyzedByRealLocalVlm: true,
      schemaValidated: scenario.schemaValidated === true,
      policyDecisionValidated: scenario.policyDecisionValidated === true,
      rawImagesDeletedAfterAnalysis: scenario.rawImagesDeletedAfterAnalysis === true,
    })
  );

const allSamples = [...matrixCaptureSamples, ...liveCaptureSamples];
const missingMetadata = allSamples.filter((sample) => !sample.metadataPresent);
const oversizedSamples = allSamples.filter((sample) => sample.pixelCount > maxImagePixels);
const analyzedSamples = allSamples.filter((sample) => sample.analyzedByRealLocalVlm);
const liveExternalSamples = liveCaptureSamples.filter((sample) => sample.proofSet === 'live-operator-vlm-matrix');

const proof = {
  proof: 'screen-vlm-resource-crop-readiness-proof',
  generatedAt: new Date().toISOString(),
  proofTier: 'P2_RETAINED_ARTIFACT_RESOURCE_AUDIT',
  modelConstants: {
    maxImagePixels,
    runtimeBinary: matrixProof.runtimeBinary,
    model: matrixProof.model,
    mmproj: matrixProof.mmproj,
  },
  retainedProofs: {
    matrixProof: {
      path: artifactPath(matrixProofPath),
      present: existsSync(resolve(repoRoot, matrixProofPath)),
      scenarioCount: matrixProof.scenarioCount,
      realWindowCaptureCount: matrixProof.realWindowCaptureCount,
      analyzedByRealLocalVlm: matrixProof.analyzedByRealLocalVlm === true,
      schemaValidated: matrixProof.schemaValidated === true,
      policyDecisionValidated: matrixProof.policyDecisionValidated === true,
      rawImagesDeletedAfterAnalysis: matrixProof.rawImagesDeletedAfterAnalysis === true,
    },
    liveOperatorProof: {
      path: artifactPath(liveOperatorProofPath),
      present: existsSync(resolve(repoRoot, liveOperatorProofPath)),
      scenarioCount: liveOperatorProof.scenarioCount,
      passedScenarioCount: liveOperatorProof.passedScenarioCount,
      liveExternalUrlProof: liveOperatorProof.liveExternalUrlProof === true,
      localVlmAnalysisProof: liveOperatorProof.localVlmAnalysisProof === true,
      policyDryRunProof: liveOperatorProof.policyDryRunProof === true,
      rawImagesDeletedAfterAnalysis: liveOperatorProof.rawImagesDeletedAfterAnalysis === true,
      productCompleteClaimed: liveOperatorProof.productCompleteClaimed === true,
    },
    managedBrowserCdpCropProof: {
      path: artifactPath(cdpCropProofPath),
      present: existsSync(resolve(repoRoot, cdpCropProofPath)),
      cropModeCaptured: cdpCropProof.captureSummary?.modes?.includes('crop') === true,
      allDeleted: cdpCropProof.captureSummary?.allDeleted === true,
      anyDesktopCapture: cdpCropProof.captureSummary?.anyDesktopCapture === true,
    },
  },
  captureBudgetSummary: {
    sampleCount: allSamples.length,
    analyzedSampleCount: analyzedSamples.length,
    liveExternalAnalyzedSampleCount: liveExternalSamples.length,
    missingMetadataCount: missingMetadata.length,
    oversizedSampleCount: oversizedSamples.length,
    maxPixelCount: Math.max(...allSamples.map((sample) => sample.pixelCount)),
    maxImageByteSize: Math.max(...allSamples.map((sample) => sample.imageByteSize)),
    allSamplesWithinPixelBudget: oversizedSamples.length === 0,
    allSamplesHaveMetadata: missingMetadata.length === 0,
    allSamplesDeleteRawImages: allSamples.every((sample) => sample.rawImagesDeletedAfterAnalysis === true),
  },
  captureSamples: allSamples,
  assertions: {
    retainedMatrixProofPresent: existsSync(resolve(repoRoot, matrixProofPath)),
    retainedLiveOperatorProofPresent: existsSync(resolve(repoRoot, liveOperatorProofPath)),
    realLocalVlmAnalyzedRetainedCaptures:
      matrixProof.analyzedByRealLocalVlm === true && liveOperatorProof.localVlmAnalysisProof === true,
    liveOperatorMatrixComplete:
      liveOperatorProof.fullRequiredMatrixComplete === true &&
      liveOperatorProof.passedScenarioCount === liveOperatorProof.scenarioCount,
    cdpCropPathExistsButDoesNotClaimVlmCropQuality:
      cdpCropProof.captureSummary?.modes?.includes('crop') === true && cdpCropProof.captureSummary?.allDeleted === true,
    allVlmInputsWithinMaxPixelBudget: oversizedSamples.length === 0 && allSamples.length > 0,
    rawImagesDeletedAfterAnalysis: allSamples.every((sample) => sample.rawImagesDeletedAfterAnalysis === true),
    productionModelSelectionStillBlocked: true,
  },
  completedChecklistClaims: [
    'retained controlled local VLM and live-operator captures are bounded below the worker max-image-pixel budget',
    'retained live-operator captures include real public/live URL and native-app VLM analysis with deletion proof',
    'managed-browser CDP crop capture path exists and deletes captured material',
  ],
  openChecklistClaims: [
    'this audit does not measure per-inference wall time, CPU time, or peak RSS',
    'this audit does not prove detector-specific VLM crop quality on cropped live pages',
    'this audit does not select a production VLM model or resource envelope',
    'authenticated-account social proof remains outside the retained public/live matrix',
  ],
  nonClaims: [
    'This audit consumes retained proof artifacts and does not rerun the VLM matrix.',
    'This audit proves bounded retained VLM input dimensions, not production resource suitability.',
    'This audit does not retain raw screenshots or upload screenshots to remote AI.',
  ],
};

if (!Object.values(proof.assertions).every(Boolean)) {
  throw new Error(`screen VLM resource/crop readiness assertions failed: ${JSON.stringify(proof.assertions)}`);
}

writeJson(proofPath, proof);
console.log(`screen-vlm-resource-crop-readiness-proof-ok:${proofPath}`);

function captureSample({
  proofSet,
  scenarioId,
  metadataPath,
  analyzedByRealLocalVlm,
  schemaValidated,
  policyDecisionValidated,
  rawImagesDeletedAfterAnalysis,
}) {
  const metadataPresent = existsSync(resolve(repoRoot, metadataPath));
  const metadata = metadataPresent ? readJson(metadataPath) : {};
  const width = Number(metadata.width ?? 0);
  const height = Number(metadata.height ?? 0);
  const pixelCount = width * height;
  return {
    proofSet,
    scenarioId,
    metadataPath: artifactPath(metadataPath),
    metadataPresent,
    captured: metadata.captured === true,
    status: metadata.status ?? null,
    captureScope: metadata.actualScope ?? metadata.requestedScope ?? null,
    width,
    height,
    pixelCount,
    maxImagePixels,
    withinMaxImagePixels: pixelCount > 0 && pixelCount <= maxImagePixels,
    imageByteSize: Number(metadata.imageByteSize ?? 0),
    imageDigestPresent: typeof metadata.imageDigest === 'string' && metadata.imageDigest.length > 0,
    rawImagePersistedInProof: metadata.rawImagePersistedInProof === true,
    analyzedByRealLocalVlm,
    schemaValidated,
    policyDecisionValidated,
    rawImagesDeletedAfterAnalysis,
  };
}

function readJson(path) {
  return JSON.parse(readFileSync(resolve(repoRoot, path), 'utf8'));
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function artifactPath(path) {
  return path.replaceAll('\\', '/');
}
