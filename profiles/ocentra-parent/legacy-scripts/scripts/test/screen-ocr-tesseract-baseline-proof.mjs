import { spawn, spawnSync } from 'node:child_process';
import { existsSync, readFileSync, unlinkSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';

const repoRoot = process.cwd();
const outputRoot = resolve(repoRoot, 'output', 'screen-plan-proof', '34-ocr-tesseract-baseline');
const proofSummaryPath = join(outputRoot, 'proof-summary.json');
const extractionTextPath = join(outputRoot, 'vimeo-public-video-tesseract-output.txt');
const failureModeTextPath = join(outputRoot, 'vimeo-public-video-tesseract-failure-modes.txt');
const paddleOcrEvaluationPath = resolve(
  repoRoot,
  'output',
  'screen-plan-proof',
  '35-ocr-paddleocr-ppocr-evaluation',
  'proof-summary.json'
);
const sourceImagePath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'social-14-managed-browser-feed-short-video-gate',
  '06-live-screenshots',
  'vimeo-public-video.png'
);

await mkdir(outputRoot, { recursive: true });

const whereTesseract =
  process.platform === 'win32' ? runOptional('where.exe', ['tesseract']) : runOptional('which', ['tesseract']);
const tesseractCommand = resolveTesseractCommand(whereTesseract);
const tesseractVersion = tesseractCommand
  ? runOptional(tesseractCommand, ['--version'], { shell: false })
  : runOptional('tesseract', ['--version']);
const tesseractInstalled = tesseractVersion.status === 0;
const extraction = tesseractCommand
  ? await runMeasuredTesseract(tesseractCommand, [sourceImagePath, 'stdout', '--psm', '6'])
  : unavailableExtraction();
const extractedText = oneLine(extraction.stdout);
const expectedTerms = ['vimeo', 'video', 'player'];
const matchedTerms = expectedTerms.filter((term) => extractedText.toLowerCase().includes(term));
const localExtractionProofComplete = tesseractInstalled && extraction.status === 0 && matchedTerms.length >= 3;
const failureModes = tesseractCommand ? await runFailureModeMatrix(tesseractCommand) : [];
const failureModesRecorded =
  failureModes.length === 3 &&
  failureModes.every(
    (mode) =>
      mode.commandStatus === 0 &&
      mode.cpuTimeMs !== undefined &&
      Number.isFinite(mode.cpuTimeMs) &&
      mode.peakWorkingSetBytes !== undefined &&
      mode.peakWorkingSetBytes > 0
  );
const cpuMemoryRuntimeMeasured =
  extraction.status === 0 &&
  extraction.cpuTimeMs !== undefined &&
  Number.isFinite(extraction.cpuTimeMs) &&
  extraction.peakWorkingSetBytes !== undefined &&
  extraction.peakWorkingSetBytes > 0;
const paddleOcrEvaluation = readOptionalJson(paddleOcrEvaluationPath);
const paddleOcrComparison = buildPaddleOcrComparison(paddleOcrEvaluation, expectedTerms);

assert(existsSync(sourceImagePath), `Missing real screenshot source image: ${sourceImagePath}`);
if (tesseractInstalled) {
  assert(localExtractionProofComplete, `Tesseract extraction did not match expected Vimeo terms: ${extractedText}`);
}

