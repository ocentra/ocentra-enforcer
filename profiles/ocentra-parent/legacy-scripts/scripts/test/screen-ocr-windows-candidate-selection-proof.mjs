import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-plan-proof', 'windows-ocr-candidate-selection');
const summaryPath = resolve(outputRoot, 'proof-summary.json');

const winRtServicePath = resolve(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'service-winrt-ocr',
  'proof-summary.json'
);
const winRtRedactionPath = resolve(
  repoRoot,
  'output',
  'screen-ai-pipeline-proof',
  'service-winrt-ocr-redaction',
  'proof-summary.json'
);
const tesseractPath = resolve(
  repoRoot,
  'output',
  'screen-plan-proof',
  '34-ocr-tesseract-baseline',
  'proof-summary.json'
);
const paddlePath = resolve(
  repoRoot,
  'output',
  'screen-plan-proof',
  '35-ocr-paddleocr-ppocr-evaluation',
  'proof-summary.json'
);

const [winRtService, winRtRedaction, tesseract, paddle] = await Promise.all([
  readJson(winRtServicePath),
  readJson(winRtRedactionPath),
  readJson(tesseractPath),
  readJson(paddlePath),
]);

const winRtAssertions = winRtService.assertions ?? {};
const winRtRedactionAssertions = winRtRedaction.assertions ?? {};
const tesseractReadiness = tesseract.baselineReadiness ?? {};
const paddleComparison = paddle.runtimeAndQualityComparison ?? {};

assertEqual(winRtService.platform, 'win32', 'WinRT service proof must be Windows scoped.');
assertEqual(winRtService.analysisRow?.providerKind, 'localOcr', 'WinRT service row must preserve local OCR provider.');
assertEqual(winRtService.analysisRow?.modelId, 'windows-winrt-ocr', 'WinRT service row must preserve model id.');
assertTrue(winRtAssertions.realWindowsServiceCaptureRequired, 'WinRT service proof must require real Windows capture.');
assertTrue(winRtAssertions.serviceAdapterRanWindowsWinRtOcr, 'WinRT service proof must run WinRT OCR.');
assertTrue(winRtAssertions.policyConsumedOcrResult, 'WinRT OCR result must feed policy dry-run.');
assertTrue(winRtAssertions.encryptedQueueDrainedAfterAnalysis, 'WinRT OCR proof must drain the encrypted queue.');
assertTrue(winRtAssertions.adapterTemporaryImageDeleted, 'WinRT OCR proof must delete adapter temp image.');
assertTrue(winRtAssertions.rawImageNotRetainedInReadModel, 'WinRT OCR proof must not retain raw image.');
assertTrue(
  winRtRedactionAssertions.serviceRedactedSensitiveOcrSnippets,
  'WinRT redaction proof must redact sensitive snippets.'
);
assertTrue(
  winRtRedactionAssertions.serviceConsumedParentSelectedRedactionPolicy,
  'WinRT redaction proof must consume parent-selected redaction policy.'
);
assertTrue(tesseractReadiness.localExtractionProofComplete, 'Tesseract baseline must extract local text.');
assertTrue(tesseractReadiness.runtimeMeasured, 'Tesseract baseline must include runtime measurement.');
assertTrue(
  tesseractReadiness.comparedAgainstPaddleOcr === false,
  'Tesseract proof must not overclaim Paddle comparison.'
);
assertEqual(
  paddleComparison.status,
  'current-runtime-executes-no-text',
  'PaddleOCR proof must show current candidate executes but extracts no text from the real proof image.'
);
assertEqual(
  paddleComparison.paddleOcrRuntimeAttempt?.extractedTextCount,
  0,
  'Current PP-OCRv5 proof must preserve the zero-text extraction result.'
);
assertTrue(paddleComparison.legacyFallbackComparedAgainstTesseract, 'PaddleOCR legacy fallback must be compared.');
assertTrue(
  paddle.placementDecision?.selectedForProduction === false,
  'PaddleOCR proof must not select PaddleOCR for production.'
);

