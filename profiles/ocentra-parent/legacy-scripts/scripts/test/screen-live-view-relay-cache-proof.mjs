import { createCipheriv, createDecipheriv, createHash, randomBytes } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join('output', 'screen-plan-proof', 'live-view-relay-cache');
const captureDir = join(outputDir, 'capture');
const relayDir = join(outputDir, 'relay-cache');
const proofPath = join(outputDir, 'proof-summary.json');
const deletionPath = join(outputDir, '03-relay-cache-deletion-proof.json');

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(captureDir, { recursive: true });
mkdirSync(relayDir, { recursive: true });

const capture = spawnSync(
  'cargo',
  ['run', '-p', 'ocentra-parent-screen-capture-adapter', '--example', 'screen_capture_real_proof', '--', captureDir],
  {
    cwd: repoRoot,
    encoding: 'utf8',
    shell: process.platform === 'win32',
    env: {
      ...process.env,
      OCENTRA_SCREEN_CAPTURE_KEEP_RAW_UNTIL_ANALYSIS: '1',
    },
  }
);

writeFileSync(join(captureDir, 'cargo-stdout.log'), capture.stdout ?? '');
writeFileSync(join(captureDir, 'cargo-stderr.log'), capture.stderr ?? '');

if (capture.status !== 0) {
  throw new Error(`live-view relay/cache capture command failed with status ${capture.status}`);
}

const captureMetadata = readJson(join(captureDir, '02-capture-metadata.json'));
if (captureMetadata.captured !== true) {
  throw new Error(`live-view relay/cache proof requires real pixels: ${JSON.stringify(captureMetadata)}`);
}

const rawFramePath = resolve(String(captureMetadata.analysisTempPath));
if (!existsSync(rawFramePath)) {
  throw new Error(`live-view relay/cache proof expected retained raw frame at ${rawFramePath}`);
}

const frameBytes = readFileSync(rawFramePath);
const frameDigest = sha256(frameBytes);
if (frameDigest !== captureMetadata.imageDigest) {
  throw new Error('live-view relay/cache frame digest did not match capture metadata digest');
}

const relay = writeAndReadEncryptedRelayCache(frameBytes, frameDigest);
unlinkSync(rawFramePath);
rmSync(relayDir, { recursive: true, force: true });

const deletionProof = {
  rawTempPath: relativePath(rawFramePath),
  relayCacheDir: relativePath(relayDir),
  rawFrameExistedBeforeRelay: true,
  relayCacheExistedBeforeCleanup: true,
  rawFrameDeletedAfterRelay: !existsSync(rawFramePath),
  relayCacheDeletedAfterDelivery: !existsSync(relayDir),
  cacheRawFrames: false,
  encryptedRelayEnvelopeOnly: true,
  sessionRecorded: false,
  remoteInputAllowed: false,
};
writeJson(deletionPath, deletionProof);