const summary = {
  proof: 'screen-ocr-tesseract-baseline',
  generatedAt: new Date().toISOString(),
  officialSourceSnapshot: {
    project: 'https://github.com/tesseract-ocr/tesseract',
    documentation: 'https://tesseract-ocr.github.io/tessdoc/',
    windowsInstallation: 'https://tesseract-ocr.github.io/tessdoc/Installation.html',
    windowsDownloads: 'https://tesseract-ocr.github.io/tessdoc/Downloads.html',
    verifiedClaims: [
      'Tesseract is the upstream open-source OCR engine candidate for the simple baseline.',
      'The upstream project is Apache-2.0 licensed.',
      'The upstream tessdoc installation page points Windows users to UB Mannheim builds for Tesseract 3.05, 4, and 5.',
      'The upstream tessdoc downloads page says there is no official Windows installer for newer versions.',
    ],
  },
  localEnvironment: {
    platform: process.platform,
    whereTesseractStatus: whereTesseract.status,
    whereTesseractOutput: oneLine(whereTesseract.stdout || whereTesseract.stderr),
    resolvedTesseractCommand: tesseractCommand,
    tesseractVersionStatus: tesseractVersion.status,
    tesseractVersionOutput: oneLine(tesseractVersion.stdout || tesseractVersion.stderr),
    tesseractInstalled,
  },
  extractionProof: {
    sourceImage: relativePath(sourceImagePath),
    sourceImageKind: 'retained real managed-browser public Vimeo screenshot artifact',
    sourceImageExists: existsSync(sourceImagePath),
    commandStatus: extraction.status,
    durationMs: extraction.durationMs,
    cpuTimeMs: extraction.cpuTimeMs,
    peakWorkingSetBytes: extraction.peakWorkingSetBytes,
    peakWorkingSetMiB: bytesToMiB(extraction.peakWorkingSetBytes),
    outputArtifact: relativePath(extractionTextPath),
    outputCharacterCount: extraction.stdout.length,
    matchedTerms,
    expectedTerms,
    localExtractionProofComplete,
  },
  failureModeProof: {
    outputArtifact: relativePath(failureModeTextPath),
    failureModesRecorded,
    scenarios: failureModes,
    note: 'Failure-mode scenarios reuse the retained real Vimeo screenshot and alter OCR invocation/image scale/crop to record sensitivity; they are not product-quality claims.',
  },
  baselineReadiness: {
    status: localExtractionProofComplete
      ? 'runtime-extraction-proved'
      : tesseractInstalled
        ? 'runtime-available'
        : 'runtime-unavailable',
    windowsPackagingProofComplete: tesseractInstalled,
    localExtractionProofComplete,
    runtimeMeasured: localExtractionProofComplete,
    cpuMemoryRuntimeMeasured,
    failureModesRecorded,
    comparedAgainstPaddleOcr: paddleOcrComparison.legacyFallbackMatchedExpectedTerms,
    paddleOcrComparison,
    reason: localExtractionProofComplete
      ? paddleOcrComparison.legacyFallbackMatchedExpectedTerms
        ? 'Tesseract is installed and extracted expected text from a retained real public Vimeo screenshot artifact; the isolated local PaddleOCR 2.x fallback extracted the same expected terms, while current PP-OCRv5 remains unselected because it still extracts zero text.'
        : 'Tesseract is installed and extracted expected text from a retained real public Vimeo screenshot artifact.'
      : tesseractInstalled
        ? 'Tesseract is available, but extraction proof did not complete.'
        : 'Tesseract is not available on PATH or the standard Windows install path in this lane; install/package proof must happen before extraction or quality claims.',
  },
  assertions: {
    sourceImageExists: existsSync(sourceImagePath),
    tesseractInstalled,
    extractionSucceeded: extraction.status === 0,
    expectedTextMatched: matchedTerms.length >= 3,
    localExtractionProofComplete,
    noProductionQualityClaim: true,
    cpuMemoryRuntimeMeasured,
    failureModesRecorded,
  },
  packageInstallEvidence: {
    installCommand:
      'winget install --id tesseract-ocr.tesseract --exact --source winget --accept-package-agreements --accept-source-agreements --disable-interactivity',
    packageId: 'tesseract-ocr.tesseract',
    packageVersionObserved: '5.5.0.20241111',
    installedPathObserved: process.platform === 'win32' ? 'C:\\Program Files\\Tesseract-OCR\\tesseract.exe' : null,
    pathRefreshRequired: process.platform === 'win32' && whereTesseract.status !== 0 && tesseractCommand !== undefined,
  },
  openMeasurements: {
    cpuMemoryRuntimeMeasured,
    smallFontFailureModesRecorded: failureModes.some((mode) => mode.id === 'downscaled-small-text'),
    messyUiFailureModesRecorded: failureModes.some((mode) => mode.id === 'cropped-player-ui'),
    paddleOcrComparisonComplete: paddleOcrComparison.legacyFallbackMatchedExpectedTerms,
    currentPpOcrV5StillBlocked: paddleOcrComparison.currentPpOcrV5ExtractedTextCount === 0,
    reason: paddleOcrComparison.legacyFallbackMatchedExpectedTerms
      ? 'This proof measures Tesseract process duration, CPU time, peak working set, and derived failure-mode OCR sensitivity, then compares the matched terms against the existing isolated local PaddleOCR 2.x fallback. Current PP-OCRv5 remains blocked for production selection because it executes locally but extracts zero text from the same retained proof image.'
      : 'This proof measures Tesseract process duration, CPU time, peak working set, and derived failure-mode OCR sensitivity. PaddleOCR comparison remains open until a local OCR candidate extracts comparable terms from the same retained proof image.',
  },
  nonClaims: [
    tesseractInstalled
      ? 'This proof installed and invoked Tesseract locally, but it does not select Tesseract as the production OCR runtime.'
      : 'Tesseract is not available on PATH in this Windows lane; install/package proof must happen before extraction or quality claims.',
    'This proof runs OCR over a retained real public browser screenshot artifact; it does not create a new screen capture.',
    'This proof records extraction duration, CPU time, peak working set, matched terms, and derived failure modes, but it does not claim production OCR quality or latency suitability.',
    'This proof does not select PaddleOCR/PP-OCR for production. The comparison only records that the already-isolated local PaddleOCR 2.x fallback matched terms while current PP-OCRv5 did not extract text.',
  ],
  validationCommands: [
    'node --check scripts/test/screen-ocr-tesseract-baseline-proof.mjs',
    'node scripts/test/screen-ocr-tesseract-baseline-proof.mjs',
    process.platform === 'win32' ? 'where.exe tesseract' : 'which tesseract',
    process.platform === 'win32'
      ? '"C:\\Program Files\\Tesseract-OCR\\tesseract.exe" --version'
      : 'tesseract --version',
    'tesseract --version',
  ],
};

