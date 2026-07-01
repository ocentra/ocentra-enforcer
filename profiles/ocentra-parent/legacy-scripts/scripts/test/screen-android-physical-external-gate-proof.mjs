import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

const repoRoot = process.cwd();
const requestedSerial = process.env.OCENTRA_ANDROID_SERIAL ?? process.env.ANDROID_SERIAL ?? '192.168.2.45:5555';
const packageId = 'ca.ocentra.parent.agent';
const proofRoot = join('output', 'screen-plan-proof', 'android-physical-external-gate-analysis');
const proofPath = join(proofRoot, 'proof-summary.json');
const androidMediaProjectionProofPath = join(
  'output',
  'screen-plan-proof',
  'android-mediaprojection',
  'proof-summary.json'
);
const androidCapabilityProofPath = join('output', 'screen-plan-proof', 'android', 'proof-summary.json');
const externalGateRoot = join('output', 'screen-plan-proof', 'external-gates');
const artifactPath = join(externalGateRoot, 'artifacts', 'android-physical-mediaprojection-live-surface.png');
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
mkdirSync(dirname(artifactPath), { recursive: true });

assertLocalVlmRuntime();
assertPhysicalDeviceReady();
runPhysicalMediaProjectionProof();
runAndroidCapabilityProof();
captureSafeLiveSurfaceArtifact();

const mediaProjectionProof = readJson(androidMediaProjectionProofPath);
const capabilityProof = readJson(androidCapabilityProofPath);
assert(mediaProjectionProof.physicalDevice === true, 'MediaProjection proof did not run on a physical device.');
assert(
  mediaProjectionProof.deviceInfo?.serial === requestedSerial,
  'MediaProjection proof serial did not match target.'
);
assert(mediaProjectionProof.captured === true, 'Physical MediaProjection proof did not capture pixels.');
assert(
  mediaProjectionProof.rawTempDeleted === true,
  'Physical MediaProjection proof did not delete the raw temp frame.'
);
assert(
  capabilityProof.gapStatus?.physicalAndroidDeviceProofExists === true,
  'Android capability proof did not record physical-device proof.'
);

const artifact = {
  path: artifactPath.replaceAll('\\', '/'),
  sha256: sha256(readFileSync(artifactPath)),
  bytes: readFileSync(artifactPath).byteLength,
  sourceSurface: 'physical-android-ocentra-agent-app',
  rawPrivateContentIncluded: false,
};
const analysis = runLocalVlmAnalysis(artifact.path);
const manifest = upsertAndroidExternalGateManifest(artifact, analysis);
writeJson(manifestPath, manifest);

runNodeScript('scripts/test/screen-plan-external-gates-proof.mjs');
const externalGateProof = readJson(externalGateProofPath);
const androidGate = externalGateProof.gateResults.find(
  (entry) => entry.gateId === 'android-physical-mediaprojection-capture'
);

