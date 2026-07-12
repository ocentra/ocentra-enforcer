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

function runGateArgs(project, args) {
  return spawnCli(process.execPath, [SCRIPT, ...args, '--root', project], {
    encoding: 'utf8',
  });
}

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
