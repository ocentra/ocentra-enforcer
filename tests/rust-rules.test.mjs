import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { spawnCli } from './cli-spawn.mjs';

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
  fs.writeFileSync(path.join(dir, 'deny.toml'), '[advisories]\nyanked = "deny"\nunmaintained = "deny"\n', 'utf8');
  fs.writeFileSync(path.join(dir, 'OWNERS'), '@ocentra/enforcer\n', 'utf8');
  for (const [rel, content] of Object.entries(files)) {
    const full = path.join(dir, rel);
    fs.mkdirSync(path.dirname(full), { recursive: true });
    fs.writeFileSync(full, content.trimStart(), 'utf8');
  }
  return dir;
}

function runGate(project) {
  return spawnCli(process.execPath, [SCRIPT, 'scan', '--root', project], {
    encoding: 'utf8',
  });
}

function runGateArgs(project, args) {
  return spawnCli(process.execPath, [SCRIPT, ...args, '--root', project], {
    encoding: 'utf8',
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

function expectFailures(project, ruleIds) {
  const result = runGate(project);
  assert.notEqual(result.status, 0, `expected gate to fail for ${ruleIds.join(', ')}`);
  const output = `${result.stdout}\n${result.stderr}`;
  for (const ruleId of ruleIds) {
    assert.match(output, new RegExp(ruleId.replace('.', '\\.'), 'u'), `expected output to contain ${ruleId}. Output:\n${output}`);
  }
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
#[derive(Debug)]
pub struct UserId(NonZeroU64);

/// User record.
#[derive(Debug)]
pub struct UserRecord {
    id: UserId,
}

/// Lookup failure.
#[derive(Debug, thiserror::Error)]
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
#[derive(Debug)]
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
    'rust-rules.config.json': JSON.stringify({
      schemaVersion: 2,
      profileName: 'strict',
      publicReexportPolicy: 'facade-only',
    }),
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

test('Cargo loose versions, copyleft licenses, stale lockfiles, and build dependencies fail scanner', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct UserId;\n',
  });
  fs.appendFileSync(
    path.join(project, 'Cargo.toml'),
    '\nlicense = "AGPL-3.0"\n[dependencies]\nserde = ">=1"\n[build-dependencies]\ncc = "1.0.0"\n',
    'utf8',
  );
  const manifest = path.join(project, 'Cargo.toml');
  const lockfile = path.join(project, 'Cargo.lock');
  const oldTime = new Date(Date.now() - 60_000);
  fs.utimesSync(lockfile, oldTime, oldTime);
  const newTime = new Date();
  fs.utimesSync(manifest, newTime, newTime);
  expectFailures(project, ['RR-9.16', 'RR-9.22', 'RR-9.25', 'RR-9.30']);
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

test('stringly Rust errors and swallowed results fail scanner', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId;

pub fn parse_user(raw: &str) -> Result<UserId, String> {
    let _ = raw.parse::<u64>();
    raw.parse::<u64>()
        .map_err(|e| e.to_string())
        .ok()
        .unwrap_or_default();
    Err("bad user")
}

pub fn parse_group(raw: &str) -> Result<UserId, &'static str> {
    Err(format!("bad {raw}"))
}

pub fn save_user(raw: &str) -> bool {
    raw.parse::<u64>().is_ok()
}

pub fn find_user(raw: &str) -> i32 {
    if raw.is_empty() {
        return -1;
    }
    1
}

pub struct ParsedUserId(u64);

impl ParsedUserId {
    pub fn new(raw: String) -> Self {
        Self(raw.parse::<u64>().unwrap_or(0))
    }
}

pub enum ParseError {
    Io(std::io::Error),
}

pub fn read_user(raw: &str) -> Result<UserId, ParseError> {
    let value = raw.parse::<u64>().unwrap_or(0);
    error!("failed {value}");
    Err(ParseError::Io(std::io::Error::other("bad")))
}

fn main() {
    let _ = run();
}

fn run() -> Result<(), ParseError> {
    Ok(())
}
`,
  });
  expectFailures(project, [
    'RR-4.7',
    'RR-4.8',
    'RR-4.9',
    'RR-4.10',
    'RR-4.11',
    'RR-4.12',
    'RR-4.13',
    'RR-4.14',
    'RR-4.15',
    'RR-4.16',
    'RR-4.17',
    'RR-4.18',
    'RR-4.19',
    'RR-4.20',
    'RR-4.21',
    'RR-4.22',
  ]);
});

test('dangerous Rust source primitives fail scanner', () => {
  const project = makeProject({
    'src/lib.rs': `
pub struct UserId(u64);
static mut READY: bool = false;
use core::cell::UnsafeCell;
use core::mem::{ManuallyDrop, MaybeUninit};

extern "C" {
    fn foreign();
}

#[no_mangle]
pub extern "C" fn exported() {}

pub fn cast_user(raw: u64) -> UserId {
    let ptr = &raw as *const u64;
    let _slot: MaybeUninit<u64> = MaybeUninit::uninit();
    let _manual = ManuallyDrop::new(raw);
    let _cell = UnsafeCell::new(raw);
    let _leaked = Box::leak(Box::new(raw));
    core::mem::forget(_manual);
    let _unchecked = [raw].get_unchecked(0);
    unsafe { core::mem::transmute::<u64, UserId>(*ptr) }
}

unsafe impl Send for UserId {}
`,
    'src/ffi/api.rs': `
pub struct RawFfi {
    pub value: u64,
}
`,
    'src/unsafe_escape_test.rs': `
#[allow(unsafe_code)]
pub fn test_escape(ptr: *const u64) -> u64 {
    unsafe { *ptr }
}
`,
  });
  expectFailures(project, [
    'RR-3.16',
    'RR-3.17',
    'RR-3.18',
    'RR-3.19',
    'RR-3.20',
    'RR-3.21',
    'RR-3.22',
    'RR-3.23',
    'RR-3.24',
    'RR-3.25',
    'RR-3.26',
    'RR-3.27',
    'RR-3.28',
    'RR-3.32',
    'RR-3.33',
  ]);
});

test('domain generic string escape hatches and bool state clusters fail scanner', () => {
  const project = makeProject({
    'src/lib.rs': `
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::borrow::Cow;
use std::fmt::Display;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

pub struct UserId;
pub struct RawUserId(pub String);
pub struct NumericUserId(u64);
pub struct User;
pub enum LookupError { Missing }
type UserIds = Vec<UserId>;

pub struct WorkflowState {
    active: bool,
    pending: bool,
    failed: bool,
    enabled: Option<bool>,
    name: Option<String>,
    timeout: Duration,
    created_at: SystemTime,
    url: String,
    file_path: String,
    user_id: String,
    data: serde_json::Value,
    by_name: BTreeMap<String, User>,
}

#[derive(Debug)]
pub struct ApiToken(String);

impl RawUserId {
    pub fn new(width: u32, height: u32) -> Self {
        Self(width.checked_add(height).unwrap().to_string())
    }
}

pub fn load_user<T: AsRef<str>>(id: T) -> Result<User, LookupError> {
    let _ = id;
    Err(LookupError::Missing)
}

pub fn show_user(id: impl Display) -> Result<User, LookupError> {
    let _ = id;
    Err(LookupError::Missing)
}

pub fn borrow_user(id: Cow<'_, str>) -> Result<User, LookupError> {
    let _ = id;
    Err(LookupError::Missing)
}

pub fn rename_user<T: Into<String>>(name: T) -> Result<User, LookupError> {
    let _ = name;
    Err(LookupError::Missing)
}

pub fn load_many(ids: Vec<String>) -> Result<Vec<User>, LookupError> {
    let _ = ids;
    Err(LookupError::Missing)
}

pub fn load_map(ids: HashMap<String, UserId>) -> Result<Vec<User>, LookupError> {
    let _ = ids;
    Err(LookupError::Missing)
}

pub fn load_sorted(map: BTreeMap<String, User>) -> Result<Vec<User>, LookupError> {
    let _ = map;
    Err(LookupError::Missing)
}

pub fn load_url(url: String, file_path: String, timeout: Duration) -> (UserId, User) {
    let _ = (url, file_path, timeout);
    (UserId, User)
}

pub fn share_state(state: Arc<Mutex<User>>) {
    let _ = state;
}
`,
  });
  expectFailures(project, [
    'RR-6.27',
    'RR-6.28',
    'RR-6.29',
    'RR-6.30',
    'RR-6.31',
    'RR-6.32',
    'RR-6.33',
    'RR-6.34',
    'RR-6.35',
    'RR-6.36',
    'RR-6.37',
    'RR-6.38',
    'RR-6.39',
    'RR-6.40',
    'RR-6.41',
    'RR-6.42',
    'RR-6.43',
    'RR-6.45',
    'RR-6.46',
    'RR-6.47',
    'RR-6.48',
    'RR-6.49',
    'RR-6.51',
    'RR-8.30',
  ]);
});

test('async runtime fire-and-forget and unbounded channels fail scanner', () => {
  const project = makeProject({
    'src/lib.rs': `
use std::sync::{Arc, Mutex};

pub async fn run_worker() {
    let state = std::sync::Mutex::new(1_u64);
    let _ = std::fs::read("state.txt");
    tokio::spawn(async move {});
    let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel::<u64>();
    client.send().await;
    loop {
        process().await;
    }
}

pub fn runtime() {
    let _runtime = tokio::runtime::Runtime::new();
    futures::executor::block_on(async {});
}

pub fn share(state: Arc<Mutex<u64>>) {
    let _ = state;
}
`,
    'src/timing_test.rs': `
#[test]
fn waits() {
    std::thread::sleep(std::time::Duration::from_millis(1));
}
`,
  });
  expectFailures(project, [
    'RR-8.16',
    'RR-8.18',
    'RR-8.19',
    'RR-8.20',
    'RR-8.21',
    'RR-8.23',
    'RR-8.25',
    'RR-8.27',
    'RR-8.28',
    'RR-8.29',
    'RR-8.30',
  ]);
});

test('Rust serde domain derives and weak assertions fail scanner', () => {
  const rustAssertMacro = 'assert';
  const project = makeProject({
    'src/lib.rs': `
#[derive(Deserialize)]
#[serde(untagged)]
pub enum UserEnvelope {
    Named { name: String },
}

pub fn verify(result: Result<u64, ()>, value: Option<u64>) {
    ${rustAssertMacro}!(result.is_ok());
    ${rustAssertMacro}!(value.is_some());
}
`,
  });
  expectFailures(project, ['RR-12.22', 'RR-12.23', 'RR-14.16', 'RR-14.18']);
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

test('CLI --profile resolves pack-owned profile config', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct UserId;\n',
  });
  const result = runGateArgs(project, ['doctor', '--json', '--profile', 'ocentra-parent', '--files', 'src/lib.rs']);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const report = JSON.parse(result.stdout);
  assert.equal(report.profileName, 'ocentra-parent');
});

test('CLI auto-loads target config when no profile or config is explicit', () => {
  const project = makeProject({
    'src/lib.rs': 'pub struct UserId;\n',
    'ocentra-enforcer.config.json': JSON.stringify({
      schemaVersion: 2,
      profileName: 'target-project',
      enforceWorkspaceFiles: false,
      requireCargoDeny: false,
      rustRoots: ['src'],
    }),
  });
  const result = runGateArgs(project, ['doctor', '--json', '--files', 'src/lib.rs']);
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const report = JSON.parse(result.stdout);
  assert.equal(report.profileName, 'target-project');
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
      '.gitignore',
      '.mcp.json',
      'ocentra-enforcer.config.json',
    ]
  );
  assert.equal(fs.existsSync(path.join(project, 'ocentra-enforcer.config.json')), false);
  assert.equal(report.files.some((file) => file.path === '.husky/pre-commit'), false);
});

test('ocentra-enforcer codex install dry-run reports target and global MCP plan without writing', () => {
  const project = fs.mkdtempSync(path.join(os.tmpdir(), 'ocentra-enforcer-codex-dry-'));
  const codexConfig = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'ocentra-enforcer-codex-home-')), 'config.toml');
  fs.writeFileSync(codexConfig, 'model = "gpt-test"\n', 'utf8');

  const result = spawnSync(
    process.execPath,
    [
      SCRIPT,
      'codex',
      'install',
      '--dry-run',
      '--json',
      '--root',
      project,
      '--profile',
      'strict',
      '--codex-config',
      codexConfig,
    ],
    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }
  );

  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const report = JSON.parse(result.stdout);
  assert.equal(report.command, 'codex-install');
  assert.equal(report.dryRun, true);
  assert.equal(report.codexMcp.changed, true);
  assert.equal(report.codexMcp.skillChanged, true);
  assert.equal(report.codexMcp.globalAgentsChanged, true);
  assert.match(report.codexMcp.block, /\[mcp_servers\.ocentra-enforcer\]/u);
  assert.equal(report.codexMcp.ledgerRoot, path.join(ROOT, '.ledger'));
  assert.match(report.codexMcp.block, /OCENTRA_LEDGER_HOME/u);
  assert.match(report.codexMcp.globalAgentsBlock, /Ledger root:/u);
  assert.equal(fs.readFileSync(codexConfig, 'utf8'), 'model = "gpt-test"\n');
  assert.equal(fs.existsSync(path.join(path.dirname(codexConfig), 'AGENTS.md')), false);
  assert.equal(fs.existsSync(path.join(project, '.mcp.json')), false);
});

test('ocentra-enforcer codex install supports global-only setup without target wiring', () => {
  const codexRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'ocentra-enforcer-codex-global-'));
  const ledgerRoot = path.join(codexRoot, 'ledger-home');
  const codexConfig = path.join(codexRoot, 'config.toml');
  fs.writeFileSync(codexConfig, 'model = "gpt-test"\n', 'utf8');

  const result = spawnSync(
    process.execPath,
    [SCRIPT, 'codex', 'install', '--json', '--codex-config', codexConfig, '--ledger-root', ledgerRoot],
    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }
  );

  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const report = JSON.parse(result.stdout);
  assert.equal(report.command, 'codex-install');
  assert.equal(report.root, null);
  assert.equal(report.target, null);
  assert.equal(report.codexMcp.applied, true);
  assert.match(fs.readFileSync(codexConfig, 'utf8'), /\[mcp_servers\.ocentra-enforcer\]/u);
  assert.match(fs.readFileSync(codexConfig, 'utf8'), /OCENTRA_LEDGER_HOME/u);
  assert.equal(fs.existsSync(path.join(ledgerRoot, '.gitignore')), true);
  assert.equal(fs.existsSync(path.join(codexRoot, 'skills', 'ocentra-enforcer', 'SKILL.md')), true);
  assert.match(fs.readFileSync(path.join(codexRoot, 'AGENTS.md'), 'utf8'), /Coordination is a Codex\/harness concern/u);
});

test('ocentra-enforcer codex install writes target wiring and global Codex MCP config idempotently', () => {
  const project = fs.mkdtempSync(path.join(os.tmpdir(), 'ocentra-enforcer-codex-write-'));
  const codexRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'ocentra-enforcer-codex-config-'));
  const ledgerRoot = path.join(codexRoot, 'ledger-home');
  const codexConfig = path.join(codexRoot, 'config.toml');
  fs.writeFileSync(codexConfig, 'model = "gpt-test"\n\n[mcp_servers.existing]\ncommand = "node"\n', 'utf8');

  const args = [
    SCRIPT,
    'codex',
    'install',
    '--json',
    '--root',
    project,
    '--profile',
    'strict',
    '--codex-config',
    codexConfig,
    '--ledger-root',
    ledgerRoot,
  ];
  const result = spawnSync(process.execPath, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });

  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  const report = JSON.parse(result.stdout);
  assert.equal(report.codexMcp.changed, true);
  assert.equal(fs.existsSync(report.codexMcp.backupPath), true);
  assert.equal(fs.existsSync(path.join(project, '.mcp.json')), true);
  assert.equal(fs.existsSync(path.join(project, '.codex', 'skills', 'ocentra-enforcer', 'SKILL.md')), true);
  assert.equal(fs.existsSync(path.join(codexRoot, 'skills', 'ocentra-enforcer', 'SKILL.md')), true);
  assert.match(fs.readFileSync(codexConfig, 'utf8'), /\[mcp_servers\.ocentra-enforcer\]/u);
  assert.match(fs.readFileSync(codexConfig, 'utf8'), /mcp\/rust-rules-mcp\.mjs/u);
  assert.match(fs.readFileSync(codexConfig, 'utf8'), /OCENTRA_LEDGER_HOME/u);
  assert.equal(fs.existsSync(path.join(ledgerRoot, '.gitignore')), true);
  assert.match(fs.readFileSync(path.join(codexRoot, 'AGENTS.md'), 'utf8'), /<!-- ocentra-enforcer:start -->/u);

  const second = spawnSync(process.execPath, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  assert.equal(second.status, 0, `${second.stdout}\n${second.stderr}`);
  const secondReport = JSON.parse(second.stdout);
  assert.equal(secondReport.codexMcp.changed, false);
  assert.equal(secondReport.codexMcp.skillChanged, false);
  assert.equal(secondReport.codexMcp.globalAgentsChanged, false);

  const doctor = spawnSync(
    process.execPath,
    [SCRIPT, 'codex', 'doctor', '--json', '--root', project, '--codex-config', codexConfig],
    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }
  );
  assert.equal(doctor.status, 0, `${doctor.stdout}\n${doctor.stderr}`);
  const doctorReport = JSON.parse(doctor.stdout);
  assert.equal(doctorReport.command, 'codex-doctor');
  assert.equal(doctorReport.ok, true);
  assert.equal(doctorReport.checks.find((check) => check.name === 'codex mcp section').ok, true);
  assert.equal(doctorReport.checks.find((check) => check.name === 'user enforcer skill').ok, true);
  assert.equal(doctorReport.checks.find((check) => check.name === 'global AGENTS enforcer block').ok, true);
  assert.equal(doctorReport.checks.find((check) => check.name === 'target .mcp.json server path').ok, true);

  const uninstall = spawnSync(
    process.execPath,
    [SCRIPT, 'codex', 'uninstall', '--json', '--codex-config', codexConfig],
    { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }
  );
  assert.equal(uninstall.status, 0, `${uninstall.stdout}\n${uninstall.stderr}`);
  const uninstallReport = JSON.parse(uninstall.stdout);
  assert.equal(uninstallReport.command, 'codex-uninstall');
  assert.equal(uninstallReport.applied, true);
  assert.doesNotMatch(fs.readFileSync(codexConfig, 'utf8'), /\[mcp_servers\.ocentra-enforcer\]/u);
  assert.equal(fs.existsSync(path.join(codexRoot, 'skills', 'ocentra-enforcer', 'SKILL.md')), false);
  assert.doesNotMatch(fs.readFileSync(path.join(codexRoot, 'AGENTS.md'), 'utf8'), /<!-- ocentra-enforcer:start -->/u);
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

test('expanded Rust hardening rules emit deterministic CLI JSON violations', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      rustRoots: ['src', 'tests', 'crates'],
    }),
    'src/lib.rs': `
#[no_mangle]
pub extern "C" fn exported() {}

pub unsafe fn dereference(ptr: *const u8) -> u8 {
    *ptr
}

pub struct AccountId(pub String);
pub struct DomainRecord {
    pub value: String,
}
pub struct ApiSecret(String);

async fn async_ping() {}
fn compute(value: u8) -> u8 { value }

pub async fn lock_then_await(lock: std::sync::Mutex<u8>) {
    let _guard = lock.lock().unwrap(); async_ping().await;
}

pub async fn retry_without_policy(values: Vec<u8>) {
    let retry = retry_without_policy_counter();
    while retry < 3 {}
    tokio::select! { _ = async_ping() => {} }
    for item in values { compute(item); }
    while compute(1) > 0 { async_ping().await; }
}

fn retry_without_policy_counter() -> u8 { 0 }

pub fn parse_user(raw: &str) -> Result<AccountId, Error> {
    let _ = serde_json::from_str::<DomainState>(raw);
    let _: String = base64_token(raw);
    let base64: String = raw.to_owned();
    let _ = base64;
    Err(Error)
}

pub fn parse(raw: &str) -> Result<AccountId, Error> {
    let _ = raw;
    Err(Error)
}

pub fn parse_packet(raw: &[u8]) -> Result<AccountId, Error> {
    let packet = raw;
    let _ = packet;
    Err(Error)
}

pub struct Error;
pub struct DomainState;
pub fn base64_token(raw: &str) -> String { raw.to_owned() }

pub struct UserDto {
    pub id: String,
}

impl TryFrom<UserDto> for AccountId {
    type Error = Error;
    fn try_from(value: UserDto) -> Result<Self, Self::Error> {
        let _ = value;
        Err(Error)
    }
}

// BUGFIX: regression marker intentionally lacks evidence.
pub fn fixed_path() {}

#[derive(Serialize)]
pub struct SerializedState {
    #[serde(default)]
    pub count: u64,
    #[serde(flatten)]
    pub extra: String,
}
`,
    'src/no_conversion_dto.rs': `
pub struct MissingDto {
    pub id: String,
}
`,
    'src/api/boundary/api.rs': `
#[derive(Serialize, Deserialize)]
pub struct BoundaryPayload {
    pub id: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub enum BoundaryEvent {
    Started,
}
`,
    'src/domain/model.rs': `
use crate::transport::UserDto;

pub struct DomainModel;
`,
    'src/tests/expanded_rules.rs': `
#[test]
#[should_panic]
fn panic_contract_missing() {
    panic!("boom");
}

#[test]
fn empty_contract() {}

#[test]
fn construction_only() {
    let _ = AccountId::new("abc");
}

#[test]
fn volatile_snapshot() {
    insta::assert!("2026-01-01 random");
}

#[test]
fn weak_result_assertions() {
    assert!(Some(Ok::<u8, ()>(1)).unwrap().is_ok());
    assert!(Some(1).is_some());
}
`,
    'crates/workspace-helper/Cargo.toml': `
[package]
name = "workspace-helper"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
`,
    'crates/workspace-helper/src/lib.rs': 'pub struct Helper;\n',
  });
  fs.appendFileSync(
    path.join(project, 'Cargo.toml'),
    `

[workspace]
members = ["crates/workspace-helper"]

[dependencies]
tokio = { version = "1.40.0" }
syn = "2.0.0"
openssl = "0.10.0"
criterion = "0.5.1"
workspace-helper = "0.1.0"
serde = "1.0.0"

[target.'cfg(unix)'.dependencies]
serde = "1.0.1"

[dev-dependencies]
criterion = "0.5.1"
`,
    'utf8',
  );
  fs.writeFileSync(path.join(project, 'deny.toml'), '[advisories]\n', 'utf8');

  const result = runGateArgs(project, ['scan', '--json']);
  assert.notEqual(result.status, 0, result.stdout || result.stderr);
  const report = JSON.parse(result.stdout);
  const actualIds = new Set(report.violations.map((violation) => violation.ruleId));
  const expectedIds = [
    'RR-3.29',
    'RR-3.30',
    'RR-3.31',
    'RR-6.44',
    'RR-6.50',
    'RR-6.52',
    'RR-8.17',
    'RR-8.22',
    'RR-8.24',
    'RR-8.26',
    'RR-9.17',
    'RR-9.18',
    'RR-9.19',
    'RR-9.20',
    'RR-9.21',
    'RR-9.23',
    'RR-9.24',
    'RR-9.26',
    'RR-9.27',
    'RR-9.28',
    'RR-9.29',
    'RR-12.16',
    'RR-12.17',
    'RR-12.18',
    'RR-12.19',
    'RR-12.20',
    'RR-12.21',
    'RR-12.24',
    'RR-12.25',
    'RR-12.26',
    'RR-12.27',
    'RR-12.28',
    'RR-12.29',
    'RR-12.30',
    'RR-14.17',
    'RR-14.19',
    'RR-14.20',
    'RR-14.21',
    'RR-14.22',
    'RR-14.23',
    'RR-14.24',
    'RR-14.25',
    'RR-14.26',
    'RR-14.27',
    'RR-14.28',
    'RR-14.29',
    'RR-14.30',
  ];
  const missingIds = expectedIds.filter((ruleId) => !actualIds.has(ruleId));
  for (const ruleId of expectedIds) {
    assert.equal(actualIds.has(ruleId), true, `${ruleId} emitted; missing=${missingIds.join(', ')} actual=${[...actualIds].sort().join(', ')}`);
  }
  for (const violation of report.violations.filter((violation) => expectedIds.includes(violation.ruleId))) {
    assert.equal(typeof violation.file, 'string');
    assert.equal(typeof violation.line, 'number');
    assert.equal(typeof violation.detail, 'string');
    assert.equal(typeof violation.doc, 'string');
    assert.equal(typeof violation.snippet, 'string');
  }
});
