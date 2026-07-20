import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { checkStagedRatchet } from '../scripts/precommit-ratchet.mjs';
import { maskRustCode } from '../scripts/rust-rules-path-core.mjs';
import { makeProject, runGateArgs } from './rust-rules-install-fixture.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const SCRIPT = path.join(ROOT, 'scripts', 'rust-rules.mjs');

function git(project, args) {
  const result = spawnSync('git', args, { cwd: project, encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  return result.stdout;
}

function commitFixture(project) {
  git(project, ['init']);
  git(project, ['config', 'user.email', 'fixtures@example.invalid']);
  git(project, ['config', 'user.name', 'Fixture']);
  git(project, ['add', '.']);
  git(project, ['commit', '-m', 'baseline']);
}

test('boundary DTO evidence recognizes underscore-separated Rust test names', () => {
  const project = makeProject({
    'src/boundary/dto.rs': `
      // BOUNDARY-INVARIANT: this module owns the external DTO wire shape.
      #[derive(serde::Serialize, serde::Deserialize)]
      pub struct InputDto { pub value: String }

      pub struct DomainValue;

      impl TryFrom<InputDto> for DomainValue {
        type Error = ();

        fn try_from(dto: InputDto) -> Result<Self, Self::Error> {
          if dto.value.is_empty() { Err(()) } else { Ok(Self) }
        }
      }

      #[cfg(test)]
      mod tests {
        #[test]
        fn wire_round_trip_is_preserved() {
          let value = InputDto { value: "valid".to_owned() };
          let encoded = serde_json::to_vec(&value).unwrap();
          let decoded: InputDto = serde_json::from_slice(&encoded).unwrap();
          assert_eq!(decoded.value, value.value);
        }

        #[test]
        fn invalid_payload_is_rejected() {
          let result = DomainValue::try_from(InputDto { value: String::new() });
          assert!(result.is_err());
        }
      }
    `,
  });
  const result = runGateArgs(project, ['scan', '--files', 'src/boundary/dto.rs', '--json']);
  const report = JSON.parse(result.stdout);
  const evidenceRules = new Set(report.violations.map((violation) => violation.ruleId));
  assert.equal(evidenceRules.has('RR-12.18'), false, result.stdout);
  assert.equal(evidenceRules.has('RR-14.25'), false, result.stdout);
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
  assert.match(fs.readFileSync(codexConfig, 'utf8'), /mcp\/ocentra-enforcer-mcp\.mjs/u);
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
  assert.match(hook, /precommit-ratchet\.mjs/u);

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

test('precommit ratchet permits staged hard-finding reductions and rejects regressions', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({ requireCargoDeny: false }),
    'src/lib.rs': 'fn value(input: String) { let _copy = input.clone(); }\n',
  });
  commitFixture(project);

  fs.writeFileSync(path.join(project, 'src', 'lib.rs'), 'fn value(input: String) { let _ = input; }\n');
  git(project, ['add', 'src/lib.rs']);
  const reduction = checkStagedRatchet({ root: project, enforcerRoot: ROOT });
  assert.equal(reduction.ok, true, JSON.stringify(reduction));

  git(project, ['commit', '-m', 'remove clone debt']);
  fs.writeFileSync(path.join(project, 'src', 'lib.rs'), 'fn value(input: String) { let _copy = input.clone(); }\n');
  git(project, ['add', 'src/lib.rs']);
  const regression = checkStagedRatchet({ root: project, enforcerRoot: ROOT });
  assert.equal(regression.ok, false, JSON.stringify(regression));
  assert.equal(regression.increased.length > 0, true);
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
    loop { async_ping().await; }
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
pub struct Missing;

#[derive(Deserialize)]
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

test('Rust domain signature classifier distinguishes owned conversion APIs from raw domain leaks', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      requireCargoDeny: false,
      rustRoots: ['src'],
    }),
    'src/canonical.rs': `
#[doc = "BRAND-INVARIANT: every usize is a valid exact zero-inclusive count."]
pub struct ZeroCount(usize);

impl From<usize> for ZeroCount {
    fn from(value: usize) -> Self { Self(value) }
}

impl PartialEq<usize> for ZeroCount {
    fn eq(&self, other: &usize) -> bool { self.0 == *other }
}

impl std::ops::Add<usize> for ZeroCount {
    type Output = usize;
    fn add(self, rhs: usize) -> Self::Output { self.0 + rhs }
}

impl ZeroCount {
    pub const fn get(self) -> usize { self.0 }
    pub const fn is_zero(self) -> bool { self.0 == 0 }
    pub const fn should_emit(self, configured_min: Self) -> bool {
        self.0 >= configured_min.0
    }
}

pub struct Bytes(Vec<u8>);
impl<const N: usize> PartialEq<&[u8; N]> for Bytes {
    fn eq(&self, other: &&[u8; N]) -> bool { self.0.as_slice() == *other }
}

#[doc = "BRAND-INVARIANT: values from zero through one hundred are valid percentages."]
pub struct Percentage(u8);

impl Percentage {
    pub fn new(value: u8) -> Result<Self, &'static str> {
        if value <= 100 { Ok(Self(value)) } else { Err("invalid percentage") }
    }

    pub const fn value(self) -> u8 { self.0 }
}

#[doc = "BRAND-INVARIANT: the string has already passed label validation."]
pub struct Label(String);

impl Label {
    pub fn try_new(value: String) -> Result<Self, &'static str> {
        if value.is_empty() { Err("invalid label") } else { Ok(Self(value)) }
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::ops::Deref for Label {
    type Target = str;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[doc = "BRAND-INVARIANT: empty text is valid report output; the wrapper preserves presentation ownership."]
pub struct RenderedReport(String);
impl From<String> for RenderedReport {
    fn from(value: String) -> Self { Self(value) }
}

#[doc = "BRAND-INVARIANT: the boolean stores one named domain state."]
pub struct ReadyState(bool);
impl ReadyState {
    pub const fn get(self) -> bool { self.0 }
}
`,
    'src/raw_domain.rs': `
pub type RawCount = usize;

pub struct InvalidCount(usize);
impl From<usize> for InvalidCount {
    fn from(value: usize) -> Self { Self(value) }
}

#[doc = "BRAND-INVARIANT: every usize is a valid exact zero-inclusive count."]
pub struct WrongRawConversion(usize);
impl From<u32> for WrongRawConversion {
    fn from(value: u32) -> Self { Self(value as usize) }
}

#[doc = "BRAND-INVARIANT: labels are required to be non-empty."]
pub struct InvalidLabel(String);
impl From<String> for InvalidLabel {
    fn from(value: String) -> Self { Self(value) }
}

#[doc = "BRAND-INVARIANT: labels are required to be non-empty."]
pub struct PublicInvalidLabel(pub String);

pub struct DomainService;
impl DomainService {
    pub fn increment_by(&mut self, amount: u32) { let _ = amount; }
    pub fn raw_total(&self) -> u64 { 0 }
    pub fn get_ready(&self, key: &str) -> bool { !key.is_empty() }
}

pub trait RawDomainContract {
    fn replace_count(&mut self, count: u32);
}
`,
  });

  const result = runGateArgs(project, [
    'scan',
    '--json',
    '--languages',
    'rust',
    '--files',
    'src/canonical.rs',
    'src/raw_domain.rs',
  ]);
  const report = JSON.parse(result.stdout);
  const canonicalBoundaryFindings = report.violations.filter(
    (violation) =>
      violation.file === 'src/canonical.rs'
      && ['RR-4.12', 'RR-6.1', 'RR-6.2', 'RR-6.5', 'RR-6.44'].includes(violation.ruleId),
  );
  assert.deepEqual(canonicalBoundaryFindings, [], result.stdout);

  const rawDomainIds = new Set(
    report.violations
      .filter((violation) => violation.file === 'src/raw_domain.rs')
      .map((violation) => violation.ruleId),
  );
  assert.equal(rawDomainIds.has('RR-6.2'), true, result.stdout);
  assert.equal(rawDomainIds.has('RR-6.5'), true, result.stdout);
  assert.equal(rawDomainIds.has('RR-6.44'), true, result.stdout);
  assert.equal(rawDomainIds.has('RR-4.12'), true, result.stdout);
  assert.equal(
    report.violations.some(
      (violation) => violation.ruleId === 'RR-6.44' && violation.detail.includes('InvalidLabel'),
    ),
    true,
    result.stdout,
  );
  assert.equal(
    report.violations.some(
      (violation) =>
        violation.ruleId === 'RR-6.44'
        && violation.detail.includes('PublicInvalidLabel'),
    ),
    true,
    result.stdout,
  );
  assert.equal(
    report.violations.some(
      (violation) => violation.ruleId === 'RR-6.44' && violation.detail.includes('WrongRawConversion'),
    ),
    true,
    result.stdout,
  );
});

