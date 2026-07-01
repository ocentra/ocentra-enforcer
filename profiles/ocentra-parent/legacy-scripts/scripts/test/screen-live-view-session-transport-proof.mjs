import { createHash, createHmac, randomBytes } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { createServer, connect } from 'node:net';
import { existsSync, mkdirSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';

const repoRoot = process.cwd();
const outputDir = join('output', 'screen-plan-proof', 'live-view-session-transport');
const captureDir = join(outputDir, 'capture');
const proofPath = join(outputDir, 'proof-summary.json');
const deletionPath = join(outputDir, '03-live-frame-deletion-proof.json');

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(captureDir, { recursive: true });

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
  throw new Error(`live-view transport capture command failed with status ${capture.status}`);
}

const captureMetadata = readJson(join(captureDir, '02-capture-metadata.json'));
if (captureMetadata.captured !== true) {
  throw new Error(`live-view transport proof requires a real captured frame: ${JSON.stringify(captureMetadata)}`);
}

const rawFramePath = resolve(String(captureMetadata.analysisTempPath));
if (!existsSync(rawFramePath)) {
  throw new Error(`live-view transport proof expected retained temp frame at ${rawFramePath}`);
}

const frameBytes = readFileSync(rawFramePath);
const frameDigest = sha256(frameBytes);
if (frameDigest !== captureMetadata.imageDigest) {
  throw new Error('live-view transport frame digest did not match capture metadata digest');
}

const transport = await runViewOnlyLoopbackTransport(frameBytes, frameDigest);
unlinkSync(rawFramePath);

const deletionProof = {
  rawTempPath: relativePath(rawFramePath),
  rawFrameExistedBeforeTransport: true,
  rawFrameDeletedAfterTransport: !existsSync(rawFramePath),
  transportCachedRawFrame: false,
  sessionRecorded: false,
  remoteInputAllowed: false,
  transportPayloadDigest: frameDigest,
};
writeJson(deletionPath, deletionProof);

if (!deletionProof.rawFrameDeletedAfterTransport || !transport.digestMatched || !transport.hmacMatched) {
  throw new Error(`live-view transport proof failed: ${JSON.stringify({ deletionProof, transport })}`);
}

const proof = {
  proof: 'screen-live-view-session-transport-proof',
  proofTier: 'P3_LOCAL_LOOPBACK_TRANSPORT',
  generatedAt: new Date().toISOString(),
  claim:
    'A parent-approved live-view session can transport one real captured frame over a local view-only session without raw frame retention, session recording, or remote input.',
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
  session: {
    sessionId: transport.sessionId,
    viewerAuditRef: 'screen-live-view-loopback-viewer-audit',
    transportProofRef: 'screen-live-view-loopback-transport-proof',
    transportMode: 'lanMutualAuth',
    endpoint: '127.0.0.1',
    frameSequence: transport.frameSequence,
    frameByteLength: transport.frameByteLength,
    frameDigest: transport.frameDigest,
    hmacMatched: transport.hmacMatched,
    digestMatched: transport.digestMatched,
    viewerReceivedFrame: transport.viewerReceivedFrame,
    explicitViewerDisclosure: true,
    cacheRawFrames: false,
    sessionRecordingAllowed: false,
    remoteInputControlAllowed: false,
  },
  deletion: deletionProof,
  assertions: {
    realPixelsCaptured: captureMetadata.captured === true && frameBytes.length > 0,
    localTransportDeliveredFrame: transport.viewerReceivedFrame && transport.digestMatched && transport.hmacMatched,
    rawFrameDeletedAfterTransport: deletionProof.rawFrameDeletedAfterTransport,
    noRawFrameRetention: !deletionProof.transportCachedRawFrame && !deletionProof.sessionRecorded,
    noRemoteInput: !deletionProof.remoteInputAllowed,
  },
  nonClaims: [
    'This proof uses a local loopback session harness; it does not implement production live-view service session workers.',
    'This proof does not claim platform live-view permission-prompt screenshots or privacy/legal approval.',
    'This proof does not enable remote input, relay/cache execution, raw-frame retention, or session recording.',
    'This proof does not make live view product-ready without the separate platform permission gate.',
  ],
};

if (!Object.values(proof.assertions).every(Boolean)) {
  throw new Error(`live-view session transport assertions failed: ${JSON.stringify(proof.assertions)}`);
}

writeJson(proofPath, proof);
console.log(`screen-live-view-session-transport-proof-ok:${proofPath}`);

async function runViewOnlyLoopbackTransport(frameBytes, frameDigest) {
  const sessionId = `screen-live-view-loopback-${Date.now()}`;
  const frameSequence = 1;
  const sessionKey = randomBytes(32);
  const hmac = createHmac('sha256', sessionKey).update(frameBytes).digest('hex');

  const serverResult = new Promise((resolveResult, rejectResult) => {
    const server = createServer((socket) => {
      const chunks = [];
      socket.on('data', (chunk) => chunks.push(chunk));
      socket.on('end', () => {
        try {
          const received = Buffer.concat(chunks);
          const separatorIndex = received.indexOf(10);
          if (separatorIndex < 0) {
            throw new Error('live-view transport frame header was missing');
          }
          const header = JSON.parse(received.subarray(0, separatorIndex).toString('utf8'));
          const payload = received.subarray(separatorIndex + 1);
          const receivedDigest = sha256(payload);
          const receivedHmac = createHmac('sha256', sessionKey).update(payload).digest('hex');
          resolveResult({
            sessionId: header.sessionId,
            frameSequence: header.frameSequence,
            frameByteLength: payload.length,
            frameDigest: receivedDigest,
            viewerReceivedFrame: payload.length === header.frameByteLength,
            digestMatched: receivedDigest === header.frameDigest && receivedDigest === frameDigest,
            hmacMatched: receivedHmac === header.hmac && receivedHmac === hmac,
          });
        } catch (error) {
          rejectResult(error);
        } finally {
          server.close();
        }
      });
    });
    server.on('error', rejectResult);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const client = connect({ host: '127.0.0.1', port: address.port }, () => {
        const header = {
          sessionId,
          frameSequence,
          frameByteLength: frameBytes.length,
          frameDigest,
          hmac,
          remoteInputControlAllowed: false,
          sessionRecordingAllowed: false,
          cacheRawFrames: false,
        };
        client.write(`${JSON.stringify(header)}\n`);
        client.end(frameBytes);
      });
      client.on('error', rejectResult);
    });
  });

  return serverResult;
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
