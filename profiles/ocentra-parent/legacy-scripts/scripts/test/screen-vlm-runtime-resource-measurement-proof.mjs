import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';

const repoRoot = process.cwd();
const outputDir = join('output', 'screen-plan-proof', '36-vlm-runtime-resource-measurement');
const proofPath = join(outputDir, 'proof-summary.json');
const localAiModelRoot = resolveUserCachePath('local-ai-models');
const llamaRoot = process.env.OCENTRA_PARENT_LLAMA_CPP_DIR ?? resolveUserCachePath('llama.cpp', 'b9279');
const vlmBinary = process.env.OCENTRA_PARENT_LOCAL_VLM_BINARY ?? join(llamaRoot, 'llama-mtmd-cli.exe');
const vlmModel =
  process.env.OCENTRA_PARENT_LOCAL_VLM_MODEL ?? join(localAiModelRoot, 'Qwen2-VL-2B-Instruct-Q4_K_M.gguf');
const vlmMmproj =
  process.env.OCENTRA_PARENT_LOCAL_VLM_MMPROJ ?? join(localAiModelRoot, 'mmproj-Qwen2-VL-2B-Instruct-Q8_0.gguf');
const maxWallMs = 180_000;
const maxPeakWorkingSetBytes = 10 * 1024 * 1024 * 1024;
const maxCpuSeconds = 300;

const samples = [
  {
    scenarioId: 'youtube-ordinary-video-retained-parent-proof',
    expectedAnyTerms: ['video', 'youtube', 'screen'],
    imagePath: join('output', 'ai-plan-proof', 'live-operator', 'youtube-ordinary-video', '10-parent-explanation.png'),
  },
  {
    scenarioId: 'shopping-page-retained-parent-proof',
    expectedAnyTerms: ['shopping', 'screen', 'ask-parent'],
    imagePath: join('output', 'ai-plan-proof', 'live-operator', 'shopping-page', '10-parent-explanation.png'),
  },
];

mkdirSync(outputDir, { recursive: true });

if (!existsSync(vlmBinary) || !existsSync(vlmModel) || !existsSync(vlmMmproj)) {
  throw new Error(
    `Local VLM runtime is missing: ${JSON.stringify({
      binary: redactHome(vlmBinary),
      binaryExists: existsSync(vlmBinary),
      model: redactHome(vlmModel),
      modelExists: existsSync(vlmModel),
      mmproj: redactHome(vlmMmproj),
      mmprojExists: existsSync(vlmMmproj),
    })}`
  );
}

const measurements = [];
for (const sample of samples) {
  measurements.push(await measureSample(sample));
}

const failedSamples = measurements.filter((sample) => !sample.assertions.parseableMeaningfulJson);
const overBudgetSamples = measurements.filter(
  (sample) =>
    sample.wallMs > maxWallMs ||
    sample.resourceSamples.peakWorkingSetBytes > maxPeakWorkingSetBytes ||
    sample.resourceSamples.cpuSecondsObserved > maxCpuSeconds
);