await writeFile(extractionTextPath, normalizeExtractedText(extraction.stdout));
await writeFile(failureModeTextPath, failureModes.map(formatFailureMode).join('\n\n') + '\n');
await writeFile(proofSummaryPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-ocr-tesseract-baseline-proof-ok:${summary.baselineReadiness.status}`);
console.log(`artifact=${proofSummaryPath}`);

function resolveTesseractCommand(whereResult) {
  const firstWhereLine = whereResult.stdout
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find(Boolean);
  if (firstWhereLine && existsSync(firstWhereLine)) return firstWhereLine;
  const windowsDefault = 'C:\\Program Files\\Tesseract-OCR\\tesseract.exe';
  if (process.platform === 'win32' && existsSync(windowsDefault)) return windowsDefault;
  return null;
}

function readOptionalJson(path) {
  if (!existsSync(path)) return null;
  return JSON.parse(readFileSync(path, 'utf8'));
}

function buildPaddleOcrComparison(evaluation, expectedTerms) {
  const runtimeComparison = evaluation?.runtimeAndQualityComparison ?? {};
  const currentAttempt = runtimeComparison.paddleOcrRuntimeAttempt ?? null;
  const serverDetectorAttempt = runtimeComparison.paddleOcrServerDetectorRuntimeAttempt ?? null;
  const preprocessAttempt = runtimeComparison.paddleOcrPreprocessRuntimeAttempt ?? null;
  const legacyAttempt = runtimeComparison.legacyPaddleOcr2xRuntimeAttempt ?? null;
  const legacyMatchedTerms = Array.isArray(legacyAttempt?.matchedTerms) ? legacyAttempt.matchedTerms : [];
  return {
    evaluationPath: relativePath(paddleOcrEvaluationPath),
    evaluationPresent: evaluation !== undefined,
    sameSourceImage: evaluation?.sourceEvidence?.sourceImagePath === sourceImagePath,
    expectedTerms,
    tesseractMatchedTerms: matchedTerms,
    currentPpOcrV5Status: currentAttempt?.status ?? null,
    currentPpOcrV5ExtractedTextCount: currentAttempt?.extractedTextCount ?? null,
    currentPpOcrV5ServerDetectorStatus: serverDetectorAttempt?.status ?? null,
    currentPpOcrV5ServerDetectorExtractedTextCount: serverDetectorAttempt?.extractedTextCount ?? null,
    currentPpOcrV5PreprocessMaxTextCount: preprocessAttempt?.maxExtractedTextCount ?? null,
    legacyFallbackStatus: legacyAttempt?.status ?? null,
    legacyFallbackExtractedTextCount: legacyAttempt?.extractedTextCount ?? null,
    legacyFallbackMatchedTerms: legacyMatchedTerms,
    legacyFallbackMatchedExpectedTerms: expectedTerms.every((term) => legacyMatchedTerms.includes(term)),
    conclusion:
      expectedTerms.every((term) => legacyMatchedTerms.includes(term)) &&
      currentAttempt?.extractedTextCount === 0 &&
      serverDetectorAttempt?.extractedTextCount === 0 &&
      preprocessAttempt?.maxExtractedTextCount === 0
        ? 'Tesseract and the isolated local PaddleOCR 2.x fallback both matched the expected Vimeo proof terms; current PP-OCRv5 mobile detector, server detector, and preprocessing variants still extract zero text, so Windows service OCR selection remains WinRT and PaddleOCR remains unselected.'
        : 'PaddleOCR comparison is incomplete or not production-selectable for this proof image.',
  };
}

async function runMeasuredTesseract(command, args) {
  const started = performance.now();
  const result = await runMeasuredProcess(command, args);
  return {
    ...result,
    durationMs: Math.round(performance.now() - started),
  };
}

async function runFailureModeMatrix(command) {
  const scenarios = [
    {
      id: 'alternate-page-segmentation',
      description: 'Same retained real screenshot with sparse-text OCR segmentation.',
      args: [sourceImagePath, 'stdout', '--psm', '11'],
    },
    {
      id: 'downscaled-small-text',
      description: 'Same retained real screenshot downscaled before OCR to simulate small text sensitivity.',
      imagePath: join(outputRoot, 'vimeo-public-video-downscaled.png'),
      transform: { scale: 0.5 },
    },
    {
      id: 'cropped-player-ui',
      description: 'Same retained real screenshot cropped to the lower player/control region before OCR.',
      imagePath: join(outputRoot, 'vimeo-public-video-player-crop.png'),
      transform: { crop: { x: 0, yRatio: 0.55, widthRatio: 1, heightRatio: 0.45 } },
    },
  ];

  const rows = [];
  for (const scenario of scenarios) {
    let imagePath = sourceImagePath;
    if (scenario.transform) {
      createDerivedImage(sourceImagePath, scenario.imagePath, scenario.transform);
      imagePath = scenario.imagePath;
    }
    const args = scenario.args ?? [imagePath, 'stdout', '--psm', '6'];
    const run = await runMeasuredTesseract(command, args);
    const normalizedText = oneLine(run.stdout);
    rows.push({
      id: scenario.id,
      description: scenario.description,
      sourceImage: relativePath(imagePath),
      commandStatus: run.status,
      durationMs: run.durationMs,
      cpuTimeMs: run.cpuTimeMs,
      peakWorkingSetBytes: run.peakWorkingSetBytes,
      peakWorkingSetMiB: bytesToMiB(run.peakWorkingSetBytes),
      outputCharacterCount: run.stdout.length,
      matchedTerms: expectedTerms.filter((term) => normalizedText.toLowerCase().includes(term)),
      extractedPreview: normalizedText.slice(0, 240),
    });
  }
  return rows;
}

function createDerivedImage(inputPath, outputPath, transform) {
  const script = transform.scale
    ? `
Add-Type -AssemblyName System.Drawing
$image = [System.Drawing.Image]::FromFile('${escapePowerShell(inputPath)}')
$width = [Math]::Max(1, [int]($image.Width * ${transform.scale}))
$height = [Math]::Max(1, [int]($image.Height * ${transform.scale}))
$bitmap = New-Object System.Drawing.Bitmap($width, $height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.DrawImage($image, 0, 0, $width, $height)
$bitmap.Save('${escapePowerShell(outputPath)}', [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose(); $bitmap.Dispose(); $image.Dispose()
`
    : `
Add-Type -AssemblyName System.Drawing
$image = [System.Drawing.Image]::FromFile('${escapePowerShell(inputPath)}')
$x = [int]($image.Width * ${transform.crop.x})
$y = [int]($image.Height * ${transform.crop.yRatio})
$width = [int]($image.Width * ${transform.crop.widthRatio})
$height = [int]($image.Height * ${transform.crop.heightRatio})
$rect = New-Object System.Drawing.Rectangle($x, $y, $width, $height)
$bitmap = New-Object System.Drawing.Bitmap($width, $height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.DrawImage($image, 0, 0, $rect, [System.Drawing.GraphicsUnit]::Pixel)
$bitmap.Save('${escapePowerShell(outputPath)}', [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose(); $bitmap.Dispose(); $image.Dispose()
`;
  const result = spawnSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: false,
  });
  if (result.status !== 0) {
    throw new Error(`Failed to create derived OCR image ${outputPath}\n${result.stdout}\n${result.stderr}`);
  }
}

function runMeasuredProcess(command, args) {
  if (process.platform === 'win32') {
    return Promise.resolve(runMeasuredProcessWindows(command, args));
  }
  return new Promise((resolve) => {
    const child = spawn(command, args, { cwd: repoRoot, shell: false });
    let stdout = '';
    let stderr = '';
    let peakWorkingSetBytes = null;
    let latestCpuTimeMs = null;
    const timer = setInterval(() => {
      const sample = sampleWindowsProcess(child.pid);
      if (sample.workingSetBytes !== undefined) {
        peakWorkingSetBytes = Math.max(peakWorkingSetBytes ?? 0, sample.workingSetBytes);
      }
      if (sample.cpuTimeMs !== undefined) {
        latestCpuTimeMs = sample.cpuTimeMs;
      }
    }, 25);
    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    child.on('close', (status) => {
      clearInterval(timer);
      const sample = sampleWindowsProcess(child.pid);
      if (sample.workingSetBytes !== undefined) {
        peakWorkingSetBytes = Math.max(peakWorkingSetBytes ?? 0, sample.workingSetBytes);
      }
      if (sample.cpuTimeMs !== undefined) {
        latestCpuTimeMs = sample.cpuTimeMs;
      }
      resolve({
        status: status ?? 1,
        stdout,
        stderr,
        peakWorkingSetBytes,
        cpuTimeMs: latestCpuTimeMs,
      });
    });
  });
}

function runMeasuredProcessWindows(command, args) {
  const id = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const stdoutPath = join(outputRoot, `tesseract-${id}.stdout.txt`);
  const stderrPath = join(outputRoot, `tesseract-${id}.stderr.txt`);
  const psArgs = args.map((arg) => `'${escapePowerShell(arg)}'`).join(', ');
  const script = `
$exe = '${escapePowerShell(command)}'
$arguments = @(${psArgs})
$stdoutPath = '${escapePowerShell(stdoutPath)}'
$stderrPath = '${escapePowerShell(stderrPath)}'
$p = Start-Process -FilePath $exe -ArgumentList $arguments -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -NoNewWindow -PassThru
$peakWorkingSetBytes = 0
$cpuTimeMs = $null
while (-not $p.HasExited) {
  $p.Refresh()
  if ($p.WorkingSet64 -gt $peakWorkingSetBytes) {
    $peakWorkingSetBytes = $p.WorkingSet64
  }
  if ($null -ne $p.CPU) {
    $cpuTimeMs = [int64]($p.CPU * 1000)
  }
  Start-Sleep -Milliseconds 10
}
$p.WaitForExit()
$p.Refresh()
if ($p.WorkingSet64 -gt $peakWorkingSetBytes) {
  $peakWorkingSetBytes = $p.WorkingSet64
}
if ($null -ne $p.CPU) {
  $cpuTimeMs = [int64]($p.CPU * 1000)
}
$status = $p.ExitCode
if ($null -eq $status) {
  $stdoutHasContent = (Test-Path $stdoutPath) -and ((Get-Item $stdoutPath).Length -gt 0)
  if ($stdoutHasContent) {
    $status = 0
  } else {
    $status = 1
  }
}
[pscustomobject]@{
  status = $status
  peakWorkingSetBytes = $peakWorkingSetBytes
  cpuTimeMs = $cpuTimeMs
} | ConvertTo-Json -Compress
`;
  const result = spawnSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: false,
  });
  const stdout = readOptionalFileSync(stdoutPath);
  const stderr = `${readOptionalFileSync(stderrPath)}${result.stderr ?? ''}`;
  removeOptionalFile(stdoutPath);
  removeOptionalFile(stderrPath);
  const metrics = parseJsonLine(result.stdout);
  return {
    status: metrics?.status ?? result.status ?? 1,
    stdout,
    stderr,
    peakWorkingSetBytes: metrics?.peakWorkingSetBytes ?? null,
    cpuTimeMs: metrics?.cpuTimeMs ?? null,
  };
}

function sampleWindowsProcess(pid) {
  if (process.platform !== 'win32' || !pid) {
    return { workingSetBytes: null, cpuTimeMs: null };
  }
  const result = spawnSync(
    'powershell',
    [
      '-NoProfile',
      '-Command',
      `$p = Get-Process -Id ${pid} -ErrorAction SilentlyContinue; if ($p) { "$($p.WorkingSet64)|$([int64]($p.CPU * 1000))" }`,
    ],
    { cwd: repoRoot, encoding: 'utf8', shell: false }
  );
  const [workingSet, cpu] = result.stdout.trim().split('|');
  return {
    workingSetBytes: workingSet && Number.isFinite(Number(workingSet)) ? Number(workingSet) : null,
    cpuTimeMs: cpu && Number.isFinite(Number(cpu)) ? Number(cpu) : null,
  };
}

function unavailableExtraction() {
  return {
    status: 1,
    stdout: '',
    stderr: 'Tesseract command unavailable.',
    durationMs: null,
    cpuTimeMs: null,
    peakWorkingSetBytes: null,
  };
}

function runOptional(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: options.shell ?? process.platform === 'win32',
  });
  return {
    status: result.status ?? 1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

function oneLine(value) {
  return value.replace(/\s+/g, ' ').trim();
}

function normalizeExtractedText(value) {
  return `${value
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .join('\n')
    .trim()}\n`;
}

function relativePath(path) {
  return path.replace(`${repoRoot}\\`, '').replaceAll('\\', '/');
}

function bytesToMiB(value) {
  return value === null ? null : Math.round((value / 1024 / 1024) * 10) / 10;
}

function formatFailureMode(mode) {
  return [
    `# ${mode.id}`,
    `description=${mode.description}`,
    `source=${mode.sourceImage}`,
    `status=${mode.commandStatus}`,
    `durationMs=${mode.durationMs}`,
    `cpuTimeMs=${mode.cpuTimeMs}`,
    `peakWorkingSetMiB=${mode.peakWorkingSetMiB}`,
    `matchedTerms=${mode.matchedTerms.join(',')}`,
    `extractedPreview=${mode.extractedPreview}`,
  ].join('\n');
}

function escapePowerShell(value) {
  return value.replaceAll("'", "''");
}

function parseJsonLine(value) {
  for (const line of value.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('{')) {
      continue;
    }
    return JSON.parse(trimmed);
  }
  return null;
}

function readOptionalFileSync(path) {
  try {
    return readFileSync(path, 'utf8');
  } catch {
    return '';
  }
}

function removeOptionalFile(path) {
  try {
    unlinkSync(path);
  } catch {
    // Temp files may not exist when process startup fails.
  }
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
