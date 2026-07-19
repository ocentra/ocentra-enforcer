import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnCli } from './cli-spawn.mjs';
import { DEFAULT_CONFIG } from '../src/rule-metadata.mjs';
import { normalizeConfig, runScanner } from '../scripts/rust-rules-scan-core.mjs';

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

function expectNoRule(project, ruleId) {
  const result = runGate(project);
  assert.doesNotMatch(`${result.stdout}\n${result.stderr}`, new RegExp(ruleId.replace('.', '\\.'), 'u'));
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

test('finite async reads and bounded test routing do not require cancellation evidence', () => {
  const project = makeProject({
    'src/read.rs': `
pub async fn read_once() -> Result<(), ()> {
    Ok(())
}
`,
    'tests/routing.rs': `
#[tokio::test]
async fn routes_a_typed_variant_through_a_bounded_channel() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel::<u8>(1);
    sender.send(7).await.expect("receiver remains available during this finite test");
    assert_eq!(receiver.recv().await, Some(7));
}
`,
  });
  const result = runGate(project);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.doesNotMatch(output, /RR-12\.29/u, output);
});

test('background tasks still require cancellation evidence', () => {
  const project = makeProject({
    'src/worker.rs': `
pub async fn start_background_work() {
    tokio::spawn(async move {});
}
`,
  });
  expectFailure(project, 'RR-12.29');
});

test('async loops still require cancellation evidence', () => {
  const project = makeProject({
    'src/worker.rs': `
pub async fn drain_forever() {
    loop {
        tokio::task::yield_now().await;
    }
}
`,
  });
  expectFailure(project, 'RR-12.29');
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

test('record-field rules ignore function parameters and expressions but retain record failures', () => {
  const passing = makeProject({
    'src/lib.rs': `
use std::path::PathBuf;
pub struct EmptyRecord {}
pub fn label(title: &str) -> PathBuf { PathBuf::from(title) }
`,
  });
  expectNoRule(passing, 'RR-6.4');
  const failing = makeProject({ 'src/lib.rs': 'pub struct Record {\n    title: String,\n}\n' });
  expectFailure(failing, 'RR-6.4');
});

test('unsafe evidence ignores quoted diagnostics but retains real unsafe code', () => {
  const passing = makeProject({ 'src/lib.rs': 'pub const MESSAGE: &str = "unsafe operation";\n' });
  expectNoRule(passing, 'RR-3.30');
  const failing = makeProject({ 'src/lib.rs': 'pub unsafe fn raw() {}\n' });
  expectFailures(failing, ['RR-3.30', 'RR-3.31', 'RR-12.30']);
});

test('test-structure rules use balanced masked bodies', () => {
  const passing = makeProject({
    'tests/fixture.rs': `
#[test]
fn fixture_text_is_not_a_test() {
    let fixture = r#"#[test]\nfn fake() {}"#;
    let bytes = b"fn byte_fixture() { 1 }";
    assert!(!fixture.is_empty());
    assert!(!bytes.is_empty());
}

#[test]
fn following_test_remains_visible() {
    assert_eq!(2 + 2, 4);
}
`,
  });
  expectNoRule(passing, 'RR-12.24');
  const failing = makeProject({ 'src/lib.rs': '#[test]\nfn empty() {}\n' });
  expectFailure(failing, 'RR-12.24');
});

test('construction-only tests accept delegated proof helpers but still reject construction alone', () => {
  const delegatedProof = makeProject({
    'src/lib.rs': `
#[test]
fn validator_fixture_has_behavioral_proof() -> Result<(), ()> {
    let validator = Validator::new()?;
    run_fixture_parity(&validator)?;
    Ok(())
}
`,
  });
  expectNoRule(delegatedProof, 'RR-12.25');

  const constructionOnly = makeProject({
    'src/lib.rs': '#[test]\nfn only_constructs() { let _value = Validator::new(); }\n',
  });
  expectFailure(constructionOnly, 'RR-12.25');
});

test('constructor and property evidence bind to local definitions and registered property targets', () => {
  const externalOnly = makeProject({ 'src/lib.rs': 'pub fn use_external(input: &str) { let _ = input.parse::<u64>(); }\n' });
  expectNoRule(externalOnly, 'RR-12.16');
  const propertyCovered = makeProject({
    'src/lib.rs': 'pub fn parse_item(input: &str) -> Result<(), ()> { let _ = input; Ok(()) }\n',
    'tests/property_parser_contracts.rs': 'proptest! { #[test] fn parser_contract(input in ".*") { let _ = parse_item(&input); } }\n',
  });
  expectNoRule(propertyCovered, 'RR-12.27');
  const missing = makeProject({ 'src/lib.rs': 'pub fn try_new(value: String) -> Result<(), ()> { let _ = value; Ok(()) }\n' });
  expectFailure(missing, 'RR-12.16');
});

test('parser rejection evidence is crate-wide and remains target-specific', () => {
  const covered = makeProject({
    'src/lib.rs': 'pub fn parse_lesson(input: &str) -> Result<(), ()> { if input.is_empty() { Err(()) } else { Ok(()) } }\n',
    'tests/lesson.rs': '#[test]\nfn rejects_invalid_lesson() { assert!(parse_lesson("invalid").is_err()); }\n',
  });
  expectNoRule(covered, 'RR-12.16');
  expectNoRule(covered, 'RR-12.17');

  const partiallyCovered = makeProject({
    'src/lib.rs': `
pub fn parse_one(input: &str) -> Result<(), ()> { let _ = input; Ok(()) }
pub fn parse_two(input: &str) -> Result<(), ()> { let _ = input; Ok(()) }
`,
    'tests/parser.rs': `
#[test]
fn rejects_invalid_parse_one() { assert!(parse_one("invalid").is_err()); }
proptest! { #[test] fn parse_one_property(input in ".*") { let _ = parse_one(&input); } }
`,
  });
  const result = runGate(partiallyCovered);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.notEqual(result.status, 0);
  assert.match(output, /RR-12\.16[\s\S]*parse_two/u);
  assert.match(output, /RR-12\.17[\s\S]*parse_two/u);
  assert.match(output, /RR-12\.27[\s\S]*parse_two/u);
  assert.doesNotMatch(output, /RR-12\.(?:16|17|27)[^\n]*parse_one lacks/u);
});

test('crate evidence cache refreshes after an external test changes in one process', () => {
  const project = makeProject({
    'src/lib.rs': 'pub fn parse_lesson(input: &str) -> Result<(), ()> { let _ = input; Ok(()) }\n',
    'tests/lesson.rs': '#[test]\nfn lesson_smoke() { let _ = parse_lesson("valid"); }\n',
  });
  const sourcePath = path.join(project, 'src', 'lib.rs');
  const config = normalizeConfig(DEFAULT_CONFIG);
  const scope = { mode: 'files', files: [sourcePath] };
  const initial = runScanner(project, config, scope);
  assert.equal(initial.some((finding) => finding.ruleId === 'RR-12.16'), true);

  fs.writeFileSync(
    path.join(project, 'tests', 'lesson.rs'),
    '#[test]\nfn rejects_invalid_lesson_input() { assert!(parse_lesson("invalid lesson input").is_err()); }\n',
    'utf8',
  );
  const refreshed = runScanner(project, config, scope);
  assert.equal(refreshed.some((finding) => finding.ruleId === 'RR-12.16'), false);
  assert.equal(refreshed.some((finding) => finding.ruleId === 'RR-12.17'), false);
});

test('fuzz evidence binds to the binary parser target and not sibling parsers', () => {
  const covered = makeProject({
    'src/lib.rs': 'pub fn parse_packet(input: &[u8]) -> Result<(), ()> { let _packet = input; Ok(()) }\n',
    'fuzz/fuzz_targets/packet.rs': 'fn fuzz_parse_packet(bytes: &[u8]) { let _ = parse_packet(bytes); }\n',
  });
  expectNoRule(covered, 'RR-12.28');

  const sibling = makeProject({
    'src/lib.rs': `
pub fn parse_packet(input: &[u8]) -> Result<(), ()> { let _packet = input; Ok(()) }
pub fn parse_label(input: &str) -> Result<(), ()> { let _label = input; Ok(()) }
`,
  });
  const result = runGate(sibling);
  const output = `${result.stdout}\n${result.stderr}`;
  assert.match(output, /RR-12\.28[\s\S]*parse_packet/u);
  assert.doesNotMatch(output, /RR-12\.28[^\n]*parse_label/u);
});

test('DTO mapper entry points satisfy conversion evidence while missing conversion fails', () => {
  const passing = makeProject({
    'src/boundary/artifact_transport.rs': 'pub struct ArtifactTransportDto;\npub struct ArtifactTransport;\nimpl ArtifactTransportDto { fn into_domain(self) -> ArtifactTransport { ArtifactTransport } }\n',
  });
  expectNoRule(passing, 'RR-14.23');
  const failing = makeProject({
    'src/boundary/artifact_transport.rs': 'pub struct ArtifactTransportDto;\npub struct ArtifactTransport;\n',
  });
  expectFailure(failing, 'RR-14.23');
});
