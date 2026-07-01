import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const outputDir = join('output', 'screen-plan-proof', 'real-capture', 'manual-parent-test-active-window');

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });

const result = spawnSync(
  'cargo',
  ['run', '-p', 'ocentra-parent-screen-capture-adapter', '--example', 'screen_capture_real_proof', '--', outputDir],
  {
    cwd: process.cwd(),
    encoding: 'utf8',
    shell: process.platform === 'win32',
  }
);

writeFileSync(join(outputDir, 'cargo-stdout.log'), result.stdout ?? '');
writeFileSync(join(outputDir, 'cargo-stderr.log'), result.stderr ?? '');

if (result.status !== 0) {
  throw new Error(`screen capture proof command failed with status ${result.status}`);
}

const metadata = JSON.parse(readFileSync(join(outputDir, '02-capture-metadata.json'), 'utf8'));
const deletion = metadata.captured ? JSON.parse(readFileSync(join(outputDir, '04-deletion-proof.json'), 'utf8')) : null;

const summary = {
  proof: 'screen-capture-real-proof',
  outputDir,
  platform: process.platform,
  captured: metadata.captured === true,
  status: metadata.status,
  realCaptureProof: metadata.captured === true && deletion?.rawImageDeleted === true,
  degradedIsCaptureProof: false,
  note:
    metadata.captured === true
      ? 'Real capture proof exists: image bytes were captured, encrypted into queue custody, and raw temp bytes were deleted.'
      : 'No real capture proof: degraded/unsupported/access-denied state was recorded honestly.',
};

writeFileSync(join(outputDir, 'proof-summary.json'), `${JSON.stringify(summary, null, 2)}\n`);

if (process.platform === 'win32' && !summary.realCaptureProof) {
  throw new Error(`Windows screen capture proof did not produce captured bytes; status=${metadata.status}`);
}

console.log(JSON.stringify(summary, null, 2));