test('Rust masking preserves code after byte character literals and masks comments and strings', () => {
  const source = `
const ASCII_A: u8 = b'a';
// impl From<usize> for CommentOnly {}
const TEXT: &str = "impl From<usize> for StringOnly {}";
impl From<usize> for TotalCount {}
`;
  const masked = maskRustCode(source);
  assert.match(masked, /impl From<usize> for TotalCount/u);
  assert.doesNotMatch(masked, /CommentOnly|StringOnly/u);
});

test('awaiting a Tokio lock acquisition is not reported as awaiting while holding a guard', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      requireCargoDeny: false,
      rustRoots: ['src'],
    }),
    'src/lib.rs': `
pub async fn increment(lock: tokio::sync::Mutex<u8>) {
    *lock.lock().await += 1;
}
`,
  });
  const result = runGateArgs(project, ['scan', '--json', '--files', 'src/lib.rs']);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some((violation) => violation.ruleId === 'RR-8.17'),
    false,
    result.stdout,
  );
});

test('Rust slice type declarations are not reported as unchecked indexing', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      requireCargoDeny: false,
      rustRoots: ['src'],
    }),
    'src/lib.rs': `
pub fn values<'a>() -> &'a [u8] { &[] }
pub fn names() -> &'static [&'static str] { &[] }
pub fn sort_paths(paths: &mut [TracedPath]) { paths.sort_by_key(|path| path.depth); }
pub fn normalize(vector: &mut [f32]) { vector.fill(0.0); }
pub fn read(buf: &mut [u8]) -> usize { buf.len() }
pub fn first(values: Vec<u8>) -> u8 { values[0] }
`,
  });
  const result = runGateArgs(project, ['scan', '--json', '--files', 'src/lib.rs']);
  const report = JSON.parse(result.stdout);
  const indexing = report.violations.filter((violation) => violation.ruleId === 'RR-5.3');
  assert.equal(indexing.length, 1, result.stdout);
  assert.match(indexing[0].source, /values\[0\]/u);
});

