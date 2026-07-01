import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..', '..');
const outputDir = join(repoRoot, 'output', 'screen-plan-proof', 'linux');
const proofPath = join(outputDir, 'proof-summary.json');
const wslgProofPath = join(repoRoot, 'output', 'screen-plan-proof', 'linux-wslg', 'proof-summary.json');

run('npm', ['run', 'build', '--workspace', '@ocentra-parent/schema-domain']);

const screenLinux = await import('@ocentra-parent/schema-domain/screen-linux-capture-capability-proof');

const generatedAt = new Date().toISOString();
const proof = screenLinux.screenLinuxCaptureCapabilityProof(generatedAt);
const wslgProof = readJson(wslgProofPath);

if (!wslgProof.selectedWindow?.captured || !wslgProof.custody?.rawImageDeleted || wslgProof.degradedIsCaptureProof) {
  throw new Error(`Existing Linux WSLg proof is not strong enough: ${JSON.stringify(wslgProof)}`);
}

const negativeChecks = [
  rejects('native readiness requires native session proof', () =>
    screenLinux.ScreenLinuxCaptureCapabilityRowSchema.safeParse({
      ...proof.rows[1],
      captureState: 'ready',
      proofState: 'nativeSessionVerified',
      deletionProofRef: 'screen-linux-native-selected-window-deletion-proof',
      productLinuxCaptureReady: true,
    })
  ),
  rejects('root-display capture is not claimed by the WSLg proof', () =>
    screenLinux.ScreenLinuxCaptureCapabilityRowSchema.safeParse({
      ...proof.rows[2],
      rootDisplayClaimed: true,
    })
  ),
  rejects('raw frame remote upload remains forbidden', () =>
    screenLinux.ScreenLinuxCaptureCapabilityRowSchema.safeParse({
      ...proof.rows[2],
      rawFrameRemoteUploadAllowed: true,
    })
  ),
  rejects('Wayland rows require PipeWire portal proof', () =>
    screenLinux.ScreenLinuxCaptureCapabilityRowSchema.safeParse({
      ...proof.rows[3],
      pipeWireRequired: false,
    })
  ),
];

if (negativeChecks.some((check) => !check.rejected)) {
  throw new Error(`Unexpected Linux capture gate result: ${JSON.stringify(negativeChecks)}`);
}

const summary = {
  proof: proof.proofId,
  generatedAt,
  claim:
    'Linux WSLg/X11 selected-window proof exists, but native Linux root-display and Wayland/PipeWire portal parity remain blocked before product readiness.',
  linuxDocsVerified: [
    {
      ref: 'xdg-desktop-portal-screencast',
      url: 'https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.ScreenCast.html',
      summary:
        'The XDG Desktop Portal ScreenCast interface opens PipeWire streams for selected monitor/window capture through a portal-mediated session.',
    },
    {
      ref: 'imagemagick-import-x-server',
      url: 'https://imagemagick.org/script/import.php',
      summary:
        'ImageMagick import captures an X server screen or selected X11 window, matching the X11 command backend boundary.',
    },
  ],
  existingWslgProof: {
    path: relativePath(wslgProofPath),
    display: wslgProof.session?.display ?? null,
    waylandDisplay: wslgProof.session?.waylandDisplay ?? null,
    selectedWindowCaptured: wslgProof.selectedWindow?.captured === true,
    selectedWindowScope: wslgProof.selectedWindow?.actualScope ?? null,
    rawImageDeleted: wslgProof.custody?.rawImageDeleted === true,
    degradedIsCaptureProof: wslgProof.degradedIsCaptureProof === true,
  },
  rows: proof.rows.map((row) => ({
    mode: row.mode,
    captureState: row.captureState,
    proofState: row.proofState,
    compositor: row.compositor,
    x11CommandBackendRequired: row.x11CommandBackendRequired,
    waylandPortalRequired: row.waylandPortalRequired,
    pipeWireRequired: row.pipeWireRequired,
    userMediatedSelectionRequired: row.userMediatedSelectionRequired,
    rootDisplayClaimed: row.rootDisplayClaimed,
    rawFrameRemoteUploadAllowed: row.rawFrameRemoteUploadAllowed,
    rawFrameRetentionDefault: row.rawFrameRetentionDefault,
    productLinuxCaptureReady: row.productLinuxCaptureReady,
    wslgProofRef: row.wslgProofRef,
    nativeSessionProofRef: row.nativeSessionProofRef,
    deletionProofRef: row.deletionProofRef,
  })),
  negativeChecks,
  gapStatus: {
    wslgX11SelectedWindowProofExists: proof.wslgSelectedWindowCaptureProved,
    nativeX11SelectedWindowProofExists: false,
    nativeX11RootDisplayProofExists: false,
    nativeWaylandPipeWireProofExists: false,
    productLinuxCaptureReady: proof.productLinuxCaptureReady,
  },
  artifacts: {
    summary: relativePath(proofPath),
  },
  nonClaims: proof.nonClaims,
};

mkdirSync(outputDir, { recursive: true });
writeFileSync(proofPath, `${JSON.stringify(summary, null, 2)}\n`);
console.log(`screen-linux-capture-capability-proof-ok:${proofPath}`);

function rejects(name, parseAttempt) {
  return { name, rejected: !parseAttempt().success };
}

function readJson(path) {
  if (!existsSync(path)) {
    throw new Error(`Missing required proof artifact: ${path}`);
  }
  return JSON.parse(readFileSync(path, 'utf8'));
}

function relativePath(path) {
  return relative(repoRoot, path).replace(/\\/gu, '/');
}

function run(command, args) {
  const runner = process.platform === 'win32' ? 'cmd' : command;
  const runnerArgs = process.platform === 'win32' ? ['/c', command, ...args] : args;
  execFileSync(runner, runnerArgs, { cwd: repoRoot, stdio: 'inherit' });
}
