import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import {
  WINDOWS_BOOTSTRAP_NAME,
  WINDOWS_LATEST_MSI_NAME,
  inspectWindowsPreviewArtifact,
  normalizeSha256,
  parseChecksumLine,
  PackageLifecycleProofError,
  sha256File,
} from '../release/windows/package-lifecycle-artifacts.mjs';
import { buildLifecycleDecision } from '../release/windows/package-lifecycle-runner.mjs';
import {
  parseServiceFailureActions,
  parseServiceFailureFlag,
} from '../release/windows/package-lifecycle-host.mjs';

test('Windows package lifecycle artifact verification checks manifest, MSI hashes, and sidecars', () => {
  const tempRoot = mkdtempSync(join(tmpdir(), 'ocentra-parent-windows-proof-'));
  try {
    const version = '0.1.1';
    const versionedName = `ocentra-parent-agent-windows-x64-v${version}.msi`;
    const latestPath = join(tempRoot, WINDOWS_LATEST_MSI_NAME);
    const versionedPath = join(tempRoot, versionedName);
    writeFileSync(versionedPath, 'ocentra-parent-msi-payload', 'utf8');
    writeFileSync(latestPath, 'ocentra-parent-msi-payload', 'utf8');
    const sha256 = sha256File(versionedPath);
    writeFileSync(join(tempRoot, `${versionedName}.sha256`), `${sha256}  ${versionedName}\n`, 'utf8');
    writeFileSync(
      join(tempRoot, `${WINDOWS_LATEST_MSI_NAME}.sha256`),
      `${sha256}  ${WINDOWS_LATEST_MSI_NAME}\n`,
      'utf8'
    );
    writeFileSync(
      join(tempRoot, WINDOWS_BOOTSTRAP_NAME),
      "throw 'Release manifest is not signed.'\nmsiexec.exe /qn /norestart\n",
      'utf8'
    );
    writeFileSync(
      join(tempRoot, 'latest-windows.json'),
      `${JSON.stringify(sampleManifest(version, versionedName, sha256), null, 2)}\n`,
      'utf8'
    );

    const proof = inspectWindowsPreviewArtifact(tempRoot);

    assert.equal(proof.status, 'verified');
    assert.equal(proof.manifest.version, version);
    assert.equal(proof.manifest.service.id, 'OcentraParentAgent');
    assert.equal(proof.manifest.service.updaterId, 'OcentraParentUpdater');
    assert.equal(proof.files.versionedMsi.sha256, sha256);
    assert.equal(proof.files.latestMsi.sha256, sha256);
    assert.deepEqual(
      proof.sidecars.map((sidecar) => sidecar.status),
      ['verified', 'verified']
    );
  } finally {
    rmSync(tempRoot, { force: true, recursive: true });
  }
});

test('Windows package lifecycle artifact verification rejects corrupted latest MSI sidecar', () => {
  const tempRoot = mkdtempSync(join(tmpdir(), 'ocentra-parent-windows-proof-'));
  try {
    const version = '0.1.1';
    const versionedName = `ocentra-parent-agent-windows-x64-v${version}.msi`;
    const versionedPath = join(tempRoot, versionedName);
    writeFileSync(versionedPath, 'original', 'utf8');
    writeFileSync(join(tempRoot, WINDOWS_LATEST_MSI_NAME), 'changed', 'utf8');
    const sha256 = sha256File(versionedPath);
    writeFileSync(join(tempRoot, `${versionedName}.sha256`), `${sha256}  ${versionedName}\n`, 'utf8');
    writeFileSync(
      join(tempRoot, `${WINDOWS_LATEST_MSI_NAME}.sha256`),
      `${sha256}  ${WINDOWS_LATEST_MSI_NAME}\n`,
      'utf8'
    );
    writeFileSync(
      join(tempRoot, WINDOWS_BOOTSTRAP_NAME),
      "throw 'Release manifest is not signed.'\nmsiexec.exe /qn /norestart\n",
      'utf8'
    );
    writeFileSync(
      join(tempRoot, 'latest-windows.json'),
      `${JSON.stringify(sampleManifest(version, versionedName, sha256), null, 2)}\n`,
      'utf8'
    );

    assert.throws(() => inspectWindowsPreviewArtifact(tempRoot), { code: 'latest-artifact-sha256-mismatch' });
  } finally {
    rmSync(tempRoot, { force: true, recursive: true });
  }
});