test('boundary serde classification is scoped to each adjacent struct attributes', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      requireCargoDeny: false,
      rustRoots: ['src'],
    }),
    'src/boundary.rs': `
#[derive(serde::Serialize, serde::Deserialize)]
pub struct EventDto {
    pub value: String,
}

#[derive(serde::Deserialize)]
pub struct EventWire {
    pub value: String,
}

pub struct NotWiredError {
    pub message: String,
}

#[derive(serde::Serialize)]
pub struct MissingSuffix {
    pub value: String,
}
`,
  });
  const result = runGateArgs(project, ['scan', '--json', '--files', 'src/boundary.rs']);
  const report = JSON.parse(result.stdout);
  const violations = report.violations.filter((violation) => violation.ruleId === 'RR-14.21');
  assert.equal(violations.length, 1, result.stdout);
  assert.match(violations[0].detail, /MissingSuffix/u);
});

test('base64 domain classification requires an executable raw-string shape', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      requireCargoDeny: false,
      rustRoots: ['src'],
    }),
    'src/redaction.rs': `
/// Redacts base64-looking secrets before persistence.
pub struct Redactor {
    pattern: String,
}
`,
    'src/domain.rs': `
pub struct UnsafePayload {
    pub payload_base64: String,
}
`,
  });
  const result = runGateArgs(project, ['scan', '--json', '--files', 'src/redaction.rs', 'src/domain.rs']);
  const report = JSON.parse(result.stdout);
  const violations = report.violations.filter((violation) => violation.ruleId === 'RR-14.28');
  assert.equal(violations.length, 1, result.stdout);
  assert.equal(violations[0].file, 'src/domain.rs');
  assert.match(violations[0].source, /payload_base64/u);
});

