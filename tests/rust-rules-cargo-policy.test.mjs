import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { makeProject, runGate, runGateArgs, expectFailure, expectFailures } from './rust-rules-fixture.mjs';

function refreshLock(project) {
  const result = spawnSync('cargo', ['generate-lockfile', '--offline'], {
    cwd: project,
    encoding: 'utf8',
    shell: false,
  });
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
}
test('Cargo wildcard dependency fails with RR-9.1', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct UserId;\n',
  });
  fs.appendFileSync(path.join(project, 'Cargo.toml'), '\n[dependencies]\nserde = "*"\n', 'utf8');
  expectFailure(project, 'RR-9.1');
});

test('locked Cargo metadata rejects a stale lock without mutating Cargo.lock', () => {
  const project = makeProject({
    'src/lib.rs': '#[derive(Debug)]\npub struct Value;\n',
    'helper/Cargo.toml': `
[package]
name = "helper"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
`,
    'helper/src/lib.rs': 'pub struct Helper;\n',
    'helper/OWNERS': '@ocentra/enforcer\n',
  });
  const manifest = path.join(project, 'Cargo.toml');
  const lockPath = path.join(project, 'Cargo.lock');
  const before = fs.readFileSync(lockPath, 'utf8');
  // Make the lock stale through the root package identity itself. Cargo's
  // handling of a newly-added local path dependency differs across platforms,
  // while a post-lock package-version change is deterministic everywhere.
  fs.writeFileSync(
    manifest,
    fs.readFileSync(manifest, 'utf8').replace('version = "0.1.0"', 'version = "0.2.0"'),
    'utf8',
  );
  fs.appendFileSync(
    manifest,
    '\n# DEPENDENCY-JUSTIFICATION: fixture dependency exercises stale-lock detection.\n[dependencies]\nhelper = { path = "helper" }\n',
    'utf8',
  );

  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.notEqual(result.status, 0, output);
  assert.match(output, /RR-9\.25/u, output);
  assert.equal(fs.readFileSync(lockPath, 'utf8'), before);
});

test('private test parse helpers do not require property or fuzz evidence', () => {
  const project = makeProject({
    'src/lib.rs': `
#[cfg(test)]
mod tests {
    fn parse_fixture(input: &str) -> &str { input }

    #[test]
    fn parses_a_fixture() {
        assert_eq!(parse_fixture("frame"), "frame");
    }
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-12\.(?:27|28)/u, output);
});

test('test code keeps test policy but is not held to production allocation rules', () => {
  const project = makeProject({
    'tests/fixture.rs': `
#[test]
fn exercises_a_fixture() {
    let values = vec![String::from("one")];
    let copied = values[0].clone();
    let rendered = copied.to_string();
    let narrowed = 7_u16 as u8;
    assert_eq!(rendered, "one");
    assert_eq!(narrowed, 7);
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-5\.[1-4]/u, output);
});

test('Cargo workspace-member paths are allowed while arbitrary local paths fail', () => {
  const project = makeProject({
    'Cargo.toml': `
[workspace]
members = ["crates/member", "crates/consumer"]

[workspace.package]
rust-version = "1.75"

[workspace.dependencies]
member = { path = "crates/member", version = "0.1.0" }
`,
    'crates/member/Cargo.toml': `
[package]
name = "member"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
`,
    'crates/member/src/lib.rs': 'pub struct MemberId;\n',
    'crates/consumer/Cargo.toml': `
[package]
name = "consumer"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[[bin]]
name = "consumer"
path = "src/main.rs"

[dependencies]
member = { path = "../member", version = "0.1.0" }
outside = { path = "../outside", version = "0.1.0" }
`,
    'crates/consumer/src/lib.rs': 'pub struct ConsumerId;\n',
    'crates/consumer/src/main.rs': 'fn main() {}\n',
  });
  const result = runGate(project);
  assert.notEqual(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /RR-9\.3/u, output);
  assert.match(output, /outside/u, output);
  assert.doesNotMatch(output, /member.*Path dependency found/u, output);
});

test('first-party vendored parser dependency passes RR-9.3 structural policy', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct MemoryGraph;\n',
  });
  fs.appendFileSync(
    path.join(project, 'Cargo.toml'),
    '\n[dependencies]\ntree-sitter-fixture-local = { path = "vendor/tree-sitter-fixture-local" }\n',
    'utf8',
  );
  const vendor = path.join(project, 'vendor', 'tree-sitter-fixture-local');
  fs.mkdirSync(path.join(vendor, 'src'), { recursive: true });
  fs.writeFileSync(
    path.join(vendor, 'Cargo.toml'),
    '[package]\nname = "tree-sitter-fixture-local"\nversion = "0.1.0"\nedition = "2021"\nrust-version = "1.75"\npublish = false\n[lib]\npath = "src/lib.rs"\n',
    'utf8',
  );
  fs.writeFileSync(path.join(vendor, 'src', 'lib.rs'), 'pub struct ParserLanguage;\n', 'utf8');
  fs.writeFileSync(path.join(vendor, 'src', 'parser.c'), 'int parser_fixture(void) { return 0; }\n', 'utf8');

  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-9\.3/u, output);
});