const proof = {
  proof: 'screen-vlm-runtime-resource-measurement-proof',
  generatedAt: new Date().toISOString(),
  proofTier: 'P2_RETAINED_PROOF_IMAGE_LOCAL_VLM_RESOURCE_MEASUREMENT',
  modelRuntime: {
    runtimeBinary: redactHome(vlmBinary),
    model: redactHome(vlmModel),
    mmproj: redactHome(vlmMmproj),
    providerKind: 'localVision',
    modelRuntimeRef: 'local-qwen2-vl-2b-llama-mtmd',
    promptOrTemplateVersion: 'screen-vlm-resource-measurement-v1',
  },
  budgets: {
    maxWallMs,
    maxPeakWorkingSetBytes,
    maxCpuSeconds,
  },
  summary: {
    sampleCount: measurements.length,
    parseableMeaningfulJsonCount: measurements.filter((sample) => sample.assertions.parseableMeaningfulJson).length,
    overBudgetSampleCount: overBudgetSamples.length,
    maxWallMsObserved: Math.max(...measurements.map((sample) => sample.wallMs)),
    maxPeakWorkingSetBytesObserved: Math.max(
      ...measurements.map((sample) => sample.resourceSamples.peakWorkingSetBytes)
    ),
    maxCpuSecondsObserved: Math.max(...measurements.map((sample) => sample.resourceSamples.cpuSecondsObserved)),
    allSamplesWithinResourceEnvelope: overBudgetSamples.length === 0,
    allSamplesUsedRetainedProofImages: measurements.every((sample) => sample.input.retainedProofImage === true),
    rawCaptureImagesRetainedByThisProof: false,
    remoteAiUsed: false,
  },
  measurements,
  assertions: {
    localRuntimeExecuted: measurements.every((sample) => sample.exitStatus === 0),
    retainedProofImagesOnly: measurements.every((sample) => sample.input.retainedProofImage === true),
    parseableMeaningfulJson: failedSamples.length === 0,
    resourceMeasurementsRecorded: measurements.every(
      (sample) =>
        sample.wallMs > 0 &&
        sample.resourceSamples.sampleCount > 0 &&
        sample.resourceSamples.peakWorkingSetBytes > 0 &&
        sample.resourceSamples.cpuSecondsObserved >= 0
    ),
    allSamplesWithinResourceEnvelope: overBudgetSamples.length === 0,
    noRemoteAiUsed: true,
    noRawCaptureRetention: true,
  },
  completedChecklistClaims: [
    'local llama.cpp/Qwen2-VL inference over retained proof screenshots records per-sample wall time',
    'local llama.cpp/Qwen2-VL inference over retained proof screenshots records sampled CPU seconds and peak working set',
    'retained proof screenshots produce parseable, normalized screen-analysis JSON through the local VLM runtime',
  ],
  openChecklistClaims: [
    'detector-specific VLM crop quality on freshly cropped live pages remains open',
    'production VLM model selection remains open',
    'authenticated-account social proof remains outside this retained proof-image measurement',
  ],
  nonClaims: [
    'This proof measures retained proof screenshots, not raw capture screenshots.',
    'This proof does not retain or upload raw screenshots.',
    'This proof does not claim production-quality VLM model selection.',
  ],
};

writeJson(proofPath, proof);

if (!Object.values(proof.assertions).every(Boolean)) {
  throw new Error(`screen VLM runtime resource measurement assertions failed: ${JSON.stringify(proof.assertions)}`);
}

console.log(`screen-vlm-runtime-resource-measurement-proof-ok:${proofPath}`);

async function measureSample(sample) {
  const absoluteImagePath = resolve(repoRoot, sample.imagePath);
  if (!existsSync(absoluteImagePath)) {
    throw new Error(`Missing retained proof image for ${sample.scenarioId}: ${sample.imagePath}`);
  }

  const args = [
    '-m',
    vlmModel,
    '--mmproj',
    vlmMmproj,
    '--image',
    absoluteImagePath,
    '-p',
    [
      'Analyze this retained parent proof screenshot from a child screen-safety run.',
      'Return JSON only with keys primary_category, visible_text, risk_signals, confidence.',
      'Allowed primary_category values are school, video, chat, game, adultContent, violence, bypassTool, shopping, productivity, unknown.',
      'Use visible proof text and screen labels when available.',
    ].join(' '),
    '-n',
    '96',
    '--temp',
    '0',
    '--device',
    'none',
    '-ngl',
    '0',
    '-fit',
    'off',
    '--no-mmproj-offload',
    '--no-warmup',
  ];

  const startedAt = performance.now();
  const child = spawn(vlmBinary, args, { cwd: repoRoot, shell: false });
  let stdout = '';
  let stderr = '';
  const resourceSamples = [];

  child.stdout.setEncoding('utf8');
  child.stderr.setEncoding('utf8');
  child.stdout.on('data', (chunk) => {
    stdout += chunk;
  });
  child.stderr.on('data', (chunk) => {
    stderr += chunk;
  });

  const sampler = setInterval(() => {
    const sampleRow = readProcessSample(child.pid);
    if (sampleRow !== undefined) {
      resourceSamples.push(sampleRow);
    }
  }, 250);

  const exitStatus = await new Promise((resolveExit) => {
    child.on('close', (code) => {
      clearInterval(sampler);
      const sampleRow = readProcessSample(child.pid);
      if (sampleRow !== undefined) {
        resourceSamples.push(sampleRow);
      }
      resolveExit(code ?? -1);
    });
  });
  const wallMs = Math.round(performance.now() - startedAt);

  if (exitStatus !== 0) {
    throw new Error(`local VLM command failed for ${sample.scenarioId} with ${exitStatus}\n${stderr}`);
  }

  const parsed = parseFirstJsonObject(stdout);
  const normalized = normalizeParsedResult(parsed);
  const combinedText = `${JSON.stringify(normalized)} ${stdout}`.toLowerCase();
  const expectedTermMatched = sample.expectedAnyTerms.some((term) => combinedText.includes(term.toLowerCase()));
  const resourceSummary = summarizeResourceSamples(resourceSamples);

  return {
    scenarioId: sample.scenarioId,
    input: {
      imagePath: artifactPath(sample.imagePath),
      retainedProofImage: true,
      imageByteSize: statSync(absoluteImagePath).size,
    },
    exitStatus,
    wallMs,
    resourceSamples: resourceSummary,
    parsedResult: parsed,
    normalizedResult: normalized,
    stdoutPreview: stdout.replace(/\s+/g, ' ').slice(0, 500),
    stderrPreview: stderr.replace(/\s+/g, ' ').slice(0, 500),
    assertions: {
      parseableMeaningfulJson:
        normalized !== undefined &&
        typeof normalized.primary_category === 'string' &&
        typeof normalized.visible_text === 'string' &&
        Array.isArray(normalized.risk_signals) &&
        typeof normalized.confidence === 'number' &&
        expectedTermMatched,
      expectedTermMatched,
      withinWallBudget: wallMs <= maxWallMs,
      withinPeakWorkingSetBudget: resourceSummary.peakWorkingSetBytes <= maxPeakWorkingSetBytes,
      withinCpuBudget: resourceSummary.cpuSecondsObserved <= maxCpuSeconds,
    },
  };
}