test('Windows package lifecycle decisions never reboot automatically and require admin before install', () => {
  assert.deepEqual(buildLifecycleDecision({ elevated: false, installRequested: true, platform: 'win32' }), {
    installAttempted: false,
    reason: 'requires-elevated-shell',
    rebootAttempted: false,
    status: 'admin-required',
  });
  assert.deepEqual(buildLifecycleDecision({ elevated: true, installRequested: false, platform: 'win32' }), {
    installAttempted: false,
    reason: 'install-flag-not-set',
    rebootAttempted: false,
    status: 'ready-not-run',
  });
  assert.deepEqual(buildLifecycleDecision({ elevated: true, installRequested: true, platform: 'win32' }), {
    installAttempted: true,
    reason: 'install-flag-set',
    rebootAttempted: false,
    status: 'install-requested',
  });
});

test('Windows package lifecycle checksum parser accepts release sidecar format', () => {
  const checksum = normalizeSha256('a'.repeat(64));
  assert.deepEqual(parseChecksumLine(`${checksum}  ocentra-parent-agent-windows-x64-latest.msi\n`), {
    fileName: 'ocentra-parent-agent-windows-x64-latest.msi',
    sha256: checksum,
  });
});

test('Windows package lifecycle parses service-manager restart actions as respawn proof input', () => {
  const parsed = parseServiceFailureActions(`
[SC] QueryServiceConfig2 SUCCESS

SERVICE_NAME: OcentraParentAgent
        RESET_PERIOD (in seconds)    : 86400
        REBOOT_MESSAGE               :
        COMMAND_LINE                 :
        FAILURE_ACTIONS              : RESTART -- Delay = 10000 milliseconds.
                                       RESTART -- Delay = 30000 milliseconds.
                                       NONE -- Delay = 0 milliseconds.
`);

  assert.equal(parsed.resetPeriodSeconds, 86400);
  assert.deepEqual(parsed.actions, [
    { delayMilliseconds: 10000, type: 'restart' },
    { delayMilliseconds: 30000, type: 'restart' },
    { delayMilliseconds: 0, type: 'none' },
  ]);
});

test('Windows package lifecycle parses the service-manager failure-actions flag', () => {
  assert.deepEqual(
    parseServiceFailureFlag(`
[SC] QueryServiceConfig2 SUCCESS

SERVICE_NAME: OcentraParentAgent
        FAILURE_ACTIONS_FLAG         : 0
`),
    { enabled: false }
  );
  assert.deepEqual(
    parseServiceFailureFlag(`
[SC] QueryServiceConfig2 SUCCESS

SERVICE_NAME: OcentraParentAgent
        FAILURE_ACTIONS_FLAG         : 1
`),
    { enabled: true }
  );
});

test('Windows package lifecycle rejects malformed service-manager failure action lines', () => {
  assert.throws(
    () =>
      parseServiceFailureActions(`
[SC] QueryServiceConfig2 SUCCESS

SERVICE_NAME: OcentraParentAgent
        RESET_PERIOD (in seconds)    : 86400
        FAILURE_ACTIONS              : RESTART
`),
    (error) =>
      error instanceof PackageLifecycleProofError && error.code === 'failure-action-line-invalid'
  );
});

function sampleManifest(version, artifactName, sha256) {
  return {
    payload: {
      artifact: {
        downloadUrl: `https://github.com/ocentra/OcentraParent/releases/download/v${version}/${artifactName}`,
        name: artifactName,
        sha256,
      },
      channel: 'stable',
      generatedAt: '2026-05-25T00:00:00.000Z',
      installer: {
        passiveArgs: '/passive /norestart',
        scope: 'per-machine',
        silentArgs: '/qn /norestart',
        type: 'msi',
      },
      package: 'ocentra-parent-agent',
      product: 'Ocentra Parent',
      schemaVersion: 1,
      service: {
        id: 'OcentraParentAgent',
        name: 'Ocentra Parent Agent',
        updaterId: 'OcentraParentUpdater',
        updaterName: 'Ocentra Parent Updater',
        wrapper: 'WinSW',
        wrapperVersion: '2.12.0',
      },
      target: 'windows-x64',
      version,
    },
    signature: {
      algorithm: 'Ed25519',
      keyId: 'test-key',
      value: 'test-signature',
    },
  };
}
