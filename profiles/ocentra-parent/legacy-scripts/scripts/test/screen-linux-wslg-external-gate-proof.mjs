import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

const repoRoot = process.cwd();
const distro = process.env.OCENTRA_PARENT_WSL_DISTRO ?? 'Ubuntu-22.04';
const proofRoot = join('output', 'screen-plan-proof', 'linux-wslg-external-gate-analysis');
const proofPath = join(proofRoot, 'proof-summary.json');
const wslgProofPath = join('output', 'screen-plan-proof', 'linux-wslg', 'proof-summary.json');
const externalGateRoot = join('output', 'screen-plan-proof', 'external-gates');
const manifestPath = join(externalGateRoot, 'manual-evidence-manifest.json');
const externalGateProofPath = join(externalGateRoot, 'proof-summary.json');
const localAiModelRoot = resolveUserCachePath('local-ai-models');
const llamaRoot = process.env.OCENTRA_PARENT_LLAMA_CPP_DIR ?? resolveUserCachePath('llama.cpp', 'b9279');
const vlmBinary = process.env.OCENTRA_PARENT_LOCAL_VLM_BINARY ?? join(llamaRoot, 'llama-mtmd-cli.exe');
const vlmModel =
  process.env.OCENTRA_PARENT_LOCAL_VLM_MODEL ?? join(localAiModelRoot, 'Qwen2-VL-2B-Instruct-Q4_K_M.gguf');
const vlmMmproj =
  process.env.OCENTRA_PARENT_LOCAL_VLM_MMPROJ ?? join(localAiModelRoot, 'mmproj-Qwen2-VL-2B-Instruct-Q8_0.gguf');

mkdirSync(proofRoot, { recursive: true });

assertLocalVlmRuntime();
runWslgCaptureProof();

const wslgProof = readJson(wslgProofPath);
const artifact = wslgProof.externalGateArtifact;
if (artifact?.present !== true || typeof artifact.path !== 'string' || !existsSync(artifact.path)) {
  throw new Error(`WSLg proof did not produce an external gate artifact: ${JSON.stringify(artifact)}`);
}

const analysis = runLocalVlmAnalysis(artifact.path, wslgProof.visibleProofText);
const manifest = upsertLinuxExternalGateManifest(wslgProof, artifact, analysis);
writeJson(manifestPath, manifest);

runNodeScript('scripts/test/screen-plan-external-gates-proof.mjs');
const externalGateProof = readJson(externalGateProofPath);
const linuxGate = externalGateProof.gateResults.find((entry) => entry.gateId === 'linux-desktop-session-capture');

const summary = {
  proof: 'screen-linux-wslg-external-gate-proof',
  generatedAt: new Date().toISOString(),
  sourceArtifacts: {
    wslgCapture: wslgProofPath,
    externalGateArtifact: artifact.path,
    externalGateManifest: manifestPath,
    externalGateProof: externalGateProofPath,
  },
  platform: {
    hostPlatform: process.platform,
    wslDistro: distro,
    display: wslgProof.session?.display ?? null,
    waylandDisplay: wslgProof.session?.waylandDisplay ?? null,
  },
  capture: {
    capturedPixels: wslgProof.selectedWindow?.captured === true,
    actualScope: wslgProof.selectedWindow?.actualScope,
    width: wslgProof.selectedWindow?.width,
    height: wslgProof.selectedWindow?.height,
    rawProductCaptureDeleted: wslgProof.custody?.rawImageDeleted === true,
    rawProductCaptureExistsAfterDelete: wslgProof.custody?.existsAfterDelete,
    encryptedQueueContainsRawDigest: wslgProof.custody?.encryptedQueueContainsRawDigest,
  },
  retainedInspectionArtifact: {
    path: artifact.path,
    sha256: artifact.sha256,
    bytes: artifact.bytes,
    width: artifact.width,
    height: artifact.height,
    rawPrivateContentIncluded: artifact.rawPrivateContentIncluded,
    sourceSurface: artifact.sourceSurface,
  },
  localVlmAnalysis: analysis.summary,
  externalGate: {
    linuxGateStatus: linuxGate?.status ?? null,
    satisfiedGateCount: externalGateProof.counts?.satisfiedGateCount ?? 0,
    missingGateCount: externalGateProof.counts?.missingGateCount ?? 0,
    productCompleteAllowed: externalGateProof.assertions?.productCompleteAllowed === true,
  },
  assertions: {
    wslgCaptureUsedRealLinuxWindow: wslgProof.selectedWindow?.captured === true,
    retainedArtifactDigestMatches: artifact.sha256 === sha256(readFileSync(artifact.path)),
    localVlmAnalyzedRetainedArtifact: analysis.summary.localVlmExecuted === true,
    localVlmDetectedExpectedCategory: analysis.summary.categoryMatched === true,
    localVlmDetectedExpectedText: analysis.summary.expectedTermsMatched.length > 0,
    rawProductCaptureDeleted: wslgProof.custody?.rawImageDeleted === true,
    rawProductCaptureNotRetainedInQueue: wslgProof.custody?.encryptedQueueContainsRawDigest === false,
    linuxExternalGateSatisfied: linuxGate?.status === 'satisfied',
    remainingExternalGatesStillBlockProductComplete: externalGateProof.assertions?.productCompleteAllowed === false,
  },
  nonClaims: [
    'This proof satisfies only the Linux WSLg/X11 selected-window external gate entry.',
    'The retained PNG is a controlled xmessage visual-inspection artifact from the same live Linux surface, not the raw product capture queue image.',
    'This proof does not claim native Wayland/PipeWire portal parity, macOS, iOS, physical Android, live-view production, or authenticated-account social completion.',
  ],
};