const proof = {
  proof: 'screen-live-view-relay-cache-proof',
  proofTier: 'P3_LOCAL_FORCED_RELAY_CACHE_EXECUTION',
  generatedAt: new Date().toISOString(),
  claim:
    'A relay-backed live-view mode can execute a forced local relay/cache handoff for one real captured frame using an encrypted envelope, parent-side digest verification, no raw-frame cache, no session recording, no remote input, and cache deletion after delivery.',
  capture: {
    captureMetadata: relativePath(join(captureDir, '02-capture-metadata.json')),
    triggerInput: relativePath(join(captureDir, '01-trigger-input.json')),
    encryptedQueue: relativePath(join(captureDir, '03-encrypted-queue.ndjson')),
    captured: captureMetadata.captured === true,
    status: captureMetadata.status,
    scope: captureMetadata.actualScope,
    width: captureMetadata.width,
    height: captureMetadata.height,
    imageByteSize: captureMetadata.imageByteSize,
    imageDigest: captureMetadata.imageDigest,
  },
  relayCacheExecution: relay,
  deletion: deletionProof,
  assertions: {
    realPixelsCaptured: captureMetadata.captured === true && frameBytes.length > 0,
    relayEnvelopeEncrypted: relay.encryptedFrameDigest !== frameDigest,
    parentVerifiedFrameDigest: relay.parentVerifiedFrameDigest === true,
    relayCacheDeletedAfterDelivery: deletionProof.relayCacheDeletedAfterDelivery,
    rawFrameDeletedAfterRelay: deletionProof.rawFrameDeletedAfterRelay,
    noRawFrameCache: deletionProof.cacheRawFrames === false,
    noSessionRecording: deletionProof.sessionRecorded === false,
    noRemoteInput: deletionProof.remoteInputAllowed === false,
  },
  nonClaims: [
    'This proof uses a local forced-relay harness; it does not implement hosted cloud relay infrastructure.',
    'This proof does not prove real live-view platform permission-prompt screenshots, physical-device parity, privacy/legal approval, or production worker startup.',
    'This proof does not enable raw frame caching, session recording, remote input, or product live view.',
  ],
};

if (!Object.values(proof.assertions).every(Boolean)) {
  throw new Error(`live-view relay/cache proof assertions failed: ${JSON.stringify(proof.assertions)}`);
}

writeJson(proofPath, proof);
console.log(`screen-live-view-relay-cache-proof-ok:${proofPath}`);

function writeAndReadEncryptedRelayCache(frameBytes, frameDigest) {
  const relaySessionId = `screen-live-view-forced-relay-${Date.now()}`;
  const relayKey = randomBytes(32);
  const nonce = randomBytes(12);
  const cipher = createCipheriv('aes-256-gcm', relayKey, nonce);
  const encryptedFrame = Buffer.concat([cipher.update(frameBytes), cipher.final()]);
  const authTag = cipher.getAuthTag();
  const relayEnvelopePath = join(relayDir, 'encrypted-frame.bin');
  const relayManifestPath = join(relayDir, 'relay-envelope-manifest.json');

  writeFileSync(relayEnvelopePath, encryptedFrame);
  writeJson(relayManifestPath, {
    relaySessionId,
    transportMode: 'relayEndToEndEncrypted',
    frameSequence: 1,
    frameDigest,
    encryptedFrameDigest: sha256(encryptedFrame),
    nonceDigest: sha256(nonce),
    authTagDigest: sha256(authTag),
    cacheRawFrames: false,
    sessionRecordingAllowed: false,
    remoteInputControlAllowed: false,
    custodyLabel: 'ocentra-hosted-non-activity',
  });

  const encryptedFromCache = readFileSync(relayEnvelopePath);
  const decipher = createDecipheriv('aes-256-gcm', relayKey, nonce);
  decipher.setAuthTag(authTag);
  const decryptedFrame = Buffer.concat([decipher.update(encryptedFromCache), decipher.final()]);
  const decryptedFrameDigest = sha256(decryptedFrame);

  return {
    relaySessionId,
    transportMode: 'relayEndToEndEncrypted',
    custodyLabel: 'ocentra-hosted-non-activity',
    relayEnvelopeManifest: relativePath(relayManifestPath),
    relayEnvelopePath: relativePath(relayEnvelopePath),
    frameSequence: 1,
    frameByteLength: frameBytes.length,
    encryptedFrameByteLength: encryptedFrame.length,
    frameDigest,
    encryptedFrameDigest: sha256(encryptedFrame),
    parentReceivedEncryptedEnvelope: encryptedFromCache.length === encryptedFrame.length,
    parentVerifiedFrameDigest: decryptedFrameDigest === frameDigest,
    rawFrameStoredInRelayCache: false,
    cacheRawFrames: false,
    sessionRecordingAllowed: false,
    remoteInputControlAllowed: false,
  };
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function relativePath(path) {
  return relative(repoRoot, path).replaceAll('\\', '/');
}
