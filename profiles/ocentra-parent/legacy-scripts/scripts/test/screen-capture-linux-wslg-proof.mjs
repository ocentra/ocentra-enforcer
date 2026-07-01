import { createHash } from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const proofRoot = join('output', 'screen-plan-proof', 'linux-wslg');
const selectedWindowDir = join(proofRoot, 'selected-window');
const externalGateArtifactsDir = join('output', 'screen-plan-proof', 'external-gates', 'artifacts');
const externalGateArtifactPath = join(externalGateArtifactsDir, 'linux-wslg-xmessage-live-surface.png');
const title = 'OcentraWslgProof';
const visibleProofText = [
  'Ocentra WSLg external gate proof',
  'Category: school productivity',
  'Policy dry run expects allow',
  'Raw product capture is deleted after analysis',
].join('\n');
const cargo = process.env.CARGO ?? '/root/.cargo/bin/cargo';
const targetDir = process.env.CARGO_TARGET_DIR ?? '/tmp/ocentra-parent-target-screen';
const examplePath = join(targetDir, 'debug', 'examples', 'screen_capture_real_proof');

if (process.platform !== 'linux') {
  throw new Error('screen-capture-linux-wslg-proof must run inside Linux/WSL.');
}

for (const command of ['xmessage', 'xwininfo', 'xwd', 'convert', 'identify']) {
  requireCommand(command);
}

rmSync(proofRoot, { recursive: true, force: true });
mkdirSync(selectedWindowDir, { recursive: true });
mkdirSync(externalGateArtifactsDir, { recursive: true });

run(cargo, ['build', '-p', 'ocentra-parent-screen-capture-adapter', '--example', 'screen_capture_real_proof'], {
  CARGO_TARGET_DIR: targetDir,
});

const xmessage = spawn('xmessage', ['-name', title, '-title', title, '-geometry', '420x180+80+80', visibleProofText]);
xmessage.stdout.on('data', (chunk) => appendLog('xmessage-stdout.log', chunk));
xmessage.stderr.on('data', (chunk) => appendLog('xmessage-stderr.log', chunk));

let externalGateArtifact = null;
try {
  await delay(1500);
  externalGateArtifact = captureExternalGateArtifact();
  run(examplePath, [selectedWindowDir], {
    OCENTRA_SCREEN_CAPTURE_WINDOW_TITLE_CONTAINS: title,
  });
} finally {
  xmessage.kill();
}

const metadata = readJson(join(selectedWindowDir, '02-capture-metadata.json'));
const deletion = readJson(join(selectedWindowDir, '04-deletion-proof.json'));
const summary = {
  proof: 'screen-capture-linux-wslg-proof',
  platform: process.platform,
  session: {
    display: process.env.DISPLAY ?? null,
    waylandDisplay: process.env.WAYLAND_DISPLAY ?? null,
    xdgSessionType: process.env.XDG_SESSION_TYPE ?? null,
  },
  selectedWindow: {
    captured: metadata.captured === true,
    status: metadata.status,
    actualScope: metadata.actualScope,
    width: metadata.width,
    height: metadata.height,
    imageByteSize: metadata.imageByteSize,
    titlePresent: metadata.titlePresent,
    windowId: metadata.windowId,
  },
  custody: {
    rawImageDeleted: deletion.rawImageDeleted === true,
    existsAfterDelete: deletion.existsAfterDelete,
    encryptedQueueContainsRawDigest: deletion.encryptedQueueContainsRawDigest,
  },
  selectedWindowArtifact: selectedWindowDir,
  visibleProofText,
  externalGateArtifact,
  degradedIsCaptureProof: false,
  nonClaims: [
    'This proves WSLg/X11 selected-window capture of a real native xmessage surface only.',
    'The retained external gate PNG is an operator-safe visual inspection artifact captured from the same controlled live Linux window; it is not the raw product capture queue image.',
    'It does not claim WSLg root display capture, native Linux Wayland portal capture, or macOS/iOS/Android physical parity.',
  ],
};

if (
  summary.selectedWindow.captured !== true ||
  summary.selectedWindow.status !== 'available' ||
  summary.selectedWindow.actualScope !== 'selectedWindow' ||
  !Number.isInteger(summary.selectedWindow.width) ||
  !Number.isInteger(summary.selectedWindow.height) ||
  summary.selectedWindow.width <= 0 ||
  summary.selectedWindow.height <= 0 ||
  !Number.isInteger(summary.selectedWindow.imageByteSize) ||
  summary.selectedWindow.imageByteSize <= 0 ||
  summary.externalGateArtifact?.present !== true ||
  summary.externalGateArtifact?.bytes <= 0 ||
  summary.custody.rawImageDeleted !== true ||
  summary.custody.existsAfterDelete !== false ||
  summary.custody.encryptedQueueContainsRawDigest !== false
) {
  writeJson(join(proofRoot, 'proof-summary.json'), summary);
  throw new Error(`Linux WSLg selected-window proof failed: ${JSON.stringify(summary, null, 2)}`);
}

writeJson(join(proofRoot, 'proof-summary.json'), summary);
console.log(`screen-capture-linux-wslg-proof-ok:${summary.selectedWindow.width}x${summary.selectedWindow.height}`);

function requireCommand(command) {
  const result = spawnSync('bash', ['-lc', `command -v ${command}`], {
    cwd: process.cwd(),
    encoding: 'utf8',
  });
  if (result.status !== 0) {
    throw new Error(`Required Linux proof command missing: ${command}`);
  }
}

function run(command, args, env = {}) {
  const result = spawnSync(command, args, {
    cwd: process.cwd(),
    encoding: 'utf8',
    env: {
      ...process.env,
      ...env,
    },
  });
  writeFileSync(join(proofRoot, `${safeName(command)}-stdout.log`), result.stdout ?? '');
  writeFileSync(join(proofRoot, `${safeName(command)}-stderr.log`), result.stderr ?? '');
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(' ')} failed with ${result.status}`);
  }
}

function captureExternalGateArtifact() {
  const xwdPath = join(proofRoot, 'linux-wslg-xmessage-live-surface.xwd');
  rmSync(externalGateArtifactPath, { force: true });
  rmSync(xwdPath, { force: true });
  run('xwd', ['-name', title, '-silent', '-out', xwdPath]);
  run('convert', [xwdPath, externalGateArtifactPath]);
  rmSync(xwdPath, { force: true });
  const identify = spawnSync('identify', ['-format', '%w %h', externalGateArtifactPath], {
    cwd: process.cwd(),
    encoding: 'utf8',
  });
  if (identify.status !== 0) {
    throw new Error(`identify failed for ${externalGateArtifactPath}: ${identify.stderr}`);
  }
  const [width, height] = identify.stdout
    .trim()
    .split(/\s+/u)
    .map((entry) => Number.parseInt(entry, 10));
  const bytes = readFileSync(externalGateArtifactPath);
  return {
    present: existsSync(externalGateArtifactPath),
    path: externalGateArtifactPath,
    sha256: sha256(bytes),
    bytes: bytes.byteLength,
    width,
    height,
    sourceSurface: 'real-wslg-x11-xmessage-window',
    rawPrivateContentIncluded: false,
  };
}

function appendLog(name, chunk) {
  writeFileSync(join(proofRoot, name), chunk, { flag: 'a' });
}

function safeName(command) {
  return command.replaceAll('/', '_').replaceAll('\\', '_').replaceAll(':', '_');
}

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function delay(milliseconds) {
  return new Promise((resolve) => {
    setTimeout(resolve, milliseconds);
  });
}