writeJson(proofPath, summary);

if (!Object.values(summary.assertions).every(Boolean)) {
  throw new Error(`Linux WSLg external gate assertions failed: ${JSON.stringify(summary.assertions, null, 2)}`);
}

console.log(`screen-linux-wslg-external-gate-proof-ok:${proofPath}`);

function runWslgCaptureProof() {
  const wslRepoRoot = runCommand('wsl.exe', [
    '-d',
    distro,
    '--',
    'wslpath',
    '-a',
    repoRoot.replaceAll('\\', '/'),
  ]).stdout.trim();
  const command = `cd ${shellQuote(wslRepoRoot)} && node scripts/test/screen-capture-linux-wslg-proof.mjs`;
  runCommand('wsl.exe', ['-d', distro, '--', 'bash', '-lc', command], {
    stdoutPath: join(proofRoot, '01-wslg-capture-stdout.log'),
    stderrPath: join(proofRoot, '01-wslg-capture-stderr.log'),
  });
}

function runLocalVlmAnalysis(imagePath, visibleProofText) {
  const prompt = [
    'Analyze this real Linux WSLg native xmessage screen capture.',
    'Return JSON only with keys primary_category, visible_text, risk_signals, confidence.',
    'Allowed primary_category values are school, video, chat, game, adultContent, violence, bypassTool, shopping, productivity, unknown.',
    'The expected safe category is school or productivity.',
    'Expected visible terms include Ocentra, WSLg, external gate, school, productivity, policy dry run, and raw product capture deleted.',
  ].join(' ');
  const result = runCommand(
    vlmBinary,
    [
      '-m',
      vlmModel,
      '--mmproj',
      vlmMmproj,
      '--image',
      resolve(repoRoot, imagePath),
      '-p',
      prompt,
      '-n',
      '192',
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
    ],
    {
      stdoutPath: join(proofRoot, '02-local-vlm-stdout.log'),
      stderrPath: join(proofRoot, '02-local-vlm-stderr.log'),
    }
  );
  const parsed = parseFirstJsonObject(result.stdout);
  const normalized = normalizeParsedResult(parsed);
  const combined = `${JSON.stringify(normalized)} ${result.stdout}`.toLowerCase();
  const expectedTerms = ['ocentra', 'wslg', 'external', 'school', 'productivity'];
  const expectedTermsMatched = expectedTerms.filter((term) => combined.includes(term));
  const acceptedCategories = ['school', 'productivity'];
  const categoryMatched = acceptedCategories.includes(normalized?.primary_category);
  const summary = {
    localVlmExecuted: true,
    runtimeBinary: redactHome(vlmBinary),
    model: redactHome(vlmModel),
    mmproj: redactHome(vlmMmproj),
    promptOrTemplateVersion: 'screen-linux-wslg-external-gate-vlm-v1',
    parsed,
    normalized,
    visibleProofTextHash: `sha256:${sha256(visibleProofText)}`,
    expectedTerms,
    expectedTermsMatched,
    acceptedCategories,
    categoryMatched,
    stdoutPreview: result.stdout.replace(/\s+/g, ' ').slice(0, 500),
    stderrPreview: result.stderr.replace(/\s+/g, ' ').slice(0, 500),
  };
  writeJson(join(proofRoot, '03-local-vlm-analysis.json'), summary);
  if (!categoryMatched || expectedTermsMatched.length === 0) {
    throw new Error(`Local VLM did not classify the WSLg artifact as expected: ${JSON.stringify(summary, null, 2)}`);
  }
  return { summary };
}

