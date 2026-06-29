import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const SCRIPT = path.join(ROOT, 'scripts', 'rust-rules.mjs');

function makeProject(files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'rust-rules-'));
  fs.mkdirSync(path.join(dir, 'src'), { recursive: true });
  fs.writeFileSync(path.join(dir, 'Cargo.toml'), `
[package]
name = "fixture"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
`, 'utf8');
  fs.writeFileSync(path.join(dir, 'Cargo.lock'), '', 'utf8');
  fs.writeFileSync(path.join(dir, 'rust-toolchain.toml'), '[toolchain]\nchannel = "1.75.0"\ncomponents = ["rustfmt", "clippy"]\n', 'utf8');
  fs.writeFileSync(path.join(dir, 'clippy.toml'), '# test fixture\n', 'utf8');
  fs.writeFileSync(path.join(dir, 'deny.toml'), '# test fixture\n', 'utf8');
  for (const [rel, content] of Object.entries(files)) {
    const full = path.join(dir, rel);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content.trimStart(), 'utf8');
  }
  return dir;
}

function runGate(project) {
  return spawnSync(process.execPath, [SCRIPT, 'scan', '--root', project], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function runGateArgs(project, args) {
  return spawnSync(process.execPath, [SCRIPT, ...args, '--root', project], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function expectFailure(project, ruleId) {
  const result = runGate(project);
  assert.notEqual(result.status, 0, `expected gate to fail for ${ruleId}`);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, new RegExp(ruleId.replace('.', '\\.'), 'u'), `expected output to contain ${ruleId}. Output:\n${output}`);
  assert.match(output, /Reason:/u, 'failure output must contain a reason');
  assert.match(output, /Fix:/u, 'failure output must contain a fix snippet');
  assert.match(output, /rules\/rust\//u, 'failure output must point at indexed rules doc');
}

test('good branded-domain fixture passes scanner', () => {
  const project = makeProject({
    'src/lib.rs': `
#![forbid(unsafe_code)]
#![deny(warnings)]

use core::num::NonZeroU64;

/// User identifier.
/// BRAND-INVARIANT: the inner value is non-zero and issued by the identity service.
pub struct UserId(NonZeroU64);

/// User record.
pub struct UserRecord {
    id: UserId,
}

/// Lookup failure.
pub enum LookupError {
    /// The user does not exist.
    NotFound,
}

/// Finds a user by branded identifier.
pub fn find_user(id: UserId) -> Result<Option<UserRecord>, LookupError> {
    let _ = id;
    Ok(None)
}
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('unwrap fails with RR-4.1 and helpful output', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId;
pub fn load_user(id: UserId) -> Option<UserId> {
    Some(id).unwrap()
}
`,
  });
  expectFailure(project, 'RR-4.1');
});

test('raw string parameter fails with RR-6.1', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId;
pub fn load_user(id: &str) -> Option<UserId> {
    let _ = id;
    None
}
`,
  });
  expectFailure(project, 'RR-6.1');
});

test('raw primitive parameter fails with RR-6.2', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId;
pub fn load_user(id: u64) -> Option<UserId> {
    let _ = id;
    None
}
`,
  });
  expectFailure(project, 'RR-6.2');
});

test('clone without justification fails with RR-5.1', () => {
  const project = makeProject({
    'src/lib.rs': `
use core::num::NonZeroU64;
/// BRAND-INVARIANT: non-zero issued value.
pub struct UserId(NonZeroU64);
impl Clone for UserId {
    fn clone(&self) -> Self { Self(self.0.clone()) }
}
`,
  });
  expectFailure(project, 'RR-5.1');
});

test('clone with justification passes clone policy', () => {
  const project = makeProject({
    'src/lib.rs': `
use core::num::NonZeroU64;
/// BRAND-INVARIANT: non-zero issued value.
pub struct UserId(NonZeroU64);
impl Clone for UserId {
    fn clone(&self) -> Self {
        // CLONE-JUSTIFICATION: NonZeroU64 is copy-like and no ownership aliasing is introduced.
        Self(self.0.clone())
    }
}
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('unsafe fails with RR-3.1', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId;
pub fn load_user(id: UserId) -> Option<UserId> {
    unsafe { core::hint::unreachable_unchecked() }
}
`,
  });
  expectFailure(project, 'RR-3.1');
});

test('wildcard import fails with RR-7.1', () => {
  const project = makeProject({
    'src/lib.rs': `
use crate::domain::*;
mod domain { pub struct UserId; }
pub struct UserRecord;
`,
  });
  expectFailure(project, 'RR-7.1');
});

test('pub use outside facade fails with RR-7.3', () => {
  const project = makeProject({
    'src/domain/mod.rs': `
pub use crate::other::UserRecord;
`,
    'src/lib.rs': `
mod other { pub struct UserRecord; }
pub mod domain;
`,
  });
  expectFailure(project, 'RR-7.3');
});

test('pub use fails even in facade when profile forbids public re-exports', () => {
  const project = makeProject({
    'src/lib.rs': `
mod domain { pub struct UserRecord; }
pub use domain::UserRecord;
`,
  });
  expectFailure(project, 'RR-7.3');
});

test('facade-only profile allows public re-export in configured facade file', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({ publicReexportPolicy: 'facade-only' }),
    'src/lib.rs': `
mod domain { pub struct UserRecord; }
pub use domain::UserRecord;
`,
  });
  const result = runGate(project);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('lint allow suppression fails with RR-2.1', () => {
  const project = makeProject({
    'src/lib.rs': `
#![allow(dead_code)]
pub struct UserId;
`,
  });
  expectFailure(project, 'RR-2.1');
});

