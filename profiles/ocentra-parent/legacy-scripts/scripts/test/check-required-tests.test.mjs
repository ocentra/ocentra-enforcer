import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { test } from 'node:test';

const repoRoot = process.cwd();
const scriptPath = join(repoRoot, 'scripts', 'check-required-tests.mjs');

function writeFile(filePath, contents) {
  mkdirSync(dirname(filePath), { recursive: true });
  writeFileSync(filePath, contents);
}

function createFixture(root) {
  const packageRoot = join(root, 'packages', 'demo-package');
  const crateRoot = join(root, 'crates', 'demo-crate');

  writeFile(join(packageRoot, 'package.json'), JSON.stringify({ name: '@ocentra-parent/demo-package' }, null, 2));
  writeFile(join(packageRoot, 'src', 'index.ts'), 'export const demoValue = 1;\n');
  writeFile(
    join(packageRoot, 'tests', 'unit', 'demo.test.ts'),
    [
      'import assert from "node:assert/strict";',
      'import { test } from "node:test";',
      '',
      'test("demo package test exists", () => {',
      '  assert.equal(1, 1);',
      '});',
      '',
    ].join('\n')
  );
  writeFile(join(packageRoot, 'tests', 'contract', '.gitkeep'), '');

  writeFile(
    join(crateRoot, 'Cargo.toml'),
    ['[package]', 'name = "demo-crate"', 'version = "0.1.0"', 'edition = "2021"', ''].join('\n')
  );
  writeFile(
    join(crateRoot, 'src', 'lib.rs'),
    [
      '#[cfg(test)]',
      'mod tests {',
      '    #[test]',
      '    fn demo_crate_test_exists() {',
      '        assert_eq!(1, 1);',
      '    }',
      '}',
      '',
    ].join('\n')
  );
  writeFile(join(crateRoot, 'proof', '.gitkeep'), '');

  return {
    packageSource: join(packageRoot, 'src', 'index.ts'),
    crateSource: join(crateRoot, 'src', 'lib.rs'),
    emptyPackageTree: join(packageRoot, 'tests', 'contract'),
    emptyCrateTree: join(crateRoot, 'proof'),
  };
}

function runHarness(root, args) {
  return spawnSync(process.execPath, [scriptPath, ...args], {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env },
  });
}

test('required test scaffold accepts placeholder trees until strict cleanup mode is enabled', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-required-tests-'));
  const fixture = createFixture(root);

  const result = runHarness(root, ['--files', fixture.packageSource, fixture.crateSource]);

  assert.equal(result.status, 0);
  assert.match(result.stdout, /Ocentra Enforcer check required-tests passed/u);
  assert.equal(result.stderr, '');
});

test('strict cleanup mode rejects empty test and proof category trees without requiring every category', () => {
  const root = mkdtempSync(join(tmpdir(), 'ocentra-required-tests-strict-'));
  const fixture = createFixture(root);

  const result = runHarness(root, ['--strict-empty-test-trees', '--files', fixture.packageSource, fixture.crateSource]);

  assert.equal(result.status, 1);
  assert.match(
    result.stderr,
    /packages\/demo-package\/tests\/contract: empty test\/proof category tree contains only \.gitkeep/u
  );
  assert.match(result.stderr, /crates\/demo-crate\/proof: empty test\/proof category tree contains only \.gitkeep/u);
  assert.equal(result.stdout, '');
});