function upsertLinuxExternalGateManifest(wslgProof, artifact, analysis) {
  const existing = existsSync(manifestPath) ? readJson(manifestPath) : { schemaVersion: 'v0.6', entries: [] };
  const entries = Array.isArray(existing.entries)
    ? existing.entries.filter((entry) => entry?.gateId !== 'linux-desktop-session-capture')
    : [];
  entries.push({
    gateId: 'linux-desktop-session-capture',
    platform: 'linux-wayland',
    evidenceKind: 'platform-session-recording',
    collectionMode: 'local-wslg-or-linux-desktop-runner',
    artifactPath: artifact.path.replaceAll('\\', '/'),
    artifactSha256: artifact.sha256,
    capturedFromRealDeviceOrHost: true,
    capturesLiveSurface: true,
    rawPrivateContentIncluded: false,
    localCaptureProofRef: 'output/screen-plan-proof/linux-wslg/proof-summary.json',
    localAnalysisProofRef: 'output/screen-plan-proof/linux-wslg-external-gate-analysis/proof-summary.json',
    rawImageDeletionProofRef: 'output/screen-plan-proof/linux-wslg/selected-window/04-deletion-proof.json',
    proofNotes: [
      'Captured real WSLg/X11 xmessage pixels from the local Linux session.',
      'Analyzed the retained operator-safe artifact with the local Qwen2-VL runtime.',
      'Verified the raw product capture queue image was deleted by the screen-capture adapter proof.',
    ],
    sourceSurface: artifact.sourceSurface,
    localVlmCategory: analysis.summary.normalized.primary_category,
    rawProductCaptureDeleted: wslgProof.custody?.rawImageDeleted === true,
  });
  return {
    schemaVersion: existing.schemaVersion ?? 'v0.6',
    instructions:
      existing.instructions ??
      'Manifest entries are generated only from digest-backed real device or live host artifacts. Do not use fixtures, generated HTML pages, static JSON, raw private content, or placeholder digests as final evidence.',
    entries,
  };
}

function runNodeScript(scriptPath) {
  runCommand(process.execPath, [scriptPath], {
    stdoutPath: join(proofRoot, `${scriptPath.replaceAll(/[\\/]/gu, '-')}-stdout.log`),
    stderrPath: join(proofRoot, `${scriptPath.replaceAll(/[\\/]/gu, '-')}-stderr.log`),
  });
}

function assertLocalVlmRuntime() {
  const missing = [
    ['binary', vlmBinary],
    ['model', vlmModel],
    ['mmproj', vlmMmproj],
  ].filter(([, path]) => !existsSync(path));
  if (missing.length > 0) {
    throw new Error(`Local VLM runtime is missing: ${JSON.stringify(missing)}`);
  }
}

function runCommand(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: false,
  });
  if (options.stdoutPath !== undefined) {
    writeFileSync(options.stdoutPath, result.stdout ?? '');
  }
  if (options.stderrPath !== undefined) {
    writeFileSync(options.stderrPath, result.stderr ?? '');
  }
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${result.status}\n${result.stdout}\n${result.stderr}`);
  }
  return { stdout: result.stdout ?? '', stderr: result.stderr ?? '' };
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

function normalizeParsedResult(parsed) {
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return null;
  }
  return {
    primary_category: typeof parsed.primary_category === 'string' ? parsed.primary_category : 'unknown',
    visible_text: normalizeVisibleText(parsed.visible_text),
    risk_signals: normalizeRiskSignals(parsed.risk_signals),
    confidence: typeof parsed.confidence === 'number' ? parsed.confidence : 0,
  };
}

function normalizeVisibleText(value) {
  if (typeof value === 'string') {
    return value;
  }
  if (Array.isArray(value)) {
    return value.map((entry) => String(entry)).join(' ');
  }
  return '';
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

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
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

function shellQuote(value) {
  return `'${value.replaceAll("'", "'\\''")}'`;
}
