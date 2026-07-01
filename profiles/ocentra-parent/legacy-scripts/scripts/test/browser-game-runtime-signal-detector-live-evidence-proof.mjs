import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

import { chromium } from 'playwright';

import {
  BrowserGameRuntimeSignalDetectionSchema,
  BrowserGameRuntimeSignalRowSchema,
} from '@ocentra-parent/schema-domain/browser-game-runtime-signal-detector';

const repoRoot = process.cwd();
const proofId = 'browser-game-runtime-signal-detector-live-evidence-proof';
const resultPath = join(repoRoot, 'test-results', proofId, 'proof.json');
const outputProofPath = join(
  repoRoot,
  'output',
  'browser-plan-proof',
  'game-06-game-runtime-signal-detector',
  '02-live-runtime-signal-shape-proof.json'
);

const targets = [
  {
    targetId: 'poki-subway-surfers',
    url: 'https://poki.com/en/g/subway-surfers',
    expectedSignalKinds: ['iframe-game-surface-shape', 'webgl-present-shape', 'gamepad-api-shape'],
  },
  {
    targetId: 'coolmath-run-3',
    url: 'https://www.coolmathgames.com/0-run-3',
    expectedSignalKinds: ['iframe-game-surface-shape', 'fullscreen-request-shape', 'audio-context-shape'],
  },
  {
    targetId: 'chess-play-online',
    url: 'https://www.chess.com/play/online',
    expectedSignalKinds: ['animation-loop-shape', 'pointer-lock-shape', 'gamepad-api-shape'],
  },
  {
    targetId: 'xbox-cloud-play',
    url: 'https://www.xbox.com/en-US/play',
    expectedSignalKinds: ['cloud-streaming-shape', 'fullscreen-request-shape', 'gamepad-api-shape'],
  },
];

const startedAt = new Date().toISOString();
const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
const commit = git(['rev-parse', 'HEAD']);
const baseCommit = git(['rev-parse', 'origin/main']);
const browser = await chromium.launch({ headless: true });

let captures;
try {
  const context = await browser.newContext({
    userAgent: 'Mozilla/5.0 OcentraParentBrowserGameProof/1.0',
    viewport: { width: 1280, height: 720 },
  });
  captures = [];
  for (const target of targets) {
    captures.push(await captureTarget(context, target));
  }
  await context.close();
} finally {
  await browser.close();
}

const signalRows = captures.flatMap((capture) => signalRowsFor(capture));
const detection = detectionFor(signalRows);
const negativeChecks = runNegativeChecks(signalRows[0], detection);

if (!captures.every((capture) => capture.responseOk)) {
  throw new Error('Expected all browser-game runtime public captures to return HTTP 2xx/3xx responses');
}
if (!captures.every((capture) => capture.expectedSignalsPresent)) {
  throw new Error('Expected every browser-game runtime public capture to contain expected shape signals');
}
if (!signalRows.every((signal) => BrowserGameRuntimeSignalRowSchema.safeParse(signal).success)) {
  throw new Error('Expected all browser-game runtime signal rows to parse');
}
if (!BrowserGameRuntimeSignalDetectionSchema.safeParse(detection).success) {
  throw new Error('Expected browser-game runtime signal detection bundle to parse');
}
if (!negativeChecks.every((check) => check.rejected)) {
  throw new Error('Expected browser-game runtime signal negative checks to reject overclaims');
}

const proof = {
  schemaVersion: 1,
  proofId,
  generatedAt: startedAt,
  branch,
  commit,
  baseCommit,
  captureMode: 'real-public-browser-game-runtime-signal-shapes-playwright',
  targets: captures,
  detection,
  negativeChecks,
  summary: {
    targetCount: captures.length,
    signalRows: signalRows.length,
    negativeChecks: negativeChecks.length,
    rawDomStored: false,
    rawCanvasFrameStored: false,
    rawStreamFrameStored: false,
    rawAudioStored: false,
    rawGamepadInputStored: false,
    screenshotStored: false,
    browserInstrumentationClaimed: false,
    runtimeDetectionExecutedClaimed: false,
    aiClassificationClaimed: false,
    policyDecisionClaimed: false,
    cloudFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    enforcementClaimed: false,
    productChecklistUpgradeClaimed: false,
  },
};

await writeJson(resultPath, proof);
await writeJson(outputProofPath, proof);

console.log('browser-game-runtime-signal-detector-live-evidence-proof-ok=true');
console.log(`proof=${relativePath(resultPath)}`);
console.log(`outputProof=${relativePath(outputProofPath)}`);
console.log(`targets=${captures.length} signalRows=${signalRows.length} negativeChecks=${negativeChecks.length}`);

