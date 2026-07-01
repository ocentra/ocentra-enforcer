import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';

const repoRoot = process.cwd();

function readRepoFile(path) {
  return readFileSync(join(repoRoot, path), 'utf8');
}

test('parent Android proof anchor stays parent-only and explicit about install/store gaps', () => {
  const packageJson = JSON.parse(readRepoFile('package.json'));
  const proofScript = readRepoFile('scripts/test/parent-android-package-proof.mjs');

  assert.equal(
    packageJson.scripts['test:parent-android-package-proof'],
    'node scripts/test/parent-android-package-proof.mjs'
  );
  assert.match(proofScript, /release:package:parent-android/u);
  assert.match(proofScript, /ca\.ocentra\.parent\.mobile/u);
  assert.match(proofScript, /android-device-or-booted-emulator-required/u);
  assert.match(proofScript, /storeDistributionState: 'manual-required'/u);
  assert.match(proofScript, /child-runtime distribution not claimed/u);
  assert.match(proofScript, /android-apk-smoke\.sh/u);
});
