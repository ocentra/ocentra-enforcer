import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  collectAllRustFiles,
  findCargoManifests,
  isIgnoredPath,
  resolveScope,
} from '../scripts/rust-rules-path-core.mjs';
import {
  collectFiles as collectGenericFiles,
  isIgnoredPath as isGenericIgnoredPath,
} from '../src/path-utils.mjs';

const CONFIG = {
  ignoreDirs: ['.git', '.tmp', 'target'],
  ignoreFileGlobs: [],
  rustRoots: ['.'],
};

function makeTree(files) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'enforcer-target-roots-'));
  for (const [relativePath, content] of Object.entries(files)) {
    const absolutePath = path.join(root, relativePath);
    fs.mkdirSync(path.dirname(absolutePath), { recursive: true });
    fs.writeFileSync(absolutePath, content, 'utf8');
  }
  return root;
}

function relativePaths(root, files) {
  return files.map((file) => path.relative(root, file).split(path.sep).join('/'));
}

test('walker skips nested target and target-* generated roots but scans legitimate lookalikes', (t) => {
  const root = makeTree({
    'crates/real/Cargo.toml': '[package]\nname = "real"\nversion = "0.1.0"\n',
    'crates/real/src/lib.rs': 'pub struct RealSource;\n',
    'crates/targeted/Cargo.toml': '[package]\nname = "targeted"\nversion = "0.1.0"\n',
    'crates/targeted/src/lib.rs': 'pub struct TargetedSource;\n',
    'src/target-helper.rs': 'pub struct TargetHelperSource;\n',
    'src/target_tools/mod.rs': 'pub struct TargetToolsSource;\n',
    'src/my-target-cache/mod.rs': 'pub struct MyTargetCacheSource;\n',
    'target/debug/build.rs': 'compile_error!("generated target was scanned");\n',
    'target-dogfood-staged-source/crates/copied/Cargo.toml': '[package]\nname = "copied"\nversion = "0.1.0"\n',
    'target-dogfood-staged-source/crates/copied/src/lib.rs': 'compile_error!("staged source was scanned");\n',
    'scratch/target-proof-final/copied/Cargo.toml': '[package]\nname = "proof-copy"\nversion = "0.1.0"\n',
    'scratch/target-proof-final/copied/src/lib.rs': 'compile_error!("proof target was scanned");\n',
    'crates/real/target-debug/build.rs': 'compile_error!("nested target was scanned");\n',
  });
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));

  assert.deepEqual(relativePaths(root, collectAllRustFiles(root, CONFIG)), [
    'crates/real/src/lib.rs',
    'crates/targeted/src/lib.rs',
    'src/my-target-cache/mod.rs',
    'src/target_tools/mod.rs',
    'src/target-helper.rs',
  ]);
  assert.deepEqual(
    relativePaths(root, resolveScope(root, CONFIG, { mode: 'all' }).files),
    [
      'crates/real/src/lib.rs',
      'crates/targeted/src/lib.rs',
      'src/my-target-cache/mod.rs',
      'src/target_tools/mod.rs',
      'src/target-helper.rs',
    ],
  );
  assert.deepEqual(relativePaths(root, findCargoManifests(root, CONFIG)), [
    'crates/real/Cargo.toml',
    'crates/targeted/Cargo.toml',
  ]);
});

test('target-* exclusion applies only to directory path segments', () => {
  assert.equal(isIgnoredPath('target-dogfood-staged-source', CONFIG, true), true);
  assert.equal(isIgnoredPath('nested/target-proof-final/src/lib.rs', CONFIG), true);
  assert.equal(isIgnoredPath('src/target-report.rs', CONFIG), false);
  assert.equal(isIgnoredPath('src/targeted/mod.rs', CONFIG), false);
  assert.equal(isIgnoredPath('src/target_tools/mod.rs', CONFIG), false);
  assert.equal(isIgnoredPath('src/my-target-cache/mod.rs', CONFIG), false);
  assert.equal(isIgnoredPath('target-dogfood-staged-source', CONFIG, false), false);
});

test('generic walker skips target-* and .tmp-* directories without hiding lookalike files', (t) => {
  const root = makeTree({
    'src/real.mjs': 'export const real = true;\n',
    'src/target-report.mjs': 'export const report = true;\n',
    'src/.tmp-report.mjs': 'export const report = true;\n',
    'target-dogfood-staged-source/generated.mjs': 'throw new Error("generated target was scanned");\n',
    '.tmp-scanner-evidence/generated.mjs': 'throw new Error("temporary evidence was scanned");\n',
    'nested/target-proof-final/generated.mjs': 'throw new Error("nested generated target was scanned");\n',
    'nested/.tmp-proof-final/generated.mjs': 'throw new Error("nested temporary evidence was scanned");\n',
  });
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));

  const config = { ignoreDirs: ['target', '.tmp'], ignoreFileGlobs: [] };
  const files = collectGenericFiles(
    root,
    [],
    config,
    (file) => path.extname(file) === '.mjs',
  );

  assert.deepEqual(relativePaths(root, files), [
    'src/.tmp-report.mjs',
    'src/real.mjs',
    'src/target-report.mjs',
  ]);
  assert.equal(isGenericIgnoredPath('target-dogfood-staged-source', config, true), true);
  assert.equal(isGenericIgnoredPath('.tmp-scanner-evidence', config, true), true);
  assert.equal(isGenericIgnoredPath('src/target-report.mjs', config), false);
  assert.equal(isGenericIgnoredPath('src/.tmp-report.mjs', config), false);
});