test('Cargo wildcard dependency fails with RR-9.1', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct UserId;\n',
  });
  fs.appendFileSync(path.join(project, 'Cargo.toml'), '\n[dependencies]\nserde = "*"\n', 'utf8');
  expectFailure(project, 'RR-9.1');
});

test('file scope scans only requested Rust file', () => {
  const project = makeProject({
    'src/good.rs': `
use core::num::NonZeroU64;
/// BRAND-INVARIANT: non-zero issued value.
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
    'crates/good/src/lib.rs': `
use core::num::NonZeroU64;
/// BRAND-INVARIANT: non-zero issued value.
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
    'crates/good/src/lib.rs': `
use core::num::NonZeroU64;
/// BRAND-INVARIANT: non-zero issued value.
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
  const result = runGateArgs(project, ['scan', '--crate', 'good-crate']);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
});

test('runtime inline string guard can be enabled from config', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      enforceRuntimeStringLiterals: true,
      rawTypeBoundaryGlobs: ['src/lib.rs'],
    }),
    'src/lib.rs': `
pub fn route_name() -> &'static str { "devices" }
`,
  });
  expectFailure(project, 'RR-18.16');
});

test('serialized raw identity fields fail when Ocentra-style guard is enabled', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      enforceSerializedPublicDomainPrimitives: true,
      rawTypeBoundaryGlobs: ['src/lib.rs'],
    }),
    'src/lib.rs': `
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub event_id: String,
}
`,
  });
  expectFailure(project, 'RR-6.26');
});

test('doctor reports usable scope', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({ requireCargoDeny: false }),
    'src/lib.rs': 'pub struct UserId;\n',
  });
  const result = runGateArgs(project, ['doctor', '--workspace']);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  assert.match(result.stdout, /PASS scope files/u);
});

test('ocentra-enforcer init dry-run reports exact adapter file plan without writing', () => {
  const project = fs.mkdtempSync(path.join(os.tmpdir(), 'ocentra-enforcer-init-'));
  const result = spawnSync(
    process.execPath,
    [
      SCRIPT,
      'init',
      '--dry-run',
      '--json',
      '--root',
      project,
      '--profile',
      'strict',
      '--adapters',
      'codex,mcp,precommit,github-actions',
    ],
    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }
  );
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const report = JSON.parse(result.stdout);
  assert.equal(report.productName, 'ocentra-enforcer');
  assert.equal(report.dryRun, true);
  assert.deepEqual(
    report.files.map((file) => file.path).sort(),
    [
      '.codex/skills/ocentra-enforcer/SKILL.md',
      '.git/hooks/pre-commit',
      '.github/workflows/codeql.yml',
      '.github/workflows/dependency-policy.yml',
      '.github/workflows/ocentra-enforcer.yml',
      '.github/workflows/sbom.yml',
      '.github/workflows/secret-scan.yml',
      '.mcp.json',
      'ocentra-enforcer.config.json',
    ]
  );
  assert.equal(fs.existsSync(path.join(project, 'ocentra-enforcer.config.json')), false);
  assert.equal(report.files.some((file) => file.path === '.husky/pre-commit'), false);
});

test('ocentra-enforcer init includes Husky only when requested', () => {
  const project = fs.mkdtempSync(path.join(os.tmpdir(), 'ocentra-enforcer-husky-'));
  const result = spawnSync(
    process.execPath,
    [SCRIPT, 'init', '--dry-run', '--json', '--root', project, '--profile', 'strict', '--adapters', 'precommit,husky'],
    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }
  );
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const report = JSON.parse(result.stdout);
  assert.equal(report.files.some((file) => file.path === '.git/hooks/pre-commit'), true);
  assert.equal(report.files.some((file) => file.path === '.husky/pre-commit'), true);
});

test('adapter templates cover POSIX pre-commit, GitHub Actions, CodeQL, dependency policy, secret scan, and SBOM', () => {
  const hook = fs.readFileSync(path.join(ROOT, 'adapters', 'git-hooks', 'pre-commit.sh'), 'utf8');
  assert.match(hook, /^#!\/bin\/sh/u);
  assert.doesNotMatch(hook, /\[\[|declare -a|function\s+[A-Za-z_]/u);

  const workflowNames = [
    'ocentra-enforcer.yml',
    'codeql.yml',
    'dependency-policy.yml',
    'secret-scan.yml',
    'sbom.yml',
  ];
  for (const workflowName of workflowNames) {
    assert.equal(fs.existsSync(path.join(ROOT, 'adapters', 'github-actions', workflowName)), true);
  }
  assert.match(fs.readFileSync(path.join(ROOT, 'adapters', 'github-actions', 'codeql.yml'), 'utf8'), /github\/codeql-action/u);
  assert.match(fs.readFileSync(path.join(ROOT, 'adapters', 'github-actions', 'dependency-policy.yml'), 'utf8'), /cargo-audit/u);
  assert.match(fs.readFileSync(path.join(ROOT, 'adapters', 'github-actions', 'secret-scan.yml'), 'utf8'), /gitleaks/u);
  assert.match(fs.readFileSync(path.join(ROOT, 'adapters', 'github-actions', 'sbom.yml'), 'utf8'), /sbom-action/u);
});