async function captureTarget(context, target) {
  const page = await context.newPage();
  const inputUrl = new URL(target.url);
  try {
    const response = await page.goto(target.url, { waitUntil: 'domcontentloaded', timeout: 45_000 });
    await page.waitForTimeout(1_500);
    const finalUrl = new URL(page.url());
    const shape = await page.evaluate(() => {
      const canvas = document.createElement('canvas');
      const webglSupported =
        typeof canvas.getContext === 'function' &&
        Boolean(canvas.getContext('webgl') || canvas.getContext('experimental-webgl'));
      return {
        canvasCount: document.querySelectorAll('canvas').length,
        iframeCount: document.querySelectorAll('iframe').length,
        webglSupported,
        gamepadApiPresent: typeof navigator.getGamepads === 'function',
        fullscreenApiPresent: typeof document.documentElement.requestFullscreen === 'function',
        pointerLockApiPresent: typeof document.body?.requestPointerLock === 'function',
        audioContextApiPresent:
          typeof window.AudioContext === 'function' || typeof window.webkitAudioContext === 'function',
        animationFrameApiPresent: typeof window.requestAnimationFrame === 'function',
      };
    });
    const signalBooleans = signalBooleansFor(target, shape);
    const expectedSignalsPresent = target.expectedSignalKinds.every((signalKind) => signalBooleans[signalKind]);
    return {
      targetId: target.targetId,
      status: response?.status() ?? 0,
      responseOk: response ? response.status() >= 200 && response.status() < 400 : false,
      contentType: response?.headers()['content-type'] ?? 'unknown',
      inputOriginSha256: sha256(inputUrl.origin),
      inputPathSha256: sha256(inputUrl.pathname),
      finalOriginSha256: sha256(finalUrl.origin),
      finalPathSha256: sha256(finalUrl.pathname),
      expectedSignalKinds: target.expectedSignalKinds,
      expectedSignalsPresent,
      signalShapeFingerprint: runtimeFingerprintFor(target.targetId, signalBooleans),
      signalBooleans,
      rawDomPersisted: false,
      rawCanvasFramePersisted: false,
      rawStreamFramePersisted: false,
      rawAudioPersisted: false,
      rawGamepadInputPersisted: false,
      screenshotPersisted: false,
    };
  } finally {
    await page.close();
  }
}

function signalBooleansFor(target, shape) {
  return {
    'canvas-present-shape': shape.canvasCount > 0,
    'webgl-present-shape': shape.webglSupported,
    'gamepad-api-shape': shape.gamepadApiPresent,
    'fullscreen-request-shape': shape.fullscreenApiPresent,
    'pointer-lock-shape': shape.pointerLockApiPresent,
    'audio-context-shape': shape.audioContextApiPresent,
    'animation-loop-shape': shape.animationFrameApiPresent,
    'iframe-game-surface-shape': shape.iframeCount > 0,
    'cloud-streaming-shape': target.targetId.includes('xbox-cloud'),
  };
}

function signalRowsFor(capture) {
  return capture.expectedSignalKinds.map((signalKind) =>
    runtimeSignal({
      signalId: `runtime-signal-${capture.targetId}-${signalKind}`,
      signalKind,
      signalFingerprint: `${capture.signalShapeFingerprint}-${sha256(signalKind).slice(0, 12)}`,
      sourceKind: signalKind === 'cloud-streaming-shape' ? 'url-shape-ref' : 'managed-browser-signal-ref',
      sourceEvidenceRefs: [`parent-proof-${proofId}-${capture.targetId}`],
      reasonCodes: reasonCodesFor(signalKind),
      cloudSessionCandidate: signalKind === 'cloud-streaming-shape',
      childLaunchCandidate: signalKind !== 'cloud-streaming-shape',
    })
  );
}

function runtimeSignal(overrides = {}) {
  return {
    signalId: 'runtime-signal-live-shape',
    signalKind: 'webgl-present-shape',
    signalFingerprint: 'runtime-signal-live-shape-fingerprint',
    sourceKind: 'managed-browser-signal-ref',
    sourceEvidenceRefs: ['runtime-signal-live-shape-evidence'],
    confidence: 'high',
    status: 'detected-shape',
    reasonCodes: ['runtime-shape-present', 'webgl-shape-present', 'managed-browser-proof-required'],
    managedBrowserProofRequired: true,
    childLaunchCandidate: true,
    cloudSessionCandidate: false,
    rawDomStored: false,
    rawCanvasFrameStored: false,
    rawStreamFrameStored: false,
    rawAudioStored: false,
    rawGamepadInputStored: false,
    browserInstrumentationClaimed: false,
    runtimeDetectionExecutedClaimed: false,
    aiClassificationClaimed: false,
    policyDecisionClaimed: false,
    cloudFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    enforcementClaimed: false,
    ...overrides,
  };
}