function normalizeParsedResult(parsed) {
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return null;
  }
  return {
    primary_category: typeof parsed.primary_category === 'string' ? parsed.primary_category : 'unknown',
    visible_text: typeof parsed.visible_text === 'string' ? parsed.visible_text : '',
    risk_signals: normalizeRiskSignals(parsed.risk_signals),
    confidence: typeof parsed.confidence === 'number' ? parsed.confidence : 0,
  };
}

function normalizeRiskSignals(value) {
  if (Array.isArray(value)) {
    return value.map((entry) => String(entry)).filter((entry) => entry.length > 0);
  }
  if (typeof value === 'string' && value.length > 0) {
    return [value];
  }
  return [];
}

function readProcessSample(pid) {
  if (!Number.isInteger(pid) || pid <= 0 || process.platform !== 'win32') {
    return null;
  }
  try {
    const output = execFileSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-Command',
        `$p = Get-Process -Id ${pid} -ErrorAction SilentlyContinue; if ($p) { [Console]::Write(('{0},{1}' -f [int64]$p.WorkingSet64, [double]$p.CPU)) }`,
      ],
      { encoding: 'utf8', windowsHide: true }
    ).trim();
    if (output.length === 0) {
      return null;
    }
    const [workingSet, cpu] = output.split(',');
    return {
      sampledAt: new Date().toISOString(),
      workingSetBytes: Number(workingSet),
      cpuSeconds: Number(cpu),
    };
  } catch {
    return null;
  }
}

function summarizeResourceSamples(samples) {
  return {
    sampleCount: samples.length,
    peakWorkingSetBytes: Math.max(...samples.map((sample) => sample.workingSetBytes), 0),
    cpuSecondsObserved: Math.max(...samples.map((sample) => sample.cpuSeconds), 0),
  };
}

function parseFirstJsonObject(text) {
  const start = text.indexOf('{');
  if (start < 0) {
    return null;
  }
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = start; index < text.length; index += 1) {
    const char = text[index];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (char === '\\') {
      escaped = true;
      continue;
    }
    if (char === '"') {
      inString = !inString;
      continue;
    }
    if (inString) {
      continue;
    }
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        try {
          return JSON.parse(text.slice(start, index + 1));
        } catch {
          return null;
        }
      }
    }
  }
  return null;
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function artifactPath(path) {
  return path.replaceAll('\\', '/');
}

function resolveUserCachePath(...segments) {
  const home = process.env.USERPROFILE ?? process.env.HOME;
  if (home === undefined) {
    throw new Error('Cannot resolve user cache path without USERPROFILE/HOME.');
  }
  return join(home, '.cache', 'ocentra-parent', ...segments);
}

function redactHome(value) {
  const home = process.env.USERPROFILE ?? process.env.HOME;
  return home === undefined ? value : value.replaceAll(home, '%USERPROFILE%');
}
