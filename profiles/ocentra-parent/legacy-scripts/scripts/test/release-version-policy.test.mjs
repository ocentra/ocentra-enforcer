import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import { evaluateReleaseVersionPolicy, isReleaseSemver } from '../release/version-policy.mjs';

function withVersionWorkspace(testBody) {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-parent-release-version-'));
  try {
    mkdirSync(join(root, 'apps', 'portal'), { recursive: true });
    mkdirSync(join(root, 'packages', 'schema-domain'), { recursive: true });
    mkdirSync(join(root, 'platforms', 'android', 'agent', 'app'), { recursive: true });
    mkdirSync(join(root, 'platforms', 'ios', 'OcentraParentAgent.xcodeproj'), { recursive: true });
    return testBody(root);
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
}

function writeJson(path, value) {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function cargoMetadata(version) {
  return JSON.stringify({
    packages: [
      {
        manifest_path: 'crates/agent-service/Cargo.toml',
        version,
      },
    ],
  });
}

test('release SemVer policy accepts stable and prerelease versions', () => {
  assert.equal(isReleaseSemver('0.1.0'), true);
  assert.equal(isReleaseSemver('1.2.3-alpha.4'), true);
  assert.equal(isReleaseSemver('01.2.3'), false);
});

test('release version policy accepts aligned workspace versions', () => {
  withVersionWorkspace((root) => {
    writeJson(join(root, 'package.json'), { version: '0.1.0' });
    writeJson(join(root, 'package-lock.json'), { version: '0.1.0' });
    writeJson(join(root, 'apps', 'portal', 'package.json'), { version: '0.1.0' });
    writeJson(join(root, 'packages', 'schema-domain', 'package.json'), { version: '0.1.0' });
    writeFileSync(join(root, 'platforms', 'android', 'agent', 'app', 'build.gradle'), "versionName = '0.1.0'\n");
    writeFileSync(
      join(root, 'platforms', 'ios', 'OcentraParentAgent.xcodeproj', 'project.pbxproj'),
      'MARKETING_VERSION = 0.1.0;\n'
    );

    const result = evaluateReleaseVersionPolicy(root, {
      cargoMetadataText: cargoMetadata('0.1.0'),
    });

    assert.equal(result.ok, true);
    assert.equal(result.version, '0.1.0');
    assert.equal(result.checkedSources.length, 7);
  });
});

test('release version policy rejects drift between runtimes', () => {
  withVersionWorkspace((root) => {
    writeJson(join(root, 'package.json'), { version: '0.1.0' });
    writeJson(join(root, 'package-lock.json'), { version: '0.1.0' });
    writeJson(join(root, 'apps', 'portal', 'package.json'), { version: '0.1.1' });
    writeJson(join(root, 'packages', 'schema-domain', 'package.json'), { version: '0.1.0' });
    writeFileSync(join(root, 'platforms', 'android', 'agent', 'app', 'build.gradle'), "versionName = '0.1.0'\n");
    writeFileSync(
      join(root, 'platforms', 'ios', 'OcentraParentAgent.xcodeproj', 'project.pbxproj'),
      'MARKETING_VERSION = 0.1.0;\n'
    );

    const result = evaluateReleaseVersionPolicy(root, {
      cargoMetadataText: cargoMetadata('0.1.0'),
    });

    assert.equal(result.ok, false);
    assert.equal(
      result.findings.some((finding) => finding.includes('Release versions are not aligned')),
      true
    );
  });
});

test('release version policy rejects platform version drift', () => {
  withVersionWorkspace((root) => {
    writeJson(join(root, 'package.json'), { version: '0.1.0' });
    writeJson(join(root, 'package-lock.json'), { version: '0.1.0' });
    writeJson(join(root, 'apps', 'portal', 'package.json'), { version: '0.1.0' });
    writeJson(join(root, 'packages', 'schema-domain', 'package.json'), { version: '0.1.0' });
    writeFileSync(join(root, 'platforms', 'android', 'agent', 'app', 'build.gradle'), "versionName = '0.1.2'\n");
    writeFileSync(
      join(root, 'platforms', 'ios', 'OcentraParentAgent.xcodeproj', 'project.pbxproj'),
      'MARKETING_VERSION = 0.1.0;\n'
    );

    const result = evaluateReleaseVersionPolicy(root, {
      cargoMetadataText: cargoMetadata('0.1.0'),
    });

    assert.equal(result.ok, false);
    assert.equal(
      result.findings.some((finding) => finding.includes('Release versions are not aligned')),
      true
    );
  });
});