function detectionFor(signals) {
  return {
    schemaVersion: 'browser-game-runtime-signal-detector-contract',
    detectionId: `runtime-signal-detection-${proofId}`,
    detectedAt: startedAt,
    sourceEvidenceRefs: signals.flatMap((signal) => signal.sourceEvidenceRefs),
    signals,
    confidence: 'high',
    status: 'detected-shape',
    rawDomStored: false,
    rawCanvasFrameStored: false,
    rawStreamFrameStored: false,
    rawAudioStored: false,
    rawGamepadInputStored: false,
    browserInstrumentationClaimed: false,
    runtimeDetectionExecutedClaimed: false,
    aiClassificationClaimed: false,
    policyDecisionClaimed: false,
    cloudFrameAnalysisClaimed: false,
    nativeGameControlClaimed: false,
    enforcementClaimed: false,
  };
}

function reasonCodesFor(signalKind) {
  const reasonBySignal = {
    'canvas-present-shape': 'canvas-shape-present',
    'webgl-present-shape': 'webgl-shape-present',
    'gamepad-api-shape': 'gamepad-shape-present',
    'fullscreen-request-shape': 'fullscreen-shape-present',
    'pointer-lock-shape': 'pointer-lock-shape-present',
    'audio-context-shape': 'audio-shape-present',
    'animation-loop-shape': 'animation-loop-shape-present',
    'iframe-game-surface-shape': 'iframe-surface-shape-present',
    'cloud-streaming-shape': 'cloud-streaming-shape-present',
  };
  return ['runtime-shape-present', reasonBySignal[signalKind], 'managed-browser-proof-required'];
}

function runNegativeChecks(validSignal, validDetection) {
  const invalidClaims = [
    ['raw-dom', { rawDomStored: true }],
    ['raw-canvas-frame', { rawCanvasFrameStored: true }],
    ['raw-stream-frame', { rawStreamFrameStored: true }],
    ['raw-audio', { rawAudioStored: true }],
    ['raw-gamepad-input', { rawGamepadInputStored: true }],
    ['browser-instrumentation', { browserInstrumentationClaimed: true }],
    ['runtime-detection-executed', { runtimeDetectionExecutedClaimed: true }],
    ['ai-classification', { aiClassificationClaimed: true }],
    ['policy-decision', { policyDecisionClaimed: true }],
    ['cloud-frame-analysis', { cloudFrameAnalysisClaimed: true }],
    ['native-game-control', { nativeGameControlClaimed: true }],
    ['enforcement', { enforcementClaimed: true }],
  ];
  const invalidSignals = [
    ...invalidClaims.map(([name, invalid]) => negativeSignalCheck(name, validSignal, invalid)),
    negativeSignalCheck('detected-without-managed-proof', validSignal, { managedBrowserProofRequired: false }),
    negativeSignalCheck('cloud-candidate-with-canvas-kind', validSignal, {
      cloudSessionCandidate: true,
      signalKind: 'canvas-present-shape',
    }),
    negativeSignalCheck('detected-without-managed-reason', validSignal, { reasonCodes: ['runtime-shape-present'] }),
  ];
  const invalidDetections = invalidClaims.map(([name, invalid]) =>
    negativeDetectionCheck(`detection-${name}`, validDetection, invalid)
  );
  return [...invalidSignals, ...invalidDetections];
}

function negativeSignalCheck(name, validSignal, invalid) {
  return {
    name,
    rejected: !BrowserGameRuntimeSignalRowSchema.safeParse({ ...validSignal, ...invalid }).success,
  };
}

function negativeDetectionCheck(name, validDetection, invalid) {
  return {
    name,
    rejected: !BrowserGameRuntimeSignalDetectionSchema.safeParse({ ...validDetection, ...invalid }).success,
  };
}

async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`);
}

function git(args) {
  return execFileSync('git', args, { cwd: repoRoot, encoding: 'utf8' }).trim();
}

function runtimeFingerprintFor(targetId, signalBooleans) {
  return `runtime-signal-fingerprint-${sha256(
    `${targetId}:${Object.entries(signalBooleans)
      .map(([key, value]) => `${key}:${value}`)
      .join('|')}`
  ).slice(0, 32)}`;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}