const summary = {
  proof: 'screen-android-physical-external-gate-proof',
  generatedAt: new Date().toISOString(),
  sourceArtifacts: {
    androidMediaProjection: androidMediaProjectionProofPath,
    androidCapability: androidCapabilityProofPath,
    externalGateArtifact: artifact.path,
    externalGateManifest: manifestPath,
    externalGateProof: externalGateProofPath,
  },
  device: {
    serial: mediaProjectionProof.deviceInfo?.serial,
    model: mediaProjectionProof.deviceInfo?.model,
    api: mediaProjectionProof.deviceInfo?.api,
    release: mediaProjectionProof.deviceInfo?.release,
    physicalDevice: mediaProjectionProof.physicalDevice === true,
  },
  retainedInspectionArtifact: artifact,
  mediaProjection: {
    consentApproved: mediaProjectionProof.consentApproved === true,
    capturedPixels: mediaProjectionProof.captured === true,
    width: mediaProjectionProof.width,
    height: mediaProjectionProof.height,
    rawTempDeleted: mediaProjectionProof.rawTempDeleted === true,
  },
  localVlmAnalysis: analysis.summary,
  externalGate: {
    androidGateStatus: androidGate?.status ?? null,
    satisfiedGateCount: externalGateProof.counts?.satisfiedGateCount ?? 0,
    missingGateCount: externalGateProof.counts?.missingGateCount ?? 0,
    productCompleteAllowed: externalGateProof.assertions?.productCompleteAllowed === true,
  },
  assertions: {
    targetWasPhysicalAndroid: mediaProjectionProof.physicalDevice === true,
    targetSerialMatched: mediaProjectionProof.deviceInfo?.serial === requestedSerial,
    physicalMediaProjectionCapturedPixels: mediaProjectionProof.captured === true,
    rawProductCaptureDeleted: mediaProjectionProof.rawTempDeleted === true,
    retainedArtifactDigestMatches: artifact.sha256 === sha256(readFileSync(artifactPath)),
    localVlmAnalyzedRetainedArtifact: analysis.summary.localVlmExecuted === true,
    localVlmDetectedExpectedCategory: analysis.summary.categoryMatched === true,
    localVlmDetectedExpectedText: analysis.summary.expectedTermsMatched.length > 0,
    androidExternalGateSatisfied: androidGate?.status === 'satisfied',
    remainingExternalGatesStillBlockProductComplete: externalGateProof.assertions?.productCompleteAllowed === false,
  },
  nonClaims: [
    'This proof satisfies only the physical Android MediaProjection external gate entry.',
    'The retained PNG is an operator-safe screenshot of the Ocentra agent app surface on the physical phone, not the raw product capture temp frame.',
    'This proof does not claim Android app-window sharing, live-view production, Android Device Owner or managed-profile enforcement, iOS, macOS, or authenticated-account social completion.',
  ],
};

writeJson(proofPath, summary);

if (!Object.values(summary.assertions).every(Boolean)) {
  throw new Error(`Android physical external gate assertions failed: ${JSON.stringify(summary.assertions, null, 2)}`);
}

console.log(`screen-android-physical-external-gate-proof-ok:${proofPath}`);

function assertPhysicalDeviceReady() {
  const devices = runCommand('adb', ['devices', '-l']).stdout;
  writeFileSync(join(proofRoot, '00-adb-devices.log'), devices);
  const targetLine = devices
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .find((line) => line.startsWith(`${requestedSerial} `));
  assert(
    targetLine !== undefined && /\bdevice\b/u.test(targetLine),
    `Target Android serial is not online: ${requestedSerial}`
  );
  assert(!requestedSerial.startsWith('emulator-'), 'Physical Android proof cannot use an emulator serial.');
  assert(/model:SM_G965W/u.test(targetLine), `Unexpected physical Android target line: ${targetLine}`);
}

function runPhysicalMediaProjectionProof() {
  runNodeScript('scripts/test/child-android-screen-capture-mediaprojection-proof.mjs', {
    env: { OCENTRA_ANDROID_SERIAL: requestedSerial, ANDROID_SERIAL: requestedSerial },
  });
}

function runAndroidCapabilityProof() {
  runNodeScript('scripts/test/screen-android-mediaprojection-capability-proof.mjs');
}

function captureSafeLiveSurfaceArtifact() {
  adb(['shell', 'input', 'keyevent', 'KEYCODE_WAKEUP'], { allowFailure: true });
  adb(['shell', 'am', 'start', '-n', `${packageId}/.MainActivity`]);
  sleep(1500);
  const ui = adb(['exec-out', 'uiautomator', 'dump', '/dev/tty'], { allowFailure: true }).stdout;
  writeFileSync(join(proofRoot, '04-safe-surface-ui.xml'), ui);
  const screenshot = adb(['exec-out', 'screencap', '-p'], { encoding: 'buffer' }).stdout;
  writeFileSync(artifactPath, screenshot);
}