test('first-party vendored upstream parser package passes RR-9.3 structural policy', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct MemoryGraph;\n',
  });
  fs.appendFileSync(
    path.join(project, 'Cargo.toml'),
    '\n[dependencies]\ntree-sitter-fixture = { package = "ocentra-tree-sitter-fixture", version = "0.1.0", path = "vendor/tree-sitter-fixture" }\n',
    'utf8',
  );
  const vendor = path.join(project, 'vendor', 'tree-sitter-fixture');
  fs.mkdirSync(path.join(vendor, 'src'), { recursive: true });
  fs.writeFileSync(
    path.join(vendor, 'Cargo.toml'),
    '[package]\nname = "ocentra-tree-sitter-fixture"\nversion = "0.1.0"\nedition = "2021"\nrust-version = "1.75"\npublish = false\n[lib]\npath = "bindings/rust/lib.rs"\n',
    'utf8',
  );
  fs.mkdirSync(path.join(vendor, 'bindings', 'rust'), { recursive: true });
  fs.writeFileSync(path.join(vendor, 'bindings', 'rust', 'lib.rs'), 'pub struct ParserLanguage;\n', 'utf8');
  fs.writeFileSync(path.join(vendor, 'src', 'parser.c'), 'int parser_fixture(void) { return 0; }\n', 'utf8');

  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-9\.3/u, output);
});

test('manifest comments do not create loose-version or native-dependency findings', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct MemoryGraph;\n',
  });
  fs.appendFileSync(
    path.join(project, 'Cargo.toml'),
    '\n[dependencies]\n# tree-sitter = ">=0.21.0"\ntree-sitter-cmake = "0.7.0"\n',
    'utf8',
  );

  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-9\.16/u, output);
  assert.doesNotMatch(output, /RR-9\.21/u, output);
});

test('exact native dependency still requires native justification', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct MemoryGraph;\n',
  });
  fs.appendFileSync(path.join(project, 'Cargo.toml'), '\n[dependencies]\ncmake = "0.1.57"\n', 'utf8');
  expectFailure(project, 'RR-9.21');
});

test('vendored parser with nested git dependency still fails RR-9.3', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct MemoryGraph;\n',
  });
  fs.appendFileSync(
    path.join(project, 'Cargo.toml'),
    '\n[dependencies]\ntree-sitter-fixture-local = { path = "vendor/tree-sitter-fixture-local" }\n',
    'utf8',
  );
  const vendor = path.join(project, 'vendor', 'tree-sitter-fixture-local');
  fs.mkdirSync(path.join(vendor, 'src'), { recursive: true });
  fs.writeFileSync(
    path.join(vendor, 'Cargo.toml'),
    '[package]\nname = "tree-sitter-fixture-local"\nversion = "0.1.0"\nedition = "2021"\nrust-version = "1.75"\npublish = false\n[dependencies]\nunsafe-parser = { git = "https://example.invalid/parser" }\n',
    'utf8',
  );
  fs.writeFileSync(path.join(vendor, 'src', 'lib.rs'), 'pub struct ParserLanguage;\n', 'utf8');
  fs.writeFileSync(path.join(vendor, 'src', 'parser.c'), 'int parser_fixture(void) { return 0; }\n', 'utf8');

  expectFailure(project, 'RR-9.3');
});

test('Cargo loose versions, copyleft licenses, and build dependencies fail scanner', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct UserId;\n',
  });
  fs.appendFileSync(
    path.join(project, 'Cargo.toml'),
    '\nlicense = "AGPL-3.0"\n[dependencies]\nserde = ">=1"\n[build-dependencies]\ncc = "1.0.0"\n',
    'utf8',
  );
  expectFailures(project, ['RR-9.16', 'RR-9.22', 'RR-9.30']);
});

test('file scope scans only requested Rust file', () => {
  const project = makeProject({
    'src/good.rs': `
use core::num::NonZeroU64;
/// BRAND-INVARIANT: non-zero issued value.
#[derive(Debug)]
pub struct UserId(NonZeroU64);
pub fn load_user(id: UserId) -> Option<UserId> { Some(id) }
`,
    'src/bad.rs': `
pub fn load_user(id: &str) -> Option<&str> { Some(id) }
`,
  });
  const result = runGateArgs(project, ['scan', '--files', 'src/good.rs']);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('file scope checks only owning Cargo manifests', () => {
  const project = makeProject({
    'Cargo.toml': `
[workspace]
members = ["crates/good", "crates/bad"]

[workspace.package]
rust-version = "1.75"
`,
    'crates/good/Cargo.toml': `
[package]
name = "good-crate"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
`,
    'crates/good/OWNERS': '@ocentra/enforcer\n',
    'crates/good/src/lib.rs': `
use core::num::NonZeroU64;
/// BRAND-INVARIANT: non-zero issued value.
#[derive(Debug)]
pub struct UserId(NonZeroU64);
pub fn load_user(id: UserId) -> Option<UserId> { Some(id) }
`,
    'crates/bad/Cargo.toml': `
[package]
name = "bad-crate"
version = "0.1.0"
edition = "2021"
`,
    'crates/bad/src/lib.rs': `
pub fn load_user(id: &str) -> Option<&str> { Some(id) }
`,
  });
  refreshLock(project);
  const result = runGateArgs(project, ['scan', '--files', 'crates/good/src/lib.rs']);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('crate scope scans selected package by Cargo package name', () => {
  const project = makeProject({
    'Cargo.toml': `
[workspace]
members = ["crates/good", "crates/bad"]

[workspace.package]
rust-version = "1.75"
`,
    'crates/good/Cargo.toml': `
[package]
name = "good-crate"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
`,
    'crates/good/OWNERS': '@ocentra/enforcer\n',
    'crates/good/src/lib.rs': `
use core::num::NonZeroU64;
/// BRAND-INVARIANT: non-zero issued value.
#[derive(Debug)]
pub struct UserId(NonZeroU64);
pub fn load_user(id: UserId) -> Option<UserId> { Some(id) }
`,
    'crates/bad/Cargo.toml': `
[package]
name = "bad-crate"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
`,
    'crates/bad/src/lib.rs': `
pub fn load_user(id: &str) -> Option<&str> { Some(id) }
`,
  });
  refreshLock(project);
  const result = runGateArgs(project, ['scan', '--crate', 'good-crate']);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});
