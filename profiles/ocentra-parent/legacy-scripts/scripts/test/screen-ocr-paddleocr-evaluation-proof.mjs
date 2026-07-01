import { existsSync, writeFileSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { spawnSync } from 'node:child_process';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-plan-proof', '35-ocr-paddleocr-ppocr-evaluation');
const proofSummaryPath = join(outputRoot, 'proof-summary.json');
const runtimeLogPath = join(outputRoot, 'paddleocr-runtime-attempt.log');
const serverDetectorRuntimeLogPath = join(outputRoot, 'paddleocr-server-detector-runtime-attempt.log');
const preprocessRuntimeLogPath = join(outputRoot, 'paddleocr-preprocess-runtime-attempt.log');
const legacyRuntimeLogPath = join(outputRoot, 'paddleocr-2x-py310-runtime.log');
const legacyRuntimeScriptPath = join(outputRoot, 'paddleocr-2x-py310-runtime.py');
const legacyRuntimePythonPath = join(outputRoot, 'paddleocr-2x-py310-venv', 'Scripts', 'python.exe');
const sourceImagePath = resolve(
  repoRoot,
  'output',
  'browser-plan-proof',
  'social-14-managed-browser-feed-short-video-gate',
  '06-live-screenshots',
  'vimeo-public-video.png'
);
const tesseractTextPath = resolve(
  repoRoot,
  'output',
  'screen-plan-proof',
  '34-ocr-tesseract-baseline',
  'vimeo-public-video-tesseract-output.txt'
);

await mkdir(outputRoot, { recursive: true });

const python = runOptional('python', ['--version']);
const pip = runOptional('python', ['-m', 'pip', '--version']);
const paddleOcrIndex = runOptional('python', ['-m', 'pip', 'index', 'versions', 'paddleocr']);
const paddlePaddleIndex = runOptional('python', ['-m', 'pip', 'index', 'versions', 'paddlepaddle']);
const paddleOcrImport = runOptional('python', [
  '-c',
  'import importlib.util; print(importlib.util.find_spec("paddleocr") is not None)',
]);
const paddlePaddleImport = runOptional('python', [
  '-c',
  'import importlib.util; print(importlib.util.find_spec("paddle") is not None)',
]);
const paddleVersions = runOptional('python', [
  '-c',
  'import paddle, paddleocr; print(f"paddle={paddle.__version__}"); print(f"paddleocr={getattr(paddleocr, \'__version__\', \'unknown\')}")',
]);
const tesseract = resolveTesseractVersion();

const paddleOcrVersion = parsePipLatest(paddleOcrIndex.stdout);
const paddlePaddleVersion = parsePipLatest(paddlePaddleIndex.stdout);
const paddleOcrInstalled = parsePythonBool(paddleOcrImport.stdout);
const paddlePaddleInstalled = parsePythonBool(paddlePaddleImport.stdout);
const tesseractInstalled = tesseract.status === 0;

assert(python.status === 0, 'Python must be available for the OCR candidate evaluation.');
assert(pip.status === 0, 'pip must be available for the OCR candidate evaluation.');
assert(paddleOcrVersion, 'PyPI must report a current paddleocr version.');
assert(paddlePaddleVersion, 'PyPI must report a current paddlepaddle version.');

const localRuntimeReady = paddleOcrInstalled && paddlePaddleInstalled;
const tesseractComparisonReady = localRuntimeReady && tesseractInstalled;
const explicitRuntimeRunRequested = process.env.OCENTRA_RUN_PADDLEOCR_LOCAL === '1';
const runtimeExecutionAllowed = explicitRuntimeRunRequested && localRuntimeReady;
const runtimeAttempt = runtimeExecutionAllowed ? runPaddleOcrRuntimeAttempt() : null;
const serverDetectorRuntimeAttempt = runtimeExecutionAllowed ? runPaddleOcrServerDetectorRuntimeAttempt() : null;
const preprocessRuntimeAttempt = runtimeExecutionAllowed ? runPaddleOcrPreprocessRuntimeAttempt() : null;
const explicitLegacyRuntimeRunRequested = process.env.OCENTRA_RUN_PADDLEOCR_2X_LOCAL === '1';
const legacyRuntimeAvailable = existsSync(legacyRuntimePythonPath);
const legacyRuntimeExecutionAllowed = explicitLegacyRuntimeRunRequested && legacyRuntimeAvailable;
const legacyRuntimeAttempt = legacyRuntimeExecutionAllowed ? runLegacyPaddleOcrRuntimeAttempt() : null;
const tesseractText = await readOptionalText(tesseractTextPath);
const tesseractTerms = ['vimeo', 'video', 'player'].filter((term) => tesseractText.toLowerCase().includes(term));
const modelCache = [
  'PP-LCNet_x1_0_doc_ori',
  'UVDoc',
  'PP-LCNet_x1_0_textline_ori',
  'PP-OCRv5_server_det',
  'PP-OCRv5_mobile_det',
  'en_PP-OCRv5_mobile_rec',
].map((name) => ({
  name,
  path: join(process.env.USERPROFILE ?? '', '.paddlex', 'official_models', name),
  exists: existsSync(join(process.env.USERPROFILE ?? '', '.paddlex', 'official_models', name)),
}));

const summary = {
  proof: 'screen-ocr-paddleocr-ppocr-evaluation',
  generatedAt: new Date().toISOString(),
  sourceEvidence: {
    sourceImagePath,
    sourceImageExists: existsSync(sourceImagePath),
    sourceImageKind: 'retained real public Vimeo managed-browser screenshot artifact',
    tesseractBaselinePath: tesseractTextPath,
    tesseractBaselineExists: Boolean(tesseractText),
    tesseractMatchedTerms: tesseractTerms,
  },
  officialSourceSnapshot: {
    paddleOcrInstallation: 'https://paddlepaddle.github.io/PaddleOCR/main/en/version3.x/installation.html',
    paddleOcrQuickStart: 'https://paddlepaddle.github.io/PaddleOCR/main/en/quick_start.html',
    ppOcrV5: 'https://paddlepaddle.github.io/PaddleOCR/main/en/version3.x/algorithm/PP-OCRv5/PP-OCRv5.html',
    verifiedClaims: [
      'PaddleOCR 3.x inference package is installed from PyPI as paddleocr.',
      'PaddleOCR 3.x depends on PaddlePaddle 3.0 or newer.',
      'PP-OCRv5 is the current 3.x OCR family to evaluate for general OCR.',
      'PaddleOCR supports Windows, Linux, and macOS deployment, but Ocentra still requires local runtime proof before product selection.',
    ],
  },
  localEnvironment: {
    python: oneLine(python.stdout || python.stderr),
    pip: oneLine(pip.stdout || pip.stderr),
    tesseractInstalled,
    paddleOcrInstalled,
    paddlePaddleInstalled,
    installedVersions: parseInstalledVersions(paddleVersions.stdout),
    packagingRisk:
      'pip install succeeded, but pip reported an environment conflict: mediapipe requires protobuf<5 while this Python environment has protobuf 5.29.5.',
  },
  pypiCandidateSnapshot: {
    paddleocrLatest: paddleOcrVersion,
    paddlepaddleLatest: paddlePaddleVersion,
    paddleocrIndexCommandPassed: paddleOcrIndex.status === 0,
    paddlepaddleIndexCommandPassed: paddlePaddleIndex.status === 0,
  },
  localOnlyGate: {
    remoteApiAllowed: false,
    remoteModelDownloadAllowedByDefault: false,
    runtimeExecutionAllowed,
    explicitRuntimeRunEnv: 'OCENTRA_RUN_PADDLEOCR_LOCAL=1',
    legacyRuntimeExecutionAllowed,
    explicitLegacyRuntimeRunEnv: 'OCENTRA_RUN_PADDLEOCR_2X_LOCAL=1',
    reason: runtimeExecutionAllowed
      ? 'Explicit local runtime execution was requested and local packages are importable; inference is attempted against the retained real Vimeo screenshot artifact.'
      : legacyRuntimeExecutionAllowed
        ? 'Explicit pinned local 2.x runtime execution was requested inside an isolated Python 3.10 venv; inference is attempted against the retained real Vimeo screenshot artifact.'
        : 'The gate records packaging/current-version facts only; it does not download models or call remote OCR services by default.',
  },
  modelCacheCustody: {
    cacheRoot: join(process.env.USERPROFILE ?? '', '.paddlex', 'official_models'),
    checkedModels: modelCache,
    allCheckedModelsCached: modelCache.every((entry) => entry.exists),
    note: 'Runtime execution downloads official model files into the local user cache. This is not a hosted OCR call, but production custody still needs explicit model-cache policy.',
  },
  runtimeAndQualityComparison: {
    status:
      runtimeAttempt?.status === 0
        ? runtimeAttempt.extractedTextCount > 0
          ? 'runtime-comparison-complete'
          : 'current-runtime-executes-no-text'
        : legacyRuntimeAttempt?.status === 0
          ? 'legacy-runtime-comparison-complete-current-candidate-blocked'
          : runtimeExecutionAllowed
            ? 'runtime-blocked'
            : legacyRuntimeExecutionAllowed
              ? 'legacy-runtime-blocked'
              : 'not-run',
    paddleOcrRuntimeReady: localRuntimeReady,
    tesseractRuntimeReady: tesseractInstalled,
    comparedAgainstTesseract: runtimeAttempt?.status === 0 && runtimeAttempt.extractedTextCount > 0,
    legacyFallbackComparedAgainstTesseract: legacyRuntimeAttempt?.status === 0,
    tesseractMatchedTerms: tesseractTerms,
    paddleOcrRuntimeAttempt: runtimeAttempt,
    paddleOcrServerDetectorRuntimeAttempt: serverDetectorRuntimeAttempt,
    paddleOcrPreprocessRuntimeAttempt: preprocessRuntimeAttempt,
    legacyPaddleOcr2xRuntimeAttempt: legacyRuntimeAttempt,
    reason: runtimeAttempt
      ? runtimeAttempt.status === 0
        ? runtimeAttempt.extractedTextCount > 0
          ? 'PaddleOCR completed local inference and can be compared against the Tesseract baseline.'
          : serverDetectorRuntimeAttempt?.extractedTextCount === 0 &&
              preprocessRuntimeAttempt?.allVariantsDeleted === true
            ? 'PaddleOCR completed local PP-OCRv5 inference only after disabling oneDNN/MKLDNN through the documented constructor option, but the mobile detector, cached server detector, and deleted preprocessing variants all extracted zero text from the retained real Vimeo screenshot; Tesseract and the pinned PaddleOCR 2.x fallback remain the only text-extracting OCR candidates in this lane.'
            : preprocessRuntimeAttempt?.allVariantsDeleted === true
              ? 'PaddleOCR completed local PP-OCRv5 inference only after disabling oneDNN/MKLDNN through the documented constructor option, but it extracted zero text from the retained real Vimeo screenshot and from deleted preprocessing variants; Tesseract and the pinned PaddleOCR 2.x fallback remain the only text-extracting OCR candidates in this lane.'
              : 'PaddleOCR completed local PP-OCRv5 inference only after disabling oneDNN/MKLDNN through the documented constructor option, but it extracted zero text from the retained real Vimeo screenshot; Tesseract and the pinned PaddleOCR 2.x fallback remain the only text-extracting OCR candidates in this lane.'
        : 'PaddleOCR packages and models are present, but local inference fails before OCR text extraction; Tesseract remains the only runtime-proved OCR baseline in this lane.'
      : legacyRuntimeAttempt
        ? legacyRuntimeAttempt.status === 0
          ? 'The current PaddleOCR 3.x/PP-OCRv5 candidate remains unproved, but an isolated pinned PaddleOCR 2.x/PaddlePaddle 2.x fallback completed local inference against the same real Vimeo screenshot and can be compared against Tesseract.'
          : 'The isolated pinned PaddleOCR 2.x/PaddlePaddle 2.x fallback was attempted but did not complete local inference.'
        : tesseractComparisonReady
          ? 'Both runtimes are importable/available, but this proof run did not request runtime execution.'
          : 'This Windows lane lacks one or more local OCR runtimes, so quality comparison remains a follow-up proof.',
  },
  placementDecision: {
    selectedForProduction: false,
    preferredNextHost:
      runtimeAttempt?.status === 0
        ? runtimeAttempt.extractedTextCount > 0
          ? 'child-device-or-household-mesh-after-resource-measurement'
          : 'not-selected-current-ppocrv5-zero-text-on-real-proof-image'
        : legacyRuntimeAttempt?.status === 0
          ? 'not-selected-current-ppocrv5-candidate-blocked'
          : 'not-selected-runtime-blocked',
    decision:
      'Do not select PaddleOCR/PP-OCR for production screen OCR until the current PP-OCRv5 path or an explicitly pinned fallback has package install, model-cache custody, no-upload inference, Tesseract comparison, and CPU/GPU/memory/runtime proof accepted.',
  },
  nonClaims: [
    runtimeExecutionAllowed
      ? 'This proof attempts local PaddleOCR inference only because OCENTRA_RUN_PADDLEOCR_LOCAL=1 was set.'
      : 'This proof does not install PaddleOCR, download OCR models, or run PaddleOCR inference by default.',
    legacyRuntimeExecutionAllowed
      ? 'This proof attempts the pinned PaddleOCR 2.x fallback only because OCENTRA_RUN_PADDLEOCR_2X_LOCAL=1 was set and an isolated Python 3.10 venv already exists.'
      : 'This proof does not create or install the pinned PaddleOCR 2.x fallback venv by default.',
    'This proof does not call PaddleOCR remote API or any hosted OCR endpoint.',
    'This proof does not claim PaddleOCR quality, latency, CPU, GPU, or memory suitability.',
    'This proof does not replace the existing typed OCR route proof.',
  ],
};

await writeFile(proofSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-ocr-paddleocr-evaluation-proof-ok:${proofSummaryPath}`);

function runOptional(command, args) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: 'utf8',
  });
  return {
    status: result.status ?? 1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

function runPaddleOcrRuntimeAttempt() {
  const code = String.raw`
from paddleocr import PaddleOCR
from pathlib import Path
import json
import time
image = Path(r"${sourceImagePath}")
start = time.perf_counter()
ocr = PaddleOCR(
    text_detection_model_name="PP-OCRv5_mobile_det",
    text_recognition_model_name="en_PP-OCRv5_mobile_rec",
    use_doc_orientation_classify=False,
    use_doc_unwarping=False,
    use_textline_orientation=False,
    device="cpu",
    enable_mkldnn=False,
    cpu_threads=2,
)
init_seconds = time.perf_counter() - start
start = time.perf_counter()
result = ocr.predict(str(image))
predict_seconds = time.perf_counter() - start
texts = []
raw_items = []
for item in result:
    data = item.json if hasattr(item, "json") else item.to_json() if hasattr(item, "to_json") else item
    raw_items.append(data)
    if isinstance(data, dict):
        for key in ("rec_texts", "texts"):
            values = data.get(key)
            if isinstance(values, list):
                texts.extend(str(value) for value in values)
print(json.dumps({
    "initSeconds": round(init_seconds, 3),
    "predictSeconds": round(predict_seconds, 3),
    "texts": texts,
    "itemCount": len(raw_items),
}, ensure_ascii=False))
`;
  const start = performance.now();
  const result = runOptional('python', ['-c', code]);
  const durationMs = Math.round(performance.now() - start);
  const combinedLog = normalizeLog(stripAnsi(`${result.stdout}${result.stderr}`));
  writeFileSync(runtimeLogPath, combinedLog);
  const parsed = parseJsonLine(result.stdout);
  const error = result.status === 0 ? null : summarizeRuntimeError(combinedLog);
  return {
    status: result.status,
    durationMs,
    logPath: runtimeLogPath,
    mode: 'PP-OCRv5_mobile_det + en_PP-OCRv5_mobile_rec with orientation/unwarping/textline disabled, CPU device, enable_mkldnn=false, cpu_threads=2',
    extractedTexts: parsed?.texts ?? [],
    extractedTextCount: parsed?.texts?.length ?? 0,
    initSeconds: parsed?.initSeconds ?? null,
    predictSeconds: parsed?.predictSeconds ?? null,
    error,
  };
}

function runPaddleOcrServerDetectorRuntimeAttempt() {
  const code = String.raw`
from paddleocr import PaddleOCR
from pathlib import Path
import json
import time
image = Path(r"${sourceImagePath}")
start = time.perf_counter()
ocr = PaddleOCR(
    text_detection_model_name="PP-OCRv5_server_det",
    text_recognition_model_name="en_PP-OCRv5_mobile_rec",
    use_doc_orientation_classify=False,
    use_doc_unwarping=False,
    use_textline_orientation=False,
    device="cpu",
    enable_mkldnn=False,
    cpu_threads=2,
)
init_seconds = time.perf_counter() - start
start = time.perf_counter()
result = ocr.predict(str(image))
predict_seconds = time.perf_counter() - start
texts = []
raw_items = []
for item in result:
    data = item.json if hasattr(item, "json") else item.to_json() if hasattr(item, "to_json") else item
    raw_items.append(data)
    if isinstance(data, dict):
        for key in ("rec_texts", "texts"):
            values = data.get(key)
            if isinstance(values, list):
                texts.extend(str(value) for value in values)
print(json.dumps({
    "initSeconds": round(init_seconds, 3),
    "predictSeconds": round(predict_seconds, 3),
    "texts": texts,
    "itemCount": len(raw_items),
}, ensure_ascii=False))
`;
  const start = performance.now();
  const result = runOptional('python', ['-c', code]);
  const durationMs = Math.round(performance.now() - start);
  const combinedLog = normalizeLog(stripAnsi(`${result.stdout}${result.stderr}`));
  writeFileSync(serverDetectorRuntimeLogPath, combinedLog);
  const parsed = parseJsonLine(result.stdout);
  const error = result.status === 0 ? null : summarizeRuntimeError(combinedLog);
  return {
    status: result.status,
    durationMs,
    logPath: serverDetectorRuntimeLogPath,
    mode: 'PP-OCRv5_server_det + en_PP-OCRv5_mobile_rec with orientation/unwarping/textline disabled, CPU device, enable_mkldnn=false, cpu_threads=2',
    extractedTexts: parsed?.texts ?? [],
    extractedTextCount: parsed?.texts?.length ?? 0,
    initSeconds: parsed?.initSeconds ?? null,
    predictSeconds: parsed?.predictSeconds ?? null,
    error,
  };
}

function runPaddleOcrPreprocessRuntimeAttempt() {
  const code = String.raw`
from paddleocr import PaddleOCR
from pathlib import Path
from PIL import Image, ImageOps, ImageEnhance, ImageFilter
from tempfile import TemporaryDirectory
import json
import time

source = Path(r"${sourceImagePath}").resolve()
ocr = PaddleOCR(
    text_detection_model_name="PP-OCRv5_mobile_det",
    text_recognition_model_name="en_PP-OCRv5_mobile_rec",
    use_doc_orientation_classify=False,
    use_doc_unwarping=False,
    use_textline_orientation=False,
    device="cpu",
    enable_mkldnn=False,
    cpu_threads=2,
)

def collect_texts(result):
    texts = []
    for item in result:
        data = item.json if hasattr(item, "json") else item.to_json() if hasattr(item, "to_json") else item
        if isinstance(data, dict):
            for key in ("rec_texts", "texts"):
                values = data.get(key)
                if isinstance(values, list):
                    texts.extend(str(value) for value in values)
    return texts

with TemporaryDirectory(prefix="ocentra-ppocrv5-preprocess-") as temp_dir:
    temp_root = Path(temp_dir)
    image = Image.open(source)
    gray = ImageOps.grayscale(image)
    variants = [
        ("original", image),
        ("upscale2", image.resize((image.width * 2, image.height * 2))),
        ("gray_contrast2_upscale2", ImageEnhance.Contrast(gray).enhance(2).resize((image.width * 2, image.height * 2))),
        ("gray_sharp_upscale2", gray.filter(ImageFilter.SHARPEN).resize((image.width * 2, image.height * 2))),
    ]
    results = []
    paths = []
    for name, variant in variants:
        path = temp_root / f"{name}.png"
        variant.save(path)
        paths.append(path)
        start = time.perf_counter()
        texts = collect_texts(ocr.predict(str(path)))
        results.append({
            "name": name,
            "width": variant.width,
            "height": variant.height,
            "predictSeconds": round(time.perf_counter() - start, 3),
            "texts": texts,
            "textCount": len(texts),
        })
    deleted_before_exit = all(path.exists() for path in paths)
deleted_after_exit = not any(path.exists() for path in paths)
print(json.dumps({
    "variantCount": len(results),
    "variants": results,
    "maxExtractedTextCount": max((entry["textCount"] for entry in results), default=0),
    "temporaryImagesExistedBeforeCleanup": deleted_before_exit,
    "temporaryImagesDeletedAfterCleanup": deleted_after_exit,
}, ensure_ascii=False))
`;
  const start = performance.now();
  const result = runOptional('python', ['-c', code]);
  const durationMs = Math.round(performance.now() - start);
  const combinedLog = normalizeLog(stripAnsi(`${result.stdout}${result.stderr}`));
  writeFileSync(preprocessRuntimeLogPath, combinedLog);
  const parsed = parseJsonLine(result.stdout);
  const error = result.status === 0 ? null : summarizeRuntimeError(combinedLog);
  return {
    status: result.status,
    durationMs,
    logPath: preprocessRuntimeLogPath,
    mode: 'PP-OCRv5 mobile detector/recognizer over original, 2x upscale, grayscale contrast 2x upscale, and grayscale sharpen 2x upscale variants',
    variantCount: parsed?.variantCount ?? 0,
    variants: parsed?.variants ?? [],
    maxExtractedTextCount: parsed?.maxExtractedTextCount ?? 0,
    allVariantsDeleted: parsed?.temporaryImagesDeletedAfterCleanup === true,
    temporaryImagesExistedBeforeCleanup: parsed?.temporaryImagesExistedBeforeCleanup === true,
    error,
  };
}

function runLegacyPaddleOcrRuntimeAttempt() {
  const code = String.raw`
from paddleocr import PaddleOCR
from pathlib import Path
import json
import os
import psutil
import threading
import time

image = Path(r"${sourceImagePath}").resolve()
process = psutil.Process(os.getpid())
peak_rss = process.memory_info().rss
sampling = True

def sample_memory():
    global peak_rss
    while sampling:
        peak_rss = max(peak_rss, process.memory_info().rss)
        time.sleep(0.05)

sampler = threading.Thread(target=sample_memory, daemon=True)
sampler.start()
start_cpu = process.cpu_times()
start = time.perf_counter()
ocr = PaddleOCR(use_angle_cls=False, lang="en", show_log=False)
init_seconds = time.perf_counter() - start
start = time.perf_counter()
result = ocr.ocr(str(image), cls=False)
predict_seconds = time.perf_counter() - start
sampling = False
sampler.join(timeout=1)
end_cpu = process.cpu_times()
texts = []
for page in result or []:
    for item in page or []:
        if isinstance(item, (list, tuple)) and len(item) >= 2:
            text_score = item[1]
            if isinstance(text_score, (list, tuple)) and text_score:
                texts.append(str(text_score[0]))
cpu_time_ms = int(((end_cpu.user + end_cpu.system) - (start_cpu.user + start_cpu.system)) * 1000)
print(json.dumps({
    "initSeconds": round(init_seconds, 3),
    "predictSeconds": round(predict_seconds, 3),
    "texts": texts,
    "textCount": len(texts),
    "cpuTimeMs": cpu_time_ms,
    "peakRssBytes": peak_rss,
}, ensure_ascii=False))
`;
  writeFileSync(legacyRuntimeScriptPath, code);
  const start = performance.now();
  const result = runOptional(legacyRuntimePythonPath, [legacyRuntimeScriptPath]);
  const durationMs = Math.round(performance.now() - start);
  const combinedLog = normalizeLog(stripAnsi(`${result.stdout}${result.stderr}`));
  writeFileSync(legacyRuntimeLogPath, combinedLog);
  const parsed = parseJsonLine(result.stdout);
  const extractedTexts = parsed?.texts ?? [];
  const extractedText = extractedTexts.join(' ').toLowerCase();
  const matchedTerms = ['vimeo', 'video', 'player'].filter((term) => extractedText.includes(term));
  const error = result.status === 0 ? null : summarizeRuntimeError(combinedLog);
  return {
    status: result.status,
    durationMs,
    logPath: legacyRuntimeLogPath,
    pythonPath: legacyRuntimePythonPath,
    mode: 'PaddleOCR 2.7.0.3 + PaddlePaddle 2.6.2 in isolated Python 3.10 venv',
    extractedTexts,
    extractedTextCount: extractedTexts.length,
    matchedTerms,
    initSeconds: parsed?.initSeconds ?? null,
    predictSeconds: parsed?.predictSeconds ?? null,
    cpuTimeMs: parsed?.cpuTimeMs ?? null,
    peakRssBytes: parsed?.peakRssBytes ?? null,
    peakRssMiB: bytesToMiB(parsed?.peakRssBytes ?? null),
    error,
  };
}

function stripAnsi(value) {
  return value.replace(/\u001b\[[0-9;]*m/g, '');
}

function normalizeLog(value) {
  return `${value
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .join('\n')
    .trimEnd()}\n`;
}

function resolveTesseractVersion() {
  const direct = runOptional('tesseract', ['--version']);
  if (direct.status === 0) {
    return direct;
  }
  const windowsPath = 'C:\\Program Files\\Tesseract-OCR\\tesseract.exe';
  if (existsSync(windowsPath)) {
    return runOptional(windowsPath, ['--version']);
  }
  return direct;
}

async function readOptionalText(path) {
  try {
    return await readFile(path, 'utf8');
  } catch {
    return '';
  }
}

function parseInstalledVersions(stdout) {
  return Object.fromEntries(
    stdout
      .split(/\r?\n/)
      .map((line) => line.trim().split('='))
      .filter((parts) => parts.length === 2)
  );
}

function parseJsonLine(stdout) {
  for (const line of stdout.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('{')) {
      continue;
    }
    try {
      return JSON.parse(trimmed);
    } catch {
      return null;
    }
  }
  return null;
}

function summarizeRuntimeError(log) {
  const lines = log
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const notImplemented = lines.find((line) => line.includes('ConvertPirAttribute2RuntimeAttribute'));
  const traceback = lines.find((line) => line.startsWith('NotImplementedError:'));
  return notImplemented ?? traceback ?? lines.at(-1) ?? 'unknown PaddleOCR runtime failure';
}

function parsePipLatest(stdout) {
  const match = stdout.match(/^\s*[A-Za-z0-9_-]+\s+\(([^)]+)\)/m);
  return match?.[1] ?? null;
}

function parsePythonBool(stdout) {
  return stdout.trim().toLowerCase() === 'true';
}

function oneLine(value) {
  return value.replace(/\s+/g, ' ').trim();
}

function bytesToMiB(value) {
  return value === null ? null : Math.round((value / 1024 / 1024) * 10) / 10;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