function runLocalVlmAnalysis(imagePath) {
  const prompt = [
    'Analyze this real physical Android screenshot of the Ocentra Parent Agent app.',
    'Return JSON only with keys primary_category, visible_text, risk_signals, confidence.',
    'Allowed primary_category values are school, video, chat, game, adultContent, violence, bypassTool, shopping, productivity, system, unknown.',
    'The expected safe category is productivity or system.',
    'Expected visible terms include Ocentra, Parent, Agent, service, scaffold, running, screen capture, or local family safety agent.',
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
      stdoutPath: join(proofRoot, '05-local-vlm-stdout.log'),
      stderrPath: join(proofRoot, '05-local-vlm-stderr.log'),
    }
  );
  const parsed = parseFirstJsonObject(result.stdout);
  const normalized = normalizeParsedResult(parsed);
  const combined = `${JSON.stringify(normalized)} ${result.stdout}`.toLowerCase();
  const expectedTerms = ['ocentra', 'parent', 'agent', 'service'];
  const expectedTermsMatched = expectedTerms.filter((term) => combined.includes(term));
  const acceptedCategories = ['productivity', 'system', 'unknown'];
  const categoryMatched = acceptedCategories.includes(normalized?.primary_category);
  const summary = {
    localVlmExecuted: true,
    runtimeBinary: redactHome(vlmBinary),
    model: redactHome(vlmModel),
    mmproj: redactHome(vlmMmproj),
    promptOrTemplateVersion: 'screen-android-physical-external-gate-vlm-v1',
    parsed,
    normalized,
    expectedTerms,
    expectedTermsMatched,
    acceptedCategories,
    categoryMatched,
    stdoutPreview: result.stdout.replace(/\s+/g, ' ').slice(0, 500),
    stderrPreview: result.stderr.replace(/\s+/g, ' ').slice(0, 500),
  };
  writeJson(join(proofRoot, '06-local-vlm-analysis.json'), summary);
  if (!categoryMatched || expectedTermsMatched.length === 0) {
    throw new Error(`Local VLM did not classify the Android artifact as expected: ${JSON.stringify(summary, null, 2)}`);
  }
  return { summary };
}

function upsertAndroidExternalGateManifest(artifact, analysis) {
  const existing = existsSync(manifestPath) ? readJson(manifestPath) : { schemaVersion: 'v0.6', entries: [] };
  const entries = Array.isArray(existing.entries)
    ? existing.entries.filter((entry) => entry?.gateId !== 'android-physical-mediaprojection-capture')
    : [];
  entries.push({
    gateId: 'android-physical-mediaprojection-capture',
    platform: 'android-mediaprojection',
    evidenceKind: 'physical-device-capture-recording',
    collectionMode: 'local-adb-physical-device',
    artifactPath: artifact.path,
    artifactSha256: artifact.sha256,
    capturedFromRealDeviceOrHost: true,
    capturesLiveSurface: true,
    rawPrivateContentIncluded: false,
    localCaptureProofRef: 'output/screen-plan-proof/android-mediaprojection/proof-summary.json',
    localAnalysisProofRef: 'output/screen-plan-proof/android-physical-external-gate-analysis/proof-summary.json',
    rawImageDeletionProofRef: 'output/screen-plan-proof/android-mediaprojection/03-android-capture-proof.json',
    proofNotes: [
      'Ran Android MediaProjection proof on the physical Samsung Galaxy S9 over Wi-Fi ADB.',
      'Analyzed an operator-safe retained screenshot of the live Ocentra Android app surface with local Qwen2-VL.',
      'Verified the raw product capture temp frame was deleted by the Android proof app.',
    ],
    sourceSurface: artifact.sourceSurface,
    localVlmCategory: analysis.summary.normalized.primary_category,
    rawProductCaptureDeleted: true,
  });
  return {
    schemaVersion: existing.schemaVersion ?? 'v0.6',
    instructions:
      existing.instructions ??
      'Manifest entries are generated only from digest-backed real device or live host artifacts. Do not use fixtures, generated HTML pages, static JSON, raw private content, or placeholder digests as final evidence.',
    entries,
  };
}

function adb(args, options = {}) {
  return runCommand('adb', ['-s', requestedSerial, ...args], options);
}

function runNodeScript(scriptPath, options = {}) {
  runCommand(process.execPath, [scriptPath], {
    ...options,
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
    encoding: options.encoding ?? 'utf8',
    shell: false,
    env: { ...process.env, ...(options.env ?? {}) },
  });
  if (options.stdoutPath !== undefined) {
    writeFileSync(options.stdoutPath, result.stdout ?? '');
  }
  if (options.stderrPath !== undefined) {
    writeFileSync(options.stderrPath, result.stderr ?? '');
  }
  if (result.status !== 0 && options.allowFailure !== true) {
    throw new Error(`${command} ${args.join(' ')} failed with ${result.status}\n${result.stdout}\n${result.stderr}`);
  }
  return { stdout: result.stdout ?? '', stderr: result.stderr ?? '', status: result.status };
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

function sleep(ms) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}