test('workspace dependency users inherit the root dependency justification', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      requireCargoDeny: false,
      rustRoots: ['src', 'crates'],
    }),
    'crates/member/Cargo.toml': `
[package]
name = "member"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[dependencies]
serde = { workspace = true }
`,
    'crates/member/src/lib.rs': 'pub struct Member;\n',
  });
  fs.appendFileSync(
    path.join(project, 'Cargo.toml'),
    `

[workspace]
members = ["crates/member"]

[workspace.dependencies]
# DEPENDENCY-JUSTIFICATION: shared serialization contract for workspace records.
serde = { version = "1.0.228", features = ["derive"] }
`,
    'utf8',
  );
  const result = runGateArgs(project, ['scan', '--json', '--files', 'crates/member/Cargo.toml']);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some((violation) => violation.ruleId === 'RR-9.18'),
    false,
    result.stdout,
  );
});

test('a substantive contiguous Cargo comment is dependency justification', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      requireCargoDeny: false,
      rustRoots: ['src'],
    }),
    'Cargo.toml': `
[package]
name = "fixture"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[dependencies]
# This client performs the optional local model download and JSON protocol
# boundary, so it is deliberately opt-in rather than part of the default build.
reqwest = { version = "0.12", default-features = false, optional = true }
`,
    'src/lib.rs': 'pub struct Fixture;\n',
  });
  const result = runGateArgs(project, ['scan', '--json', '--files', 'Cargo.toml']);
  const report = JSON.parse(result.stdout);
  assert.equal(
    report.violations.some((violation) => violation.ruleId === 'RR-9.18'),
    false,
    result.stdout,
  );
});

test('erased test-harness errors are not treated as application-domain errors', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      requireCargoDeny: false,
      rustRoots: ['src', 'tests'],
    }),
    'src/lib.rs': `
pub fn production() -> Result<(), Box<dyn std::error::Error>> { Ok(()) }

#[test]
fn fixture() -> Result<(), Box<dyn std::error::Error>> {
    let value = "fixture".to_string();
    let copied = value.clone();
    assert_eq!(copied, "fixture");
    Ok(())
}
`,
  });
  const result = runGateArgs(project, ['scan', '--json', '--files', 'src/lib.rs']);
  const report = JSON.parse(result.stdout);
  const productionOnlyRules = new Set(['RR-4.4', 'RR-5.1', 'RR-5.2']);
  const productionOnlyFindings = report.violations.filter((violation) =>
    productionOnlyRules.has(violation.ruleId),
  );
  assert.equal(productionOnlyFindings.length, 1, result.stdout);
  assert.equal(productionOnlyFindings[0].ruleId, 'RR-4.4');
  assert.equal(productionOnlyFindings[0].file, 'src/lib.rs');
});

test('clone and allocation policy applies to core code but not an owned transport boundary', () => {
  const project = makeProject({
    'rust-rules.config.json': JSON.stringify({
      requireCargoDeny: false,
      rustRoots: ['src'],
      rawTypeBoundaryGlobs: ['src/transport.rs'],
      boundaryOwnerNote: 'Transport owns wire-format strings and graph-record allocation.',
    }),
    'src/core.rs': `
pub fn core() {
    let value = "core".to_string();
    let _copied = value.clone();
}
`,
    'src/transport.rs': `
pub fn encode() {
    let value = "wire".to_string();
    let _copied = value.clone();
}
`,
  });
  const result = runGateArgs(project, ['scan', '--json', '--files', 'src/core.rs', 'src/transport.rs']);
  const report = JSON.parse(result.stdout);
  const findings = report.violations.filter((violation) =>
    new Set(['RR-5.1', 'RR-5.2']).has(violation.ruleId),
  );
  assert.equal(findings.length, 2, result.stdout);
  assert.equal(findings.every((violation) => violation.file === 'src/core.rs'), true);
});