const summary = {
  proof: 'screen-ocr-windows-candidate-selection-proof',
  generatedAt: new Date().toISOString(),
  proofTier: 'P2_AGGREGATE_SELECTION_PROOF',
  platformScope: 'windows-service-ocr-only',
  selectedCurrentRoute: {
    providerKind: winRtService.analysisRow.providerKind,
    modelId: winRtService.analysisRow.modelId,
    modelRuntimeRef: winRtService.analysisRow.modelRuntimeRef,
    promptOrTemplateVersion: winRtService.analysisRow.promptOrTemplateVersion,
    reason:
      'WinRT OCR is the current Windows service OCR route because it has real service capture, local OCR, policy dry-run, read-model, redaction, queue drain, and raw-image deletion proof.',
  },
  fallbackCandidates: [
    {
      candidate: 'tesseract-5.5-windows',
      status: 'measured-fallback-baseline',
      matchedTerms: tesseract.extractionProof?.matchedTerms ?? [],
      durationMs: tesseract.extractionProof?.durationMs ?? null,
      peakWorkingSetMiB: tesseract.extractionProof?.peakWorkingSetMiB ?? null,
      reason:
        'Tesseract extracted expected text from a retained real public Vimeo screenshot artifact with CPU and memory measurement, but it is not selected as production OCR.',
    },
    {
      candidate: 'paddleocr-3.x-ppocrv5',
      status: 'not-selected-zero-text-on-real-proof-image',
      error: paddleComparison.paddleOcrRuntimeAttempt?.error ?? null,
      extractedTextCount: paddleComparison.paddleOcrRuntimeAttempt?.extractedTextCount ?? null,
      initSeconds: paddleComparison.paddleOcrRuntimeAttempt?.initSeconds ?? null,
      predictSeconds: paddleComparison.paddleOcrRuntimeAttempt?.predictSeconds ?? null,
      reason:
        'Current PP-OCRv5 package/model cache exists and local inference runs with enable_mkldnn=false, but it extracts zero text from the retained real Vimeo proof image, so it cannot be selected.',
    },
    {
      candidate: 'paddleocr-2.x-pinned-python310',
      status: 'measured-legacy-fallback-candidate',
      matchedTerms: paddleComparison.legacyPaddleOcr2xRuntimeAttempt?.matchedTerms ?? [],
      peakRssMiB: paddleComparison.legacyPaddleOcr2xRuntimeAttempt?.peakRssMiB ?? null,
      reason:
        'Pinned PaddleOCR 2.x can extract comparable Vimeo text locally, but dependency, custody, packaging, and quality review remain before selection.',
    },
  ],
  evidenceArtifacts: {
    winRtService: relativePath(winRtServicePath),
    winRtRedaction: relativePath(winRtRedactionPath),
    tesseractBaseline: relativePath(tesseractPath),
    paddleOcrEvaluation: relativePath(paddlePath),
  },
  assertions: {
    windowsServiceOcrSelected: true,
    selectedRouteHasRealCaptureProof: true,
    selectedRouteHasDeletionProof: true,
    selectedRouteHasRedactionProof: true,
    selectedRouteHasPolicyReadModelProof: true,
    tesseractFallbackMeasured: true,
    paddleCurrentCandidateExecutesButExtractsNoText: true,
    paddleNotSelectedForProduction: true,
    noCrossPlatformOcrClaim: true,
    noProductionQualityClaim: true,
  },
  nonClaims: [
    'This proof selects the current Windows service OCR route only; it does not close macOS, Linux, Android, or iOS OCR parity.',
    'This proof does not rerun a new live capture; it aggregates retained real service and OCR proof artifacts already produced in the screen plan.',
    'This proof does not claim final production OCR quality, broad language coverage, authenticated social coverage, or VLM completion.',
    'This proof does not select PaddleOCR or Tesseract as the production OCR runtime.',
  ],
};

await mkdir(outputRoot, { recursive: true });
await writeFile(summaryPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-ocr-windows-candidate-selection-proof-ok:${summaryPath}`);

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'));
}

function relativePath(path) {
  return path.slice(repoRoot.length + 1).replaceAll('\\', '/');
}

function assertTrue(value, message) {
  if (value !== true) {
    throw new Error(message);
  }
}

function assertEqual(actual, expected, message) {
  if (actual !== expected) {
    throw new Error(`${message} Expected ${expected}, got ${actual}.`);
  }
}
